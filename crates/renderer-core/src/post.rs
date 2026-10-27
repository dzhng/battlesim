//! Shared bloom and tone-map resource ownership.
//!
//! The platform shell supplies the serialized bloom/grade uniforms. This
//! module owns the intermediate targets, bind groups and ordered fullscreen
//! passes, so native and browser use one post-processing implementation.
use wgpu::*;

use crate::pipelines::Pipelines;
use crate::{Targets, HDR_FORMAT};

struct Stage {
    pipeline: RenderPipeline,
    group: BindGroup,
    target: TextureView,
}

pub struct PostResources {
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

pub fn new(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    bloom_bytes: &[u8],
    grade_bytes: &[u8],
) -> PostResources {
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("post"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    let uniform = |label, bytes: &[u8]| crate::buffer(device, label, BufferUsages::UNIFORM, bytes);
    let bloom = uniform("post-bloom", bloom_bytes);
    let grade = uniform("post-grade", grade_bytes);
    let s = BindingResource::Sampler(&sampler);
    let bind = |layout: &str, resources: &[BindingResource]| {
        crate::bind(device, pipelines.layout(layout), layout, resources)
    };
    let mut stages = Vec::new();
    let [mut w, mut h] = targets.size.map(|v| v / 2);
    let bright = texture(device, "bloom-bright", [w, h]);
    stages.push(Stage {
        pipeline: pipelines.render("bloomHighpass").clone(),
        group: bind(
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
    for level in 0..pipelines.sizes.bloom_levels {
        let pipeline = pipelines.render(&format!("bloomBlur{level}"));
        let (horizontal, vertical) = (
            texture(device, "bloom-horizontal", [w, h]),
            texture(device, "bloom-vertical", [w, h]),
        );
        for (from, to, step) in [
            (&source, &horizontal, [1.0 / w as f32, 0.0]),
            (&horizontal, &vertical, [0.0, 1.0 / h as f32]),
        ] {
            let step = uniform("post-step", bytemuck::cast_slice(&step));
            stages.push(Stage {
                pipeline: pipeline.clone(),
                group: bind(
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
    let composite = horizontals.swap_remove(0);
    let mut resources = vec![s.clone()];
    resources.extend(levels.iter().map(BindingResource::TextureView));
    resources.push(bloom.as_entire_binding());
    stages.push(Stage {
        pipeline: pipelines.render("bloomComposite").clone(),
        group: bind("compositeLayout", &resources),
        target: composite.clone(),
    });
    stages.push(Stage {
        pipeline: pipelines.render("postLook").clone(),
        group: bind(
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
    PostResources { stages }
}

impl PostResources {
    pub fn encode(&self, encoder: &mut CommandEncoder) {
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
