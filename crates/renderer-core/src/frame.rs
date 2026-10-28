//! One production GPU frame over the shared renderer's concrete owners.
//! Shells prepare presentation records, transport assets and present/read back
//! the display texture; they do not select or encode renderer passes.
use presentation::records::View;
use wgpu::*;

use crate::depth::DepthPrepass;
use crate::effects::EffectsResources;
use crate::environment::Environment;
use crate::fog::Fog;
use crate::fog_mask::FogMaskResources;
use crate::grass::Grass;
use crate::ground::Ground;
use crate::models::{Models, Surface};
use crate::overlay::OverlayResources;
use crate::pipelines::{Bound, Pipelines};
use crate::post::PostResources;
use crate::scenery::Scenery;
use crate::shadow::ShadowResources;
use crate::sky::Sky;
use crate::world::WorldColour;
use crate::Targets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStage {
    SunShadow,
    Depth,
    FogSight,
    World,
    Effects,
    FogMask,
    Overlays,
    Post,
    FogRim,
}

impl FrameStage {
    pub const ORDER: [Self; 9] = [
        Self::SunShadow,
        Self::Depth,
        Self::FogSight,
        Self::World,
        Self::Effects,
        Self::FogMask,
        Self::Overlays,
        Self::Post,
        Self::FogRim,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::SunShadow => "sun-shadow",
            Self::Depth => "depth-prepass",
            Self::FogSight => "fog-sight",
            Self::World => "world",
            Self::Effects => "effects",
            Self::FogMask => "fog-mask",
            Self::Overlays => "overlays",
            Self::Post => "post",
            Self::FogRim => "fog-rim",
        }
    }
}

/// Optional player-facing geometry shares the world's depth and the frame's
/// display output. A shell without player overlays supplies none.
pub struct OverlayFrame<'a> {
    pub resources: &'a OverlayResources,
    pub vertices: &'a Buffer,
    pub instances: &'a Buffer,
    pub vertex_count: u32,
}

pub struct FrameInputs<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub view: &'a View,
    pub targets: &'a Targets,
    pub sky: &'a Sky,
    pub environment: &'a Environment,
    pub shadow: &'a ShadowResources,
    pub ground: &'a Ground,
    pub grass: Option<&'a Grass>,
    pub models: &'a Models,
    pub scenery: &'a Scenery,
    pub effects: &'a EffectsResources,
    pub fog: &'a Fog,
    pub fog_mask: &'a FogMaskResources,
    pub post: &'a PostResources,
    pub overlays: Option<OverlayFrame<'a>>,
}

/// Pipeline policy for a complete production frame, shared on every target.
pub struct FrameRenderer {
    depth: DepthPrepass,
    world: WorldColour,
    caster: Bound,
    caster_cutout: Bound,
    ghost: Bound,
}

impl FrameRenderer {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            depth: DepthPrepass::new(pipelines),
            world: WorldColour::new(pipelines),
            caster: pipelines.bound("modelCaster"),
            caster_cutout: pipelines.bound("modelCasterCutout"),
            ghost: pipelines.bound("modelGhost"),
        }
    }

    /// May submit and replace `encoder` while loading regional scar pages.
    /// Initial/final timestamp markers belong before/after this call, so they
    /// span those ordered submissions as well as the final command buffer.
    pub fn encode(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        for stage in FrameStage::ORDER {
            // Regional ground draws replace the encoder; their render-pass
            // labels provide boundaries without a debug group crossing submit.
            if stage != FrameStage::World {
                encoder.push_debug_group(stage.label());
            }
            match stage {
                FrameStage::SunShadow => self.encode_shadows(inputs, encoder),
                FrameStage::Depth => self.depth.encode(inputs, encoder),
                FrameStage::FogSight => inputs.fog.encode(encoder),
                FrameStage::World => self.world.encode(inputs, encoder),
                FrameStage::Effects => inputs.effects.encode(encoder, inputs.targets),
                FrameStage::FogMask => inputs.fog_mask.encode(encoder, inputs.targets),
                FrameStage::Overlays => {
                    self.encode_ghosts(inputs, encoder);
                    if let Some(overlay) = &inputs.overlays {
                        overlay.resources.encode_geometry(
                            encoder,
                            inputs.targets,
                            &inputs.environment.camera,
                            overlay.vertices,
                            overlay.instances,
                            overlay.vertex_count,
                        );
                    }
                }
                FrameStage::Post => inputs.post.encode(encoder),
                FrameStage::FogRim => inputs.fog_mask.encode_rim(encoder, inputs.targets),
            }
            if stage != FrameStage::World {
                encoder.pop_debug_group();
            }
        }
        if let Some(overlay) = &inputs.overlays {
            overlay
                .resources
                .encode_composite(encoder, inputs.targets, &inputs.targets.display);
        }
    }

    fn encode_shadows(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        inputs
            .shadow
            .encode_cascades(encoder, |cascade, camera, pass| {
                let camera = ("typegpuCameraLayout", camera);
                inputs.ground.draw_casters(pass, camera, cascade);
                for (pipeline, surface) in [
                    (&self.caster, Surface::Solid),
                    (&self.caster_cutout, Surface::Cutout),
                ] {
                    pipeline.set(pass, &[camera, inputs.models.group()]);
                    inputs.models.draw_casters(pass, surface, cascade);
                }
                inputs
                    .scenery
                    .draw_casters(pass, camera, inputs.models.statics(), cascade);
            });
    }

    fn encode_ghosts(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
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
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &t.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        let mut groups = inputs.environment.groups().to_vec();
        groups.push(models.group());
        self.ghost.set(&mut pass, &groups);
        models.draw_ghosts(&mut pass);
    }
}
