//! The world's passes (`frame/worldPass.ts`), in the order `battleFrame`
//! encodes them: the depth prepass, then the lit colour pass with its fog
//! mask, then the placement ghosts over both. Today only the model layer
//! draws into them; the ground and the scenery join the same passes.
use wgpu::*;

use crate::frame::{Inputs, Pass};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};

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

/// The frame's depth: every opaque surface before any colour, so the colour
/// pass shades each sample once (`encodeDepth`).
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
        for (bound, surface) in [
            (&self.solid, Surface::Solid),
            (&self.cutout, Surface::Cutout),
        ] {
            bound.set(&mut pass, &[camera, models.group()]);
            models.draw(&mut pass, surface);
        }
    }
}

/// The lit world over the sky, at the prepass's depth, with the fog mask
/// beside it; glass last, over everything (`encode`).
pub struct WorldColour {
    surfaces: Vec<(Bound, Surface)>,
}

impl WorldColour {
    pub fn new(pipelines: &Pipelines) -> Self {
        let surfaces = [
            ("modelOpaque", Surface::Opaque),
            ("modelCutout", Surface::Cutout),
            ("modelRooms", Surface::Room),
            ("modelBlended", Surface::Blended),
        ];
        Self {
            surfaces: surfaces
                .into_iter()
                .map(|(name, surface)| (pipelines.bound(name), surface))
                .collect(),
        }
    }
}

impl Pass for WorldColour {
    fn name(&self) -> &'static str {
        "world"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let t = inputs.targets;
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &t.hdr_msaa,
                    depth_slice: None,
                    resolve_target: Some(&t.hdr),
                    // Kept for the ghosts, which draw over it.
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &t.fog_mask_msaa,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Discard,
                    },
                }),
            ],
            depth_stencil_attachment: Some(depth(&t.depth, LoadOp::Load, false)),
            ..Default::default()
        });
        let models = inputs.models;
        let mut groups = inputs.environment.groups().to_vec();
        groups.push(models.group());
        for (bound, surface) in &self.surfaces {
            bound.set(&mut pass, &groups);
            models.draw(&mut pass, *surface);
        }
    }
}

/// Where the UI's units will stand: translucent, over the finished world,
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
