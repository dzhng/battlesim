//! Which shots a side's publications show, the one derivation the muzzle
//! flashes, the firing soldiers' poses and the gunfire sounds read. A mount's
//! shot counter rising is a shot. A hull's mount fires from its own muzzle
//! along its bearing and elevation, on the tick its counter rises. A squad's
//! rise goes to the soldiers who start new rounds, one round each, matched
//! by kind, in the next publication: the simulation flies a round first on
//! the tick after the one that fired it. A round still flying (its stretch
//! starts where last tick's ended) is not a new launch. A unit first seen,
//! or seen again, shows no shot until its counter rises once more.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::publication::{EffectPublication, Segment, Shooter};
use crate::seed::Key;
use crate::world::{MountMuzzle, Vec3};

/// One shot the publication shows, at the start of its tick.
#[derive(Clone, Debug)]
pub struct Launch {
    pub shooter: f64,
    pub mount: usize,
    /// The soldier who fired (a squad's), or None for a hull's mount.
    pub soldier: Option<f64>,
    /// Where the round leaves.
    pub at: Vec3,
    /// The unit direction it leaves along.
    pub dir: Vec3,
    pub kind: Arc<str>,
    /// Fired from a hull's mount.
    pub hull: bool,
}

/// A flight: its round's kind, its shooter and where a stretch of it ends.
type FlightKey = (Arc<str>, Option<u64>, [u64; 3]);

fn flight_key(s: &Segment, p: &Vec3) -> FlightKey {
    (
        s.kind.clone(),
        s.shooter.map(f64::to_bits),
        p.map(f64::to_bits),
    )
}

#[derive(Default)]
pub struct LaunchTracker {
    /// Shot counters by shooter key, as last published.
    counters: HashMap<u64, Vec<f64>>,
    /// Where the last publication's still-flying stretches ended, with
    /// whether each was shown.
    flying: HashMap<FlightKey, Vec<bool>>,
    /// This publication's segments (by index) chosen as tracers.
    pub tracers: HashSet<usize>,
    /// Squads' rises in the last publication, by shooter key and mount,
    /// with the round they fired: its flight shows a publication later,
    /// when the mount may name the next round.
    owed: HashMap<u64, Vec<(f64, Arc<str>)>>,
}

impl LaunchTracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The launches `pub_` shows. Call once per publication, in order; `gap`
    /// says ticks were skipped since the last one. `chance` is a round
    /// kind's chance to show as a tracer (None: always).
    pub fn note(
        &mut self,
        pub_: &EffectPublication,
        gap: bool,
        chance: Option<&dyn Fn(&str) -> f64>,
    ) -> Vec<Launch> {
        let mut out = Vec::new();
        let mut starts: HashMap<u64, Vec<&Segment>> = HashMap::new();
        for s in &pub_.segments {
            let Some(shooter) = s.shooter else { continue };
            if s.path.len() < 2 {
                continue;
            }
            if !gap && self.flying.contains_key(&flight_key(s, &s.path[0])) {
                continue; // a round still flying
            }
            starts.entry(shooter.to_bits()).or_default().push(s);
        }
        let mut seen = HashSet::new();
        let mut owed: HashMap<u64, Vec<(f64, Arc<str>)>> = HashMap::new();
        for u in &pub_.shooters {
            let key = u.key.to_bits();
            seen.insert(key);
            let before = self
                .counters
                .insert(key, u.mounts.iter().map(|m| m.shots).collect());
            let due = if gap { None } else { self.owed.get(&key) };
            for (m, mount) in u.mounts.iter().enumerate() {
                // First seen: no shot.
                let rose = before.as_ref().map_or(0.0, |b| {
                    mount.shots - b.get(m).copied().unwrap_or(mount.shots)
                });
                if u.half.is_some() {
                    if rose > 0.0 {
                        out.push(hull_launch(u, m));
                    }
                    continue;
                }
                if rose > 0.0 {
                    let rises = owed.entry(key).or_default();
                    if rises.len() <= m {
                        rises.resize(m + 1, (0.0, mount.kind.clone()));
                    }
                    rises[m] = (rose, mount.kind.clone());
                }
                let Some((mut left, kind)) = due.and_then(|d| d.get(m)).cloned() else {
                    continue;
                };
                for id in u.members.iter().flatten() {
                    for s in starts.get(&id.to_bits()).into_iter().flatten() {
                        if left <= 0.0 || s.kind != kind {
                            continue;
                        }
                        left -= 1.0;
                        let (a, b) = (s.path[0], s.path[1]);
                        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                        let len = crate::world::length(d);
                        let len = if len == 0.0 || len.is_nan() { 1.0 } else { len };
                        out.push(Launch {
                            shooter: u.key,
                            mount: m,
                            soldier: Some(*id),
                            at: a,
                            dir: [d[0] / len, d[1] / len, d[2] / len],
                            kind: s.kind.clone(),
                            hull: false,
                        });
                    }
                }
            }
        }
        self.owed = owed;
        // Units no longer seen start over when they are seen again.
        self.counters.retain(|k, _| seen.contains(k));
        let mut flying: HashMap<FlightKey, Vec<bool>> = HashMap::new();
        self.tracers.clear();
        for (index, s) in pub_.segments.iter().enumerate() {
            if s.path.len() < 2 {
                continue;
            }
            let probability = chance.map_or(1.0, |c| c(&s.kind));
            // The authority publishes in round order; a queue keeps separate
            // choices when paths share an exact endpoint.
            let previous = if gap {
                None
            } else {
                self.flying
                    .get_mut(&flight_key(s, &s.path[0]))
                    .and_then(|q| (!q.is_empty()).then(|| q.remove(0)))
            };
            let visible = previous.unwrap_or_else(|| {
                probability >= 1.0
                    || Key::new()
                        .num(pub_.tick)
                        .num(index as f64)
                        .text(&s.kind)
                        .num(s.shooter.unwrap_or(-1.0))
                        .nums(&s.path[0])
                        .unit()
                        < probability
            });
            if visible {
                self.tracers.insert(index);
            }
            if &*s.hit == "none" {
                flying
                    .entry(flight_key(s, &s.path[s.path.len() - 1]))
                    .or_default()
                    .push(visible);
            }
        }
        self.flying = flying;
        out
    }
}

/// `p` rotated by `angle` about the vertical through `origin`.
fn rotate_z(p: Vec3, origin: Vec3, angle: f64) -> Vec3 {
    let (s, c) = libm::sincos(angle);
    let (x, y) = (p[0] - origin[0], p[1] - origin[1]);
    [
        (x * c - y * s) + origin[0],
        (x * s + y * c) + origin[1],
        p[2],
    ]
}

/// The muzzle's offset from the unit's position for a hull heading `yaw`,
/// its carrier along `carried` and the mount along `bearing`
/// (`mountMuzzle.ts` `muzzleOffset`).
pub fn muzzle_offset(m: &MountMuzzle, yaw: f64, carried: f64, bearing: f64) -> Vec3 {
    let muzzle = rotate_z(m.muzzle, [0.0; 3], bearing);
    let pivot = rotate_z(m.pivot, m.ring, carried);
    let ring = rotate_z(m.ring, [0.0; 3], yaw);
    [
        pivot[0] - m.ring[0] + ring[0] + muzzle[0],
        pivot[1] - m.ring[1] + ring[1] + muzzle[1],
        pivot[2] - m.ring[2] + ring[2] + muzzle[2],
    ]
}

/// A hull's shot from mount `m`: from its own muzzle, along its aim.
fn hull_launch(u: &Shooter, m: usize) -> Launch {
    let mount = &u.mounts[m];
    let at = mount.muzzle.as_ref().map_or([0.0; 3], |muzzle| {
        let carried = muzzle.on.map_or(u.yaw, |on| u.mounts[on].bearing);
        muzzle_offset(muzzle, u.yaw, carried, mount.bearing)
    });
    let (s, c) = libm::sincos(mount.bearing);
    let ce = libm::cos(mount.elevation);
    Launch {
        shooter: u.key,
        mount: m,
        soldier: None,
        at: [
            u.position[0] + at[0],
            u.position[1] + at[1],
            u.position[2] + at[2],
        ],
        dir: [ce * c, ce * s, libm::sin(mount.elevation)],
        kind: mount.kind.clone(),
        hull: true,
    }
}
