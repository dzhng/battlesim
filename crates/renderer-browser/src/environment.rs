use renderer_core::environment::EnvironmentBindings;
use renderer_core::pipelines::Pipelines;
use renderer_core::shadow::ShadowResources;
use wgpu::*;

/// Browser shell owner for the nine real resources consumed by the exported
/// environment bind group. Asset/sky generation can replace the sampled
/// textures later without changing the binding contract.
pub struct BrowserEnvironment {
    pub bindings: EnvironmentBindings,
    pub sky: Texture,
    pub pmrem: Texture,
    pub dfg: Texture,
    pub sampler: Sampler,
    pub cast_lights: Buffer,
    sky_light: Buffer,
    sky_bake_pipeline: renderer_core::pipelines::Bound,
    sky_bake_group: BindGroup,
    pmrem_ping: Texture,
    pmrem_sampler: Sampler,
    pmrem_geometry: Vec<Buffer>,
    pmrem_bake_pipeline: renderer_core::pipelines::Bound,
    pmrem_filter_pipeline: renderer_core::pipelines::Bound,
}

const SKY_LUT: [u32; 2] = [384, 192];
const PMREM: [u32; 2] = [336, 256];
const DFG: [u32; 2] = [16, 16];

impl BrowserEnvironment {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        presentation_environment: &Buffer,
        shadow: &ShadowResources,
    ) -> Self {
        let cast_lights = renderer_core::buffer(
            device,
            "shared-environment-cast-lights",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 1024],
        );
        let sky_light = renderer_core::buffer(
            device,
            "shared-environment-sky-light",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 160],
        );
        let sampled = |label: &str, format: TextureFormat, size: [u32; 2]| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        let sky = sampled(
            "shared-environment-sky",
            TextureFormat::Rgba16Float,
            SKY_LUT,
        );
        let pmrem = sampled(
            "shared-environment-pmrem",
            TextureFormat::Rgba16Float,
            PMREM,
        );
        let dfg = sampled("shared-environment-dfg", TextureFormat::Rg16Float, DFG);
        let pmrem_ping = sampled(
            "shared-environment-pmrem-ping",
            TextureFormat::Rgba16Float,
            PMREM,
        );
        let sky_words = vec![0x3c00u16; (SKY_LUT[0] * SKY_LUT[1] * 4) as usize];
        let pmrem_words = vec![0x3c00u16; (PMREM[0] * PMREM[1] * 4) as usize];
        let sky_bytes = bytemuck::cast_slice(&sky_words);
        let pmrem_bytes = bytemuck::cast_slice(&pmrem_words);
        for (texture, bytes, size, row_bytes) in [
            (&sky, sky_bytes, SKY_LUT, SKY_LUT[0] * 8),
            (&pmrem, pmrem_bytes, PMREM, PMREM[0] * 8),
        ] {
            queue.write_texture(
                texture.as_image_copy(),
                bytes,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(size[1]),
                },
                Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.write_texture(
            dfg.as_image_copy(),
            bytemuck::cast_slice(&vec![0x3800u16; (DFG[0] * DFG[1] * 2) as usize]),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(DFG[0] * 4),
                rows_per_image: Some(DFG[1]),
            },
            Extent3d {
                width: DFG[0],
                height: DFG[1],
                depth_or_array_layers: 1,
            },
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("shared-environment-sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let max_mip = (SKY_LUT[0] / 4).ilog2();
        let count = max_mip - 4 + 1 + 6;
        let pmrem_geometry = (0..count)
            .map(|lod| {
                let size = 1u32 << max_mip.saturating_sub(lod).max(4);
                renderer_core::buffer(
                    device,
                    "shared-environment-pmrem-planes",
                    BufferUsages::VERTEX,
                    bytemuck::cast_slice(&pmrem_planes(size)),
                )
            })
            .collect();
        let bindings = EnvironmentBindings::new(
            device,
            pipelines.layout("environment"),
            &[
                presentation_environment.as_entire_binding(),
                cast_lights.as_entire_binding(),
                BindingResource::TextureView(&sky.create_view(&Default::default())),
                BindingResource::TextureView(&pmrem.create_view(&Default::default())),
                BindingResource::TextureView(&dfg.create_view(&Default::default())),
                BindingResource::Sampler(&sampler),
                shadow.receiver.as_entire_binding(),
                BindingResource::TextureView(&shadow.depth),
                BindingResource::Sampler(&shadow.compare),
            ],
        );
        let sky_bake_group = renderer_core::bind(
            device,
            pipelines.layout("bake - Automatic Bind Group & Layout"),
            "shared-environment-sky-bake",
            &[sky_light.as_entire_binding()],
        );
        Self {
            bindings,
            sky,
            pmrem,
            dfg,
            sampler: sampler.clone(),
            cast_lights,
            sky_light,
            sky_bake_pipeline: pipelines.bound("bake"),
            sky_bake_group,
            pmrem_ping,
            pmrem_sampler: sampler,
            pmrem_geometry,
            pmrem_bake_pipeline: pipelines.bound("pmremBake"),
            pmrem_filter_pipeline: pipelines.bound("pmremFilter"),
        }
    }

    /// Bake the resolved presentation sky record into the shared LUT.
    pub fn bake_sky(&self, device: &Device, queue: &Queue, sky: &[u8]) {
        queue.write_buffer(&self.sky_light, 0, sky);
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("shared-environment-sky-bake"),
        });
        let view = self.sky.create_view(&TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shared-environment-sky-lut"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        self.sky_bake_pipeline.set(
            &mut pass,
            &[("bake - Automatic Bind Group & Layout", &self.sky_bake_group)],
        );
        pass.draw(0..3, 0..1);
        drop(pass);
        queue.submit([encoder.finish()]);
        self.bake_pmrem(device, queue);
    }

    fn bake_pmrem(&self, device: &Device, queue: &Queue) {
        let max_mip = (SKY_LUT[0] / 4).ilog2();
        let cube = 1u32 << max_mip;
        let count = max_mip - 4 + 1 + 6;
        let sky_view = self.sky.create_view(&TextureViewDescriptor::default());
        let source = renderer_core::bind(
            device,
            // pmremBake uses the same texture/sampler layout as the simple
            // exported sampled contract.
            &self.pmrem_bake_pipeline.pipeline.get_bind_group_layout(0),
            "shared-environment-pmrem-source",
            &[
                BindingResource::TextureView(&sky_view),
                BindingResource::Sampler(&self.pmrem_sampler),
            ],
        );
        let atlas_view = self.pmrem.create_view(&TextureViewDescriptor::default());
        let ping_view = self
            .pmrem_ping
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("shared-environment-pmrem"),
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-environment-pmrem-bake"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &atlas_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.pmrem_bake_pipeline
                .set(&mut pass, &[("sampled", &source)]);
            pass.set_vertex_buffer(0, self.pmrem_geometry[0].slice(..));
            pass.draw(0..36, 0..1);
        }
        for lod in 1..count {
            let size = 1u32 << max_mip.saturating_sub(lod).max(4);
            let x = 3 * size * (lod + 4).saturating_sub(max_mip);
            let y = 4 * (cube - size);
            let params = renderer_core::buffer(
                device,
                "shared-environment-pmrem-params",
                BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                bytemuck::cast_slice(&[
                    (lod as f32 / (count - 1) as f32).powi(2),
                    (lod as f32 / (count - 1) as f32) * max_mip as f32,
                    max_mip as f32,
                    0.0,
                ]),
            );
            let filtering = renderer_core::bind(
                device,
                &self.pmrem_filter_pipeline.pipeline.get_bind_group_layout(0),
                "shared-environment-pmrem-filter",
                &[
                    BindingResource::TextureView(if lod == 1 { &atlas_view } else { &ping_view }),
                    BindingResource::Sampler(&self.pmrem_sampler),
                    params.as_entire_binding(),
                ],
            );
            let target = if lod == 1 { &ping_view } else { &atlas_view };
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-environment-pmrem-filter"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: if lod == 1 {
                            LoadOp::Clear(Color::TRANSPARENT)
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_viewport(
                x as f32,
                y as f32,
                (3 * size) as f32,
                (2 * size) as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(x, y, 3 * size, 2 * size);
            self.pmrem_filter_pipeline
                .set(&mut pass, &[("filtering", &filtering)]);
            pass.set_vertex_buffer(0, self.pmrem_geometry[lod as usize].slice(..));
            pass.draw(0..36, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

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
