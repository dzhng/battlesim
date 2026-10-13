//! What a frame's camera means for everything Rust chooses by view: where the
//! eye is, how many pixels a metre covers at a metre's distance, the view's
//! side planes, and the matrices, the camera uniform and the sky's rays a
//! renderer uploads as they are (`camera3d.ts`' math, to the bit). The one
//! tier rule every drawn model takes is [`model_detail`]
//! (`modelDetail.ts`): nothing off screen, one of four mesh tiers by
//! projected height, or the impostor card; and what it casts with,
//! [`caster_tier`].
use crate::records::{
    Camera as CameraRecord, View as ViewRecord, FOG_FACES, FOG_GROUND, FOG_PAINTED_FACES,
    FOG_UNITS, NONE, VIEW_PASS,
};
use crate::shadow::{camera_record, invert, multiply, screen_ray, IDENTITY};
use crate::world::{Detail, Vec3};

/// The camera a frame is drawn for (sim space).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f64,
    pub near: f64,
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
/// Mesh tiers every appearance carries, finest first (`TIER_COUNT`).
pub const TIERS: usize = 4;

/// The tier a model drawn at `tier` casts into the sun's cascades with: one
/// coarser, since a cascade texel is larger than the detail between tiers.
pub fn caster_tier(tier: usize) -> usize {
    (tier + 1).min(TIERS - 1)
}
/// What a batch of models at `tier` drawn for `pass` casts with
/// (`Batch.caster`): in view, [`caster_tier`]; an impostor card and a ghost
/// nothing (`NONE`). A battle's models and a lab's own take it alike.
pub fn batch_caster(tier: u32, pass: u32) -> u32 {
    if pass == VIEW_PASS && tier != IMPOSTOR as u32 {
        caster_tier(tier as usize) as u32
    } else {
        NONE
    }
}
/// How fog of war takes a model (`Batch.fog`), by what it is: a posed body
/// or a whole vehicle is a unit, drawn only while identified; remains
/// (`lies`) are seen or unseen whole, as the ground under them; a model in a
/// `state` (a building's piece, a prop, a thrown piece) is the world's faces,
/// with the ground's paint when movers stand on it (`ground`).
pub fn model_fog(lies: bool, state: bool, ground: bool) -> u32 {
    match (lies, state, ground) {
        (true, ..) => FOG_GROUND,
        (_, false, _) => FOG_UNITS,
        (_, true, true) => FOG_PAINTED_FACES,
        (_, true, false) => FOG_FACES,
    }
}
/// A model's sun shadow reaches this far past it.
pub const SHADOW_MARGIN_M: f64 = 3.0;

/// `math`'s `mat4.lookAt`, stored in f32 as the GPU holds it.
pub(crate) fn look_at(eye: Vec3, center: Vec3, up: [f64; 3]) -> [f32; 16] {
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
        x[0],
        y[0],
        z[0],
        0.0,
        x[1],
        y[1],
        z[1],
        0.0,
        x[2],
        y[2],
        z[2],
        0.0,
        dot(x),
        dot(y),
        dot(z),
        1.0,
    ]
    .map(|v| v as f32)
}

/// Reverse-Z perspective (`perspectiveReverseZ`): near at depth 1, `far` at
/// 0, or an infinite far plane without one; f32 as the GPU holds it.
pub(crate) fn perspective_reverse_z(
    fov_y: f64,
    aspect: f64,
    near: f64,
    far: Option<f64>,
) -> [f32; 16] {
    let f = 1.0 / libm::tan(fov_y / 2.0);
    let mut m = [0.0f32; 16];
    m[0] = (f / aspect) as f32;
    m[5] = f as f32;
    m[11] = -1.0;
    m[10] = far.map_or(0.0, |far| (near / (far - near)) as f32);
    m[14] = far.map_or(near, |far| near * far / (far - near)) as f32;
    m
}

impl View {
    pub fn new(camera: &Camera) -> Self {
        let [w, h] = camera.size;
        let (w, h) = (f64::from(w.max(1)), f64::from(h.max(1)));
        let fov = camera.fov_y;
        let view = look_at(camera.eye, camera.target, camera.up);
        let projection = perspective_reverse_z(fov, w / h, camera.near, None);
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
            pixels_per_metre: h / (2.0 * libm::tan(fov / 2.0)),
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

    /// Whether a box (min x, y, z, max x, y, z) meets the view's sides: each
    /// has its nearest corner inside.
    pub fn sees_box(&self, b: &[f64; 6]) -> bool {
        self.sides.iter().all(|p| {
            let n = p.normal;
            let at = |k: usize| if n[k] >= 0.0 { b[k + 3] } else { b[k] };
            n[0] * at(0) + n[1] * at(1) + n[2] * at(2) + p.constant >= 0.0
        })
    }

    /// The eye's distance to a box's nearest point.
    pub fn distance_to_box(&self, b: &[f64; 6]) -> f64 {
        let d = |k: usize| self.eye[k].min(b[k + 3]).max(b[k]) - self.eye[k];
        let (x, y, z) = (d(0), d(1), d(2));
        (x * x + y * y + z * z).sqrt()
    }

    /// Whether a sphere reaches into the view's sides.
    pub fn sees_sphere(&self, c: Vec3, radius: f64) -> bool {
        self.sides.iter().all(|p| {
            p.normal[0] * c[0] + p.normal[1] * c[1] + p.normal[2] * c[2] + p.constant >= -radius
        })
    }

    /// The camera every world shader binds (`Camera`) at `time` seconds.
    pub fn camera(&self, camera: &Camera, time: f64) -> CameraRecord {
        let size = camera.size.map(|s| s.max(1) as f32);
        camera_record(
            &self.view_proj(),
            self.eye.map(|v| v as f32),
            camera.near as f32,
            size,
            time as f32,
        )
    }

    /// Projection · view, as the GPU holds it.
    pub fn view_proj(&self) -> [f32; 16] {
        multiply(&self.projection, &self.view)
    }

    /// The frame's `View` record. The sky's rays are the unit rays from the
    /// eye through the top-left, top-right and bottom-left corners of the
    /// near plane (`screenRay`); the shader interpolates them before
    /// normalising.
    pub fn record(&self, camera: &Camera) -> ViewRecord {
        let [w, h] = camera.size.map(|s| f64::from(s.max(1)));
        let inverse = invert(&self.view_proj()).unwrap_or(IDENTITY);
        let corner = |x: f64, y: f64| screen_ray(&inverse, self.eye, x, y);
        let (origin, across, down) = (corner(-1.0, 1.0), corner(1.0, 1.0), corner(-1.0, -1.0));
        let ray = |d: [f64; 3]| [d[0] as f32, d[1] as f32, d[2] as f32, 0.0];
        let point = |p: Vec3| [p[0] as f32, p[1] as f32, p[2] as f32, 1.0];
        ViewRecord {
            view: self.view,
            projection: self.projection,
            eye: point(self.eye),
            target: point(self.target),
            size: [
                w as f32,
                h as f32,
                (1.0 / h) as f32,
                self.pixels_per_metre as f32,
            ],
            sky_origin: ray(origin),
            sky_dx: ray(std::array::from_fn(|i| across[i] - origin[i])),
            sky_dy: ray(std::array::from_fn(|i| down[i] - origin[i])),
        }
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

/// The buildings' tier (`presentation.buildings.lod_px_per_m`): a chunk
/// `distance` metres off at its nearest point takes tier t while a metre there
/// covers more than `lod[t]` pixels, else the coarsest. A window is the same
/// size on a house and a tower, so it follows distance, not height.
pub fn building_tier(view: &View, distance: f64, lod: &[f64; 3]) -> usize {
    let px = view.pixels_per_metre / distance.max(1e-3);
    lod.iter().position(|&t| px > t).unwrap_or(lod.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::GHOST_PASS;

    #[test]
    fn a_model_in_view_casts_from_the_next_coarser_tier_and_a_card_or_ghost_casts_nothing() {
        assert_eq!(batch_caster(0, VIEW_PASS), 1);
        assert_eq!(batch_caster(2, VIEW_PASS), 3);
        // The coarsest tier has none coarser: it casts from itself.
        assert_eq!(batch_caster(3, VIEW_PASS), 3);
        assert_eq!(batch_caster(IMPOSTOR as u32, VIEW_PASS), NONE);
        assert_eq!(batch_caster(1, GHOST_PASS), NONE);
    }

    #[test]
    fn fog_takes_a_model_by_what_it_is() {
        // A posed body or a whole vehicle is a unit: never fogged.
        assert_eq!(model_fog(false, false, false), FOG_UNITS);
        // Remains lie seen or unseen whole, as the ground under them.
        assert_eq!(model_fog(true, false, false), FOG_GROUND);
        // A model in a state is the world's: face by face, with the ground's
        // paint when movers stand on it.
        assert_eq!(model_fog(false, true, false), FOG_FACES);
        assert_eq!(model_fog(false, true, true), FOG_PAINTED_FACES);
    }
}
