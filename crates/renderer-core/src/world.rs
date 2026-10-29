//! The production world policy: scar-page submissions, grass growth,
//! sky, fogged materials, scenery and glass in one shared owner. Its passes
//! resolve and keep their samples for the water after them.
use crate::fog::Layer;
use crate::frame::{FrameInputs, FrameView};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
use crate::Targets;
use wgpu::*;

pub struct WorldColour {
    surfaces: [(Bound, Surface); 3],
    glass: Bound,
    cards: Bound,
}

fn begin_world_pass<'a>(
    encoder: &'a mut CommandEncoder,
    targets: &'a Targets,
    label: &'static str,
    first: bool,
) -> RenderPass<'a> {
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[
            Some(RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.lit),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &targets.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.fog_mask),
                ops: Operations {
                    load: if first {
                        LoadOp::Clear(Color::TRANSPARENT)
                    } else {
                        LoadOp::Load
                    },
                    store: StoreOp::Store,
                },
            }),
        ],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    })
}

fn groups<'a>(inputs: &'a FrameInputs<'_>, layer: Layer) -> [(&'static str, &'a BindGroup); 3] {
    let [camera, environment] = inputs.environment.groups();
    [camera, environment, inputs.fog.group(layer)]
}

impl WorldColour {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            surfaces: [
                (pipelines.bound("modelOpaque"), Surface::Opaque),
                (pipelines.bound("modelCutout"), Surface::Cutout),
                (pipelines.bound("modelRooms"), Surface::Room),
            ],
            glass: pipelines.bound("modelBlended"),
            cards: pipelines.bound("modelCards"),
        }
    }

    pub fn encode(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        let (queue, ground) = (inputs.queue, inputs.ground);
        let scars = &ground.material.scars;
        if let Some(upload) = scars.grass_upload() {
            scars.apply(queue, upload);
        }
        if let Some(grass) = inputs.grass {
            grass.encode_build(encoder, ground.layout_group());
        }
        inputs.sky.prepare(queue, inputs.view);
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &inputs.targets.hdr_msaa,
                    depth_slice: None,
                    resolve_target: Some(&inputs.targets.lit),
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            inputs.sky.draw(&mut pass);
        }
        let regional = scars.regional();
        if regional {
            let done = std::mem::replace(
                encoder,
                inputs.device.create_command_encoder(&Default::default()),
            );
            queue.submit([done.finish()]);
            for (index, (rect, upload)) in scars.region_uploads().iter().enumerate() {
                scars.apply(queue, upload);
                let mut region = inputs.device.create_command_encoder(&Default::default());
                let mut pass =
                    begin_world_pass(&mut region, inputs.targets, "ground-region", index == 0);
                let [x, y, w, h] = *rect;
                pass.set_scissor_rect(x, y, w, h);
                ground.draw_terrain(&mut pass, &groups(inputs, Layer::PaintedGround));
                drop(pass);
                queue.submit([region.finish()]);
            }
        } else if let Some(upload) = scars.view_upload() {
            scars.apply(queue, upload);
        }
        let drew = !scars.region_uploads().is_empty();
        let mut pass = begin_world_pass(encoder, inputs.targets, "world", !drew);
        if !regional {
            ground.draw_terrain(&mut pass, &groups(inputs, Layer::PaintedGround));
        }
        ground.draw_skirt(&mut pass, &groups(inputs, Layer::Faces));
        for layer in [
            Layer::Units,
            Layer::Faces,
            Layer::Ground,
            Layer::PaintedFaces,
        ] {
            let mut groups = groups(inputs, layer).to_vec();
            groups.push(inputs.models.group());
            for (pipeline, surface) in &self.surfaces {
                pipeline.set(&mut pass, &groups);
                inputs.models.draw_fogged(&mut pass, *surface, layer);
            }
            if let Some(cards) = inputs.models.card_group() {
                let mut groups = inputs.environment.groups().to_vec();
                groups.extend([inputs.fog.group(layer), cards]);
                self.cards.set(&mut pass, &groups);
                inputs.models.draw_cards(&mut pass, layer);
            }
        }
        inputs.scenery.draw(
            &mut pass,
            &groups(inputs, Layer::Faces),
            inputs.models.statics(),
        );
        if inputs.frame_view != FrameView::GroundClasses {
            if let Some(grass) = inputs.grass {
                grass.draw(&mut pass, &groups(inputs, Layer::PaintedGround));
            }
        }
        ground.draw_backdrop(&mut pass, &groups(inputs, Layer::PaintedGround));
        if inputs.frame_view != FrameView::GroundClasses {
            let mut groups = inputs.environment.groups().to_vec();
            groups.push(inputs.models.group());
            self.glass.set(&mut pass, &groups);
            inputs.models.draw(&mut pass, Surface::Blended);
        }
    }
}
