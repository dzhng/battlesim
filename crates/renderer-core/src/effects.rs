//! Shared GPU ownership for the lit-world effects pass.
//!
//! Effect decoding, atlas construction, and presentation records stay in the
//! platform shell. Once a shell has a bindable atlas/environment group and a
//! packed instance byte stream, this owner performs the same buffer lifetime,
//! attachment setup, and six-vertex instanced draw.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::Targets;

pub struct EffectsResources {
    pipeline: RenderPipeline,
    group: BindGroup,
    instances: Buffer,
    instance_stride: u64,
    count: u32,
}

impl EffectsResources {
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        resources: &[BindingResource<'_>],
        instance_stride: u64,
        initial_capacity: u64,
    ) -> Self {
        let instances = device.create_buffer(&BufferDescriptor {
            label: Some("effect-instances"),
            size: initial_capacity.max(instance_stride),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline: pipelines.render("effects").clone(),
            group: crate::bind(device, pipelines.layout("effects"), "effects", resources),
            instances,
            instance_stride,
            count: 0,
        }
    }

    /// Replace this frame's packed instances, growing by doubling as needed.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, bytes: &[u8]) {
        self.count = (bytes.len() as u64 / self.instance_stride) as u32;
        if bytes.is_empty() {
            return;
        }
        let need = bytes.len() as u64;
        if self.instances.size() < need {
            self.instances = device.create_buffer(&BufferDescriptor {
                label: Some("effect-instances"),
                size: need.saturating_mul(2),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.instances, 0, bytes);
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    /// Draw effects over lit colour and the fog mask, preserving the native
    /// and browser pass attachment contract.
    pub fn encode(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        if self.count == 0 {
            return;
        }
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("effects"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &targets.lit,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &targets.fog_mask,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
            ],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}

impl EffectsResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.instances);
    }
}
