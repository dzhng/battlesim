//! What a frame's camera means for everything Rust chooses by view: where the
//! eye is, how many pixels a metre covers at a metre's distance, the view's
//! side planes, and the matrices (`detailView.ts`, `camera3d.ts`). The one
//! tier rule every drawn model takes is [`model_detail`]
//! (`modelDetail.ts`): nothing off screen, one of four mesh tiers by
//! projected height, or the impostor card.
use crate::world::{Detail, Vec3};

/// 18b's camera input (sim space).
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: [f32; 3],
    pub fov_y: f32,
    pub near: f32,
    /// Drawing size in device pixels.
    pub size: [u32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub constant: f64,
}

#[derive(Clone, Debug)]
pub struct View {
    pub eye: Vec3,
    pub target: Vec3,
    /// Eye to target, metres.
    pub distance: f64,
    /// Device pixels per metre at one metre: `height / (2·tan(fovY/2))`.
    pub pixels_per_metre: f64,
    /// The four side planes (near and far are not tested).
    pub sides: [Plane; 4],
    /// Sim → view, column-major, float32 as the GPU holds it.
    pub view: [f32; 16],
    /// Reverse-Z, infinite far, column-major.
    pub projection: [f32; 16],
}

pub const CULLED: i32 = -1;
pub const IMPOSTOR: i32 = 4;
/// A model's sun shadow reaches this far past it.
pub const SHADOW_MARGIN_M: f64 = 3.0;

fn look_at(eye: Vec3, center: Vec3, up: [f64; 3]) -> [f32; 16] {
    let mut z = [eye[0] - center[0], eye[1] - center[1], eye[2] - center[2]];
    if z.iter().all(|c| c.abs() < 1e-6) {
        let mut m = [0.0; 16];
        (m[0], m[5], m[10], m[15]) = (1.0, 1.0, 1.0, 1.0);
        return m;
    }
    let len = 1.0 / (z[0] * z[0] + z[1] * z[1] + z[2] * z[2]).sqrt();
    z = [z[0] * len, z[1] * len, z[2] * len];
    let unit = |v: [f64; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len == 0.0 {
            [0.0; 3]
        } else {
            let inv = 1.0 / len;
            [v[0] * inv, v[1] * inv, v[2] * inv]
        }
    };
    let x = unit([
        up[1] * z[2] - up[2] * z[1],
        up[2] * z[0] - up[0] * z[2],
        up[0] * z[1] - up[1] * z[0],
    ]);
    let y = unit([
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ]);
    let dot = |a: [f64; 3]| -(a[0] * eye[0] + a[1] * eye[1] + a[2] * eye[2]);
    [
        x[0], y[0], z[0], 0.0, x[1], y[1], z[1], 0.0, x[2], y[2], z[2], 0.0, dot(x), dot(y), dot(z),
        1.0,
    ]
    .map(|v| v as f32)
}

fn perspective_reverse_z(fov_y: f64, aspect: f64, near: f64) -> [f32; 16] {
    let f = 1.0 / (fov_y / 2.0).tan();
    let mut m = [0.0f32; 16];
    m[0] = (f / aspect) as f32;
    m[5] = f as f32;
    m[11] = -1.0;
    m[14] = near as f32;
    m
}

impl View {
    pub fn new(camera: &Camera) -> Self {
        let [w, h] = camera.size;
        let (w, h) = (f64::from(w.max(1)), f64::from(h.max(1)));
        let fov = f64::from(camera.fov_y);
        let up = camera.up.map(f64::from);
        let view = look_at(camera.eye, camera.target, up);
        let projection = perspective_reverse_z(fov, w / h, f64::from(camera.near));
        let (p, v) = (projection.map(f64::from), view.map(f64::from));
        // Rows 0, 1 and 3 of projection · view.
        let row = |r: usize| -> [f64; 4] {
            std::array::from_fn(|c| (0..4).map(|k| p[k * 4 + r] * v[c * 4 + k]).sum())
        };
        let (r0, r1, r3) = (row(0), row(1), row(3));
        let plane = |a: [f64; 4], b: [f64; 4], sign: f64| {
            let n = [a[0] + sign * b[0], a[1] + sign * b[1], a[2] + sign * b[2]];
            let c = a[3] + sign * b[3];
            let mut inv = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
            if inv > 0.0 {
                inv = 1.0 / inv.sqrt();
            }
            Plane {
                normal: [n[0] * inv, n[1] * inv, n[2] * inv],
                constant: c * inv,
            }
        };
        let d = [
            camera.target[0] - camera.eye[0],
            camera.target[1] - camera.eye[1],
            camera.target[2] - camera.eye[2],
        ];
        Self {
            eye: camera.eye,
            target: camera.target,
            distance: (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
            pixels_per_metre: h / (2.0 * (fov / 2.0).tan()),
            sides: [
                plane(r3, r0, 1.0),
                plane(r3, r0, -1.0),
                plane(r3, r1, 1.0),
                plane(r3, r1, -1.0),
            ],
            view,
            projection,
        }
    }

    /// Whether a sphere reaches into the view's sides.
    pub fn sees_sphere(&self, c: Vec3, radius: f64) -> bool {
        self.sides.iter().all(|p| {
            p.normal[0] * c[0] + p.normal[1] * c[1] + p.normal[2] * c[2] + p.constant >= -radius
        })
    }
}

/// The tier (0–3) or [`IMPOSTOR`] for a model `size` metres tall at `distance`.
pub fn detail_at(detail: &Detail, view: &View, size: f64, distance: f64, impostor: bool) -> i32 {
    let px = size * view.pixels_per_metre / distance.max(1e-3);
    let [t0, t1, t2] = detail.lod_px;
    if px > t0 {
        0
    } else if px > t1 {
        1
    } else if px > t2 {
        2
    } else if impostor && px < detail.impostor_px {
        IMPOSTOR
    } else {
        3
    }
}

/// [`CULLED`], a tier, or [`IMPOSTOR`] for a model standing at `at`, `size`
/// metres tall and within `radius` of its centre (half its size up). A model
/// in the air casts its shadow far from itself: given where its foot's
/// shadow lands (`shadow_at`), it draws while either is in view.
pub fn model_detail(
    detail: &Detail,
    view: &View,
    at: Vec3,
    size: f64,
    radius: f64,
    impostor: bool,
    shadow_at: Option<Vec3>,
) -> i32 {
    let c = [at[0], at[1], at[2] + size / 2.0];
    let r = radius + SHADOW_MARGIN_M;
    if !view.sees_sphere(c, r) {
        let Some(s) = shadow_at else { return CULLED };
        if !view.sees_sphere([s[0], s[1], s[2] + size / 2.0], r) {
            return CULLED;
        }
    }
    let d = [c[0] - view.eye[0], c[1] - view.eye[1], c[2] - view.eye[2]];
    detail_at(
        detail,
        view,
        size,
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
        impostor,
    )
}
