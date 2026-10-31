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
use crate::overlay::{Geometry, OverlayResources};
use crate::paint::{PaintGeometry, PaintResources};
use crate::pipelines::{Bound, Pipelines};
use crate::post::{PostMode, PostResources};
use crate::scenery::Scenery;
use crate::shadow::ShadowResources;
use crate::sky::Sky;
use crate::water::{WaterDetail, WaterPass, WaveField};
use crate::world::WorldColour;
use crate::xray::XrayResources;
use crate::Targets;

/// Player display and inspector outputs share the same prepared GPU frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrameView {
    #[default]
    Final,
    World,
    OverlaysOnBlack,
    OverlaysOnWhite,
    FogMask,
    GroundMask,
    GroundClasses,
    Paint,
}

impl FrameView {
    pub fn mask_view(self) -> crate::fog_mask::FogMaskView {
        use crate::fog_mask::FogMaskView;
        match self {
            Self::FogMask => FogMaskView::Fog,
            Self::GroundMask => FogMaskView::Ground,
            Self::GroundClasses => FogMaskView::Classes,
            _ => FogMaskView::None,
        }
    }

    fn post_mode(self) -> Option<PostMode> {
        match self {
            Self::Final | Self::World => Some(PostMode::Look),
            Self::FogMask | Self::GroundMask => Some(PostMode::Ungraded),
            Self::GroundClasses => Some(PostMode::Raw),
            Self::OverlaysOnBlack | Self::OverlaysOnWhite | Self::Paint => None,
        }
    }

    fn has_display_overlays(self) -> bool {
        matches!(
            self,
            Self::Final | Self::OverlaysOnBlack | Self::OverlaysOnWhite
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStage {
    SunShadow,
    Depth,
    FogSight,
    World,
    Water,
    Effects,
    FogMask,
    Overlays,
    Post,
    FogRim,
}

impl FrameStage {
    pub const ORDER: [Self; 10] = [
        Self::SunShadow,
        Self::Depth,
        Self::FogSight,
        Self::World,
        Self::Water,
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
            Self::Water => "water",
            Self::Effects => "effects",
            Self::FogMask => "fog-mask",
            Self::Overlays => "overlays",
            Self::Post => "post",
            Self::FogRim => "fog-rim",
        }
    }
}

/// Player paint uses world-only depth before unit coverage joins it.
pub struct PaintFrame<'a> {
    pub resources: &'a PaintResources,
    pub geometry: PaintGeometry<'a>,
    pub shown: bool,
}

pub struct FrameInputs<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub view: &'a View,
    pub frame_view: FrameView,
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
    pub water: &'a WaterPass,
    pub water_detail: WaterDetail,
    pub waves: &'a WaveField,
    pub post: &'a PostResources,
    pub overlays: &'a OverlayResources,
    pub overlay_geometry: Geometry<'a>,
    pub paint: Option<PaintFrame<'a>>,
    pub xray: Option<&'a XrayResources>,
    pub high_air: Option<&'a crate::high_air::HighAir>,
}

/// Pipeline policy for a complete production frame, shared on every target.
pub struct FrameRenderer {
    frames: std::cell::Cell<u64>,
    depth: DepthPrepass,
    world: WorldColour,
    caster: Bound,
    caster_cutout: Bound,
    ghost: Bound,
}

impl FrameRenderer {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            frames: Default::default(),
            depth: DepthPrepass::new(pipelines),
            world: WorldColour::new(pipelines),
            caster: pipelines.bound("modelCaster"),
            caster_cutout: pipelines.bound("modelCasterCutout"),
            ghost: pipelines.bound("modelGhost"),
        }
    }

    pub fn frames(&self) -> u64 {
        self.frames.get()
    }

    /// May submit and replace `encoder` while loading regional scar pages.
    /// Initial/final timestamp markers belong before/after this call, so they
    /// span those ordered submissions as well as the final command buffer.
    pub fn encode(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        self.encode_with_post(inputs, encoder, None);
    }

    pub fn encode_with_post(
        &self,
        inputs: &FrameInputs<'_>,
        encoder: &mut CommandEncoder,
        post_mode_override: Option<PostMode>,
    ) {
        inputs.ground.submissions.reset();
        inputs.models.submissions.reset();
        inputs.scenery.submissions.reset();
        let display_overlays = inputs.xray.is_some()
            || inputs.overlay_geometry.opaque.is_some()
            || inputs.overlay_geometry.translucent.is_some();
        let player_depth = display_overlays || inputs.paint.is_some();
        for stage in FrameStage::ORDER {
            // Regional ground draws replace the encoder; their render-pass
            // labels provide boundaries without a debug group crossing submit.
            if stage != FrameStage::World {
                encoder.push_debug_group(stage.label());
            }
            match stage {
                FrameStage::SunShadow => self.encode_shadows(inputs, encoder),
                FrameStage::Depth => {
                    self.depth.encode_world(inputs, encoder, player_depth);
                    if player_depth {
                        if let Some(paint) = &inputs.paint {
                            paint.resources.encode(
                                encoder,
                                inputs.targets,
                                &inputs.environment.camera,
                                paint.geometry,
                                paint.shown,
                            );
                        }
                        if let Some(xray) = inputs.xray {
                            xray.encode(
                                encoder,
                                inputs.targets,
                                &inputs.environment.camera,
                                inputs.models,
                            );
                        } else {
                            inputs.overlays.clear(encoder, inputs.targets);
                        }
                        self.depth.encode_models(inputs, encoder);
                    }
                }
                FrameStage::FogSight => inputs.fog.encode(encoder),
                FrameStage::World => self.world.encode(inputs, encoder),
                FrameStage::Water => {
                    inputs.waves.encode(encoder);
                    if inputs.frame_view != FrameView::GroundClasses {
                        let refracted = inputs.water_detail == WaterDetail::Full;
                        inputs.water.encode(inputs, encoder, refracted);
                    }
                }
                FrameStage::Effects => {
                    if inputs.frame_view != FrameView::GroundClasses {
                        inputs.effects.encode(encoder, inputs.targets);
                    }
                }
                FrameStage::FogMask => inputs.fog_mask.encode(encoder, inputs.targets),
                FrameStage::Overlays => {
                    if matches!(inputs.frame_view, FrameView::Final | FrameView::World) {
                        self.encode_ghosts(inputs, encoder);
                    }
                    if display_overlays && inputs.frame_view.has_display_overlays() {
                        inputs.overlays.encode_geometry(
                            encoder,
                            inputs.targets,
                            &inputs.environment.camera,
                            inputs.overlay_geometry,
                        );
                    }
                }
                FrameStage::Post => {
                    let post_mode = post_mode_override.or_else(|| inputs.frame_view.post_mode());
                    if let Some(mode) = post_mode {
                        inputs.post.encode(encoder, mode);
                    } else {
                        let value = if inputs.frame_view == FrameView::OverlaysOnWhite {
                            1.0
                        } else {
                            0.0
                        };
                        encoder.begin_render_pass(&RenderPassDescriptor {
                            label: Some("inspector-backdrop"),
                            color_attachments: &[Some(RenderPassColorAttachment {
                                view: &inputs.targets.display,
                                depth_slice: None,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Clear(Color {
                                        r: value,
                                        g: value,
                                        b: value,
                                        a: 1.0,
                                    }),
                                    store: StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                    }
                }
                FrameStage::FogRim => {
                    if post_mode_override.is_some() || inputs.frame_view.post_mode().is_some() {
                        inputs.fog_mask.encode_rim(encoder, inputs.targets);
                    }
                    // In HDR mode, also draw the rim into the HDR output at reference white.
                    if post_mode_override == Some(PostMode::Hdr) {
                        inputs.fog_mask.encode_rim_hdr(encoder, &inputs.post.hdr_output);
                    }
                }
            }
            if stage != FrameStage::World {
                encoder.pop_debug_group();
            }
        }
        if inputs.frame_view == FrameView::Paint {
            if inputs.paint.as_ref().is_some_and(|paint| paint.shown) {
                inputs
                    .overlays
                    .encode_paint(encoder, &inputs.targets.display);
            }
        } else if display_overlays && inputs.frame_view.has_display_overlays() {
            inputs
                .overlays
                .encode_composite(encoder, inputs.targets, &inputs.targets.display);
        }
        if let Some(high_air) = inputs.high_air {
            if inputs.frame_view == FrameView::Final {
                high_air.encode(
                    encoder,
                    inputs.targets,
                    inputs.environment,
                    inputs.models,
                    inputs.fog,
                    inputs.post,
                );
            } else {
                high_air.clear(encoder);
            }
        }
        self.frames.set(self.frames.get() + 1);
    }

    fn encode_shadows(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder) {
        inputs
            .shadow
            .encode_cascades(encoder, |cascade, camera, pass| {
                let camera = ("cameraLayout", camera);
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
