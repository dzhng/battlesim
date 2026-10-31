//! Shared bloom and tone-map resource ownership.
//!
//! The runtime supplies canonical bloom/grade uniform bytes. This
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PostMode {
    #[default]
    Look,
    Ungraded,
    Raw,
    Hdr,
}

pub struct PostResources {
    stages: Vec<Stage>,
    finals: [RenderPipeline; 3],
    display: TextureView,
    high_plane: RenderPipeline,
    final_group: BindGroup,
    bloom: Buffer,
    grade: Buffer,
    steps: Vec<Buffer>,
    hdr_pipeline: RenderPipeline,
    /// The HDR output texture view for rendering.
    pub hdr_output: TextureView,
    pub hdr_texture: Texture,
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
    queue: &Queue,
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
    let uniform = |label, bytes: &[u8]| {
        crate::buffer(
            device,
            queue,
            label,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            bytes,
        )
    };
    let bloom = uniform("post-bloom", bloom_bytes);
    let grade = uniform("post-grade", grade_bytes);
    let s = BindingResource::Sampler(&sampler);
    let bind = |layout: &str, resources: &[BindingResource]| {
        crate::bind(device, pipelines.layout(layout), layout, resources)
    };
    let mut stages = Vec::new();
    let mut steps = Vec::new();
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
            steps.push(step);
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
    let final_group = bind(
        "finalLayout",
        &[
            s,
            BindingResource::TextureView(&targets.hdr),
            BindingResource::TextureView(&composite),
            grade.as_entire_binding(),
        ],
    );
    let hdr_texture = device.create_texture(&TextureDescriptor {
        label: Some("hdr-output"),
        size: Extent3d {
            width: targets.size[0],
            height: targets.size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: HDR_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    PostResources {
        stages,
        finals: ["postLook", "postUngraded", "postRaw"].map(|name| pipelines.render(name).clone()),
        display: targets.display.clone(),
        high_plane: pipelines.render("postHighPlane").clone(),
        final_group,
        bloom,
        grade,
        steps,
        hdr_pipeline: pipelines.render("postHdr").clone(),
        hdr_output: hdr_texture.create_view(&Default::default()),
        hdr_texture,
    }
}

impl PostResources {
    /// Grade the straight high-air colour and premultiply by its coverage.
    /// World bloom and grading are shared; no second lighting policy is baked.
    pub fn encode_high_plane(&self, encoder: &mut CommandEncoder, output: &TextureView) {
        Self::screen(
            encoder,
            &self.high_plane,
            &self.final_group,
            output,
            "high-air-look",
        );
    }

    fn screen(
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        group: &BindGroup,
        output: &TextureView,
        label: &str,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn write(&self, queue: &Queue, bloom: &[u8], grade: &[u8]) -> Result<(), String> {
        if bloom.len() > self.bloom.size() as usize || grade.len() > self.grade.size() as usize {
            return Err("post record exceeds allocated uniform".into());
        }
        queue.write_buffer(&self.bloom, 0, bloom);
        queue.write_buffer(&self.grade, 0, grade);
        Ok(())
    }

    pub fn encode(&self, encoder: &mut CommandEncoder, mode: PostMode) {
        if mode == PostMode::Look || mode == PostMode::Hdr {
            for stage in &self.stages {
                Self::screen(
                    encoder,
                    &stage.pipeline,
                    &stage.group,
                    &stage.target,
                    "post",
                );
            }
        }
        if mode == PostMode::Hdr {
            Self::screen(
                encoder,
                &self.hdr_pipeline,
                &self.final_group,
                &self.hdr_output,
                "post-hdr",
            );
            return;
        }
        let pipeline = &self.finals[match mode {
            PostMode::Look => 0,
            PostMode::Ungraded => 1,
            PostMode::Raw | PostMode::Hdr => 2,
        }];
        Self::screen(
            encoder,
            pipeline,
            &self.final_group,
            &self.display,
            "post-final",
        );
    }
}

impl PostResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.bloom);
        memory.buffer(&self.grade);
        for step in &self.steps {
            memory.buffer(step);
        }
        for stage in &self.stages {
            memory.view(&stage.target);
        }
        memory.view(&self.display);
        memory.view(&self.hdr_output);
        memory.texture(&self.hdr_texture);
    }
}
