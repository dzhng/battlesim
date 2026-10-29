//! A side's observation as the effects read it (`EffectPublication`): its
//! visible flight and blasts (a round landing in standing water splashes),
//! the shot counters and hulls of every unit the side sees (own units and
//! identified enemies, apart by side key) and where each wades, the
//! downed airframes still falling (which smoke), and every wreck and part of
//! a destroyed building it knows (which smoke, each with its kind's look).
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sim::publication::reader::{Observation, Projectile, WeaponPose};

use crate::seed::side_key;
use crate::static_world::ground::water::StandingWater;
use crate::units::riding;
use crate::world::{EffectsSection, MountMuzzle, UnitsSection, Vec3};

/// A visible stretch of one round's flight this tick.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Segment {
    /// The flown path, at least two points; it bends at each ricochet.
    pub path: Vec<Vec3>,
    pub ricochets: Vec<Ricochet>,
    pub kind: Arc<str>,
    /// The soldier who fired it, or None for a vehicle's gun.
    pub shooter: Option<f64>,
    /// What it struck at the path's end: none, ground, water, hull, prop,
    /// soldier or metal_prop.
    pub hit: Arc<str>,
    pub normal: Option<Vec3>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Ricochet {
    pub point: usize,
    pub normal: Vec3,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Blast {
    pub point: Vec3,
    pub radius: f64,
    pub kind: Arc<str>,
}

/// One weapon mount as published: a rise in `shots` is a shot.
#[derive(Clone, Debug, Deserialize)]
pub struct Mount {
    pub bearing: f64,
    pub elevation: f64,
    pub shots: f64,
    /// The round kind it fires (its first weapon row).
    pub kind: Arc<str>,
    /// Where a hull's mount fires from; None for a soldier's weapon.
    pub muzzle: Option<MountMuzzle>,
}

/// A unit whose shots the side sees.
#[derive(Clone, Debug, Deserialize)]
pub struct Shooter {
    /// Unique across both sides for the battle's life (a side key).
    pub key: f64,
    pub position: Vec3,
    /// A hull's half extents; None for infantry.
    pub half: Option<Vec3>,
    #[serde(default)]
    pub airborne: bool,
    #[serde(default)]
    pub smoking: bool,
    pub yaw: f64,
    pub members: Vec<Option<f64>>,
    pub mounts: Vec<Mount>,
    /// Where it stands in water: its hull's centre, or each soldier in it.
    #[serde(default)]
    pub wading: Vec<Wading>,
}

/// A hull's centre or a soldier standing in water, on its surface, and how
/// deep the water stands there.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Wading {
    pub at: Vec3,
    pub depth_m: f64,
}

/// Something the side knows smokes: a wreck today.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SmokeSource {
    pub key: String,
    /// Its look: a `presentation.effects.smoke` row.
    pub kind: String,
    pub center: Vec3,
    pub yaw: f64,
    pub half: Vec3,
}

/// A hull the side watched die this tick.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CookOff {
    pub center: Vec3,
    /// Its hull's top, metres above `center`.
    pub height: f64,
    /// Where its thrown turret lands, or None.
    pub landing: Option<Vec3>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectPublication {
    pub tick: f64,
    pub segments: Vec<Segment>,
    pub blasts: Vec<Blast>,
    #[serde(skip_serializing, default)]
    pub shooters: Vec<Shooter>,
    pub smokes: Vec<SmokeSource>,
    #[serde(default)]
    pub cook_offs: Vec<CookOff>,
    /// Own aircraft flare releases observed since the previous publication.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flare_deploys: Vec<Vec3>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grinds: Vec<Grind>,
}

/// Where a hull grinds a metal body it shoves this tick, and the way it
/// drives.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Grind {
    pub point: Vec3,
    pub heading: [f64; 2],
}

/// Reads observations into effect publications for one side.
pub struct EffectFeed {
    blue: bool,
    /// Each smoke source's key, by its kind and centre's bits.
    keys: HashMap<(Arc<str>, u64, u64), String>,
    flares: HashMap<u64, u32>,
}

impl EffectFeed {
    pub fn new(side: &str) -> Self {
        Self {
            blue: side == "blue",
            keys: HashMap::new(),
            flares: HashMap::new(),
        }
    }

    /// What `o` publishes for effects (cook-offs are the watcher's to add).
    pub fn publication(
        &mut self,
        o: &Observation,
        units: &UnitsSection,
        effects: &EffectsSection,
        water: Option<&StandingWater>,
    ) -> EffectPublication {
        let own_even = true;
        let mut flare_deploys = Vec::new();
        let mut seen_flares = HashSet::new();
        for u in &o.own {
            let key = u.id.to_bits();
            seen_flares.insert(key);
            let Some(flares) = u.flares.as_ref() else {
                self.flares.remove(&key);
                continue;
            };
            let charges = flares.charges.max(0.0) as u32;
            if let Some(previous) = self.flares.insert(key, charges) {
                for _ in 0..previous.saturating_sub(charges) {
                    flare_deploys.push(u.position);
                }
            }
        }
        self.flares.retain(|key, _| seen_flares.contains(key));
        let shooter = |key: f64,
                       kind: &str,
                       position: Vec3,
                       yaw: f64,
                       members: &[Option<f64>],
                       poses: &[WeaponPose],
                       smoking: bool| {
            let unit = units.types.get(kind);
            let ty = effects.types.get(kind);
            Shooter {
                key,
                position,
                half: ty.and_then(|t| t.half),
                airborne: unit.is_some_and(|u| u.airborne),
                smoking,
                yaw,
                members: members.to_vec(),
                mounts: poses
                    .iter()
                    .filter_map(|p| {
                        let m = p.mount as usize;
                        let row = unit?.mounts.get(m)?;
                        Some(Mount {
                            bearing: p.bearing,
                            elevation: p.elevation,
                            shots: p.shots.unwrap_or(f64::NAN),
                            kind: row.weapon.as_str().into(),
                            muzzle: ty.and_then(|t| t.muzzles.get(m).copied().flatten()),
                        })
                    })
                    .collect(),
                wading: vec![],
            }
        };
        let mut shooters = Vec::with_capacity(o.own.len() + o.identified.len() + o.crashes.len());
        for u in &o.own {
            let mut s = shooter(
                side_key(u.id, own_even),
                &u.kind,
                u.position,
                u.yaw,
                &u.member_ids,
                &u.weapon_poses,
                u.smoking,
            );
            if !riding(u) {
                s.wading = wading(water, &s, &u.members);
            }
            shooters.push(s);
        }
        for e in &o.identified {
            let mut s = shooter(
                side_key(e.id, !own_even),
                &e.kind,
                e.position,
                e.yaw,
                &e.member_ids,
                &e.weapon_poses,
                e.smoking,
            );
            s.wading = wading(water, &s, &e.members);
            shooters.push(s);
        }
        // A downed airframe smokes all the way down, keyed as it was alive.
        for c in &o.crashes {
            shooters.push(shooter(
                side_key(c.id, c.own),
                &c.kind,
                c.position,
                c.yaw,
                &[],
                &[],
                true,
            ));
        }
        let mut smokes = Vec::new();
        for p in o.known_props.iter() {
            let Some(look) = (!p.destroyed)
                .then(|| effects.smokes.get(&*p.kind))
                .flatten()
            else {
                continue;
            };
            let key = self
                .keys
                .entry((p.kind.clone(), p.center[0].to_bits(), p.center[1].to_bits()))
                .or_insert_with(|| format!("{}:{},{}", p.kind, p.center[0], p.center[1]))
                .clone();
            smokes.push(SmokeSource {
                key,
                kind: look.clone(),
                center: [p.center[0], p.center[1], p.base_z],
                yaw: p.yaw,
                half: p.half,
            });
        }
        EffectPublication {
            tick: o.tick,
            segments: o
                .projectiles
                .iter()
                .map(|p| Segment {
                    path: p.path.clone(),
                    ricochets: p
                        .ricochets
                        .iter()
                        .map(|r| Ricochet {
                            point: r.point as usize,
                            normal: r.normal,
                        })
                        .collect(),
                    kind: p.kind.clone(),
                    shooter: p.shooter_member,
                    hit: splashed(water, p),
                    normal: p.impact_normal,
                })
                .collect(),
            blasts: o
                .blasts
                .iter()
                .map(|b| Blast {
                    point: b.point,
                    radius: b.radius,
                    kind: b.kind.clone(),
                })
                .collect(),
            shooters,
            smokes,
            cook_offs: Vec::new(),
            flare_deploys,
            grinds: o
                .grinds
                .iter()
                .map(|g| Grind {
                    point: g.point,
                    heading: g.heading,
                })
                .collect(),
        }
    }

    /// Whether the presented side is blue (own ids even, the enemy's odd).
    pub fn blue(&self) -> bool {
        self.blue
    }
}

/// Where `s` stands in `water`: a hull on the ground at its centre, else
/// each of its soldiers (`members`) in it (`StandingWater::under`: below
/// its surface, so not on a deck over it), on the water's surface.
fn wading(water: Option<&StandingWater>, s: &Shooter, members: &[Vec3]) -> Vec<Wading> {
    let Some(water) = water.filter(|_| !s.airborne) else {
        return vec![];
    };
    let centre = [s.position];
    let points = if s.half.is_some() {
        &centre[..]
    } else {
        members
    };
    points
        .iter()
        .filter_map(|&[x, y, z]| {
            let wet = water.under([x, y, z])?;
            Some(Wading {
                at: [x, y, wet.surface_z],
                depth_m: wet.depth_m,
            })
        })
        .collect()
}

/// What round `p` struck: water where it lands on the ground in `water`
/// (the simulation stops it on the surface, as on ground, so it is looked
/// for just under where it stopped; a round on a deck over the water is
/// dry), so it splashes.
fn splashed(water: Option<&StandingWater>, p: &Projectile) -> Arc<str> {
    let wet = |[x, y, z]: Vec3| water.is_some_and(|w| w.under([x, y, z - 0.05]).is_some());
    match p.path.last() {
        Some(&end) if &*p.hit == "ground" && wet(end) => "water".into(),
        _ => p.hit.clone(),
    }
}
