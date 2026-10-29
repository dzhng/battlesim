//! The frame-level bind groups every lit pass takes (`frame/environmentFrame.ts`
//! and `world/environment.ts`): the camera and the environment light (the
//! world's `Sun` and `Aerial` records, the cast lights, the sky lookup, its
//! PMREM and the DFG table). Each frame the camera, the environment's view
//! and the cast lights the presentation chose are written. The sun's cascades are the shadow layer's (`shadow.rs`); fog's
//! groups are fog's (`fog.rs`).
use presentation::effects::lights::CAST_LIGHTS_MAX;
use presentation::light::LightSettings;
use presentation::records::{Camera as CameraRecord, CastLight, View};
use wgpu::*;

use crate::frame::HDR_FORMAT;
use crate::gpu::{bind, buffer};
use crate::pipelines::Pipelines;
use crate::shadow::SunShadow;

/// `Environment`'s members before the `Sun` record: the view and the observer.
const SUN_AT: usize = 80;
/// `CastLights`: a header (count, wrap) and up to 24 lights.
const CAST_LIGHTS_BYTES: usize = 16 + CAST_LIGHTS_MAX * size_of::<CastLight>();
/// The sky lookup's width; the PMREM's levels follow from it (`pmrem.ts`).
const SKY_LUT_WIDTH: u32 = 384;
/// The split-sum table (`shaders/dfgLut.ts`): read from the browser's file.
const DFG_LUT_TS: &str = include_str!("../../../packages/battle-renderer/src/shaders/dfgLut.ts");
const DFG_LUT_SIZE: u32 = 16;

pub struct Environment {
    pub camera: BindGroup,
    pub environment: BindGroup,
    /// The camera uniform, the environment uniform, its PMREM and the cast
    /// lights, which the effects bind beside their own resources.
    pub camera_buffer: Buffer,
    pub data: Buffer,
    pub pmrem: TextureView,
    pub cast_lights: Buffer,
    /// The environment uniform's light, set once.
    image: Vec<f32>,
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
    /// The environment for the world's light, prefiltering the sky's
    /// lookup. The wind rides in the camera (`Camera.wind`).
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        sky_lut: &TextureView,
        light: &LightSettings,
        shadow: &SunShadow,
    ) -> Self {
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let camera_buffer = buffer(
            device,
            "camera",
            uniform,
            bytemuck::bytes_of(&CameraRecord::default()),
        );
        let camera = bind(
            device,
            pipelines.layout("typegpuCameraLayout"),
            "camera",
            &[camera_buffer.as_entire_binding()],
        );

        let (pmrem, top_mip) = pmrem(device, queue, pipelines, sky_lut);
        let mut image = vec![0.0f32; SUN_AT / 4];
        image.extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(&light.sun())));
        image[SUN_AT / 4 + 8] = top_mip as f32;
        image.extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(&light.aerial())));
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
                shadow.receiver.as_entire_binding(),
                BindingResource::TextureView(&shadow.depth),
                BindingResource::Sampler(&shadow.compare),
            ],
        );

        Self {
            camera,
            environment,
            camera_buffer,
            data,
            pmrem,
            cast_lights,
            image,
        }
    }

    /// The frame-level groups by the layout each fills.
    pub fn groups(&self) -> [(&'static str, &BindGroup); 2] {
        [
            ("typegpuCameraLayout", &self.camera),
            ("environment", &self.environment),
        ]
    }

    /// The frame's camera, and the environment's view for its `view`, the
    /// haze's distances measured from what the camera looks at.
    pub fn prepare(&mut self, queue: &Queue, view: &View, camera: &CameraRecord) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera));
        self.image[..16].copy_from_slice(&view.view);
        self.image[16..19].copy_from_slice(&view.target[..3]);
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
