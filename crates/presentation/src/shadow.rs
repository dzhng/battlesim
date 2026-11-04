//! The sun's shadow cascades, fitted when the camera changes for both clients: where
//! shadow receivers are in view depth (local terrain bounds clipped to the camera), the practical splits over that range, each cascade's light fitted
//! to its slice and snapped to its own texels, its biases and PCF radius, and
//! the records a renderer uploads as they are ([`SunShadowFrame`]).
//!
//! Matrices are held in f32 and vectors in f64, with the `math` package's
//! operation order shared by both clients. The splits run over the receiver
//! range, not from the camera's near
//! plane, so a camera a kilometre up does not spend its cascades on air.
use crate::light::{CascadeSettings, LightSettings};
use crate::records::{Camera as CameraRecord, SunCascade, SunShadow};
use crate::static_world::ground::partition::TerrainPartition;
use crate::view::{look_at, perspective_reverse_z, Camera};

/// Where an aircraft's shadow lies: thrown along the sun (`shadow_fall`, per
/// metre of height) from its actual height above the ground under it.
pub fn aircraft_shadow(position: [f64; 3], ground_z: f64, shadow_fall: [f64; 2]) -> [f64; 3] {
    let height = position[2] - ground_z;
    [
        position[0] + shadow_fall[0] * height,
        position[1] + shadow_fall[1] * height,
        ground_z,
    ]
}

/// Executed receiver work in the latest frame, never the cached fit's old cost.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct ReceiverStats {
    /// Hierarchy and leaf bounds tested by the spatial query.
    pub bounds_visited: usize,
    /// Individual triangle bounds tested against the camera.
    pub triangle_tests: usize,
    /// Surviving bounds passed to exact depth clipping.
    pub clip_tests: usize,
    pub reused: bool,
}

/// Installed immutable receiver inputs and their last camera fit.
pub(crate) struct ShadowFit {
    light: LightSettings,
    terrain: std::sync::Arc<TerrainPartition>,
    cached: Option<(Camera, SunShadowFrame)>,
    pub stats: ReceiverStats,
}

impl ShadowFit {
    pub fn new((light, terrain): (LightSettings, std::sync::Arc<TerrainPartition>)) -> Self {
        Self {
            light,
            terrain,
            cached: None,
            stats: ReceiverStats::default(),
        }
    }

    pub fn invalidate(&mut self) {
        self.cached = None;
        self.stats = ReceiverStats::default();
    }

    pub fn frame(&mut self, camera: &Camera) -> SunShadowFrame {
        self.stats = ReceiverStats::default();
        if let Some((previous, fit)) = &self.cached {
            if previous == camera {
                self.stats.reused = true;
                return fit.clone();
            }
        }
        let fit = fit_shadow(&self.light, camera, Some(&self.terrain), &mut self.stats);
        self.cached = Some((*camera, fit.clone()));
        fit
    }
}

/// The receiver layout's cascade slots: `SunShadow` and the depth array
/// hold this many. Cascade count belongs to the receiver layout.
pub const CASCADES: usize = 4;
pub const ALL_CASCADES: u32 = (1 << CASCADES) - 1;
/// How far above its slice's highest corner each cascade's light sits, so
/// casters just off screen still land in its depth range.
const LIGHT_MARGIN: f64 = 300.0;
/// A cascade light's near plane.
const LIGHT_NEAR: f64 = 1.0;
/// The far plane the fit unprojects the camera's corners with (its own
/// projection's is infinite).
const FAR_FALLBACK: f64 = 1e7;
/// The cascade light's pinned up: no other basis near a vertical sun.
const LIGHT_UP: [f64; 3] = [0.0, 1.0, 0.0];
/// Headroom above the highest ground for what stands on it (m).
const STANDING_M: f64 = 25.0;

type V3 = [f64; 3];
type M4 = [f32; 16];

/// The fitted cascade volumes used by both renderers, prepared once per walk.
pub struct CasterVolumes {
    planes: Vec<[crate::clip::ClipPlane; 6]>,
}

impl CasterVolumes {
    pub fn new(casters: &[crate::records::Camera]) -> Self {
        Self {
            planes: casters
                .iter()
                .map(|c| crate::clip::clip_planes(&c.view_proj))
                .collect(),
        }
    }

    /// Bit i is set when bounds meet cascade i, including boundary contact.
    pub fn mask(&self, bounds: &[f64; 6]) -> u32 {
        self.planes.iter().enumerate().fold(0, |mask, (i, planes)| {
            mask | (u32::from(crate::clip::intersects(bounds, planes)) << i)
        })
    }

    pub fn intersects(&self, bounds: &[f64; 6]) -> bool {
        self.planes
            .iter()
            .any(|p| crate::clip::intersects(bounds, p))
    }
}

/// One frame's sun shadow: the receiver block every lit shader reads, and
/// each fitted cascade's caster camera (`casters[i]` renders cascade `i`;
/// as many as are active).
#[derive(Clone, Debug, PartialEq)]
pub struct SunShadowFrame {
    pub receiver: SunShadow,
    pub casters: Vec<CameraRecord>,
}

impl SunShadowFrame {
    /// Before any fit: no cascade active, so no receiver is in one.
    pub fn cold(light: &LightSettings) -> Self {
        let mut receiver = SunShadow::default();
        for i in 0..CASCADES {
            inactive(&mut receiver, i);
        }
        receiver.control = [light.cascades.max_far_m as f32, 0.0, 0.0, 0.0];
        Self {
            receiver,
            casters: Vec::new(),
        }
    }
}

/// Fit over the immutable terrain owner; absent or offscreen receivers collapse
/// to just past the near plane. The splits, light, biases and records are shared.
pub fn sun_shadow(
    light: &LightSettings,
    camera: &Camera,
    terrain: Option<&TerrainPartition>,
) -> SunShadowFrame {
    fit_shadow(light, camera, terrain, &mut ReceiverStats::default())
}

fn fit_shadow(
    light: &LightSettings,
    camera: &Camera,
    terrain: Option<&TerrainPartition>,
    stats: &mut ReceiverStats,
) -> SunShadowFrame {
    let settings = &light.cascades;
    let [w, h] = camera.size.map(|s| f64::from(s.max(1)));
    let aspect = w / h;
    let view = look_at(camera.eye, camera.target, camera.up);
    let (near, far) = receiver_range(camera, &view, aspect, terrain, stats);

    let count = CASCADES;
    let map_size = settings.map_size.max(16);
    let projection_far = FAR_FALLBACK;
    let extent_far = projection_far.max(settings.max_far_m);
    let capped_far = projection_far.min(settings.max_far_m);
    let split_near = camera.near.max(near);
    let capped_far = (split_near * 1.5).max(capped_far.min(far));
    let breaks = breaks(split_near, capped_far, count, settings.split_lambda);
    let basis = Basis::toward(light.sun_direction());
    let world = invert(&view).unwrap_or(IDENTITY);
    let projection = perspective_reverse_z(camera.fov_y, aspect, camera.near, Some(projection_far));
    let inverse_projection = invert(&projection).unwrap_or(IDENTITY);

    // The near and far quads, unprojected from reverse-Z clip space (near
    // is z = 1, far z = 0); the far one pulled in to the receivers' far.
    let quad = |z: f64| {
        [[1.0, 1.0], [1.0, -1.0], [-1.0, -1.0], [-1.0, 1.0]]
            .map(|[x, y]| transform(&[x, y, z], &inverse_projection))
    };
    let at_depth = |depth: f64| quad(0.0).map(|v| scale(v, depth / v[2].abs()));
    let main_near = if split_near > camera.near {
        at_depth(split_near)
    } else {
        quad(1.0)
    };
    let main_far = at_depth(capped_far);

    let mut receiver = SunShadow::default();
    let mut casters = Vec::with_capacity(count);
    for i in 0..count {
        let lerp_quad =
            |t: f64| std::array::from_fn::<V3, 4, _>(|j| lerp(main_near[j], main_far[j], t));
        let slice_near = if i == 0 {
            main_near
        } else {
            lerp_quad(breaks[i - 1])
        };
        let slice_far = if i == count - 1 {
            main_far
        } else {
            lerp_quad(breaks[i])
        };
        let extent = extent(&slice_near, &slice_far, extent_far - split_near);
        let texel = extent / f64::from(map_size);
        let corners: [V3; 8] = std::array::from_fn(|k| {
            transform(
                if k < 4 {
                    &slice_near[k]
                } else {
                    &slice_far[k - 4]
                },
                &world,
            )
        });
        let (position, depth_span) = basis.centre(&corners, texel);
        // Deep enough for the slice seen along the sun, with the margin at both ends.
        let light_far = depth_span + 2.0 * LIGHT_MARGIN;
        let target = sub(position, basis.depth);
        let half = extent / 2.0;
        let light_view = look_at(position, target, LIGHT_UP);
        let light_projection =
            orthographic_reverse_z(-half, half, half, -half, LIGHT_NEAR, light_far);
        let view_projection = multiply(&light_projection, &light_view);
        let radius = (settings.softness_m / texel).clamp(1.0, 4.0);
        let interval = [if i == 0 { 0.0 } else { breaks[i - 1] }, breaks[i]];
        receiver.cascades[i] = SunCascade {
            matrix: view_projection,
            bias: [
                settings.depth_bias * (i + 1) as f64,
                normal_bias(settings, texel, radius),
                radius,
                0.0,
            ]
            .map(|v| v as f32),
            interval: [interval[0] as f32, interval[1] as f32, 0.0, 0.0],
        };
        let size = map_size as f32;
        casters.push(camera_record_of(
            &view_projection,
            position,
            LIGHT_NEAR,
            [size, size],
            0.0,
        ));
    }
    for i in count..CASCADES {
        inactive(&mut receiver, i);
    }
    receiver.control = [capped_far as f32, 0.0, split_near as f32, 0.0];
    SunShadowFrame { receiver, casters }
}

/// The camera uniform for a camera given by its view-projection (`Camera`
/// in every world shader): its inverse, eye, near, viewport and clock.
pub fn camera_record(
    view_proj: &[f32; 16],
    eye: [f32; 3],
    near: f32,
    size: [f32; 2],
    time: f32,
) -> CameraRecord {
    CameraRecord {
        view_proj: *view_proj,
        inv_view_proj: invert(view_proj).unwrap_or(IDENTITY),
        eye,
        znear: near,
        width: size[0],
        height: size[1],
        time,
        pad: 0.0,
        wind: [Default::default()],
        waves: [0.0; 4],
    }
}

fn camera_record_of(view_proj: &M4, eye: V3, near: f64, size: [f32; 2], time: f32) -> CameraRecord {
    camera_record(view_proj, eye.map(|v| v as f32), near as f32, size, time)
}

/// The box round a world mesh (ten floats a vertex, position first), a
/// metre below its lowest vertex and raised to hold what stands on the
/// ground; none for an empty mesh.
pub fn map_box(mesh: &[f32]) -> Option<[f64; 6]> {
    if mesh.is_empty() {
        return None;
    }
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for v in mesh.chunks(10) {
        for k in 0..3 {
            b[k] = b[k].min(f64::from(v[k]));
            b[k + 3] = b[k + 3].max(f64::from(v[k]));
        }
    }
    b[2] -= 1.0;
    b[5] += STANDING_M;
    Some(b)
}

/// Exact frustum clipping of conservative local receiver bounds. Unlike a
/// screen-ray grid this keeps thin/edge receivers and the near-plane interior.
fn receiver_range(
    camera: &Camera,
    view: &M4,
    aspect: f64,
    terrain: Option<&TerrainPartition>,
    stats: &mut ReceiverStats,
) -> (f64, f64) {
    let empty = (camera.near, camera.near * 2.0);
    let Some(terrain) = terrain else { return empty };
    let projection = perspective_reverse_z(camera.fov_y, aspect, camera.near, None);
    let matrix = multiply(&projection, view);
    let planes = crate::clip::clip_planes(&matrix);
    let inverse = invert(&matrix).unwrap_or(IDENTITY);
    let corners = [[-1., -1., 1.], [1., -1., 1.], [1., 1., 1.], [-1., 1., 1.]]
        .map(|p| transform(&p, &inverse));
    let (mut near, mut far) = (f64::INFINITY, 0.0f64);
    let (visited, triangles) = terrain.receiver_bounds(&planes, 1.0, STANDING_M, |bounds| {
        stats.clip_tests += 1;
        if let Some((lo, hi)) = crate::clip::depth_range(bounds, &planes, &corners, view) {
            near = near.min(lo);
            far = far.max(hi);
        }
    });
    stats.bounds_visited = visited;
    stats.triangle_tests = triangles;
    if near.is_finite() {
        (camera.near.max(near), far)
    } else {
        empty
    }
}

/// The unit direction from `eye` through the near-plane unprojection of an
/// NDC pixel (`screenRayFrom`).
pub(crate) fn screen_ray(inverse: &M4, eye: V3, x: f64, y: f64) -> V3 {
    let m = inverse.map(f64::from);
    let row = |r: usize| m[r] * x + m[4 + r] * y + m[8 + r] + m[12 + r];
    let w = row(3);
    let iw = if w != 0.0 { 1.0 / w } else { 0.0 };
    normalize([
        row(0) * iw - eye[0],
        row(1) * iw - eye[1],
        row(2) * iw - eye[2],
    ])
}

/// Practical splits: each internal break the midpoint of the uniform and
/// logarithmic depths at its fraction, as a fraction of the receiver range
/// (the receiver shader normalises its depth the same way); the last is 1.
fn breaks(near: f64, far: f64, count: usize, lambda: f64) -> Vec<f64> {
    let mut out: Vec<f64> = (1..count)
        .map(|i| {
            let uniform = i as f64 / count as f64;
            let logarithmic =
                (near * libm::pow(far / near, i as f64 / count as f64) - near) / (far - near);
            uniform + (logarithmic - uniform) * lambda
        })
        .collect();
    out.push(1.0);
    out
}

/// A slice's square extent: the longer of its far quad's diagonal and its
/// near-to-far diagonal, widened by the fade margin over `span` (the
/// unbounded far less the split near: the browser's source's asymmetry).
fn extent(near: &[V3; 4], far: &[V3; 4], span: f64) -> f64 {
    let anchor = far[0];
    let across = distance(anchor, far[2]);
    let diagonal = distance(anchor, near[2]);
    let linear_depth = far[0][2] / span;
    across.max(diagonal) + 0.25 * linear_depth * linear_depth * span
}

/// Normal offset (m): a few of the cascade's texels for each texel of PCF
/// radius, with a floor, so the wider near kernel never self-shadows lit
/// ground in a fine noise while near cascades keep contact.
fn normal_bias(settings: &CascadeSettings, texel: f64, radius: f64) -> f64 {
    settings
        .normal_bias_min_m
        .max(settings.normal_bias_texels * radius * texel)
}

/// An unfitted cascade's record: the identity over the interval [1, 1].
fn inactive(receiver: &mut SunShadow, i: usize) {
    let mut matrix = [0.0; 16];
    for d in [0, 5, 10, 15] {
        matrix[d] = 1.0;
    }
    receiver.cascades[i] = SunCascade {
        matrix,
        bias: [0.0; 4],
        interval: [1.0, 1.0, 0.0, 0.0],
    };
}

/// The light's orthonormal frame: `depth` points from the scene toward the
/// sun, `right` and `up` span its map.
struct Basis {
    right: V3,
    up: V3,
    depth: V3,
}

impl Basis {
    fn toward(sun: V3) -> Self {
        let depth = unit_or(sun, [0.0, 0.0, 1.0]);
        let right = unit_or(cross(LIGHT_UP, depth), [1.0, 0.0, 0.0]);
        Self {
            right,
            up: cross(depth, right),
            depth,
        }
    }

    /// The light's position for a slice's world corners: their light-space
    /// centre snapped down to the texel grid, raised up the sun ray past the
    /// highest corner by the margin; and the slice's depth along the sun.
    fn centre(&self, corners: &[V3; 8], texel: f64) -> (V3, f64) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for c in corners {
            for (k, axis) in [self.right, self.up, self.depth].iter().enumerate() {
                let v = dot(*c, *axis);
                if v < lo[k] {
                    lo[k] = v;
                }
                if v > hi[k] {
                    hi[k] = v;
                }
            }
        }
        let cx = ((lo[0] + hi[0]) / 2.0 / texel).floor() * texel;
        let cy = ((lo[1] + hi[1]) / 2.0 / texel).floor() * texel;
        let cz = hi[2] + LIGHT_MARGIN;
        let position = add_scaled(
            add_scaled(scale(self.right, cx), self.up, cy),
            self.depth,
            cz,
        );
        (position, hi[2] - lo[2])
    }
}

pub(crate) const IDENTITY: M4 = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Orthographic reverse-Z (`orthographicReverseZ`): near at depth 1, far 0.
fn orthographic_reverse_z(left: f64, right: f64, top: f64, bottom: f64, near: f64, far: f64) -> M4 {
    let mut m = IDENTITY;
    m[0] = (2.0 / (right - left)) as f32;
    m[5] = (2.0 / (top - bottom)) as f32;
    m[10] = (1.0 / (far - near)) as f32;
    m[12] = (-(right + left) / (right - left)) as f32;
    m[13] = (-(top + bottom) / (top - bottom)) as f32;
    m[14] = (far / (far - near)) as f32;
    m
}

/// `a · b`, column-major, in f64 and stored in f32 (`mat4.multiply`).
pub(crate) fn multiply(a: &M4, b: &M4) -> M4 {
    let (a, b) = (a.map(f64::from), b.map(f64::from));
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        let b = &b[c * 4..c * 4 + 4];
        (b[0] * a[r] + b[1] * a[4 + r] + b[2] * a[8 + r] + b[3] * a[12 + r]) as f32
    })
}

/// The inverse (`mat4.invert`), or none for a singular matrix.
pub(crate) fn invert(m: &M4) -> Option<M4> {
    let a = m.map(f64::from);
    let [a00, a01, a02, a03, a10, a11, a12, a13, a20, a21, a22, a23, a30, a31, a32, a33] = a;
    let b00 = a00 * a11 - a01 * a10;
    let b01 = a00 * a12 - a02 * a10;
    let b02 = a00 * a13 - a03 * a10;
    let b03 = a01 * a12 - a02 * a11;
    let b04 = a01 * a13 - a03 * a11;
    let b05 = a02 * a13 - a03 * a12;
    let b06 = a20 * a31 - a21 * a30;
    let b07 = a20 * a32 - a22 * a30;
    let b08 = a20 * a33 - a23 * a30;
    let b09 = a21 * a32 - a22 * a31;
    let b10 = a21 * a33 - a23 * a31;
    let b11 = a22 * a33 - a23 * a32;
    let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    if det == 0.0 || det.is_nan() {
        return None;
    }
    let det = 1.0 / det;
    Some(
        [
            a11 * b11 - a12 * b10 + a13 * b09,
            a02 * b10 - a01 * b11 - a03 * b09,
            a31 * b05 - a32 * b04 + a33 * b03,
            a22 * b04 - a21 * b05 - a23 * b03,
            a12 * b08 - a10 * b11 - a13 * b07,
            a00 * b11 - a02 * b08 + a03 * b07,
            a32 * b02 - a30 * b05 - a33 * b01,
            a20 * b05 - a22 * b02 + a23 * b01,
            a10 * b10 - a11 * b08 + a13 * b06,
            a01 * b08 - a00 * b10 - a03 * b06,
            a30 * b04 - a31 * b02 + a33 * b00,
            a21 * b02 - a20 * b04 - a23 * b00,
            a11 * b07 - a10 * b09 - a12 * b06,
            a00 * b09 - a01 * b07 + a02 * b06,
            a31 * b01 - a30 * b03 - a32 * b00,
            a20 * b03 - a21 * b01 + a22 * b00,
        ]
        .map(|v| (v * det) as f32),
    )
}

/// A point through a matrix, divided by its w (`vec3.transformMat4`).
fn transform(p: &V3, m: &M4) -> V3 {
    let m = m.map(f64::from);
    let [x, y, z] = *p;
    let w = m[3] * x + m[7] * y + m[11] * z + m[15];
    let w = if w == 0.0 || w.is_nan() { 1.0 } else { w };
    [
        (m[0] * x + m[4] * y + m[8] * z + m[12]) / w,
        (m[1] * x + m[5] * y + m[9] * z + m[13]) / w,
        (m[2] * x + m[6] * y + m[10] * z + m[14]) / w,
    ]
}

fn scale(v: V3, s: f64) -> V3 {
    v.map(|c| c * s)
}

fn add_scaled(a: V3, b: V3, s: f64) -> V3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn lerp(a: V3, b: V3, t: f64) -> V3 {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn distance(a: V3, b: V3) -> f64 {
    let d = sub(b, a);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// `vec3.normalize`: the zero vector stays zero.
fn normalize(v: V3) -> V3 {
    let mut len = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if len > 0.0 {
        len = 1.0 / len.sqrt();
    }
    scale(v, len)
}

/// `v` made unit, or `fallback` when it is too short to have a direction.
fn unit_or(v: V3, fallback: V3) -> V3 {
    if (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() > 1e-9 {
        normalize(v)
    } else {
        fallback
    }
}
