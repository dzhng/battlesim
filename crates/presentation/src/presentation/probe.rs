//! The last frame as the labs' scenes probe it.
use super::*;

impl Presentation {
    /// The last frame as the labs' scenes probe it, JSON: the effects'
    /// state, instances and offered lights, the soldiers and vehicles as
    /// posed, the corpses lying, and a cook-off's pieces.
    pub fn probe_json(&self) -> String {
        let Some(l) = &self.loaded else {
            return "null".into();
        };
        let poses = l.driver.poses();
        let units = &l.units;
        let (mut locals, mut worlds) = (Vec::new(), Vec::new());
        let name = |kind: &str,
                    side: &str,
                    id: f64,
                    slot: usize,
                    op: Option<usize>,
                    active: Option<usize>| {
            units
                .resolve(kind, side, id, slot, op, active)
                .unwrap_or("")
                .to_owned()
        };
        let stats = l.effects.stats();
        let lights = &l.batch.lights;
        let looks = &l.input.effects.wreck_looks;
        let pieces: Vec<_> = l
            .pieces
            .iter()
            .filter_map(|p| {
                let state = p.state?;
                let _ = looks;
                Some(serde_json::json!({
                    "appearance": &*p.appearance,
                    "state": state,
                    "sunk": -p.motion.map_or(0.0, |m| m[14]),
                }))
            })
            .collect();
        serde_json::json!({
            "effects": {
                "live": stats.live,
                "instances": stats.instances,
                "dropped": stats.dropped,
                "sources": stats.sources,
                "tick": stats.tick,
            },
            "effectInstances": l.batch.data[..l.batch.count * EFFECT_FLOATS]
                .chunks(EFFECT_FLOATS)
                .map(|d| serde_json::json!({
                    "shape": d[12],
                    "at": [d[0], d[1], d[2]],
                    "to": [d[4], d[5], d[6]],
                    "rgb": [d[8], d[9], d[10]],
                    "size": d[3],
                    "rays": d[6],
                }))
                .collect::<Vec<_>>(),
            "castLights": (0..lights.count)
                .map(|i| {
                    let r = &lights.data[i * 8..i * 8 + 8];
                    serde_json::json!({
                        "at": [r[0], r[1], r[2]],
                        "radius": r[3],
                        "rgb": [r[4], r[5], r[6]],
                        "cause": &*lights.causes[i],
                    })
                })
                .collect::<Vec<_>>(),
            "lying": poses.corpses.iter().map(|c| c.soldier).collect::<Vec<_>>(),
            "cookOffPieces": pieces,
            "soldiers": poses.soldiers.iter().map(|s| serde_json::json!({
                "appearance": name(&s.kind, &s.side, s.soldier, s.slot, s.operator_mount, s.active_mount),
                "position": s.position,
                "yaw": s.facing,
                "clip": &*s.clip,
                "phase": s.phase,
                // His weapon's muzzle where his body draws it, crossfaded and kicked.
                "muzzle": s.active_mount.and_then(|m| {
                    crate::units::drawn::soldier_muzzle(units, &l.tables, s, m, &mut worlds)
                }),
            })).collect::<Vec<_>>(),
            "vehicles": poses.vehicles.iter().map(|v| {
                let a = v.articulation;
                serde_json::json!({
                    "appearance": name(&v.kind, &v.side, v.unit, 0, None, None),
                    "position": v.position,
                    "yaw": v.yaw,
                    "tint": units.tints.get(&*v.side),
                    "articulation": {
                        "turret_yaw": a.turret_yaw, "gun_pitch": a.gun_pitch, "recoil": a.recoil,
                        "hmg_yaw": a.hmg_yaw, "hmg_pitch": a.hmg_pitch, "travel_l": a.travel_l,
                        "travel_r": a.travel_r, "deploy": a.deploy, "rotor": a.rotor,
                        "rotor_blur": a.rotor_blur,
                    },
                    // Each mount's muzzle where the hull draws it, rocked and articulated.
                    "muzzles": (0..units.types.get(&*v.kind).map_or(0, |t| t.roles.len()))
                        .filter_map(|m| {
                            let (name, at) = crate::units::drawn::vehicle_muzzle(
                                units, v, m, &mut locals, &mut worlds,
                            )?;
                            Some(serde_json::json!({ "name": name, "at": at }))
                        })
                        .collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        })
        .to_string()
    }
}
