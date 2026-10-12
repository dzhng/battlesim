//! The sun's cascades (`world/shadow.ts`): a depth array, one layer per
//! cascade, each cleared and drawn from its caster camera, and the receiver
//! block every lit material samples them by. The presentation crate fits
//! both to the frame's camera; this only uploads and draws.
use wgpu::*;

use crate::frame::{Inputs, Pass, DEPTH_FORMAT};
use crate::gpu::{bind, buffer};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
use presentation::records::{Camera, SunShadow as Receiver};
use presentation::shadow::{SunShadowFrame, CASCADES};

pub struct SunShadow {
    layers: Vec<TextureView>,
    /// The whole array, as receivers sample it.
    pub depth: TextureView,
    pub receiver: Buffer,
    pub compare: Sampler,
    cameras: Vec<Buffer>,
    groups: Vec<BindGroup>,
    /// Cascades the last fit made.
    active: usize,
}

impl SunShadow {
    /// The cascades at `map_size` texels a side, as `cold` fits them.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        map_size: u32,
        cold: &SunShadowFrame,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("sun-depth"),
            size: Extent3d {
                width: map_size,
                height: map_size,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let layers = (0..CASCADES as u32)
            .map(|layer| {
                texture.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let depth = texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let receiver = buffer(
            device,
            "sun-shadow",
            uniform,
            bytemuck::bytes_of(&cold.receiver),
        );
        let cameras: Vec<Buffer> = (0..CASCADES)
            .map(|_| {
                buffer(
                    device,
                    "sun-caster",
                    uniform,
                    bytemuck::bytes_of(&Camera::default()),
                )
            })
            .collect();
        let groups = cameras
            .iter()
            .map(|c| {
                bind(
                    device,
                    pipelines.layout("typegpuCameraLayout"),
                    "sun-caster",
                    &[c.as_entire_binding()],
                )
            })
            .collect();
        let compare = device.create_sampler(&SamplerDescriptor {
            label: Some("sun-compare"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            compare: Some(CompareFunction::GreaterEqual),
            ..Default::default()
        });
        Self {
            layers,
            depth,
            receiver,
            compare,
            cameras,
            groups,
            active: 0,
        }
    }

    /// Uploads the frame's fit.
    pub fn prepare(&mut self, queue: &Queue, fit: &SunShadowFrame) {
        queue.write_buffer(
            &self.receiver,
            0,
            bytemuck::bytes_of::<Receiver>(&fit.receiver),
        );
        for (buffer, camera) in self.cameras.iter().zip(&fit.casters) {
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(camera));
        }
        self.active = fit.casters.len().min(CASCADES);
    }
}

/// One clear-to-far depth pass per fitted cascade, drawing every caster:
/// the ground's and the models' (`encodeShadows`).
pub struct ShadowPass {
    solid: Bound,
    cutout: Bound,
}

impl ShadowPass {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            solid: pipelines.bound("modelCaster"),
            cutout: pipelines.bound("modelCasterCutout"),
        }
    }
}

impl Pass for ShadowPass {
    fn name(&self) -> &'static str {
        "sun-shadow"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let shadow = inputs.shadow;
        for (layer, camera) in shadow.layers.iter().zip(&shadow.groups).take(shadow.active) {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("sun-cascade"),
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: layer,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            let camera = ("typegpuCameraLayout", camera);
            inputs.ground.draw_casters(&mut pass, camera);
            let models = inputs.models;
            for (bound, surface) in [
                (&self.solid, Surface::Solid),
                (&self.cutout, Surface::Cutout),
            ] {
                bound.set(&mut pass, &[camera, models.group()]);
                models.draw_casters(&mut pass, surface);
            }
        }
    }
}
