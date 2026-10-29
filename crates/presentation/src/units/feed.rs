//! The observation as the pose driver reads it: a side's
//! interpolated frame becomes a [`FeedFrame`] of every own unit and
//! identified enemy with its soldiers by member id, its weapon poses, every
//! downed airframe still falling, and the fallen the side knows of. Each
//! soldier's position is his own, blended by his id between ticks.
//!
//! It counts each soldier's shots from the launches each publication shows,
//! the one derivation the muzzle flashes and the gunfire sounds read too, so
//! the soldier who kneels to fire is the one whose flash and sound fire.
use std::collections::HashMap;
use std::sync::Arc;

use sim::publication::reader::{Corpse, MemberLean, Observation, WeaponPose};

use super::driver::{FeedFallen, FeedFrame, FeedMount, FeedUnit};
use super::interpolate::{FrameSample, Pose};
use super::slot;
use crate::effects::launches::LaunchTracker;
use crate::effects::publication::EffectFeed;
use crate::seed::side_key;
use crate::world::{EffectsSection, UnitsSection};

/// What a unit's publication says of its soldiers and mounts.
struct Published<'a> {
    ids: &'a [Option<f64>],
    slots: &'a [f64],
    active: &'a [Option<f64>],
    leans: &'a [Option<MemberLean>],
    poses: &'a [WeaponPose],
}

pub struct ObservationFeed {
    side: Arc<str>,
    enemy: Arc<str>,
    observation: Option<Arc<Observation>>,
    corpses: Option<Arc<Vec<Corpse>>>,
    effects: EffectFeed,
    launches: LaunchTracker,
    last_tick: f64,
    /// Each soldier's launches so far, per mount, by soldier id bits.
    shots: HashMap<u64, Vec<f64>>,
    /// Each unit's mount records, by side key (blue even): a downed
    /// airframe falls with its turret as last seen.
    mounts: HashMap<u64, Vec<FeedMount>>,
    out: FeedFrame,
}

impl ObservationFeed {
    /// A feed for the side `side` (`"blue"` or `"red"`).
    pub fn new(side: &str) -> Self {
        Self {
            side: side.into(),
            enemy: if side == "blue" { "red" } else { "blue" }.into(),
            observation: None,
            corpses: None,
            effects: EffectFeed::new(side),
            launches: LaunchTracker::default(),
            last_tick: -1.0,
            shots: HashMap::new(),
            mounts: HashMap::new(),
            out: FeedFrame::default(),
        }
    }

    /// The feed frame for one drawn sample: its poses and the publication
    /// they blend toward, never an older one, so a soldier who has left the
    /// poses is already among the fallen and plays his death. The fallen
    /// list is kept until the publication's corpses change.
    pub fn frame(
        &mut self,
        sample: &FrameSample,
        units: &UnitsSection,
        effects: &EffectsSection,
    ) -> &FeedFrame {
        let o = &sample.observation;
        if !self
            .observation
            .as_ref()
            .is_some_and(|seen| Arc::ptr_eq(seen, o))
        {
            self.observation = Some(o.clone());
            if !self
                .corpses
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(c, &o.corpses))
            {
                self.corpses = Some(o.corpses.clone());
                let (side, enemy) = (&self.side, &self.enemy);
                self.out.fallen = Arc::new(
                    o.corpses
                        .iter()
                        .filter_map(|c| {
                            Some(FeedFallen {
                                soldier: c.soldier?,
                                position: c.position,
                                yaw: c.yaw,
                                kind: c.kind.clone(),
                                slot: c.slot as usize,
                                side: if c.own { side } else { enemy }.clone(),
                            })
                        })
                        .collect(),
                );
            }
            if o.tick != self.last_tick {
                if o.tick < self.last_tick {
                    self.launches.reset();
                    self.shots.clear();
                }
                let gap = self.last_tick >= 0.0 && o.tick != self.last_tick + 1.0;
                self.last_tick = o.tick;
                let publication = self.effects.publication(o, units, effects, None);
                for l in self.launches.note(&publication, gap, None) {
                    let Some(soldier) = l.soldier else { continue };
                    let counts = self.shots.entry(soldier.to_bits()).or_default();
                    if counts.len() <= l.mount {
                        counts.resize(l.mount + 1, 0.0);
                    }
                    counts[l.mount] += 1.0;
                }
            }
        }
        let mut list = std::mem::take(&mut self.out.units);
        let mut n = 0;
        for pose in &sample.own {
            let Some(u) = o.own.iter().find(|u| u.id == pose.id) else {
                continue;
            };
            let published = Published {
                ids: &u.member_ids,
                slots: &u.member_slots,
                active: &u.member_active_mounts,
                leans: &u.member_leans,
                poses: &u.weapon_poses,
            };
            let pinned = &*u.suppression == "pinned";
            self.unit(slot(&mut list, n), pose, &u.kind, true, &published, pinned);
            n += 1;
        }
        for pose in &sample.identified {
            let Some(e) = o.identified.iter().find(|e| e.id == pose.id) else {
                continue;
            };
            let published = Published {
                ids: &e.member_ids,
                slots: &e.member_slots,
                active: &e.member_active_mounts,
                leans: &e.member_leans,
                poses: &e.weapon_poses,
            };
            // The side cannot know an enemy's suppression.
            self.unit(slot(&mut list, n), pose, &e.kind, false, &published, false);
            n += 1;
        }
        // A downed airframe falls on as the vehicle it was, crewless, its
        // turret as last seen, tipping into the fall.
        for c in &sample.crashes {
            let side = if c.own { &self.side } else { &self.enemy };
            let unit = slot(&mut list, n);
            n += 1;
            unit.id = c.id;
            unit.kind = c.kind.clone();
            unit.side = side.clone();
            unit.position = c.position;
            unit.yaw = c.yaw;
            unit.soldiers.clear();
            unit.mounts.clear();
            if let Some(m) = self
                .mounts
                .get(&side_key(c.id, &**side == "blue").to_bits())
            {
                unit.mounts.extend_from_slice(m);
            }
            unit.deployment = None;
            unit.pinned = false;
            unit.attitude = Some((c.pitch, c.roll));
        }
        list.truncate(n);
        self.out.units = list;
        self.out.time = sample.time;
        &self.out
    }

    fn unit(
        &mut self,
        out: &mut FeedUnit,
        pose: &Pose,
        kind: &Arc<str>,
        own: bool,
        published: &Published,
        pinned: bool,
    ) {
        let side = if own { &self.side } else { &self.enemy };
        out.id = pose.id;
        out.kind = kind.clone();
        out.side = side.clone();
        out.position = pose.position;
        out.yaw = pose.yaw;
        out.deployment = pose.deployment;
        out.pinned = pinned;
        out.attitude = None;
        let mut n = 0;
        for (k, &position) in pose.members.iter().enumerate() {
            let Some(id) = pose.member_ids.get(k).copied().flatten() else {
                continue;
            };
            let at = published.ids.iter().position(|&p| p == Some(id));
            let active_mount = at
                .and_then(|i| published.active.get(i).copied().flatten())
                .map(|m| m as usize);
            let soldier = slot(&mut out.soldiers, n);
            n += 1;
            soldier.id = id;
            soldier.slot = at
                .and_then(|i| published.slots.get(i))
                .map_or(0, |&s| s as usize);
            soldier.active_mount = active_mount;
            soldier.position = position;
            soldier.shots = active_mount
                .and_then(|m| self.shots.get(&id.to_bits())?.get(m).copied())
                .unwrap_or(0.0);
            // Leans are discrete: the driver eases the slide from this visible fact.
            soldier.lean = at
                .and_then(|i| published.leans.get(i)?.as_ref())
                .map(|l| l.at);
        }
        out.soldiers.truncate(n);
        // Mount records in the rules' mount order, kept for a later crash.
        let mounts = self
            .mounts
            .entry(side_key(pose.id, &**side == "blue").to_bits())
            .or_default();
        mounts.clear();
        for p in published.poses {
            let m = p.mount as usize;
            if mounts.len() <= m {
                mounts.resize(m + 1, FeedMount::default());
            }
            mounts[m] = FeedMount {
                operator: p.operator,
                bearing: p.bearing,
                elevation: p.elevation,
                shots: p.shots.unwrap_or(0.0),
            };
        }
        out.mounts.clear();
        out.mounts.extend_from_slice(mounts);
    }
}
