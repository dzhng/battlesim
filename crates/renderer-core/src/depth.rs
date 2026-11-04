//! Reverse-Z depth in the same two phases the player paint and x-ray read.
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

    /// Ground, scenery and buildings hide player marks; units have not joined
    /// depth yet, so their hidden fragments can be measured against this world.
    pub fn encode_world(
        &self,
        inputs: &FrameInputs<'_>,
        encoder: &mut CommandEncoder,
        split: bool,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("depth-prepass-world"),
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
        let camera = ("cameraLayout", &inputs.environment.camera);
        inputs.ground.draw_depth(&mut pass, camera);
        inputs
            .scenery
            .draw_depth(&mut pass, camera, inputs.models.statics());
        for (pipeline, surface) in [
            (&self.solid, Surface::Solid),
            (&self.cutout, Surface::Cutout),
        ] {
            pipeline.set(&mut pass, &[camera, inputs.models.group()]);
            if split {
                inputs.models.draw_buildings(&mut pass, surface);
            } else {
                inputs.models.draw(&mut pass, surface);
            }
        }
    }

    /// Complete world depth is copied before grass adds its sample coverage.
    /// Overlay marks then remain continuous over grass but behind units.
    pub fn encode_models(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("depth-prepass-models"),
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &inputs.targets.depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            let camera = ("cameraLayout", &inputs.environment.camera);
            for (pipeline, surface) in [
                (&self.solid, Surface::Solid),
                (&self.cutout, Surface::Cutout),
            ] {
                pipeline.set(&mut pass, &[camera, inputs.models.group()]);
                inputs.models.draw_non_buildings(&mut pass, surface);
            }
        }
        encoder.copy_texture_to_texture(
            inputs.targets.depth.texture().as_image_copy(),
            inputs.targets.overlay_depth.texture().as_image_copy(),
            inputs.targets.depth.texture().size(),
        );
    }
}
