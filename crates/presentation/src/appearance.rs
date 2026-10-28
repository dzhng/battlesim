//! Authored model facts derived from bundle geometry and skeleton clips.
use crate::assets::{Container, View};
use crate::units::skin::{joint_worlds, mul, node_worlds, Mat4};
use crate::world::{bounds_size, Clip, Node, Trs, Vec3};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Floats per joint sample in a clip table: rotation, translation, scale.
const SAMPLE_FLOATS: usize = 8;
/// A skeleton clip channel's modes (`CHANNEL_ABSENT`, `CHANNEL_ANIMATED`).
const ABSENT: u8 = 0;
const ANIMATED: u8 = 2;
/// Each rig's muzzle node, by the role that draws its mount (`MOUNT_NODES`).
const MOUNT_MUZZLES: [(&str, &str); 2] = [("gun", "muzzle"), ("hmg", "hmg_muzzle")];
/// A gun's pitch where its model states none, degrees [down, up].
const GUN_PITCH_DEG: [f64; 2] = [-10.0, 20.0];
const HMG_PITCH_DEG: [f64; 2] = [-10.0, 45.0];
const BLADE_OUTER: f64 = 0.6;
const MAST_REACH: f64 = 0.05;
/// The least angle between two blades round a hub, degrees.
const BLADE_GAP_DEG: f64 = 20.0;
/// `v` as a typed array's handle.
fn view_of(v: &Value) -> Result<View, String> {
    serde_json::from_value(v.clone()).map_err(|_| "expected a typed array".to_string())
}

fn trs_json(t: &Value) -> Value {
    json!({ "t": t["t"], "r": t["r"], "s": t["s"] })
}

fn mat4(v: &Value) -> Result<Mat4, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("a matrix: {e}"))
}

/// A box of `positions`, as float32 as they are stored (`positionsBounds`).
fn positions_bounds(positions: impl Iterator<Item = [f32; 3]>) -> [Vec3; 2] {
    let mut b = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in positions {
        for c in 0..3 {
            b[0][c] = b[0][c].min(f64::from(p[c]));
            b[1][c] = b[1][c].max(f64::from(p[c]));
        }
    }
    b
}

/// How big a box is drawn (`boundsSize`), as its record.
fn size_json(b: [Vec3; 2]) -> Value {
    let size = bounds_size(b);
    json!({ "size": size.size, "radius": size.radius })
}

/// `m · p` (`vec3.transformMat4`, affine).
fn transform(m: &Mat4, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}

/// A skeleton's clips laid out densely for one body:
/// every clip's frames, joint-major within a frame, each joint a rotation,
/// translation and uniform scale; a channel a clip leaves absent takes the
/// body's bind. And each clip's record.
fn clip_table(
    skeleton: &Container<Value>,
    binds: &[Trs],
) -> Result<(Vec<f32>, Map<String, Value>), String> {
    let n = binds.len();
    let rate = skeleton.content["sample_hz"]
        .as_f64()
        .ok_or("no sample_hz")?;
    let mut data = Vec::new();
    let mut clips = Map::new();
    for clip in skeleton.content["clips"].as_array().ok_or("no clips")? {
        let frames = clip["frames"].as_u64().ok_or("a clip has no frames")? as usize;
        let name = clip["name"].as_str().ok_or("a clip has no name")?;
        let r_modes = skeleton.u8s(view_of(&clip["rotation_modes"])?)?;
        let t_modes = skeleton.u8s(view_of(&clip["translation_modes"])?)?;
        let rotations = skeleton.i16s(view_of(&clip["rotations"])?)?;
        let translations = skeleton.f32s(view_of(&clip["translations"])?)?;
        if r_modes.len() != n || t_modes.len() != n {
            return Err(format!("clip {name}: joints differ from the body's"));
        }
        let offset = data.len();
        data.resize(offset + frames * n * SAMPLE_FLOATS, 0.0);
        let (mut r, mut t) = (0, 0);
        for (j, bind) in binds.iter().enumerate() {
            for f in 0..frames {
                let o = offset + (f * n + j) * SAMPLE_FLOATS;
                let at = |mode: u8, base: usize| base + if mode == ANIMATED { f } else { 0 };
                for c in 0..4 {
                    data[o + c] = if r_modes[j] == ABSENT {
                        bind.r[c] as f32
                    } else {
                        (f64::from(rotations[at(r_modes[j], r) * 4 + c]) / 32767.0) as f32
                    };
                }
                for c in 0..3 {
                    data[o + 4 + c] = if t_modes[j] == ABSENT {
                        bind.t[c] as f32
                    } else {
                        translations[at(t_modes[j], t) * 3 + c]
                    };
                }
                // Binds are uniform-scaled.
                data[o + 7] = bind.s[0] as f32;
            }
            let advance = |mode: u8| match mode {
                ABSENT => 0,
                ANIMATED => frames,
                _ => 1,
            };
            r += advance(r_modes[j]);
            t += advance(t_modes[j]);
        }
        clips.insert(
            name.to_owned(),
            json!({
                "offset": offset,
                "frames": frames,
                "duration": clip["duration"],
                "rate": rate,
                "loop": clip["loop"],
                "stride": clip["stride_m"],
            }),
        );
    }
    Ok((data, clips))
}

/// A skinned body's record and its clip table.
pub(crate) fn skinned(
    bundle: &Container<Value>,
    skeleton: &Container<Value>,
) -> Result<(Value, Vec<f32>), String> {
    let c = &bundle.content;
    let skeleton_id = c["skeleton"].as_str().ok_or("no skeleton")?;
    let list = c["joints"].as_array().ok_or("no joints")?;
    let joints: Vec<Value> = list
        .iter()
        .map(|j| json!({ "name": j["name"], "parent": j["parent"], "bind": trs_json(&j["bind"]) }))
        .collect();
    let nodes: Vec<Node> =
        serde_json::from_value(Value::Array(joints.clone())).map_err(|e| format!("joints: {e}"))?;
    let binds: Vec<Trs> = nodes.iter().map(|n| n.bind).collect();
    let inverse: Vec<Mat4> = list
        .iter()
        .map(|j| mat4(&j["inverse_bind"]))
        .collect::<Result<_, _>>()?;
    let (table, clips) = clip_table(skeleton, &binds).map_err(|e| format!("{skeleton_id}: {e}"))?;
    let typed: BTreeMap<String, Clip> =
        serde_json::from_value(Value::Object(clips.clone())).map_err(|e| format!("clips: {e}"))?;
    // The finest tier's box in a pose: skinned under its joints' worlds.
    let tier = &c["tiers"][0];
    let positions = bundle.f32s(view_of(&tier["positions"])?)?;
    let bones = bundle.u8s(view_of(&tier["joints"])?)?;
    let weights = bundle.u16s(view_of(&tier["weights"])?)?;
    let posed = |pose: &Value| -> Result<Value, String> {
        let clip = pose["clip"].as_str().ok_or("a pose has no clip")?;
        let phase = pose["phase"].as_f64().ok_or("a pose has no phase")?;
        let mut worlds = Vec::new();
        joint_worlds(&nodes, &typed, &table, clip, phase, None, &mut worlds);
        let skinning: Vec<Mat4> = worlds
            .iter()
            .zip(&inverse)
            .map(|(w, i)| mul(w, i))
            .collect();
        let skinned = positions
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .map(|(v, p)| {
                let p = p.map(f64::from);
                let mut sum = [0.0; 3];
                for k in 0..4 {
                    let weight = f64::from(weights[v * 4 + k]) / 65535.0;
                    if weight != 0.0 {
                        let q = transform(&skinning[usize::from(bones[v * 4 + k])], p);
                        sum = [0, 1, 2].map(|i| sum[i] + q[i] * weight);
                    }
                }
                sum.map(|s| s as f32)
            });
        Ok(size_json(positions_bounds(skinned)))
    };
    let corpses = c["corpse_poses"].as_array().ok_or("no corpse poses")?;
    let mut bounds = vec![posed(&c["far_pose"])?];
    for pose in corpses {
        bounds.push(posed(pose)?);
    }
    let muzzle = c["sockets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|s| s["name"] == "muzzle")
        .map_or(
            Value::Null,
            |s| json!({ "joint": s["joint"], "offset": trs_json(&s["offset"]) }),
        );
    let record = json!({
        "kind": "skinned",
        "joints": joints,
        "clips": clips,
        "farPose": c["far_pose"],
        "corpsePoses": corpses,
        "muzzle": muzzle,
        "inverseBinds": list.iter().map(|j| j["inverse_bind"].clone()).collect::<Vec<_>>(),
        "tiers": c["tiers"].as_array().map_or(0, Vec::len),
        "bounds": bounds,
        "card": true,
    });
    Ok((record, table))
}

fn extra(node: &Value, key: &str) -> Option<f64> {
    node["extras"][key].as_f64()
}

/// The finest tier's positions of an articulated node.
fn node_positions(bundle: &Container<Value>, node: &Value) -> Result<Vec<f32>, String> {
    bundle.f32s(view_of(&node["tiers"][0]["positions"])?)
}

/// Largest distance of a node's finest-tier vertex from its local axis
/// along `axis` (1: +Y, a wheel's axle; 2: +Z, a rotor's mast).
fn reach(positions: &[f32], axis: usize) -> f64 {
    let (a, b) = if axis == 1 { (0, 2) } else { (0, 1) };
    let r2 = positions.as_chunks::<3>().0.iter().fold(0.0f64, |r2, p| {
        let (x, y) = (f64::from(p[a]), f64::from(p[b]));
        r2.max(x * x + y * y)
    });
    r2.sqrt()
}

/// A rotor's blades and mast, from its finest tier (`rotorShape`): its
/// vertices out past [`BLADE_OUTER`] of its reach grouped by angle round
/// its axis, and those near its axis its mast and head.
fn rotor_shape(positions: &[f32], radius: f64) -> Value {
    let points: Vec<[f64; 3]> = positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| p.map(f64::from))
        .collect();
    let near = |p: &[f64; 3]| libm::hypot(p[0], p[1]) < MAST_REACH * radius;
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in points.iter().filter(|p| near(p)) {
        (low, high) = (low.min(p[2]), high.max(p[2]));
    }
    let mast_radius = points
        .iter()
        .filter(|p| p[2] < low / 2.0 && near(p))
        .fold(0.0f64, |r, p| r.max(libm::hypot(p[0], p[1])));
    let (mast_below, mast_above, mast_radius) = if mast_radius > 0.0 {
        (-low, high.max(0.0), mast_radius)
    } else {
        (0.0, 0.0, 0.0)
    };
    let shape = |blades: usize, chord: f64, rest: f64| {
        json!({
            "blades": blades, "chord": chord, "rest": rest,
            "mast_below": mast_below, "mast_above": mast_above, "mast_radius": mast_radius,
        })
    };
    let mut at: Vec<(f64, f64, f64)> = points
        .iter()
        .filter(|p| libm::hypot(p[0], p[1]) > BLADE_OUTER * radius)
        .map(|p| (libm::atan2(p[1], p[0]), p[0], p[1]))
        .collect();
    if at.is_empty() {
        return shape(0, 0.0, 0.0);
    }
    at.sort_by(|m, n| m.0.total_cmp(&n.0));
    // Start a group after the widest gap, so none straddles the wrap.
    let mut start = 0;
    let mut widest = at[0].0 + 2.0 * PI - at[at.len() - 1].0;
    for i in 1..at.len() {
        if at[i].0 - at[i - 1].0 > widest {
            (widest, start) = (at[i].0 - at[i - 1].0, i);
        }
    }
    let gap_max = BLADE_GAP_DEG * PI / 180.0;
    let mut groups: Vec<Vec<(f64, f64, f64)>> = Vec::new();
    for k in 0..at.len() {
        let v = at[(start + k) % at.len()];
        match groups.last_mut() {
            Some(last) if (v.0 - last[last.len() - 1].0 + 2.0 * PI) % (2.0 * PI) < gap_max => {
                last.push(v)
            }
            _ => groups.push(vec![v]),
        }
    }
    let centre = |g: &[(f64, f64, f64)]| {
        let (x, y) = g.iter().fold((0.0, 0.0), |(x, y), v| (x + v.1, y + v.2));
        libm::atan2(y, x)
    };
    let chord = groups
        .iter()
        .map(|g| {
            let (s, c) = libm::sincos(centre(g));
            let across = g.iter().map(|v| -v.1 * s + v.2 * c);
            let (lo, hi) = across.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), a| {
                (lo.min(a), hi.max(a))
            });
            hi - lo
        })
        .fold(f64::NEG_INFINITY, f64::max);
    shape(groups.len(), chord, centre(&groups[0]))
}

/// A pitching node's own limits, else `fallback`, radians.
fn pitch_of(node: Option<&Value>, fallback: [f64; 2]) -> [f64; 2] {
    let deg = PI / 180.0;
    let (lo, hi) = match node {
        Some(n) => (extra(n, "pitch_min_deg"), extra(n, "pitch_max_deg")),
        None => (None, None),
    };
    if lo.is_none() && hi.is_none() {
        return fallback.map(|d| d * deg);
    }
    [
        lo.unwrap_or(fallback[0]) * deg,
        hi.unwrap_or(fallback[1]) * deg,
    ]
}

/// An articulated vehicle's record: its nodes, its rig (`articulationRig`)
/// and its box at rest.
pub(crate) fn articulated(bundle: &Container<Value>) -> Result<Value, String> {
    let list = bundle.content["nodes"].as_array().ok_or("no nodes")?;
    let find = |name: &str| list.iter().position(|n| n["name"] == name);
    let index = |i: Option<usize>| i.map_or(-1, |i| i as i64);
    let (gun, hmg_gun) = (find("gun"), find("hmg_gun"));
    let (mut wheels, mut tracks, mut rotors, mut deploy) = (vec![], vec![], vec![], vec![]);
    for (i, node) in list.iter().enumerate() {
        let name = node["name"].as_str().unwrap_or("");
        if name.starts_with("wheel_") {
            let radius = match extra(node, "radius_m") {
                Some(r) => r,
                None => reach(&node_positions(bundle, node)?, 1),
            };
            if radius > 0.0 {
                let left = node["pivot"][1].as_f64().unwrap_or(0.0) >= 0.0;
                wheels.push(json!({ "node": i, "radius": radius, "left": left }));
            }
        }
        if name.starts_with("rotor_") {
            let positions = node_positions(bundle, node)?;
            let radius = extra(node, "radius_m").unwrap_or_else(|| reach(&positions, 2));
            if radius > 0.0 {
                let mut rotor = rotor_shape(&positions, radius);
                rotor["node"] = json!(i);
                rotor["radius"] = json!(radius);
                rotors.push(rotor);
            }
        }
        if let Some(pitch) = extra(node, "link_pitch_m").filter(|&p| p > 0.0) {
            if name == "track_L" || name == "track_R" {
                tracks.push(json!({ "node": i, "left": name == "track_L", "linkPitch": pitch }));
            }
        }
        if let (Some(start), Some(end)) = (extra(node, "deploy_start"), extra(node, "deploy_end")) {
            if end > start {
                let e = |k: &str| extra(node, k).unwrap_or(0.0);
                deploy.push(json!({
                    "node": i, "start": start, "end": end,
                    "move": [e("deploy_move_x"), e("deploy_move_y"), e("deploy_move_z")],
                    "turn": [e("deploy_turn_x"), e("deploy_turn_y"), e("deploy_turn_z")],
                }));
            }
        }
    }
    let stroke = gun
        .and_then(|g| extra(&list[g], "recoil_max_m"))
        .filter(|s| s.is_finite());
    let muzzles: Map<String, Value> = MOUNT_MUZZLES
        .iter()
        .filter_map(|(role, muzzle)| Some((role.to_string(), json!(find(muzzle)?))))
        .collect();
    let rig = json!({
        "turret": index(find("turret")),
        "gun": index(gun),
        "hmg": index(find("hmg")),
        "hmgGun": index(hmg_gun),
        "pitch": {
            "gun": pitch_of(gun.map(|g| &list[g]), GUN_PITCH_DEG),
            "hmg": pitch_of(hmg_gun.map(|g| &list[g]), HMG_PITCH_DEG),
        },
        "stroke": stroke,
        "wheels": wheels,
        "tracks": tracks,
        "rotors": rotors,
        "deploy": deploy,
        "muzzles": muzzles,
    });
    let nodes: Vec<Value> = list
        .iter()
        .map(|n| json!({ "name": n["name"], "parent": n["parent"], "bind": trs_json(&n["bind"]) }))
        .collect();
    let typed: Vec<Node> =
        serde_json::from_value(Value::Array(nodes.clone())).map_err(|e| format!("nodes: {e}"))?;
    let mut worlds = Vec::new();
    let binds: Vec<Trs> = typed.iter().map(|n| n.bind).collect();
    node_worlds(&typed, &binds, None, &mut worlds);
    let mut placed = Vec::new();
    for (node, world) in list.iter().zip(&worlds) {
        for p in node_positions(bundle, node)?.as_chunks::<3>().0 {
            placed.push(transform(world, p.map(f64::from)).map(|v| v as f32));
        }
    }
    let box_ = positions_bounds(placed.into_iter());
    Ok(json!({
        "kind": "articulated",
        "nodes": nodes,
        "rig": rig,
        "tiers": list.first().and_then(|n| n["tiers"].as_array()).map_or(0, Vec::len),
        "bounds": [size_json(box_)],
        "plan": [box_[0][0], box_[0][1], box_[1][0], box_[1][1]],
    }))
}
