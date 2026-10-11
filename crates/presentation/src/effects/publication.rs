//! A side's observation as the effects read it (`EffectPublication`): its
//! visible flight and blasts, the shot counters and hulls of every unit the
//! side sees (own units and identified enemies, apart by side key), the
//! downed airframes still falling (which smoke), and every wreck and part of
//! a destroyed building it knows (which smoke, each with its kind's look).
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sim::publication::reader::{Observation, WeaponPose};

use crate::seed::{js_number, side_key};
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
    /// What it struck at the path's end: none, ground, hull, prop or soldier.
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

/// A hull the side watched die this tick (`cookOff.ts`).
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
    #[serde(skip_serializing)]
    pub shooters: Vec<Shooter>,
    pub smokes: Vec<SmokeSource>,
    #[serde(default)]
    pub cook_offs: Vec<CookOff>,
}

/// Reads observations into effect publications for one side.
pub struct EffectFeed {
    blue: bool,
    /// Each smoke source's key, by its kind and centre's bits.
    keys: HashMap<(Arc<str>, u64, u64), String>,
}

impl EffectFeed {
    pub fn new(side: &str) -> Self {
        Self {
            blue: side == "blue",
            keys: HashMap::new(),
        }
    }

    /// What `o` publishes for effects (cook-offs are the watcher's to add).
    pub fn publication(
        &mut self,
        o: &Observation,
        units: &UnitsSection,
        effects: &EffectsSection,
    ) -> EffectPublication {
        let own_even = true;
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
                    .map(|p| {
                        let m = p.mount as usize;
                        Mount {
                            bearing: p.bearing,
                            elevation: p.elevation,
                            shots: p.shots.unwrap_or(f64::NAN),
                            kind: unit
                                .and_then(|u| u.mounts.get(m))
                                .map_or_else(|| "default".into(), |r| r.weapon.as_str().into()),
                            muzzle: ty.and_then(|t| t.muzzles.get(m).copied().flatten()),
                        }
                    })
                    .collect(),
            }
        };
        let mut shooters = Vec::with_capacity(o.own.len() + o.identified.len() + o.crashes.len());
        for u in &o.own {
            shooters.push(shooter(
                side_key(u.id, own_even),
                &u.kind,
                u.position,
                u.yaw,
                &u.member_ids,
                &u.weapon_poses,
                u.smoking,
            ));
        }
        for e in &o.identified {
            shooters.push(shooter(
                side_key(e.id, !own_even),
                &e.kind,
                e.position,
                e.yaw,
                &e.member_ids,
                &e.weapon_poses,
                e.smoking,
            ));
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
                .or_insert_with(|| {
                    format!(
                        "{}:{},{}",
                        p.kind,
                        js_number(p.center[0]),
                        js_number(p.center[1])
                    )
                })
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
                    hit: p.hit.clone(),
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
        }
    }

    /// Whether the presented side is blue (own ids even, the enemy's odd).
    pub fn blue(&self) -> bool {
        self.blue
    }
}
