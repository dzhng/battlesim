//! The world's passes (`frame/worldPass.ts`), in the order `battleFrame`
//! encodes them: the depth prepass, then the grass's growth, the sky and
//! the lit colour pass with its fog mask, then the water over their samples,
//! both resolved for the effects and the fog mask passes, then the placement
//! ghosts over the fogged world.
//! The ground, the scenery and the models draw into them.
use wgpu::*;

use crate::fog::Layer;
use crate::frame::{Inputs, Pass, Targets};
use crate::gpu::bind;
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
        inputs
            .scenery
            .draw_depth(&mut pass, camera, models.statics());
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
/// the skirt, the models, the scenery, the grass, the backdrop, and glass
/// last, over everything. Both resolve, the lit world into `lit` and the mask
/// into `fog_mask`, and keep their samples for the water.
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
                let mut pass = world_pass(&mut region, inputs.targets, "ground-region", first);
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
        let mut pass = world_pass(encoder, inputs.targets, "world", !drew);
        if !regional {
            ground.draw_terrain(&mut pass, &painted_ground);
        }
        let faces = fogged(Layer::Faces);
        inputs.ground.draw_skirt(&mut pass, &faces);
        let models = inputs.models;
        // The models by how fog takes them, as `worldPass.ts` draws them.
        for layer in [
            Layer::Units,
            Layer::Faces,
            Layer::Ground,
            Layer::PaintedFaces,
        ] {
            let mut groups = fogged(layer);
            groups.push(models.group());
            for (bound, surface) in &self.surfaces {
                bound.set(&mut pass, &groups);
                models.draw_fogged(&mut pass, *surface, layer);
            }
        }
        inputs.scenery.draw(&mut pass, &faces, models.statics());
        inputs.grass.draw(&mut pass, &painted_ground);
        inputs.ground.draw_backdrop(&mut pass, &painted_ground);
        // Glass takes no fog: it draws over what is fogged behind it.
        let mut groups = frame.to_vec();
        groups.push(models.group());
        self.glass.set(&mut pass, &groups);
        models.draw(&mut pass, Surface::Blended);
    }
}

/// A pass of the lit world over the sky and the prepass's depth, with the
/// fog mask beside it (cleared when `first`); both resolve, and keep their
/// samples for the pass after (a region of the ground, or the water).
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

/// The water over the world's samples, resolved again into `lit` and
/// `fog_mask` (`encode`), over a copy of `lit` (`scene_colour`) and its
/// mips, which it shows of its bed by depth, refracted and blurred where
/// its guard (`fog_mask_under`) says the bed is seen and unmarked: it reads
/// depth and writes none, and draws over any pane it lies behind.
pub struct Water {
    mip: RenderPipeline,
    seed: RenderPipeline,
    guard: RenderPipeline,
    /// Each mip's view and the group that reads the one before it, the
    /// copy's then the guard's, and the guard's first level and its group.
    mips: Vec<(TextureView, BindGroup)>,
    guards: Vec<(TextureView, BindGroup)>,
    seeded: (TextureView, BindGroup),
}

impl Water {
    /// The water's pass over `targets`, its guard seeded from the fog mask
    /// and `paint`, the fog's ground paint.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        paint: &TextureView,
    ) -> Self {
        let taps = device.create_sampler(&SamplerDescriptor {
            label: Some("scene-colour-mips"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let level = |texture: &Texture, mip| {
            texture.create_view(&TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let colour = &targets.scene_colour_texture;
        let mips = (1..colour.mip_level_count())
            .map(|mip| {
                let source = level(colour, mip - 1);
                let group = bind(
                    device,
                    pipelines.layout("sceneMipLayout"),
                    "scene-colour-mip",
                    &[
                        BindingResource::TextureView(&source),
                        BindingResource::Sampler(&taps),
                    ],
                );
                (level(colour, mip), group)
            })
            .collect();
        let guard = &targets.fog_mask_under_texture;
        let guards = (1..guard.mip_level_count())
            .map(|mip| {
                let source = level(guard, mip - 1);
                let group = bind(
                    device,
                    pipelines.layout("guardMipLayout"),
                    "water-guard-mip",
                    &[BindingResource::TextureView(&source)],
                );
                (level(guard, mip), group)
            })
            .collect();
        let seeded = (
            level(guard, 0),
            bind(
                device,
                pipelines.layout("guardSeedLayout"),
                "water-guard-seed",
                &[
                    BindingResource::TextureView(&targets.fog_mask),
                    BindingResource::TextureView(paint),
                ],
            ),
        );
        Self {
            mip: pipelines.render("sceneColourMip").clone(),
            seed: pipelines.render("waterGuardSeed").clone(),
            guard: pipelines.render("waterGuardMip").clone(),
            mips,
            guards,
            seeded,
        }
    }
}

impl Pass for Water {
    fn name(&self) -> &'static str {
        "water"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let t = inputs.targets;
        // The world as it stands is what the water shows of its bed, and its
        // guard how seen and how marked each pixel of it is.
        let size = Extent3d {
            width: t.size[0],
            height: t.size[1],
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            t.lit_texture.as_image_copy(),
            t.scene_colour_texture.as_image_copy(),
            size,
        );
        let draws = self
            .mips
            .iter()
            .map(|m| (&self.mip, m))
            .chain([(&self.seed, &self.seeded)])
            .chain(self.guards.iter().map(|m| (&self.guard, m)));
        for (pipeline, (view, group)) in draws {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("water-under-mip"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
        let resolved = |view, resolve_target| {
            Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: Some(resolve_target),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("water"),
            color_attachments: &[
                resolved(&t.hdr_msaa, &t.lit),
                resolved(&t.fog_mask_msaa, &t.fog_mask),
            ],
            depth_stencil_attachment: Some(depth(&t.depth, LoadOp::Load, true)),
            ..Default::default()
        });
        let mut groups = inputs.environment.groups().to_vec();
        groups.push(inputs.fog.water_group());
        inputs.ground.draw_water(&mut pass, &groups);
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
