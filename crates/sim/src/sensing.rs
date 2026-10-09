//! Optical sensing: which enemies each unit's own eyes identify this tick.
//! Rays run from the observer's eye to the target's visibility samples (a
//! vehicle's hull centre and top, each living soldier). Solid geometry blocks;
//! foliage attenuates reach continuously and blocks outright past a full run;
//! forest ground conceals a target by class, independently of tree crowns.
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
    if !observer.alive() || !target.alive() {
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

/// How far sight of directional `range` reaches through foliage of `depth`
/// (Q21): attenuated continuously, `None` once the foliage blocks outright.
pub fn foliage_reach(range: f64, depth: f64, s: &SensorRules) -> Option<f64> {
    if depth >= s.foliage_full_block {
        None
    } else if depth == 0.0 {
        Some(range)
    } else {
        Some(range * (-depth).exp())
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

/// A visibility sample: the member it belongs to (none for a hull), where
/// it is, and the concealment its ground and building give it.
type Sample = (Option<usize>, V3, f64);

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

/// Detection multiplier for a target sample: the strongest concealment of
/// its forest ground and, for a garrisoned squad, its building.
fn target_concealment(world: &WorldGeometry, target: &Unit, at: V3, rules: &Rules) -> f64 {
    let s = &rules.sensors;
    let shelter = crate::garrison::shelter(target, rules);
    world
        .forest_concealment(target.hull.is_none(), at.x, at.y)
        .min(1.0 + (s.building_range_multiplier - 1.0) * shelter)
}

/// Whether enough of the living unit benefits from concealment to qualify for HIDDEN.
pub fn concealed(world: &WorldGeometry, target: &Unit, rules: &Rules) -> bool {
    if target.hull.is_some() {
        target_concealment(world, target, target.position, rules) < 1.0
    } else {
        let mut living = 0;
        let mut hidden = 0;
        for at in target.member_positions() {
            living += 1;
            hidden += usize::from(target_concealment(world, target, at, rules) < 1.0);
        }
        living > 0 && hidden as f64 > living as f64 * rules.sensors.squad_hidden_fraction
    }
}

/// Sensor rules the geometry relies on (the fog sweep stops a ray once its
/// reach has shrunk behind it).
pub fn validate(s: &SensorRules) {
    assert!(s.squad_identification_fraction > 0.0 && s.squad_identification_fraction <= 1.0);
    assert!((0.0..1.0).contains(&s.squad_hidden_fraction));
    assert!(
        s.foliage_full_block > 0.0,
        "sensors.foliage_full_block must be positive"
    );
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
        .filter(|u| u.side != side && u.alive())
        .map(|u| {
            let samples = samples(u)
                .into_iter()
                .map(|(member, at)| (member, at, target_concealment(world, u, at, rules)))
                .collect();
            (u, u.footprint_radius(body), samples)
        })
        .collect();
    for observer in units
        .iter()
        .filter(|u| u.side == side && u.alive() && due(u))
    {
        let from = eyes(observer, rules);
        let sight = crate::sight::of(observer, rules);
        let concealed_range_multiplier =
            observer.unit_type(rules).sensors.concealed_range_multiplier;
        let observer_radius = observer.footprint_radius(body);
        for (target, target_radius, samples) in &targets {
            let (target, target_radius) = (*target, *target_radius);
            // Every eye and sample lies within `spread` of the two centres, so
            // no sample can be seen past the widest reach across that arc.
            let spread = observer_radius + target_radius;
            let to = target.position - observer.position;
            let distance = to.xy().length();
            let arc = if distance > spread {
                (spread / distance).asin()
            } else {
                std::f64::consts::PI
            };
            if to.length() > sight.reach_within(libm::atan2(to.y, to.x), arc) + spread {
                continue;
            }
            let mut seen = Vec::new();
            let mut any = false;
            for &(member, at, sheltered) in samples {
                let concealment = (sheltered * concealed_range_multiplier).min(1.0);
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
