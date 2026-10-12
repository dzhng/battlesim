//! The sky (`world/sky.ts`): the clear-sky lookup baked once from the
//! world's light, then each frame the shown sky along the view's rays, with
//! its sun disc and clouds, into the multisampled HDR target, resolved.
use presentation::records::SkyLight;
use wgpu::*;

use crate::frame::{Inputs, Pass, HDR_FORMAT};
use crate::gpu::buffer;
use crate::pipelines::Pipelines;

/// The sky lookup's size: equirectangular, 2:1 (`SKY_LUT_WIDTH/HEIGHT`).
const LUT: [u32; 2] = [384, 192];
/// The view's three corner rays (`Rays`).
const RAYS_BYTES: usize = 48;

pub struct Sky {
    pipeline: RenderPipeline,
    group: BindGroup,
    rays: Buffer,
    /// The clear-sky lookup: the environment light and the haze read it too.
    pub lut: TextureView,
}

fn uniform(device: &Device, label: &str, bytes: &[u8]) -> Buffer {
    buffer(
        device,
        label,
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        bytes,
    )
}

impl Sky {
    /// Builds the sky for `light` and bakes its lookup.
    pub fn new(device: &Device, queue: &Queue, pipelines: &Pipelines, light: &SkyLight) -> Self {
        let lut = device.create_texture(&TextureDescriptor {
            label: Some("sky-lut"),
            size: Extent3d {
                width: LUT[0],
                height: LUT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let lut_view = lut.create_view(&Default::default());
        let light = uniform(device, "sky-light", bytemuck::bytes_of(light));
        let rays = uniform(device, "sky-rays", &[0; RAYS_BYTES]);
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sky"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bake = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sky-bake"),
            layout: pipelines.layout("bake - Automatic Bind Group & Layout"),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light.as_entire_binding(),
            }],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("sky-lut"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &lut_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipelines.render("bake"));
            pass.set_bind_group(0, &bake, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);

        let group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sky-background"),
            layout: pipelines.layout("background - Automatic Bind Group & Layout"),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&lut_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: rays.as_entire_binding(),
                },
            ],
        });
        Self {
            pipeline: pipelines.render("background").clone(),
            group,
            rays,
            lut: lut_view,
        }
    }
}

impl Pass for Sky {
    fn name(&self) -> &'static str {
        "sky"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let (view, targets) = (inputs.view, inputs.targets);
        let rays: &[f32] = &[view.sky_origin, view.sky_dx, view.sky_dy].concat();
        inputs
            .queue
            .write_buffer(&self.rays, 0, bytemuck::cast_slice(rays));
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sky"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.hdr),
                // Kept: the world pass draws over it. It resolves too, since
                // Metal skips the resolve of a tile a pass draws nothing into.
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}
