//! A felled tree's mesh posed as it falls and lies: the browser's
//! `felledPoint` (`scenery/felled.ts`), its vertex stage's CPU mirror, run
//! over every vertex of a tier. `world/felled.gd` owns the pose
//! (`fell_pose`) and when to bake; this does the per-vertex work.

use godot::prelude::*;

/// A landing tree's foot kicks off its stump this many trunk radii along the fall.
const KICK_RADII: f64 = 2.0;

/// A felled tree's pose (`FellPose`): `[toward x, toward y, angle, hinge,
/// trunk, squash, spread, landed, crown]`.
pub type Pose = [f64; 9];

fn past_trunk(d: f64, radius: f64, k: f64) -> f64 {
    let m = d.abs();
    d.signum() * (m.min(radius) + (m - radius).max(0.0) * k)
}

/// A point `p` of the tree, scaled and turned by its yaw, relative to its
/// foot (sim space; leaves when `foliage`), in pose `f` (`felledPoint`).
pub fn felled_point(p: [f64; 3], foliage: bool, f: &Pose) -> [f64; 3] {
    let (tx, ty, angle, hinge, trunk, squash, spread, landed, crown) = (f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
    let core = if foliage || p[2] > crown { 0.0 } else { trunk };
    let a = p[0] * tx + p[1] * ty;
    let (sx, sy) = (p[0] - tx * a, p[1] - ty * a);
    let side = (sx * sx + sy * sy).sqrt();
    let k = if side > 0.0 { past_trunk(side, core, spread) / side } else { 0.0 };
    let z = (p[2] - hinge).max(0.0);
    let (s, c) = angle.sin_cos();
    let along = a * c + z * s + KICK_RADII * trunk * landed;
    let up = z * c - past_trunk(a, core, squash) * s - (hinge - trunk) * landed;
    [tx * along + sx * k, ty * along + sy * k, hinge + up]
}

/// Poses felled trees' meshes.
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct FelledPose {}

#[godot_api]
impl FelledPose {
    /// `vertices` (one surface of a tier, Godot space, unscaled) as the tree
    /// packed in `pack` (a `model_batches.gd` instance: its yaw and scale, its
    /// foot) lies in pose `pose`, in world space.
    #[func]
    fn pose(&self, vertices: PackedVector3Array, foliage: bool, pack: PackedFloat32Array, pose: PackedFloat64Array) -> PackedVector3Array {
        let k = pack.as_slice();
        let f: Pose = pose.as_slice().try_into().expect("a felled pose is nine numbers");
        // Its scale across (sim x and y) and up (sim z), its yaw, its foot.
        let sxy = ((k[0] * k[0] + k[4] * k[4] + k[8] * k[8]) as f64).sqrt();
        let sz = ((k[1] * k[1] + k[5] * k[5] + k[9] * k[9]) as f64).sqrt();
        let yaw = (-(k[8] as f64) / sxy).atan2(k[0] as f64 / sxy);
        let (s, c) = yaw.sin_cos();
        let foot = [k[3] as f64, -(k[11] as f64), k[7] as f64];
        let out: Vec<Vector3> = vertices
            .as_slice()
            .iter()
            .map(|v| {
                // Godot (x, y, z) is sim (x, −z, y).
                let (x, y, z) = (v.x as f64 * sxy, -(v.z as f64) * sxy, v.y as f64 * sz);
                let p = felled_point([x * c - y * s, x * s + y * c, z], foliage, &f);
                let w = [foot[0] + p[0], foot[1] + p[1], foot[2] + p[2]];
                Vector3::new(w[0] as f32, w[2] as f32, -w[1] as f32)
            })
            .collect();
        PackedVector3Array::from(out.as_slice())
    }

    /// `normals` turned with the tree packed in `pack`: its yaw, then the
    /// fall of `pose` about the axis across it. Not pressed.
    #[func]
    fn turn(&self, normals: PackedVector3Array, pack: PackedFloat32Array, pose: PackedFloat64Array) -> PackedVector3Array {
        let k = pack.as_slice();
        let f = pose.as_slice();
        let sxy = ((k[0] * k[0] + k[4] * k[4] + k[8] * k[8]) as f64).sqrt();
        let yaw = (-(k[8] as f64) / sxy).atan2(k[0] as f64 / sxy);
        // The axis across the fall, sim (−ty, tx, 0), in Godot space (x, z, −y).
        let axis = [-f[1], 0.0, -f[0]];
        let norm = (axis[0] * axis[0] + axis[2] * axis[2]).sqrt().max(1e-12);
        let axis = [axis[0] / norm, 0.0, axis[2] / norm];
        let (sa, ca) = f[2].sin_cos();
        let (sy, cy) = yaw.sin_cos();
        let out: Vec<Vector3> = normals
            .as_slice()
            .iter()
            .map(|n| {
                // About Godot's +y by the yaw.
                let v = [n.x as f64 * cy + n.z as f64 * sy, n.y as f64, -(n.x as f64) * sy + n.z as f64 * cy];
                // Rodrigues about the axis by the fall's angle.
                let d = axis[0] * v[0] + axis[1] * v[1] + axis[2] * v[2];
                let cross = [axis[1] * v[2] - axis[2] * v[1], axis[2] * v[0] - axis[0] * v[2], axis[0] * v[1] - axis[1] * v[0]];
                let r = [0, 1, 2].map(|i| v[i] * ca + cross[i] * sa + axis[i] * d * (1.0 - ca));
                let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-12);
                Vector3::new((r[0] / l) as f32, (r[1] / l) as f32, (r[2] / l) as f32)
            })
            .collect();
        PackedVector3Array::from(out.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The browser's `fellPose` and `felledPoint` for a tree 12 m tall toward
    /// (0.6, 0.8): at 1, 2.8 and 5 s, four points (from the TypeScript).
    #[test]
    fn felled_points_are_the_browsers() {
        let poses: [(Pose, [[f64; 3]; 4]); 3] = [
            ([0.6, 0.8, 0.2361924969804743, 0.4, 0.3, 1.0, 1.0, 0.0, 4.0], [[0.09633515006749871, 0.1951135334233316, 0.3485194411788784], [0.5643776261984075, 0.38583683493121, 2.9184534603317402], [2.5453956514184672, 1.893860868557957, 7.484790186609964], [-0.5050804814757548, 2.993226024698994, 10.799302460852692]]),
            ([0.6, 0.8, 1.5563546270494681, 0.4, 0.3, 0.26980407583318466, 0.602955966234294, 0.9926100844142647, 4.0], [[0.3272458684922279, 0.5029944913229707, 0.08076193305591006], [2.0935235430631294, 2.424698057417506, 0.2982902768917322], [5.362256545651307, 6.245241444850302, 0.059783371436827604], [5.652008024524516, 9.7468492422251, 0.5617260640320374]]),
            ([0.6, 0.8, 1.5966612795880064, 0.4, 0.3, 0.26436781609195403, 0.6, 1.0, 4.0], [[0.3245862068965517, 0.4994482758620689, 0.08007358543362952], [2.094857522724545, 2.4264766969660605, 0.19277199985940593], [5.330302360855302, 6.207069814473736, -0.24011493184610655], [5.6680796085613405, 9.757439478081789, 0.13157382518341704]]),
        ];
        let points: [([f64; 3], bool); 4] = [([0.1, 0.2, 0.2], false), ([0.2, -0.1, 3.0], false), ([1.5, 0.5, 8.0], true), ([-2.0, 1.0, 11.0], true)];
        for (pose, want) in poses {
            for (k, (p, foliage)) in points.iter().enumerate() {
                let got = felled_point(*p, *foliage, &pose);
                for a in 0..3 {
                    assert!((got[a] - want[k][a]).abs() < 1e-9, "{got:?} != {:?}", want[k]);
                }
            }
        }
    }
}
