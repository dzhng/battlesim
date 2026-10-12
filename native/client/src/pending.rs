//! TEMPORARY: the presentation crate's API the browser-first lane is adding
//! (shadow fit, ground meshes, grass field), stubbed with the agreed
//! signatures so the native passes type-check. Deleted at the merge.
#![allow(dead_code, unused_variables)]

pub mod records {
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct Camera {
        pub view_proj: [f32; 16],
        pub inv_view_proj: [f32; 16],
        pub eye: [f32; 3],
        pub znear: f32,
        pub width: f32,
        pub height: f32,
        pub time: f32,
        pub pad: f32,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct SunShadow {
        pub cascades: [f32; 96],
        pub control: [f32; 4],
    }
}

pub mod shadow {
    use super::records::{Camera, SunShadow};
    use presentation::light::LightSettings;
    pub struct SunShadowFrame {
        pub receiver: SunShadow,
        pub casters: Vec<Camera>,
    }
    impl SunShadowFrame {
        pub fn cold(light: &LightSettings) -> Self {
            let mut cascades = [0.0f32; 96];
            for c in 0..4 {
                for d in [0, 5, 10, 15] {
                    cascades[c * 24 + d] = 1.0;
                }
                cascades[c * 24 + 20] = 1.0;
                cascades[c * 24 + 21] = 1.0;
            }
            Self {
                receiver: SunShadow {
                    cascades,
                    control: [2600.0, 0.0, 0.0, 0.0],
                },
                casters: vec![],
            }
        }
    }
    pub fn camera_record(
        view_proj: &[f32; 16],
        eye: [f32; 3],
        near: f32,
        size: [f32; 2],
        time: f32,
    ) -> Camera {
        todo!()
    }
    pub fn sun_shadow(
        light: &LightSettings,
        camera: &presentation::view::Camera,
        map_box: Option<&[f64; 6]>,
    ) -> SunShadowFrame {
        SunShadowFrame::cold(light)
    }
    pub fn map_box(mesh: &[f32]) -> Option<[f64; 6]> {
        if mesh.is_empty() {
            return None;
        }
        let mut b = [f64::MAX, f64::MAX, f64::MAX, f64::MIN, f64::MIN, f64::MIN];
        for v in mesh.chunks(10) {
            for k in 0..3 {
                b[k] = b[k].min(v[k] as f64);
                b[k + 3] = b[k + 3].max(v[k] as f64);
            }
        }
        b[2] -= 1.0;
        b[5] += 25.0;
        Some(b)
    }
}

pub mod mesh {
    pub const VERTEX_FLOATS: usize = 10;
    fn tri(out: &mut Vec<f32>, p: [[f64; 3]; 3]) {
        let (a, b, c) = (p[0], p[1], p[2]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
        for q in p {
            out.extend([q[0], q[1], q[2], n[0] / l, n[1] / l, n[2] / l].map(|x| x as f32));
            out.extend([0.0f32; 4]);
        }
    }
    pub fn terrain(positions: &[f32], indices: &[u32], blocked: Option<(&[u8], u8)>) -> Vec<f32> {
        let mut out = vec![];
        let at = |i: u32| {
            let i = i as usize * 3;
            [
                positions[i] as f64,
                positions[i + 1] as f64,
                positions[i + 2] as f64,
            ]
        };
        for t in indices.chunks(3) {
            tri(&mut out, [at(t[0]), at(t[1]), at(t[2])]);
        }
        out
    }
    pub fn skirt(positions: &[f32]) -> Vec<f32> {
        vec![]
    }
    pub fn water(cells: &[f32]) -> Vec<f32> {
        vec![]
    }
    pub fn backdrop(map_box: &[f64; 6], reach_m: f64) -> Vec<f32> {
        let [x0, y0, z0, x1, y1, _] = *map_box;
        let z = z0 + 1.0;
        let r = reach_m;
        let mut out = vec![];
        let mut band = |a: f64, b: f64, c: f64, d: f64| {
            tri(&mut out, [[a, b, z], [c, b, z], [c, d, z]]);
            tri(&mut out, [[a, b, z], [c, d, z], [a, d, z]]);
        };
        band(x0 - r, y0 - r, x1 + r, y0);
        band(x0 - r, y1, x1 + r, y1 + r);
        band(x0 - r, y0, x0, y1);
        band(x1, y0, x1 + r, y1);
        out
    }
}

pub mod grass {
    use presentation::static_world::ground::biome::GrassRules;
    use presentation::static_world::ground::grass::KindFacts;
    pub struct TierMesh<'a> {
        pub positions: &'a [f32],
        pub normals: &'a [i16],
        pub uvs: &'a [f32],
        pub colors: &'a [u8],
    }
    pub struct KindMesh<'a> {
        pub tiers: Vec<TierMesh<'a>>,
        pub top: f64,
    }
    pub struct PackedShapes {
        pub shapes: Vec<f32>,
        pub facts: Vec<KindFacts>,
        pub max_blades: u32,
    }
    pub const FIELD_SEGMENTS: [u32; 2] = [4, 2];
    pub const LIMITS: presentation::static_world::ground::grass::Limits =
        presentation::static_world::ground::grass::Limits {
            variation: 1.2,
            tallest: 0.9,
        };
    pub fn pack_shapes(kinds: &[KindMesh]) -> Result<PackedShapes, String> {
        Ok(PackedShapes {
            shapes: vec![0.0; 16],
            facts: kinds
                .iter()
                .map(|_| KindFacts {
                    mesh_height: 0.1,
                    top: 0.1,
                    blades: 1.0,
                    root_width: 0.01,
                    bases: [0, 0],
                })
                .collect(),
            max_blades: 1,
        })
    }
    pub fn strip_indices(blades: u32, segments: u32) -> Vec<u16> {
        vec![0, 0, 0]
    }
    pub fn blade_vertices(segments: u32) -> u32 {
        1
    }
    pub struct Window {
        pub x0: f64,
        pub y0: f64,
        pub tiles_x: u32,
        pub tiles_y: u32,
        pub reach: f64,
    }
    pub fn metres_per_pixel(fov_y: f64, height_px: f64) -> f64 {
        2.0 * (fov_y / 2.0).tan() / height_px
    }
    pub fn window(eye: [f64; 3], lowest: f64, metres_per_px: f64, rules: &GrassRules) -> Window {
        Window {
            x0: 0.0,
            y0: 0.0,
            tiles_x: 0,
            tiles_y: 0,
            reach: 0.0,
        }
    }
    pub fn frame_params(
        words: &mut [u32],
        view_proj: &[f32; 16],
        eye: [f64; 3],
        metres_per_px: f64,
        window: &Window,
    ) {
        todo!()
    }
}
