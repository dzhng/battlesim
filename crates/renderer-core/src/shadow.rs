//! Shared directional-shadow resource ownership.
//!
//! Presentation fits cascades and the shared frame draws their caster resources:
//! one depth-array layer per cascade, one receiver uniform, one camera buffer
//! per cascade, and a greater-equal comparison sampler. Every target consumes
//! the same canonical camera and receiver records.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::{bind, buffer};

pub use presentation::shadow::CASCADES;

pub struct ShadowResources {
    layers: Vec<TextureView>,
    /// The complete depth array bound by lit receivers.
    pub depth: TextureView,
    pub receiver: Buffer,
    pub compare: Sampler,
    cameras: Vec<Buffer>,
    groups: Vec<BindGroup>,
    active: usize,
}

impl ShadowResources {
    /// Allocate the shared shadow target and receiver/caster bindings.
    /// Camera storage follows the canonical presentation record layout.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        map_size: u32,
        receiver_bytes: &[u8],
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
            format: TextureFormat::Depth32Float,
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
        let receiver = buffer(device, queue, "sun-shadow", uniform, receiver_bytes);
        let cameras: Vec<Buffer> = (0..CASCADES)
            .map(|_| {
                buffer(
                    device,
                    queue,
                    "sun-caster",
                    uniform,
                    &[0; size_of::<presentation::records::Camera>()],
                )
            })
            .collect();
        let groups = cameras
            .iter()
            .map(|camera| {
                bind(
                    device,
                    pipelines.layout("cameraLayout"),
                    "sun-caster",
                    &[camera.as_entire_binding()],
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

    pub fn prepare(&mut self, queue: &Queue, receiver_bytes: &[u8], cameras: &[&[u8]]) {
        queue.write_buffer(&self.receiver, 0, receiver_bytes);
        for (buffer, bytes) in self.cameras.iter().zip(cameras.iter().copied()) {
            queue.write_buffer(buffer, 0, bytes);
        }
        self.active = cameras.len().min(CASCADES);
    }

    pub fn active(&self) -> usize {
        self.active
    }

    /// Encode the same clear-to-far cascade attachment around each target's
    /// concrete ground, indexed model and scenery owners.
    pub fn encode_cascades(
        &self,
        encoder: &mut CommandEncoder,
        mut draw: impl FnMut(usize, &BindGroup, &mut RenderPass<'_>),
    ) {
        for (cascade, layer) in self.layers.iter().take(self.active).enumerate() {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("sun-cascade"),
                color_attachments: &[],
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
            draw(cascade, &self.groups[cascade], &mut pass);
        }
    }
}

impl ShadowResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.receiver);
        for camera in &self.cameras {
            memory.buffer(camera);
        }
        memory.view(&self.depth);
        for view in &self.layers {
            memory.view(view);
        }
    }
}
