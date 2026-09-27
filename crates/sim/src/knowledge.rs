//! What each side knows of the enemy, built only from its own sensing. The
//! team shares identification; each own unit's own-sensor list stays separate.
use std::collections::{BTreeMap, BTreeSet};

use contract::ids::{Tick, UnitId};
use contract::observation::{
    ApproximateContact, ContactId, ContactSource, IdentifiedUnit, ObservedTargetId,
};
use contract::scenario::Rules;

use crate::ground::{GroundLayer, KnownGround};
use crate::rng::Rng;

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

/// An uncertain area the side holds. `emitter` links it to its source
/// internally so re-identification can retire it; it is never exported.
#[derive(Clone, Debug)]
pub struct Contact {
    pub id: ContactId,
    pub source: ContactSource,
    pub center: V2,
    pub evidence_tick: Tick,
    pub expires_tick: Tick,
    pub(crate) emitter: UnitId,
}

pub struct SideKnowledge {
    /// Keyed by the authority's enemy id; the side only ever sees `Track::id`.
    tracks: BTreeMap<UnitId, Track>,
    next_id: u32,
    /// Own unit → enemies its own sensors identify this tick.
    own_sensors: BTreeMap<UnitId, Vec<UnitId>>,
    contacts: Vec<Contact>,
    next_contact: u32,
    /// Enemy shots heard of this tick: (shooter, where it stood).
    pending_fire: Vec<(UnitId, V2)>,
    /// Observation-uncertainty stream: where inside its area a contact is reported.
    rng: Rng,
    /// Enemies this side watched die: their attacks are complete (W17).
    destroyed: BTreeSet<UnitId>,
    /// Fallen soldiers (by soldier id) this side has seen; remembered for good.
    corpses: BTreeSet<u32>,
    /// The ground as this side last saw it.
    ground: KnownGround,
}

impl SideKnowledge {
    pub fn new(seed: u64, ground: &GroundLayer) -> Self {
        SideKnowledge {
            tracks: BTreeMap::new(),
            next_id: 0,
            own_sensors: BTreeMap::new(),
            contacts: Vec::new(),
            next_contact: 0,
            pending_fire: Vec::new(),
            rng: Rng::new(seed),
            destroyed: BTreeSet::new(),
            corpses: BTreeSet::new(),
            ground: KnownGround::new(ground),
        }
    }

    /// The ground as this side last saw it.
    pub fn ground(&self) -> &KnownGround {
        &self.ground
    }

    /// Learn the ground `fog` shows as it is now (the fog sweep's rule).
    pub fn learn_ground(
        &mut self,
        layer: &GroundLayer,
        fog: &contract::observation::VisibilityField,
    ) {
        self.ground.learn(layer, fog);
    }

    /// The side saw this enemy die: its track ends without a last-seen area.
    pub fn saw_destroyed(&mut self, unit: UnitId) {
        self.tracks.remove(&unit);
        self.contacts.retain(|c| c.emitter != unit);
        self.destroyed.insert(unit);
    }

    pub fn knows_destroyed(&self, unit: UnitId) -> bool {
        self.destroyed.contains(&unit)
    }

    pub fn note_corpse(&mut self, soldier: u32) {
        self.corpses.insert(soldier);
    }

    pub fn knows_corpse(&self, soldier: u32) -> bool {
        self.corpses.contains(&soldier)
    }

    /// An enemy fired: firing is disclosed map-wide, whatever the line of sight.
    pub fn note_fire(&mut self, shooter: UnitId, at: V2) {
        self.pending_fire.push((shooter, at));
    }

    fn new_contact(
        &mut self,
        source: ContactSource,
        center: V2,
        tick: Tick,
        lifetime: Tick,
        emitter: UnitId,
    ) {
        self.next_contact += 1;
        self.contacts.push(Contact {
            id: ContactId(self.next_contact),
            source,
            center,
            evidence_tick: tick,
            expires_tick: tick + lifetime,
            emitter,
        });
    }

    /// Turn this tick's firing evidence and lost identifications into areas.
    fn update_contacts(&mut self, tick: Tick, rules: &Rules) {
        let s = &rules.sensors;
        let lifetime = (s.contact_lifetime_s * rules.tick_hz as f64).round() as Tick;
        let radius = s.contact_radius_m;
        // Losing identification leaves a fading area around the last sighting.
        let lost: Vec<(UnitId, V2)> = self
            .tracks
            .iter()
            .filter(|(_, t)| t.last_seen + 1 == tick)
            .map(|(u, t)| (*u, t.position.xy()))
            .collect();
        for (unit, at) in lost {
            self.new_contact(ContactSource::LastSeen, at, tick, lifetime, unit);
        }
        // Identification replaces any area linked to what is now seen.
        let seen: Vec<UnitId> = self
            .tracks
            .iter()
            .filter(|(_, t)| t.last_seen == tick)
            .map(|(u, _)| *u)
            .collect();
        self.contacts.retain(|c| !seen.contains(&c.emitter));
        for (shooter, at) in std::mem::take(&mut self.pending_fire) {
            if seen.contains(&shooter) {
                continue;
            }
            // One report per firing episode: refresh while the shooter stays
            // inside the area it produced; a shot from outside starts a new one.
            if let Some(c) = self.contacts.iter_mut().find(|c| {
                c.source == ContactSource::Firing
                    && c.emitter == shooter
                    && (c.center - at).length() <= radius
            }) {
                c.evidence_tick = tick;
                c.expires_tick = tick + lifetime;
                continue;
            }
            let r = radius * self.rng.unit().sqrt();
            let a = std::f64::consts::TAU * self.rng.unit();
            self.new_contact(
                ContactSource::Firing,
                at + v2(a.cos(), a.sin()) * r,
                tick,
                lifetime,
                shooter,
            );
        }
        self.contacts.retain(|c| c.expires_tick >= tick);
    }

    pub fn contacts(&self, rules: &Rules) -> impl Iterator<Item = ApproximateContact> + '_ {
        let radius = rules.sensors.contact_radius_m;
        self.contacts.iter().map(move |c| ApproximateContact {
            id: c.id,
            source: c.source,
            center: [c.center.x, c.center.y],
            radius,
            evidence_tick: c.evidence_tick,
            expires_tick: c.expires_tick,
        })
    }

    /// The side's track of an enemy unit, while identified or within grace.
    pub fn track(&self, unit: UnitId) -> Option<&Track> {
        self.tracks.get(&unit)
    }

    /// Enemies identified this tick, keyed by their authority id (sim-internal).
    pub fn identified_now(&self, tick: Tick) -> impl Iterator<Item = (UnitId, &Track)> {
        self.tracks
            .iter()
            .filter(move |(_, t)| t.last_seen == tick)
            .map(|(u, t)| (*u, t))
    }

    /// The enemy behind a side-scoped handle, for validating commands.
    pub fn unit_for(&self, id: ObservedTargetId) -> Option<UnitId> {
        self.tracks
            .iter()
            .find(|(_, t)| t.id == id)
            .map(|(u, _)| *u)
    }

    pub fn contact(&self, id: ContactId) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.id == id)
    }

    pub fn all_contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// Whether this side identifies `unit` this tick.
    pub fn identifies(&self, unit: UnitId, tick: Tick) -> bool {
        self.tracks.get(&unit).is_some_and(|t| t.last_seen == tick)
    }

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
                    .map(|&k| unit.members[k].position)
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
        self.update_contacts(tick, rules);
        self.tracks.retain(|_, t| t.last_seen + grace >= tick);
    }

    /// Enemies identified this tick, with only what was observed: where it
    /// was seen, the soldiers seen and, being in view, its weapons' poses.
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
                            let p = unit.members[k].position;
                            [p.x, p.y, p.z]
                        })
                        .collect(),
                    member_ids: t.members.iter().map(|&k| unit.members[k].id).collect(),
                    weapon_poses: unit.mounts.iter().map(crate::weapons::pose).collect(),
                    reversing: unit.reversing,
                }
            })
    }

    /// Whether `observer`'s own sensors identified `target` at the last sensing.
    pub fn own_sees(&self, observer: UnitId, target: UnitId) -> bool {
        self.own_sensors
            .get(&observer)
            .is_some_and(|seen| seen.contains(&target))
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
        d.u64(self.next_id as u64)
            .u64(self.next_contact as u64)
            .u64(self.rng.state());
        d.u64(self.contacts.len() as u64);
        for c in &self.contacts {
            d.u64(c.id.0 as u64)
                .u64(c.source as u64)
                .f64(c.center.x)
                .f64(c.center.y)
                .u64(c.evidence_tick)
                .u64(c.expires_tick)
                .u64(c.emitter.0 as u64);
        }
        d.u64(self.pending_fire.len() as u64);
        for (shooter, at) in &self.pending_fire {
            d.u64(shooter.0 as u64).f64(at.x).f64(at.y);
        }
        d.u64(self.destroyed.len() as u64);
        for u in &self.destroyed {
            d.u64(u.0 as u64);
        }
        d.u64(self.corpses.len() as u64);
        for s in &self.corpses {
            d.u64(*s as u64);
        }
        self.ground.digest(d);
        // Guidance reads these a tick later, so they are carried state.
        d.u64(self.own_sensors.len() as u64);
        for (observer, seen) in &self.own_sensors {
            d.u64(observer.0 as u64).u64(seen.len() as u64);
            for t in seen {
                d.u64(t.0 as u64);
            }
        }
        d.u64(self.tracks.len() as u64);
        for (target, t) in &self.tracks {
            d.u64(target.0 as u64).u64(t.id.0 as u64).u64(t.last_seen);
            d.f64(t.velocity.x).f64(t.velocity.y);
            d.f64(t.position.x).f64(t.position.y).f64(t.position.z);
            d.f64(t.yaw).u64(t.members.len() as u64);
            for k in &t.members {
                d.u64(*k as u64);
            }
        }
    }
}
