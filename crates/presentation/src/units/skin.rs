//! Joint and part transforms for drawing: a skinned body's joints sampled
//! from its dense clip table (`clipTable.ts` builds it at install), and an
//! articulated model's node worlds
//! from its posed locals. A renderer draws a body by its
//! palette, each joint's `world · inverseBind`, and a vehicle by its node
//! worlds; neither knows about clips.
//!
//! Matrices are column-major `[f64; 16]`; what a renderer takes is the
//! [`affine`] of one, its basis rows and origin as float32.
use std::collections::BTreeMap;

use super::pose::ClipBlend;
use crate::world::{Clip, Node, Quat, Trs, Vec3};

pub type Mat4 = [f64; 16];

/// Floats per joint sample in a clip table: rotation (x, y, z, w), then
/// translation and uniform scale.
const SAMPLE_FLOATS: usize = 8;

/// `a · b` (`mat4.multiply`).
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0; 16];
    for c in 0..4 {
        let [b0, b1, b2, b3] = [b[c * 4], b[c * 4 + 1], b[c * 4 + 2], b[c * 4 + 3]];
        for r in 0..4 {
            out[c * 4 + r] = b0 * a[r] + b1 * a[4 + r] + b2 * a[8 + r] + b3 * a[12 + r];
        }
    }
    out
}

/// A local's matrix (`mat4.fromRotationTranslationScale`). The rotation is
/// taken as it is: a quantised sample a few ulps off unit length scales by
/// as much, as the kernel's does.
#[rustfmt::skip]
pub fn trs(t: Vec3, q: Quat, s: Vec3) -> Mat4 {
    let [x, y, z, w] = q;
    let (x2, y2, z2) = (x + x, y + y, z + z);
    let (xx, xy, xz) = (x * x2, x * y2, x * z2);
    let (yy, yz, zz) = (y * y2, y * z2, z * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);
    [
        (1.0 - (yy + zz)) * s[0], (xy + wz) * s[0], (xz - wy) * s[0], 0.0,
        (xy - wz) * s[1], (1.0 - (xx + zz)) * s[1], (yz + wx) * s[1], 0.0,
        (xz + wy) * s[2], (yz - wx) * s[2], (1.0 - (xx + yy)) * s[2], 0.0,
        t[0], t[1], t[2], 1.0,
    ]
}

fn trs_of(local: &Trs) -> Mat4 {
    trs(local.t, local.r, local.s)
}

/// The inverse of an affine transform.
fn invert_affine(m: &Mat4) -> Mat4 {
    let (a, b, c) = ([m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]);
    // Rows of the inverse 3×3 are the cross products of its columns over the determinant.
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let (r0, r1, r2) = (cross(b, c), cross(c, a), cross(a, b));
    let det = a[0] * r0[0] + a[1] * r0[1] + a[2] * r0[2];
    let rows = [
        r0.map(|v| v / det),
        r1.map(|v| v / det),
        r2.map(|v| v / det),
    ];
    let mut out = [0.0; 16];
    for (r, row) in rows.iter().enumerate() {
        out[r] = row[0];
        out[4 + r] = row[1];
        out[8 + r] = row[2];
        out[12 + r] = -(row[0] * m[12] + row[1] * m[13] + row[2] * m[14]);
    }
    out[15] = 1.0;
    out
}

/// The 3×4 affine a renderer takes: rows basis x, basis y, basis z, origin.
pub fn affine(m: &Mat4) -> [f32; 12] {
    [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14].map(|i| m[i] as f32)
}

/// The two frames either side of `phase` (0..1 of the clip) and the weight
/// of the second (`clipFrames`). A looping clip wraps its phase (the bake
/// samples both ends, so they agree); a one-shot clip holds its ends.
pub fn clip_frames(clip: &Clip, phase: f64) -> (usize, usize, f64) {
    let last = clip.frames.saturating_sub(1);
    if last == 0 {
        return (0, 0, 0.0);
    }
    let p = if clip.looping {
        phase - phase.floor()
    } else {
        phase.clamp(0.0, 1.0)
    };
    let frame = (p * clip.duration * clip.rate).min(last as f64);
    let f0 = (frame.floor() as usize).min(last);
    (f0, (f0 + 1).min(last), frame - f0 as f64)
}

/// One joint's sampled local: rotation, translation, uniform scale.
#[derive(Clone, Copy)]
struct Local {
    q: Quat,
    t: Vec3,
    s: f64,
}

/// The kernel's shortest-arc slerp (`slerpQ`), normalised.
fn slerp(a: Quat, b: Quat, w: f64) -> Quat {
    if w <= 0.0 {
        return a;
    }
    if w >= 1.0 {
        return b;
    }
    let mut cosine = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if cosine < 0.0 {
        cosine = -cosine;
        b.map(|v| -v)
    } else {
        b
    };
    let (ka, kb) = if cosine > 0.9995 {
        (1.0 - w, w)
    } else {
        let theta = (1.0 - cosine * cosine).max(0.0).sqrt().atan2(cosine);
        (((1.0 - w) * theta).sin(), (w * theta).sin())
    };
    let q = [0, 1, 2, 3].map(|i| a[i] * ka + b[i] * kb);
    let length = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    q.map(|v| v / if length > 0.0 { length } else { 1.0 })
}

fn mix(a: Local, b: Local, w: f64) -> Local {
    let lerp = |x: f64, y: f64| x + (y - x) * w;
    Local {
        q: slerp(a.q, b.q, w),
        t: [
            lerp(a.t[0], b.t[0]),
            lerp(a.t[1], b.t[1]),
            lerp(a.t[2], b.t[2]),
        ],
        s: lerp(a.s, b.s),
    }
}

/// Joint `j` of a `joints`-joint body in a clip starting at float `offset`
/// of `table`, at frames `(f0, f1, w)`.
fn sample(
    table: &[f32],
    offset: usize,
    (f0, f1, w): (usize, usize, f64),
    j: usize,
    joints: usize,
) -> Local {
    let read = |f: usize| {
        let o = offset + (f * joints + j) * SAMPLE_FLOATS;
        let v = |i: usize| f64::from(table[o + i]);
        Local {
            q: [v(0), v(1), v(2), v(3)],
            t: [v(4), v(5), v(6)],
            s: v(7),
        }
    };
    mix(read(f0), read(f1), w)
}

/// A body's joint worlds (model space) playing `clip` at `phase`, faded
/// over by `blend`, into `out`: its bind pose where it lacks the clip.
/// `table` is its clip table (`Appearance::Skinned::table`).
pub fn joint_worlds(
    joints: &[Node],
    clips: &BTreeMap<String, Clip>,
    table: &[f32],
    clip: &str,
    phase: f64,
    blend: Option<&ClipBlend>,
    out: &mut Vec<Mat4>,
) {
    out.clear();
    let n = joints.len();
    let main = clips.get(clip).map(|c| (c.offset, clip_frames(c, phase)));
    let fade = blend.filter(|b| b.weight > 0.0).and_then(|b| {
        clips
            .get(&*b.clip)
            .map(|c| (c.offset, clip_frames(c, b.phase), b.weight))
    });
    for (j, joint) in joints.iter().enumerate() {
        let m = match main {
            None => trs_of(&joint.bind),
            Some((offset, frames)) => {
                let mut local = sample(table, offset, frames, j, n);
                if let Some((offset, frames, weight)) = fade {
                    local = mix(local, sample(table, offset, frames, j, n), weight);
                }
                trs(local.t, local.q, [local.s; 3])
            }
        };
        let world = match usize::try_from(joint.parent) {
            Ok(parent) => mul(&out[parent], &m),
            Err(_) => m,
        };
        out.push(world);
    }
}

/// Each joint's inverse bind: `given` where the appearance carries them,
/// else the inverse of its bind world (the chain of bind locals).
pub fn inverse_binds(joints: &[Node], given: &[Mat4]) -> Vec<Mat4> {
    if !given.is_empty() {
        return given.to_vec();
    }
    let mut worlds = Vec::with_capacity(joints.len());
    node_worlds(
        joints,
        &joints.iter().map(|j| j.bind).collect::<Vec<_>>(),
        None,
        &mut worlds,
    );
    worlds.iter().map(invert_affine).collect()
}

/// A body's palette: each joint's `world · inverseBind`, appended to `out`.
pub fn palette(worlds: &[Mat4], inverse_binds: &[Mat4], out: &mut Vec<[f32; 12]>) {
    out.extend(
        worlds
            .iter()
            .zip(inverse_binds)
            .map(|(w, ib)| affine(&mul(w, ib))),
    );
}

/// An articulated model's node worlds from its locals (parents first), into
/// `out`. A whole-model motion (a falling airframe's tilt) carries every root.
pub fn node_worlds(nodes: &[Node], locals: &[Trs], motion: Option<&Mat4>, out: &mut Vec<Mat4>) {
    out.clear();
    for (node, local) in nodes.iter().zip(locals) {
        let m = trs_of(local);
        let world = match (usize::try_from(node.parent), motion) {
            (Ok(parent), _) => mul(&out[parent], &m),
            (Err(_), Some(motion)) => mul(motion, &m),
            (Err(_), None) => m,
        };
        out.push(world);
    }
}
