//! The world's passes (`frame/worldPass.ts`), in the order `battleFrame`
//! encodes them: the depth prepass, then the grass's growth, the sky and
//! the lit colour pass with its fog mask, both resolved for the effects and
//! the fog mask passes, then the placement ghosts over the fogged world.
//! The ground and the models draw into them; the scenery joins the same
//! passes.
use wgpu::*;

use crate::frame::{Inputs, Pass};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
use crate::sky::Sky;

fn depth(
    view: &TextureView,
    load: LoadOp<f32>,
    read_only: bool,
) -> RenderPassDepthStencilAttachment<'_> {
    RenderPassDepthStencilAttachment {
        view,
        depth_ops: (!read_only).then_some(Operations {
            load,
            store: StoreOp::Store,
        }),
        stencil_ops: None,
    }
}

/// The frame's depth: every opaque surface before any colour, the world's
/// ground first, so the colour pass shades each sample once (`encodeDepth`).
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
}

impl Pass for DepthPrepass {
    fn name(&self) -> &'static str {
        "depth-prepass"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("depth-prepass"),
            depth_stencil_attachment: Some(depth(&inputs.targets.depth, LoadOp::Clear(0.0), false)),
            ..Default::default()
        });
        let models = inputs.models;
        let [camera, ..] = inputs.environment.groups();
        inputs.ground.draw_depth(&mut pass, camera);
        for (bound, surface) in [
            (&self.solid, Surface::Solid),
            (&self.cutout, Surface::Cutout),
        ] {
            bound.set(&mut pass, &[camera, models.group()]);
            models.draw(&mut pass, surface);
        }
    }
}

/// The grass grown for the view, then the sky, then the lit world over it at
/// the prepass's depth, with the fog mask beside it (`encode`): the terrain,
/// the skirt, the models, the grass, the backdrop, the water, and glass last, over
/// everything. Both resolve: the lit world into `lit`, the mask into
/// `fog_mask`.
pub struct WorldColour {
    sky: Sky,
    surfaces: Vec<(Bound, Surface)>,
    glass: Bound,
}

impl WorldColour {
    pub fn new(pipelines: &Pipelines, sky: Sky) -> Self {
        let surfaces = [
            ("modelOpaque", Surface::Opaque),
            ("modelCutout", Surface::Cutout),
            ("modelRooms", Surface::Room),
        ];
        Self {
            sky,
            surfaces: surfaces
                .into_iter()
                .map(|(name, surface)| (pipelines.bound(name), surface))
                .collect(),
            glass: pipelines.bound("modelBlended"),
        }
    }
}

impl Pass for WorldColour {
    fn name(&self) -> &'static str {
        "world"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let (queue, ground) = (inputs.queue, inputs.ground);
        let scars = &ground.scars;
        if let Some(upload) = scars.grass_upload() {
            scars.apply(queue, upload);
        }
        inputs.grass.encode_build(encoder, ground.layout_group());
        self.sky.encode(inputs, encoder);
        let frame = inputs.environment.groups();
        // Too many marks to hold at once: the terrain draws region by region,
        // each submitted after its scars load (the frame so far first).
        let regional = scars.regional();
        if regional {
            let done = std::mem::replace(
                encoder,
                inputs.device.create_command_encoder(&Default::default()),
            );
            queue.submit([done.finish()]);
            for (i, upload) in scars.region_uploads().iter().enumerate() {
                scars.apply(queue, upload);
                let mut region = inputs.device.create_command_encoder(&Default::default());
                let mut pass = world_pass(&mut region, inputs.targets, "ground-region", i == 0);
                ground.draw_terrain(&mut pass, &frame);
                drop(pass);
                queue.submit([region.finish()]);
            }
        } else if let Some(upload) = scars.view_upload() {
            scars.apply(queue, upload);
        }
        let mut pass = world_pass(encoder, inputs.targets, "world", !regional);
        if !regional {
            ground.draw_terrain(&mut pass, &frame);
        }
        inputs.ground.draw_skirt(&mut pass, &frame);
        let models = inputs.models;
        let mut groups = frame.to_vec();
        groups.push(models.group());
        for (bound, surface) in &self.surfaces {
            bound.set(&mut pass, &groups);
            models.draw(&mut pass, *surface);
        }
        inputs.grass.draw(&mut pass, &frame);
        inputs.ground.draw_backdrop(&mut pass, &frame);
        inputs.ground.draw_water(&mut pass, &frame);
        self.glass.set(&mut pass, &groups);
        models.draw(&mut pass, Surface::Blended);
    }
}

/// A pass of the lit world over the sky and the prepass's depth, with the
/// fog mask beside it (cleared when `first`); both resolve.
fn world_pass<'a>(
    encoder: &'a mut CommandEncoder,
    t: &'a crate::frame::Targets,
    label: &'static str,
    first: bool,
) -> RenderPass<'a> {
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[
            Some(RenderPassColorAttachment {
                view: &t.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&t.lit),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &t.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&t.fog_mask),
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
        depth_stencil_attachment: Some(depth(&t.depth, LoadOp::Load, false)),
        ..Default::default()
    })
}

/// Where the UI's units will stand: translucent, over the fogged world,
/// reading its depth and writing none (`encodeGhosts`).
pub struct Ghosts {
    ghost: Bound,
}

impl Ghosts {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            ghost: pipelines.bound("modelGhost"),
        }
    }
}

impl Pass for Ghosts {
    fn name(&self) -> &'static str {
        "ghosts"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let models = inputs.models;
        if !models.has_ghosts() {
            return;
        }
        let t = inputs.targets;
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("placement-ghosts"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &t.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&t.hdr),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(depth(&t.depth, LoadOp::Load, true)),
            ..Default::default()
        });
        let mut groups = inputs.environment.groups().to_vec();
        groups.push(models.group());
        self.ghost.set(&mut pass, &groups);
        models.draw_ghosts(&mut pass);
    }
}
