//! The frame-level bind groups every lit pass takes (`frame/environmentFrame.ts`
//! and `world/environment.ts`): the camera, the environment light (the
//! world's `Sun` and `Aerial` records, the cast lights, the sky lookup, its
//! PMREM and the DFG table), and the fog. Each frame the camera, the
//! environment's view and the cast lights the presentation chose are
//! written.
//!
//! Not built yet, and bound so that every shader reads them as absent:
//! - the sun's cascades (step 4): no receiver falls inside a cascade, so
//!   every surface takes the whole sun;
//! - fog of war's sight (its inputs come with the static world): every fog
//!   word zero (`enabled` 0, nothing painted), so every surface is seen.
use presentation::records::{Aerial, CastLight, Sun, View};
use wgpu::*;

use crate::frame::HDR_FORMAT;
use crate::gpu::{bind, buffer};
use crate::pipelines::Pipelines;

/// `Camera` in every shader: view-projection, its inverse, the eye, the near
/// plane, the drawing size and the clock, in 160 bytes.
const CAMERA_BYTES: usize = 160;
/// `Environment`'s members before the `Sun` record: the view and the observer.
const SUN_AT: usize = 80;
/// `CastLights`: a header (count, wrap) and up to 24 lights.
const CAST_LIGHTS_MAX: usize = 24;
const CAST_LIGHTS_BYTES: usize = 16 + CAST_LIGHTS_MAX * 32;
/// `SunShadow`: four cascades (matrix, bias, interval) and the control.
const CASCADES: usize = 4;
const SUN_SHADOW_BYTES: usize = CASCADES * 96 + 16;
/// Enough zeroed bytes for any of the fog group's structs or arrays.
const FOG_ZEROS: usize = 4096;
/// The sky lookup's width; the PMREM's levels follow from it (`pmrem.ts`).
const SKY_LUT_WIDTH: u32 = 384;
/// The split-sum table (`shaders/dfgLut.ts`): read from the browser's file.
const DFG_LUT_TS: &str = include_str!("../../../packages/battle-renderer/src/shaders/dfgLut.ts");
const DFG_LUT_SIZE: u32 = 16;

pub struct Environment {
    pub camera: BindGroup,
    pub environment: BindGroup,
    pub fog: BindGroup,
    /// The camera uniform, the environment uniform, its PMREM and the cast
    /// lights, which the effects bind beside their own resources.
    pub camera_buffer: Buffer,
    pub data: Buffer,
    pub pmrem: TextureView,
    pub cast_lights: Buffer,
    /// The environment uniform's light part, set once.
    image: Vec<f32>,
}

/// Column-major `a · b`.
fn mul(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum()
    })
}

/// The inverse of a column-major 4×4 (cofactors), or zeros if singular.
fn invert(m: &[f64; 16]) -> [f64; 16] {
    let a = |r: usize, c: usize| m[c * 4 + r];
    let mut inv = [0.0; 16];
    for r in 0..4 {
        for c in 0..4 {
            // The cofactor of (c, r): the transpose, for the adjugate.
            let rows: Vec<usize> = (0..4).filter(|&i| i != c).collect();
            let cols: Vec<usize> = (0..4).filter(|&j| j != r).collect();
            let e = |i: usize, j: usize| a(rows[i], cols[j]);
            let minor = e(0, 0) * (e(1, 1) * e(2, 2) - e(1, 2) * e(2, 1))
                - e(0, 1) * (e(1, 0) * e(2, 2) - e(1, 2) * e(2, 0))
                + e(0, 2) * (e(1, 0) * e(2, 1) - e(1, 1) * e(2, 0));
            let sign = if (r + c) % 2 == 0 { 1.0 } else { -1.0 };
            inv[c * 4 + r] = sign * minor;
        }
    }
    let det: f64 = (0..4).map(|c| a(0, c) * inv[c * 4]).sum();
    if det == 0.0 {
        return [0.0; 16];
    }
    inv.map(|v| v / det)
}

/// The camera uniform's bytes for `view` (`cameraUniform.ts`).
pub fn camera_words(view: &View, clock: f32) -> [f32; CAMERA_BYTES / 4] {
    let view_proj = mul(&view.projection.map(f64::from), &view.view.map(f64::from));
    let mut words = [0.0; CAMERA_BYTES / 4];
    for i in 0..16 {
        words[i] = view_proj[i] as f32;
    }
    for (i, v) in invert(&view_proj).iter().enumerate() {
        words[16 + i] = *v as f32;
    }
    words[32..35].copy_from_slice(&view.eye[..3]);
    // The near plane, from the reverse-Z projection's `near` term.
    words[35] = view.projection[14];
    words[36] = view.size[0];
    words[37] = view.size[1];
    words[38] = clock;
    words
}

fn texture(
    device: &Device,
    label: &str,
    size: [u32; 3],
    format: TextureFormat,
    usage: TextureUsages,
) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: size[2],
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// The six cube faces of one PMREM level's square `size` in the atlas, as
/// two triangles each (`pmremPlanes`): position, uv and face.
fn pmrem_planes(size: u32) -> Vec<f32> {
    let border = 1.0 / (size as f32 - 2.0);
    let uv = [
        -border,
        -border,
        1.0 + border,
        -border,
        1.0 + border,
        1.0 + border,
        -border,
        -border,
        1.0 + border,
        1.0 + border,
        -border,
        1.0 + border,
    ];
    let faces = [3.0f32, 1.0, 5.0, 0.0, 4.0, 2.0];
    let mut data = vec![0.0; 36 * 6];
    for (face, &slot) in faces.iter().enumerate() {
        let x = ((face % 3) * 2) as f32 / 3.0 - 1.0;
        let y = if face > 2 { 0.0 } else { -1.0 };
        let xy = [
            x,
            y,
            x + 2.0 / 3.0,
            y,
            x + 2.0 / 3.0,
            y + 1.0,
            x,
            y,
            x + 2.0 / 3.0,
            y + 1.0,
            x,
            y + 1.0,
        ];
        for v in 0..6 {
            let at = (slot as usize * 6 + v) * 6;
            data[at..at + 6].copy_from_slice(&[
                xy[v * 2],
                xy[v * 2 + 1],
                0.0,
                uv[v * 2],
                uv[v * 2 + 1],
                slot,
            ]);
        }
    }
    data
}

/// The sky lookup prefiltered for roughness into a cube atlas (`pmrem.ts`):
/// the atlas and its top mip.
fn pmrem(
    device: &Device,
    queue: &Queue,
    pipelines: &Pipelines,
    lut: &TextureView,
) -> (Texture, u32) {
    let max_mip = (SKY_LUT_WIDTH / 4).ilog2();
    let cube = 1u32 << max_mip;
    let (width, height) = (3 * cube.max(112), 4 * cube);
    let count = max_mip - 4 + 1 + 6;
    let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    let atlas = texture(device, "pmrem", [width, height, 1], HDR_FORMAT, usage);
    let ping = texture(device, "pmrem-ping", [width, height, 1], HDR_FORMAT, usage);
    let (atlas_view, ping_view) = (
        atlas.create_view(&Default::default()),
        ping.create_view(&Default::default()),
    );
    let sampler = device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    let size = |lod: u32| 1u32 << max_mip.saturating_sub(lod).max(4);
    let geometry: Vec<Buffer> = (0..count)
        .map(|lod| {
            buffer(
                device,
                "pmrem-planes",
                BufferUsages::VERTEX,
                bytemuck::cast_slice(&pmrem_planes(size(lod))),
            )
        })
        .collect();
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut draw =
        |pipeline: &str, group: &BindGroup, target: &TextureView, lod: u32, clear: bool| {
            let s = size(lod);
            let x = 3 * s * (lod + 4).saturating_sub(max_mip);
            let y = 4 * (cube - s);
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pmrem"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: if clear {
                            LoadOp::Clear(Color::TRANSPARENT)
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_viewport(x as f32, y as f32, (3 * s) as f32, (2 * s) as f32, 0.0, 1.0);
            pass.set_scissor_rect(x, y, 3 * s, 2 * s);
            pass.set_pipeline(pipelines.render(pipeline));
            pass.set_bind_group(0, group, &[]);
            pass.set_vertex_buffer(0, geometry[lod as usize].slice(..));
            pass.draw(0..36, 0..1);
        };
    let s = BindingResource::Sampler(&sampler);
    let bake = bind(
        device,
        pipelines.layout("sampled"),
        "pmrem-bake",
        &[BindingResource::TextureView(lut), s.clone()],
    );
    draw("pmremBake", &bake, &atlas_view, 0, true);
    let filter = |source: &TextureView, params: [f32; 4]| {
        let params = buffer(
            device,
            "pmrem-params",
            BufferUsages::UNIFORM,
            bytemuck::cast_slice(&params),
        );
        bind(
            device,
            pipelines.layout("filtering"),
            "pmrem-filter",
            &[
                BindingResource::TextureView(source),
                s.clone(),
                params.as_entire_binding(),
            ],
        )
    };
    for lod in 1..count {
        let target = lod as f32 / (count - 1) as f32;
        let previous = (lod - 1) as f32 / (count - 1) as f32;
        let roughness = (target * target - previous * previous).sqrt() * (target * 1.25);
        let top = max_mip as f32;
        let level = (max_mip as i32 - lod as i32) as f32;
        draw(
            "pmremFilter",
            &filter(&atlas_view, [roughness, level + 1.0, top, 0.0]),
            &ping_view,
            lod,
            lod == 1,
        );
        draw(
            "pmremFilter",
            &filter(&ping_view, [0.0, level, top, 0.0]),
            &atlas_view,
            lod,
            false,
        );
    }
    queue.submit([encoder.finish()]);
    (atlas, max_mip)
}

/// The DFG table's half floats, from the browser's source.
fn dfg_lut() -> Vec<u16> {
    let words: Vec<u16> = DFG_LUT_TS
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|t| t.strip_prefix("0x"))
        .filter_map(|h| u16::from_str_radix(h, 16).ok())
        .collect();
    assert_eq!(
        words.len(),
        (DFG_LUT_SIZE * DFG_LUT_SIZE * 2) as usize,
        "dfgLut.ts"
    );
    words
}

impl Environment {
    /// The environment for the world's light, prefiltering the sky's lookup.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        sky_lut: &TextureView,
        sun: &Sun,
        aerial: &Aerial,
    ) -> Self {
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let camera_buffer = buffer(device, "camera", uniform, &[0; CAMERA_BYTES]);
        let camera = bind(
            device,
            pipelines.layout("typegpuCameraLayout"),
            "camera",
            &[camera_buffer.as_entire_binding()],
        );

        let (pmrem, top_mip) = pmrem(device, queue, pipelines, sky_lut);
        let mut image = vec![0.0f32; SUN_AT / 4];
        image.extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(sun)));
        image[SUN_AT / 4 + 8] = top_mip as f32;
        image.extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(aerial)));
        let data = buffer(device, "environment", uniform, bytemuck::cast_slice(&image));
        let cast_lights = buffer(device, "cast-lights", uniform, &[0; CAST_LIGHTS_BYTES]);
        let pmrem = pmrem.create_view(&Default::default());
        let dfg = texture(
            device,
            "dfg",
            [DFG_LUT_SIZE, DFG_LUT_SIZE, 1],
            TextureFormat::Rg16Float,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        );
        queue.write_texture(
            dfg.as_image_copy(),
            bytemuck::cast_slice(&dfg_lut()),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(DFG_LUT_SIZE * 4),
                rows_per_image: None,
            },
            dfg.size(),
        );
        let linear = device.create_sampler(&SamplerDescriptor {
            label: Some("environment"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        // Every cascade's interval lies below zero: no receiver is inside one.
        let mut shadow = [0.0f32; SUN_SHADOW_BYTES / 4];
        for c in 0..CASCADES {
            shadow[c * 24 + 20..c * 24 + 22].copy_from_slice(&[-2.0, -1.0]);
        }
        shadow[CASCADES * 24] = 1.0;
        let sun_shadow = buffer(device, "sun-shadow", uniform, bytemuck::cast_slice(&shadow));
        let sun_depth = texture(
            device,
            "sun-depth",
            [1, 1, 1],
            TextureFormat::Depth32Float,
            TextureUsages::TEXTURE_BINDING,
        );
        let sun_depth = sun_depth.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let compare = device.create_sampler(&SamplerDescriptor {
            label: Some("sun-compare"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            compare: Some(CompareFunction::Greater),
            ..Default::default()
        });
        let environment = bind(
            device,
            pipelines.layout("environment"),
            "environment",
            &[
                data.as_entire_binding(),
                cast_lights.as_entire_binding(),
                BindingResource::TextureView(sky_lut),
                BindingResource::TextureView(&pmrem),
                BindingResource::TextureView(&dfg.create_view(&Default::default())),
                BindingResource::Sampler(&linear),
                sun_shadow.as_entire_binding(),
                BindingResource::TextureView(&sun_depth),
                BindingResource::Sampler(&compare),
            ],
        );

        let zeros = |usage| buffer(device, "fog-off", usage, &[0; FOG_ZEROS]);
        let (u, s) = (zeros(BufferUsages::UNIFORM), zeros(BufferUsages::STORAGE));
        let paint = texture(
            device,
            "fog-paint",
            [1, 1, 1],
            TextureFormat::Rgba8Unorm,
            TextureUsages::TEXTURE_BINDING,
        );
        let wholes = texture(
            device,
            "fog-wholes",
            [1, 1, 1],
            TextureFormat::R32Uint,
            TextureUsages::TEXTURE_BINDING,
        );
        let fog = bind(
            device,
            pipelines.layout("fogLayout"),
            "fog-off",
            &[
                u.as_entire_binding(),
                u.as_entire_binding(),
                s.as_entire_binding(),
                s.as_entire_binding(),
                s.as_entire_binding(),
                s.as_entire_binding(),
                u.as_entire_binding(),
                BindingResource::TextureView(&paint.create_view(&Default::default())),
                BindingResource::TextureView(&wholes.create_view(&Default::default())),
            ],
        );
        Self {
            camera,
            environment,
            fog,
            camera_buffer,
            data,
            pmrem,
            cast_lights,
            image,
        }
    }

    /// The frame-level groups by the layout each fills.
    pub fn groups(&self) -> [(&'static str, &BindGroup); 3] {
        [
            ("typegpuCameraLayout", &self.camera),
            ("environment", &self.environment),
            ("fogLayout", &self.fog),
        ]
    }

    /// The camera and the environment's view for `view`, the haze's
    /// distances measured from `observer` (the camera's target).
    pub fn prepare(&mut self, queue: &Queue, view: &View, observer: [f64; 3], clock: f32) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&camera_words(view, clock)),
        );
        self.image[..16].copy_from_slice(&view.view);
        for (i, v) in observer.iter().enumerate() {
            self.image[16 + i] = *v as f32;
        }
        queue.write_buffer(&self.data, 0, bytemuck::cast_slice(&self.image));
    }

    /// The frame's cast lights as the presentation chose them for the view,
    /// each reaching `wrap` round a surface past facing it (`setCastLights`).
    pub fn set_cast_lights(&self, queue: &Queue, lights: &[CastLight], wrap: f32) {
        let lights = &lights[..lights.len().min(CAST_LIGHTS_MAX)];
        let header = [lights.len() as f32, wrap, 0.0, 0.0];
        queue.write_buffer(&self.cast_lights, 0, bytemuck::cast_slice(&header));
        if !lights.is_empty() {
            queue.write_buffer(&self.cast_lights, 16, bytemuck::cast_slice(lights));
        }
    }
}
