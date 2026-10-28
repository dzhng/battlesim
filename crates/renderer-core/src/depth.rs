//! The production reverse-Z depth prepass over the shared scene owners.
use crate::frame::FrameInputs;
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
use wgpu::*;

pub struct DepthPrepass {
    solid: Bound,
    cutout: Bound,
}

impl DepthPrepass {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            solid: pipelines.bound("modelPrepass"),
            cutout: pipelines.bound("modelPrepassCutout"),
        }
    }

    pub fn encode(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("depth-prepass"),
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &inputs.targets.depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        let camera = ("typegpuCameraLayout", &inputs.environment.camera);
        inputs.ground.draw_depth(&mut pass, camera);
        inputs
            .scenery
            .draw_depth(&mut pass, camera, inputs.models.statics());
        for (pipeline, surface) in [
            (&self.solid, Surface::Solid),
            (&self.cutout, Surface::Cutout),
        ] {
            pipeline.set(&mut pass, &[camera, inputs.models.group()]);
            inputs.models.draw(&mut pass, surface);
        }
    }
}
