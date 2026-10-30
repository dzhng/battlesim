//! The sky: the clear-sky lookup baked once from the
//! world's light, then each frame the shown sky along the view's rays, with
//! its sun disc and clouds, into the multisampled HDR target, resolved.
use presentation::records::{Rays, SkyLight, View};
use wgpu::*;

use crate::buffer;
use crate::pipelines::Pipelines;
use crate::HDR_FORMAT;

pub struct Sky {
    pipeline: RenderPipeline,
    group: BindGroup,
    rays: Buffer,
    light: Buffer,
    /// The clear-sky lookup: the environment light and the haze read it too.
    pub lut: TextureView,
}

fn uniform(device: &Device, queue: &Queue, label: &str, bytes: &[u8]) -> Buffer {
    buffer(
        device,
        queue,
        label,
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        bytes,
    )
}

impl Sky {
    /// Builds the sky for `light` and bakes its lookup.
    pub fn new(device: &Device, queue: &Queue, pipelines: &Pipelines, light: &SkyLight) -> Self {
        let [width, height] = pipelines.sizes.sky_lut;
        let lut = device.create_texture(&TextureDescriptor {
            label: Some("sky-lut"),
            size: Extent3d {
                width,
                height,
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
        let light = uniform(device, queue, "sky-light", bytemuck::bytes_of(light));
        let rays = uniform(
            device,
            queue,
            "sky-rays",
            bytemuck::bytes_of(&Rays::default()),
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sky"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bake = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sky-bake"),
            layout: pipelines.layout("skyBakeLayout"),
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
            layout: pipelines.layout("skyBackgroundLayout"),
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
            light,
            lut: lut_view,
        }
    }
}

impl Sky {
    /// Upload the view's canonical corner rays for the sky background.
    pub fn prepare(&self, queue: &Queue, view: &View) {
        queue.write_buffer(&self.rays, 0, bytemuck::bytes_of(&view.rays));
    }

    pub fn draw(&self, pass: &mut RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Sky {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.rays);
        memory.buffer(&self.light);
        memory.view(&self.lut);
    }
}
