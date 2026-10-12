//! Post (`world/post.ts`), the look: the HDR world's bright parts blurred over
//! five halving levels and composited, then graded and tone-mapped (AgX) into
//! the display target. The bloom's and grade's numbers are the world light's
//! records; only the blur's texel steps come from the target size.
use presentation::records::{Bloom, Grade, View};
use wgpu::*;

use crate::frame::{Pass, Targets, HDR_FORMAT};
use crate::pipelines::Pipelines;

/// Bloom levels (`BLOOM_KERNEL_RADII`'s length).
const LEVELS: usize = 5;

struct Stage {
    pipeline: RenderPipeline,
    group: BindGroup,
    target: TextureView,
}

pub struct Post {
    /// Highpass, each level's two blur axes, the composite, then the look.
    stages: Vec<Stage>,
}

fn texture(device: &Device, label: &str, size: [u32; 2]) -> TextureView {
    device
        .create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn uniform(device: &Device, bytes: &[u8]) -> Buffer {
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("post"),
        size: bytes.len() as u64,
        usage: BufferUsages::UNIFORM,
        mapped_at_creation: true,
    });
    buffer
        .get_mapped_range_mut(..)
        .expect("mapped at creation")
        .copy_from_slice(bytes);
    buffer.unmap();
    buffer
}

/// A bind group of `layout` with `resources` at bindings 0, 1, …
fn bind(
    device: &Device,
    pipelines: &Pipelines,
    layout: &str,
    resources: &[BindingResource],
) -> BindGroup {
    let entries: Vec<BindGroupEntry> = resources
        .iter()
        .enumerate()
        .map(|(i, r)| BindGroupEntry {
            binding: i as u32,
            resource: r.clone(),
        })
        .collect();
    device.create_bind_group(&BindGroupDescriptor {
        label: Some(layout),
        layout: pipelines.layout(layout),
        entries: &entries,
    })
}

impl Post {
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        bloom: &Bloom,
        grade: &Grade,
    ) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("post"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let bloom = uniform(device, bytemuck::bytes_of(bloom));
        let grade = uniform(device, bytemuck::bytes_of(grade));
        let s = BindingResource::Sampler(&sampler);
        let mut stages = Vec::new();

        let [mut w, mut h] = targets.size.map(|v| v / 2);
        let bright = texture(device, "bloom-bright", [w, h]);
        stages.push(Stage {
            pipeline: pipelines.render("bloomHighpass").clone(),
            group: bind(
                device,
                pipelines,
                "highpassLayout",
                &[
                    s.clone(),
                    BindingResource::TextureView(&targets.hdr),
                    bloom.as_entire_binding(),
                ],
            ),
            target: bright.clone(),
        });
        let mut source = bright;
        let mut levels = Vec::new();
        let mut horizontals = Vec::new();
        for level in 0..LEVELS {
            let pipeline = pipelines.render(&format!("bloomBlur{level}"));
            let (horizontal, vertical) = (
                texture(device, "bloom-horizontal", [w, h]),
                texture(device, "bloom-vertical", [w, h]),
            );
            for (from, to, step) in [
                (&source, &horizontal, [1.0 / w as f32, 0.0]),
                (&horizontal, &vertical, [0.0, 1.0 / h as f32]),
            ] {
                let step = uniform(device, bytemuck::cast_slice(&step));
                stages.push(Stage {
                    pipeline: pipeline.clone(),
                    group: bind(
                        device,
                        pipelines,
                        "blurLayout",
                        &[
                            s.clone(),
                            BindingResource::TextureView(from),
                            step.as_entire_binding(),
                        ],
                    ),
                    target: to.clone(),
                });
            }
            source = vertical.clone();
            levels.push(vertical);
            horizontals.push(horizontal);
            [w, h] = [w / 2, h / 2];
        }
        // The composite reuses the first level's horizontal target.
        let composite = horizontals.swap_remove(0);
        let mut resources = vec![s.clone()];
        resources.extend(levels.iter().map(BindingResource::TextureView));
        resources.push(bloom.as_entire_binding());
        stages.push(Stage {
            pipeline: pipelines.render("bloomComposite").clone(),
            group: bind(device, pipelines, "compositeLayout", &resources),
            target: composite.clone(),
        });
        stages.push(Stage {
            pipeline: pipelines.render("postLook").clone(),
            group: bind(
                device,
                pipelines,
                "finalLayout",
                &[
                    s,
                    BindingResource::TextureView(&targets.hdr),
                    BindingResource::TextureView(&composite),
                    grade.as_entire_binding(),
                ],
            ),
            target: targets.display.clone(),
        });
        Self { stages }
    }
}

impl Pass for Post {
    fn name(&self) -> &'static str {
        "post"
    }

    fn encode(&self, _: &Queue, _: &View, _: &Targets, encoder: &mut CommandEncoder) {
        for stage in &self.stages {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &stage.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&stage.pipeline);
            pass.set_bind_group(0, &stage.group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
