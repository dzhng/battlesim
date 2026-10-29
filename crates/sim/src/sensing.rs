//! Optical sensing: which enemies each unit's own eyes identify this tick.
//! Rays run from the observer's eye to the target's visibility samples (a
//! vehicle's hull centre and top, each living soldier). Solid geometry blocks;
//! foliage attenuates reach continuously and blocks outright past a full run;
//! forest ground composes with authored unit concealment, independently of tree crowns.
use contract::catalog::AltitudeLayer;
use contract::ids::{Side, UnitId};
use contract::scenario::{Rules, SensorRules};

use crate::math::V3;
use crate::sight::Sight;
use crate::units::Unit;
use crate::world::WorldGeometry;

/// Height of an infantry visibility sample above the soldier's feet.
const SOLDIER_SAMPLE_M: f64 = 1.0;

/// Each observer runs its line-of-sight identification every this many
/// ticks, staggered by unit id so the load is even (27 perf): between runs
/// its last sightings stand. The side's knowledge still folds them in every
/// tick ([`crate::knowledge::SideKnowledge::update`]), so a track's position,
/// `last_seen` and grace, and everything fire, flight and damage read from
/// them, stay per tick; only gaining or losing sight of an enemy lags, by at
/// most `SENSE_EVERY - 1` ticks.
pub const SENSE_EVERY: u64 = 2;

/// Whether `observer` identifies afresh on `tick`.
pub fn due(observer: &Unit, tick: u64) -> bool {
    (tick + observer.id.0 as u64).is_multiple_of(SENSE_EVERY)
}

/// A sighting kept from an observer's last run, as it stands now: `None`
/// once the observer or target has fallen, or too few seen soldiers survive.
pub fn kept(s: &Sighting, units: &[Unit], rules: &Rules) -> Option<Sighting> {
    let (observer, target) = (&units[s.observer.0 as usize], &units[s.target.0 as usize]);
    if !observer.on_field() || !target.on_field() {
        return None;
    }
    if s.members.is_empty() {
        return Some(s.clone());
    }
    let members: Vec<usize> = s
        .members
        .iter()
        .copied()
        .filter(|&k| target.members[k].alive())
        .collect();
    squad_identified(target, members.len(), rules).then(|| Sighting {
        members,
        ..s.clone()
    })
}

fn squad_identified(target: &Unit, seen: usize, rules: &Rules) -> bool {
    let living = target.members.iter().filter(|m| m.alive()).count();
    living > 0 && seen as f64 >= living as f64 * rules.sensors.squad_identification_fraction
}

/// One observer identifying one target this tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Sighting {
    pub observer: UnitId,
    pub target: UnitId,
    /// Indices of the target's members seen (empty for vehicles).
    pub members: Vec<usize>,
}

pub fn eye(unit: &Unit, rules: &Rules) -> V3 {
    let h = match unit.unit_type(rules).hull() {
        Some(hull) => hull.eye_m,
        None => rules.physics.infantry_eye_m,
    };
    unit.position + crate::math::v3(0.0, 0.0, h)
}

/// Where a unit's sight starts: its eye, or in a building one eye per
/// occupied facade (27 perf, see
/// [`crate::garrison::facade_eyes`]). The fog sweep, identification and the
/// published `UnitSight::eyes` the renderer's fog draws all read this one
/// list. There is no extra all-round roof sensor.
pub fn eyes(unit: &Unit, rules: &Rules) -> Vec<V3> {
    let facades = crate::garrison::facade_eyes(unit, rules);
    if facades.is_empty() {
        vec![eye(unit, rules)]
    } else {
        facades
    }
}

/// Whether `unit` sees the ground at all. High air never does: it flies
/// lowered for play but still as far above the battle as a real jet, so the
/// ground and its fog are out of its sight (air-to-air sight is unaffected).
pub fn sees_ground(unit: &Unit) -> bool {
    unit.layer() != AltitudeLayer::HighAir
}

/// How far sight of directional `range` reaches through foliage of `depth`
/// (Q21): attenuated continuously, `None` once the foliage blocks outright.
pub fn foliage_reach(range: f64, depth: f64, s: &SensorRules) -> Option<f64> {
    if depth >= s.foliage_full_block {
        None
    } else if depth == 0.0 {
        Some(range)
    } else {
        Some(range * libm::exp(-depth))
    }
}

/// Whether `eye` identifies a sample at `target` with `sight`'s reach toward it.
pub fn sees_point(
    world: &WorldGeometry,
    eye: V3,
    target: V3,
    sight: &Sight,
    concealment: f64,
    s: &SensorRules,
) -> bool {
    let to = target - eye;
    let range = sight.range_at(libm::atan2(to.y, to.x));
    let distance = to.length();
    if distance > range * concealment || !world.sight_clear(eye, target) {
        return false; // solid cover makes foliage integration irrelevant
    }
    let foliage = world.foliage_depth(eye, target);
    foliage_reach(range * concealment, foliage, s).is_some_and(|reach| distance <= reach)
}

/// Whether one of `side`'s living units sees a round in flight at
/// `position`, fired from `layer`: from its eyes, by its sight in that
/// layer, as a round is a target like any other (one fired from the ground
/// is out of sight of an observer that doesn't see the ground).
pub fn side_sees_round(
    world: &WorldGeometry,
    rules: &Rules,
    units: &[Unit],
    side: Side,
    position: V3,
    layer: AltitudeLayer,
) -> bool {
    units.iter().any(|u| {
        if u.side != side || !u.on_field() || (layer == AltitudeLayer::Ground && !sees_ground(u)) {
            return false;
        }
        let sight = crate::sight::of_layer(u, rules, layer);
        eyes(u, rules)
            .into_iter()
            .any(|eye| sees_point(world, eye, position, &sight, 1.0, &rules.sensors))
    })
}

/// A visibility sample: the member it belongs to (none for a hull), where
/// it is, and its net target concealment.
type Sample = (Option<usize>, V3, f64);

/// Observer-adjusted concealment, never extending full sight.
pub fn effective(target: f64, spotting: f64) -> f64 {
    if spotting >= 1.0 {
        (target * spotting).min(1.0)
    } else {
        libm::pow(target.min(1.0), 1.0 / spotting)
    }
}

/// Visibility samples of a unit, with the member index they belong to.
fn samples(unit: &Unit) -> Vec<(Option<usize>, V3)> {
    match unit.hull {
        Some(h) => vec![
            (None, unit.position + crate::math::v3(0.0, 0.0, h.z)),
            (None, unit.position + crate::math::v3(0.0, 0.0, 2.0 * h.z)),
        ],
        None => unit
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .map(|(k, _)| {
                (
                    Some(k),
                    unit.members[k].position + crate::math::v3(0.0, 0.0, SOLDIER_SAMPLE_M),
                )
            })
            .collect(),
    }
}

/// The strongest concealment of forest ground and garrison shelter.
pub fn ground(forest: f64, shelter: f64, s: &SensorRules) -> f64 {
    forest.min(1.0 + (s.building_range_multiplier - 1.0) * shelter)
}

/// Target concealment in evaluation order: unit × ground, then movement.
pub fn target(unit: f64, ground: f64, moving: bool, s: &SensorRules) -> f64 {
    (unit * ground) * if moving { s.moving_concealment } else { 1.0 }
}

/// The ground and net concealment of one sample. Aircraft fly above both.
fn sample(world: &WorldGeometry, u: &Unit, at: V3, rules: &Rules) -> (f64, f64) {
    if u.airborne() {
        return (1.0, 1.0);
    }
    let ground = ground(
        world.forest_concealment(at.x, at.y),
        crate::garrison::shelter(u),
        &rules.sensors,
    );
    (
        ground,
        target(
            u.unit_type(rules).concealment,
            ground,
            u.moving(),
            &rules.sensors,
        ),
    )
}

/// Whether enough samples have both ground and net concealment for HIDDEN.
pub fn concealed(world: &WorldGeometry, target: &Unit, rules: &Rules) -> bool {
    let samples = samples(target);
    let hidden = samples
        .iter()
        .filter(|(_, at)| {
            let (ground, target) = sample(world, target, *at, rules);
            ground < 1.0 && target < 1.0
        })
        .count();
    !samples.is_empty()
        && hidden as f64 > samples.len() as f64 * rules.sensors.squad_hidden_fraction
}

/// Every sighting by `side`'s units for which `due` holds, in observer then
/// target order.
pub fn evaluate(
    world: &WorldGeometry,
    units: &[Unit],
    rules: &Rules,
    side: Side,
    due: impl Fn(&Unit) -> bool,
) -> Vec<Sighting> {
    let s = &rules.sensors;
    let mut out = Vec::new();
    // The living enemy, once: fallen squads stay in the list all battle.
    let body = rules.physics.soldier_radius_m;
    // Each with its samples and their concealment, which no observer changes.
    let targets: Vec<(&Unit, f64, Vec<Sample>)> = units
        .iter()
        .filter(|u| u.side != side && u.on_field())
        .map(|u| {
            let samples = samples(u)
                .into_iter()
                .map(|(member, at)| (member, at, sample(world, u, at, rules).1))
                .collect();
            (u, u.footprint_radius(body), samples)
        })
        .collect();
    for observer in units
        .iter()
        .filter(|u| u.side == side && u.on_field() && due(u))
    {
        // An observer that doesn't see the ground still runs the same
        // optical pass for air targets, so air-to-air sensing remains
        // authoritative here.
        let sees_ground = sees_ground(observer);
        let from = eyes(observer, rules);
        let spotting = observer.unit_type(rules).sensors.spotting;
        let observer_radius = observer.footprint_radius(body);
        for (target, target_radius, samples) in &targets {
            let (target, target_radius) = (*target, *target_radius);
            let sight = crate::sight::of_layer(observer, rules, target.layer());
            if !sees_ground && target.layer() == AltitudeLayer::Ground {
                continue;
            }
            // Every eye and sample lies within `spread` of the two centres, so
            // no sample can be seen past the widest reach across that arc.
            let spread = observer_radius + target_radius;
            let to = target.position - observer.position;
            let distance = to.xy().length();
            let arc = if distance > spread {
                libm::asin(spread / distance)
            } else {
                std::f64::consts::PI
            };
            if to.length() > sight.reach_within(libm::atan2(to.y, to.x), arc) + spread {
                continue;
            }
            let mut seen = Vec::new();
            let mut any = false;
            for &(member, at, target_concealment) in samples {
                let concealment = effective(target_concealment, spotting);
                if from
                    .iter()
                    .any(|&eye| sees_point(world, eye, at, &sight, concealment, s))
                {
                    any = true;
                    if let Some(k) = member {
                        seen.push(k);
                    } else {
                        break; // one hull sample is enough
                    }
                }
            }
            if any && (target.hull.is_some() || squad_identified(target, seen.len(), rules)) {
                out.push(Sighting {
                    observer: observer.id,
                    target: target.id,
                    members: seen,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::effective;

    #[test]
    fn spotting_cancels_or_deepens_concealment_within_full_sight() {
        assert_eq!(effective(0.5, 3.0), 1.0, "good spotting caps at full sight");
        assert_eq!(
            effective(2.8, 0.8),
            1.0,
            "poor spotting also caps at full sight"
        );
        assert_eq!(
            effective(1.0, 0.8),
            1.0,
            "unconcealed targets stay unchanged"
        );
        assert!(effective(0.5, 0.8) < effective(0.5, 1.0));
        assert!(effective(0.5, 2.0) > effective(0.5, 1.0));
        assert_eq!(effective(0.25, 0.5), 0.0625, "poor spotting uses a power");
    }
}
