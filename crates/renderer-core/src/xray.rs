//! Hidden surface-fragment coverage and the player's unit silhouettes.
//! Pipeline exports own reverse-Z bias, maximum blending and premultiplication;
//! this owner supplies only counter lifetime, world-depth sampling and draws.
use wgpu::*;

use crate::fog::Layer;
use crate::models::{Models, Surface};
use crate::pipelines::{Bound, Pipelines};
use crate::Targets;

pub struct XrayResources {
    counts: Buffer,
    minimum: Buffer,
    capacity: u32,
    fraction: f32,
    coverage_enabled: bool,
    count_pipeline: Bound,
    draw_pipeline: Bound,
    count_layout: BindGroupLayout,
    read_layout: BindGroupLayout,
    count_group: BindGroup,
    read_group: BindGroup,
    depth: TextureView,
}

fn counters(device: &Device, capacity: u32) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("model-xray-coverage"),
        size: u64::from(capacity) * 8,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn groups(
    device: &Device,
    count_layout: &BindGroupLayout,
    read_layout: &BindGroupLayout,
    counts: &Buffer,
    minimum: &Buffer,
    depth: &TextureView,
) -> (BindGroup, BindGroup) {
    (
        crate::bind(
            device,
            count_layout,
            "xray-count",
            &[
                counts.as_entire_binding(),
                BindingResource::TextureView(depth),
            ],
        ),
        crate::bind(
            device,
            read_layout,
            "xray-read",
            &[counts.as_entire_binding(), minimum.as_entire_binding()],
        ),
    )
}

impl XrayResources {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        targets: &Targets,
        minimum_hidden_fraction: f32,
    ) -> Result<Self, String> {
        if !minimum_hidden_fraction.is_finite()
            || minimum_hidden_fraction <= 0.0
            || minimum_hidden_fraction > 1.0
        {
            return Err("x-ray minimum hidden fragment fraction must be in (0, 1]".into());
        }
        let capacity = 16;
        let counts = counters(device, capacity);
        // A scalar's complete uniform block avoids four-byte mapped bindings
        // rejected by some production WebGPU adapters.
        let minimum = crate::buffer(
            device,
            queue,
            "xray-minimum",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            bytemuck::cast_slice(&[minimum_hidden_fraction, 0.0, 0.0, 0.0]),
        );
        let count_layout = pipelines.layout("xrayCountLayout").clone();
        let read_layout = pipelines.layout("xrayReadLayout").clone();
        let depth = targets.overlay_depth.clone();
        let (count_group, read_group) = groups(
            device,
            &count_layout,
            &read_layout,
            &counts,
            &minimum,
            &depth,
        );
        Ok(Self {
            counts,
            minimum,
            capacity,
            fraction: minimum_hidden_fraction,
            coverage_enabled: true,
            count_pipeline: pipelines.bound("modelXrayCount"),
            draw_pipeline: pipelines.bound("modelXray"),
            count_layout,
            read_layout,
            count_group,
            read_group,
            depth,
        })
    }

    /// Counter indices are the actual draw instance indices, including holes
    /// in the presentation's publication; population alone cannot size them.
    pub fn prepare(&mut self, device: &Device, models: &Models) {
        let count = models.xray_instance_count().max(1);
        if count <= self.capacity {
            return;
        }
        self.capacity = count.saturating_mul(2).max(16);
        self.counts = counters(device, self.capacity);
        self.rebuild_groups(device);
    }

    pub fn rebind_depth(&mut self, device: &Device, targets: &Targets) {
        self.depth = targets.overlay_depth.clone();
        self.rebuild_groups(device);
    }

    fn rebuild_groups(&mut self, device: &Device) {
        (self.count_group, self.read_group) = groups(
            device,
            &self.count_layout,
            &self.read_layout,
            &self.counts,
            &self.minimum,
            &self.depth,
        );
    }

    /// Disabling the coverage heuristic still draws eligible hidden fragments.
    pub fn set_coverage_enabled(&mut self, queue: &Queue, enabled: bool) {
        self.coverage_enabled = enabled;
        queue.write_buffer(
            &self.minimum,
            0,
            bytemuck::bytes_of(&if enabled { self.fraction } else { 0.0 }),
        );
    }

    /// Encode between world/building depth and unit depth. The overlay remains
    /// multisampled for the later display-overlay pass to load and resolve.
    pub fn encode(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        camera: &BindGroup,
        models: &Models,
    ) {
        let _untracked = models.submissions.exclude();
        encoder.clear_buffer(&self.counts, 0, None);
        if self.coverage_enabled && models.has_xray_meshes() {
            encoder.copy_texture_to_texture(
                targets.depth.texture().as_image_copy(),
                targets.overlay_depth.texture().as_image_copy(),
                targets.depth.texture().size(),
            );
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("unit-xray-coverage"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.overlay_msaa,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Discard,
                    },
                })],
                ..Default::default()
            });
            self.count_pipeline.set(
                &mut pass,
                &[
                    ("cameraLayout", camera),
                    models.group(),
                    ("xrayCountLayout", &self.count_group),
                ],
            );
            models.draw_fogged(&mut pass, Surface::Opaque, Layer::Units);
        }
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("unit-xray"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        self.draw_pipeline.set(
            &mut pass,
            &[
                ("cameraLayout", camera),
                models.group(),
                ("xrayReadLayout", &self.read_group),
            ],
        );
        models.draw_fogged(&mut pass, Surface::Opaque, Layer::Units);
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.counts);
        memory.buffer(&self.minimum);
        memory.view(&self.depth);
    }
}
