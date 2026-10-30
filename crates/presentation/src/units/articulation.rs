//! How an articulated model's parts move for an [`Articulation`]
//! (`articulation.ts`), over the rig its appearance carries:
//! - the turret yaws about its local +Z; the gun pitches about its local +Y
//!   within its rig's limits and recoils back along its own bore (local −X);
//! - the HMG yaws and pitches the same way, relative to what it rides;
//! - wheels roll about their axle (+Y) by travel over radius, each side by
//!   its own travel, so a tank turning in place counter-rotates them;
//! - tracks scroll their links by travel over link pitch (a material offset);
//! - rotors turn about +Z by the distance their tips swept over their reach,
//!   so every rotor's tips run at one speed;
//! - deploying parts move and turn over their own window of progress.
use super::pose::Articulation;
use crate::world::{Node, Quat, Rig, TrackSide, Trs, Vec3};

/// `a · b` (`quat.multiply`).
pub(crate) fn quat_mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        ax * bw + aw * bx + ay * bz - az * by,
        ay * bw + aw * by + az * bx - ax * bz,
        az * bw + aw * bz + ax * by - ay * bx,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn axis_angle(axis: Vec3, angle: f64) -> Quat {
    let (s, c) = libm::sincos(angle * 0.5);
    [s * axis[0], s * axis[1], s * axis[2], c]
}

/// `v` turned by `q` (`vec3.transformQuat`).
pub(crate) fn rotate(q: Quat, v: Vec3) -> Vec3 {
    let [qx, qy, qz, qw] = q;
    let uv = [
        qy * v[2] - qz * v[1],
        qz * v[0] - qx * v[2],
        qx * v[1] - qy * v[0],
    ];
    let uuv = [
        qy * uv[2] - qz * uv[1],
        qz * uv[0] - qx * uv[2],
        qx * uv[1] - qy * uv[0],
    ];
    let w2 = qw * 2.0;
    [
        v[0] + uv[0] * w2 + uuv[0] * 2.0,
        v[1] + uv[1] * w2 + uuv[1] * 2.0,
        v[2] + uv[2] * w2 + uuv[2] * 2.0,
    ]
}

/// Euler degrees about x, then y, then z (`quat.fromDegrees(…, "xyz")`).
fn from_degrees_xyz(x: f64, y: f64, z: f64) -> Quat {
    let half = |d: f64| libm::sincos(d * std::f64::consts::PI / 180.0 / 2.0);
    let ((s1, c1), (s2, c2), (s3, c3)) = (half(x), half(y), half(z));
    [
        s1 * c2 * c3 + c1 * s2 * s3,
        c1 * s2 * c3 - s1 * c2 * s3,
        c1 * c2 * s3 + s1 * s2 * c3,
        c1 * c2 * c3 - s1 * s2 * s3,
    ]
}

/// Fresh locals, one per node, holding each node's bind.
pub fn rest_locals(nodes: &[Node]) -> Vec<Trs> {
    nodes.iter().map(|n| n.bind).collect()
}

const Y: Vec3 = [0.0, 1.0, 0.0];
const Z: Vec3 = [0.0, 0.0, 1.0];

/// Every node's local for `input` into `out` (one per node).
pub fn articulate(out: &mut [Trs], nodes: &[Node], rig: &Rig, input: &Articulation) {
    for (local, node) in out.iter_mut().zip(nodes) {
        *local = node.bind;
    }
    // A node turned about its own axis from its bind.
    let turn = |out: &mut [Trs], node: usize, axis: Vec3, angle: f64| {
        out[node].r = quat_mul(nodes[node].bind.r, axis_angle(axis, angle));
    };
    let index = |i: i32| usize::try_from(i).ok();
    if let Some(turret) = index(rig.turret) {
        turn(out, turret, Z, input.turret_yaw);
    }
    if let Some(gun) = index(rig.gun) {
        turn(
            out,
            gun,
            Y,
            -input.gun_pitch.clamp(rig.pitch.gun[0], rig.pitch.gun[1]),
        );
        // Recoil runs the gun back along its own bore, pitched with it.
        let back = rotate(
            out[gun].r,
            [
                -input.recoil.min(rig.stroke.unwrap_or(f64::INFINITY)),
                0.0,
                0.0,
            ],
        );
        let t = &mut out[gun].t;
        *t = [t[0] + back[0], t[1] + back[1], t[2] + back[2]];
    }
    if let Some(hmg) = index(rig.hmg) {
        turn(out, hmg, Z, input.hmg_yaw);
    }
    if let Some(hmg_gun) = index(rig.hmg_gun) {
        turn(
            out,
            hmg_gun,
            Y,
            -input.hmg_pitch.clamp(rig.pitch.hmg[0], rig.pitch.hmg[1]),
        );
    }
    for wheel in &rig.wheels {
        let travel = if wheel.left {
            input.travel_l
        } else {
            input.travel_r
        };
        turn(out, wheel.node, Y, travel / wheel.radius);
    }
    for rotor in &rig.rotors {
        turn(out, rotor.node, Z, input.rotor / rotor.radius);
        if input.rotor_blur > 0.0 {
            out[rotor.node].s = [0.0; 3];
        }
    }
    for m in &rig.deploy {
        let s = ((input.deploy - m.start) / (m.end - m.start)).clamp(0.0, 1.0);
        if s == 0.0 {
            continue;
        }
        let local = &mut out[m.node];
        for i in 0..3 {
            local.t[i] += m.shift[i] * s;
        }
        local.r = quat_mul(
            local.r,
            from_degrees_xyz(m.turn[0] * s, m.turn[1] * s, m.turn[2] * s),
        );
    }
}

/// Each track's link offset, in links, for the material's scroll: `[left, right]`.
pub fn track_scroll(rig: &Rig, input: &Articulation) -> [f64; 2] {
    let of = |side, travel| {
        rig.tracks
            .iter()
            .find(|t| t.side == side)
            .map_or(0.0, |t| travel / t.link_pitch)
    };
    [
        of(TrackSide::Left, input.travel_l),
        of(TrackSide::Right, input.travel_r),
    ]
}
