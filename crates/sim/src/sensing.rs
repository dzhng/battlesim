//! Optical sensing: which enemies each unit's own eyes identify this tick.
//! Rays run from the observer's eye to the target's visibility samples (a
//! vehicle's hull centre and top, each living soldier). Solid geometry blocks;
//! foliage attenuates reach continuously and blocks outright past a full run;
//! the foliage a target stands under conceals it by class (Q21).
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
/// once the observer or target has fallen, or every soldier it saw has.
pub fn kept(s: &Sighting, units: &[Unit]) -> Option<Sighting> {
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
    (!members.is_empty()).then(|| Sighting {
        members,
        ..s.clone()
    })
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

/// Detection-range multiplier for a target standing at `at`: 1 in the open
/// or on cleared ground, its foliage's class multiplier under trees (Q21).
pub fn concealment_multiplier(infantry: bool, world: &WorldGeometry, at: V3) -> f64 {
    world.foliage_at(at.x, at.y).concealment(infantry)
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
    let range = sight.range_at(to.y.atan2(to.x));
    let distance = to.length();
    if distance > range * concealment {
        return false; // cheap reject before any ray
    }
    let foliage = world.foliage_depth(eye, target);
    foliage_reach(range * concealment, foliage, s)
        .is_some_and(|reach| distance <= reach && world.sight_clear(eye, target))
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

/// Detection multiplier for a target sample: the strongest concealment of
/// its forest ground and, for a garrisoned squad, its building.
fn target_concealment(world: &WorldGeometry, target: &Unit, at: V3, rules: &Rules) -> f64 {
    let s = &rules.sensors;
    let shelter = crate::garrison::shelter(target, rules);
    concealment_multiplier(target.hull.is_none(), world, at)
        .min(1.0 + (s.building_range_multiplier - 1.0) * shelter)
}

/// Sensor rules the geometry relies on (the fog sweep stops a ray once its
/// reach has shrunk behind it).
pub fn validate(s: &SensorRules) {
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
    let targets: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side != side && u.alive())
        .collect();
    for observer in units
        .iter()
        .filter(|u| u.side == side && u.alive() && due(u))
    {
        let from = eyes(observer, rules);
        let sight = crate::sight::of(observer, rules);
        for &target in &targets {
            // Every eye and sample lies within `spread` of the two centres, so
            // no sample can be seen past the widest reach across that arc.
            let body = rules.physics.soldier_radius_m;
            let spread = observer.footprint_radius(body) + target.footprint_radius(body);
            let to = target.position - observer.position;
            let distance = to.xy().length();
            let arc = if distance > spread {
                (spread / distance).asin()
            } else {
                std::f64::consts::PI
            };
            if to.length() > sight.reach_within(to.y.atan2(to.x), arc) + spread {
                continue;
            }
            let mut seen = Vec::new();
            let mut any = false;
            for (member, at) in samples(target) {
                let concealment = target_concealment(world, target, at, rules);
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
            if any {
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
