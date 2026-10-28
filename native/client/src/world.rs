//! The world's passes (`frame/worldPass.ts`), in the order `battleFrame`
//! encodes them: the depth prepass, then the grass's growth, the sky and
//! the lit colour pass with its fog mask, both resolved for the effects and
//! the fog mask passes, then the placement ghosts over the fogged world.
//! The ground, the scenery and the models draw into them.
use wgpu::*;

use crate::fog::Layer;
use crate::frame::{Inputs, Pass};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
use crate::sky::Sky;

use renderer_core::world::{Surface as CoreSurface, WorldInputs};

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

pub struct DepthPrepass(renderer_core::depth::DepthPrepass);

impl DepthPrepass {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self(renderer_core::depth::DepthPrepass::new(pipelines))
    }
}

impl Pass for DepthPrepass {
    fn name(&self) -> &'static str {
        "depth-prepass"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        self.0.encode(inputs, encoder);
    }
}

/// The grass grown for the view, then the sky, then the lit world over it at
/// the prepass's depth, with the fog mask beside it (`encode`): the terrain,
/// the skirt, the models, the scenery, the grass, the backdrop, the water,
/// and glass last, over everything. Both resolve: the lit world into `lit`, the mask into
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
        self.sky.prepare(inputs);
        let adapter = WorldAdapter {
            world: self,
            inputs,
        };
        renderer_core::world::encode_sky(&adapter, inputs.targets, encoder);
        let frame = inputs.environment.groups();
        // Each surface binds the fog group of what it is (`fog::Layer`).
        let fogged = |layer| {
            let mut groups = frame.to_vec();
            groups.push(inputs.fog.group(layer));
            groups
        };
        let painted_ground = fogged(Layer::PaintedGround);
        // Too many marks to hold at once: the terrain draws region by region,
        // each submitted after its scars load (the frame so far first).
        let regional = scars.regional();
        if regional {
            let done = std::mem::replace(
                encoder,
                inputs.device.create_command_encoder(&Default::default()),
            );
            queue.submit([done.finish()]);
            for (i, (rect, upload)) in scars.region_uploads().iter().enumerate() {
                scars.apply(queue, upload);
                let mut region = inputs.device.create_command_encoder(&Default::default());
                let first = i == 0;
                let mut pass = renderer_core::world::begin_world_pass(
                    &mut region,
                    inputs.targets,
                    "ground-region",
                    first,
                    true,
                );
                let [x, y, w, h] = *rect;
                pass.set_scissor_rect(x, y, w, h);
                ground.draw_terrain(&mut pass, &painted_ground);
                drop(pass);
                queue.submit([region.finish()]);
            }
        } else if let Some(upload) = scars.view_upload() {
            scars.apply(queue, upload);
        }
        let drew = !scars.region_uploads().is_empty();
        let adapter = WorldAdapter {
            world: self,
            inputs,
        };
        renderer_core::world::encode_world(
            &adapter,
            inputs.targets,
            encoder,
            !drew,
            false,
            !regional,
        );
    }
}

struct WorldAdapter<'a> {
    world: &'a WorldColour,
    inputs: &'a Inputs<'a>,
}

impl WorldAdapter<'_> {
    fn fogged(&self, layer: Layer) -> Vec<(&'static str, &'_ BindGroup)> {
        let mut groups = self.inputs.environment.groups().to_vec();
        groups.push(self.inputs.fog.group(layer));
        groups
    }
}

impl WorldInputs for WorldAdapter<'_> {
    fn draw_sky(&self, pass: &mut RenderPass<'_>) {
        self.world.sky.draw(pass);
    }

    fn draw_ground(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs.ground.draw_terrain(pass, &self.fogged(layer));
    }

    fn draw_skirt(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs.ground.draw_skirt(pass, &self.fogged(layer));
    }

    fn bind_models(&self, pass: &mut RenderPass<'_>, layer: Layer, surface: CoreSurface) {
        let groups = self.fogged(layer);
        let mut groups = groups;
        groups.push(self.inputs.models.group());
        let pipeline = match surface {
            CoreSurface::Opaque => &self.world.surfaces[0].0,
            CoreSurface::Cutout => &self.world.surfaces[1].0,
            CoreSurface::Room => &self.world.surfaces[2].0,
            CoreSurface::Blended => &self.world.glass,
        };
        pipeline.set(pass, &groups);
    }

    fn draw_models(&self, pass: &mut RenderPass<'_>, layer: Layer, surface: CoreSurface) {
        let surface = match surface {
            CoreSurface::Opaque => Surface::Opaque,
            CoreSurface::Cutout => Surface::Cutout,
            CoreSurface::Room => Surface::Room,
            CoreSurface::Blended => Surface::Blended,
        };
        self.inputs.models.draw_fogged(pass, surface, layer);
    }

    fn draw_scenery(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs
            .scenery
            .draw(pass, &self.fogged(layer), self.inputs.models.statics());
    }

    fn draw_grass(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs.grass.draw(pass, &self.fogged(layer));
    }

    fn draw_backdrop(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs.ground.draw_backdrop(pass, &self.fogged(layer));
    }

    fn draw_water(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        self.inputs.ground.draw_water(pass, &self.fogged(layer));
    }

    fn bind_glass(&self, pass: &mut RenderPass<'_>) {
        let mut groups = self.inputs.environment.groups().to_vec();
        groups.push(self.inputs.models.group());
        self.world.glass.set(pass, &groups);
    }

    fn draw_glass(&self, pass: &mut RenderPass<'_>) {
        self.inputs.models.draw(pass, Surface::Blended);
    }
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
