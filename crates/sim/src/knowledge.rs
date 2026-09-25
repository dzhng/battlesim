//! What each side knows of the enemy, built only from its own sensing. The
//! team shares identification; each own unit's own-sensor list stays separate.
use std::collections::BTreeMap;

use contract::ids::{Tick, UnitId};
use contract::observation::{IdentifiedUnit, ObservedTargetId};
use contract::scenario::Rules;

use crate::math::{v2, V2, V3};
use crate::sensing::Sighting;
use crate::units::Unit;

/// A side's standing knowledge of one enemy unit.
#[derive(Clone, Debug)]
pub struct Track {
    pub id: ObservedTargetId,
    pub last_seen: Tick,
    pub position: V3,
    pub yaw: f64,
    pub velocity: V2,
    /// Members seen at the last sighting (infantry).
    pub members: Vec<usize>,
}

#[derive(Default)]
pub struct SideKnowledge {
    /// Keyed by the authority's enemy id; the side only ever sees `Track::id`.
    tracks: BTreeMap<UnitId, Track>,
    next_id: u32,
    /// Own unit → enemies its own sensors identify this tick.
    own_sensors: BTreeMap<UnitId, Vec<UnitId>>,
}

impl SideKnowledge {
    /// Fold this tick's sightings in. A track lapses (its id is retired) once
    /// it has gone unseen for longer than the acquisition grace.
    pub fn update(&mut self, tick: Tick, sightings: &[Sighting], units: &[Unit], rules: &Rules) {
        let grace = (rules.sensors.acquisition_grace_s * rules.tick_hz as f64).round() as Tick;
        let dt = 1.0 / rules.tick_hz as f64;
        self.own_sensors.clear();
        let mut seen: BTreeMap<UnitId, Vec<usize>> = BTreeMap::new();
        for s in sightings {
            self.own_sensors
                .entry(s.observer)
                .or_default()
                .push(s.target);
            let members = seen.entry(s.target).or_default();
            members.extend(&s.members);
            members.sort_unstable();
            members.dedup();
        }
        for (target, members) in seen {
            let unit = &units[target.0 as usize];
            // Where the side saw it: a vehicle's hull, or the centroid of the
            // soldiers actually seen, never the hidden rest of a squad.
            let observed = if members.is_empty() {
                unit.position
            } else {
                let sum = members
                    .iter()
                    .map(|&k| unit.member_position(k))
                    .fold(crate::math::v3(0.0, 0.0, 0.0), |a, p| a + p);
                sum * (1.0 / members.len() as f64)
            };
            match self.tracks.get_mut(&target) {
                Some(track) if track.last_seen + grace >= tick => {
                    // Continuous or within grace: same handle, observed motion.
                    let elapsed = (tick - track.last_seen) as f64 * dt;
                    track.velocity = (observed.xy() - track.position.xy()) * (1.0 / elapsed);
                    track.last_seen = tick;
                    track.position = observed;
                    track.yaw = unit.yaw;
                    track.members = members;
                }
                _ => {
                    self.next_id += 1;
                    self.tracks.insert(
                        target,
                        Track {
                            id: ObservedTargetId(self.next_id),
                            last_seen: tick,
                            position: observed,
                            yaw: unit.yaw,
                            velocity: v2(0.0, 0.0),
                            members,
                        },
                    );
                }
            }
        }
        self.tracks.retain(|_, t| t.last_seen + grace >= tick);
    }

    /// Enemies identified this tick, with only what was observed.
    pub fn identified<'a>(
        &'a self,
        tick: Tick,
        units: &'a [Unit],
        rules: &'a Rules,
    ) -> impl Iterator<Item = IdentifiedUnit> + 'a {
        self.tracks
            .iter()
            .filter(move |(_, t)| t.last_seen == tick)
            .map(move |(target, t)| {
                let unit = &units[target.0 as usize];
                IdentifiedUnit {
                    id: t.id,
                    kind: unit.kind,
                    cost: crate::units::cost(unit.kind, rules),
                    position: [t.position.x, t.position.y, t.position.z],
                    yaw: t.yaw,
                    velocity: [t.velocity.x, t.velocity.y],
                    members: t
                        .members
                        .iter()
                        .map(|&k| {
                            let p = unit.member_position(k);
                            [p.x, p.y, p.z]
                        })
                        .collect(),
                }
            })
    }

    /// The observed handles `observer`'s own sensors identify this tick.
    pub fn own_sensor(&self, observer: UnitId) -> Vec<ObservedTargetId> {
        self.own_sensors
            .get(&observer)
            .into_iter()
            .flatten()
            .filter_map(|t| self.tracks.get(t).map(|track| track.id))
            .collect()
    }

    /// Fold knowledge state into a digest.
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.next_id as u64);
        for (target, t) in &self.tracks {
            d.u64(target.0 as u64).u64(t.id.0 as u64).u64(t.last_seen);
            d.f64(t.velocity.x).f64(t.velocity.y);
        }
    }
}
