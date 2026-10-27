//! Shared reverse-Z depth prepass orchestration.

use wgpu::*;

use crate::pipelines::{Bound, Pipelines};

/// The draw operations supplied by a platform's frame resources.
///
/// The core owns ordering, depth attachment setup and model pipeline binding;
/// a shell only adapts its concrete ground, scenery and model owners to these
/// three draw operations.
pub trait DepthInputs {
    fn depth_view(&self) -> &TextureView;
    fn camera_group(&self) -> (&'static str, &BindGroup);
    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>);
    fn draw_scenery_depth(&self, pass: &mut RenderPass<'_>);
    fn draw_models_depth(&self, pass: &mut RenderPass<'_>, pipeline: &Bound, cutout: bool);
}

/// Every opaque surface before colour, using the shared reverse-Z target.
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

    pub fn encode<I: DepthInputs>(&self, inputs: &I, encoder: &mut CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("depth-prepass"),
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: inputs.depth_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        inputs.draw_ground_depth(&mut pass);
        inputs.draw_scenery_depth(&mut pass);
        inputs.draw_models_depth(&mut pass, &self.solid, false);
        inputs.draw_models_depth(&mut pass, &self.cutout, true);
    }
}
