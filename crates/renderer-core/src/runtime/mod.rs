//! Resource composition and canonical presentation preparation shared by all shells.
mod assets;
mod controls;
mod stats;
pub use assets::{PendingAssets, ReadyAssets};
mod world;
pub use controls::Controls;
pub use world::WorldGeometry;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use presentation::light::LightSettings;
use presentation::presentation::Presentation;
use presentation::records::{FogLook, View};
use presentation::shadow::SunShadowFrame;
use wgpu::*;

use crate::bundle::BundleSource;
use crate::effects::{EffectSheets, EffectsResources};
use crate::environment::Environment;
use crate::fog::Fog;
use crate::fog_mask::{self, FogMaskResources};
use crate::frame::{FrameInputs, FrameRenderer, PaintFrame};
use crate::grass::Grass;
use crate::ground::{Ground, GroundData};
use crate::models::Models;
use crate::overlay::{self, OverlayResources};
use crate::paint::{PaintGeometry, PaintResources};
use crate::pipelines::Pipelines;
use crate::post::{self, PostResources};
use crate::scenery::Scenery;
use crate::shadow::ShadowResources;
use crate::sky::Sky;
use crate::terrain::TerrainUpload;
use crate::xray::XrayResources;
use crate::Targets;

static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);

#[derive(Clone)]
pub struct RuntimeConfig {
    pub size: [u32; 2],
    pub light: LightSettings,
    pub fog_look: FogLook,
    /// None disables grass; otherwise density scales the authored field.
    pub grass_density: Option<f64>,
    pub xray_min_hidden_fraction: Option<f32>,
    pub paint: Option<crate::paint::Paint>,
    pub separate_high_air: bool,
    pub controls: Controls,
}

/// CPU inputs rather than platform-assembled GPU resources. Presentation is
/// borrowed only during synchronous admission and preparation.
pub struct RuntimeWorld<'a, S: BundleSource> {
    pub presentation: &'a mut Presentation,
    pub assets: &'a S,
    pub geometry: Arc<WorldGeometry>,
    pub effect_sheets: Arc<EffectSheets>,
}

/// External player geometry; resource lifetimes remain with its caller.
#[derive(Default)]
pub struct PlayerGeometry<'a> {
    pub overlay: overlay::Geometry<'a>,
    pub paint: PaintGeometry<'a>,
}

pub struct Runtime {
    frame_metadata: stats::FrameMetadata,
    pub pipelines: Arc<Pipelines>,
    pub targets: Targets,
    pub environment: Environment,
    pub shadow: ShadowResources,
    pub ground: Ground,
    pub grass: Option<Grass>,
    pub models: Models,
    pub scenery: Scenery,
    pub effects: EffectsResources,
    pub fog: Fog,
    pub sky: Sky,
    pub frame: FrameRenderer,
    pub post: PostResources,
    pub fog_mask: FogMaskResources,
    pub overlays: OverlayResources,
    pub xray: Option<XrayResources>,
    pub paint: Option<PaintResources>,
    pub high_air: Option<crate::high_air::HighAir>,
    config: RuntimeConfig,
    geometry: Arc<WorldGeometry>,
    overlay_glow: [f32; 4],
    generation: u32,
    refresh_statics: bool,
    refresh_knowledge: bool,
    prepared_frame: Option<u32>,
    pending_catalog_frame: Option<u32>,
}

impl Runtime {
    /// Construct from the shared built-in pipeline catalog. Platform shells
    /// do not provide a second manifest or shader source loader.
    pub fn new<S: BundleSource>(
        device: &Device,
        queue: &Queue,
        mut config: RuntimeConfig,
        world: RuntimeWorld<'_, S>,
    ) -> Result<Self, String> {
        config.size = crate::render_size(config.size);
        if let Some(paint) = config.paint {
            paint.validate()?;
        }
        let pipelines = Arc::new(crate::pipelines::build_builtin(device)?);
        let targets = Targets::new(device, config.size);
        let sky = Sky::new(device, queue, &pipelines, &config.light.sky());
        let cold = SunShadowFrame::cold(&config.light);
        let shadow = ShadowResources::new(
            device,
            queue,
            &pipelines,
            config.light.cascades.map_size,
            bytemuck::bytes_of(&cold.receiver),
        );
        let environment = Environment::new(
            device,
            queue,
            &pipelines,
            &sky.lut,
            &config.light.sun(),
            &config.light.aerial(),
            &shadow,
        );
        let presentation = world.presentation;
        let resolved = presentation
            .ground()
            .ok_or("renderer world has no presentation ground")?;
        let tables = resolved.terrain_tables(resolved.terrain_field()?);
        let plots = resolved.plot_records(true);
        let ground = Ground::from_data(
            device,
            queue,
            &pipelines,
            GroundData {
                material: TerrainUpload {
                    params: bytemuck::cast_slice(&tables.params),
                    nodes: bytemuck::cast_slice(&tables.nodes),
                    plots: bytemuck::cast_slice(&plots),
                    surfaces: bytemuck::cast_slice(&tables.surfaces),
                    surface_index: bytemuck::cast_slice(&tables.surface_index),
                    scars: bytemuck::cast_slice(&tables.scars),
                },
                heights: &resolved.source.grid.pages(),
                terrain: &world.geometry.terrain,
                skirt: &world.geometry.skirt,
                backdrop: &world.geometry.backdrop,
                water: &world.geometry.water,
            },
        );
        let grass = assets::grass(
            device,
            queue,
            &pipelines,
            world.assets,
            presentation.catalog(),
            &ground.heights,
            config.grass_density,
        )?;
        let mut models = Models::new(
            device,
            queue,
            &pipelines,
            world.assets,
            presentation.catalog(),
        )?;
        models.set_high_apart(config.separate_high_air);
        let scenery = Scenery::new(
            device,
            queue,
            &pipelines,
            world.assets,
            presentation.catalog(),
        )?;
        let effects = EffectsResources::new(
            device,
            queue,
            &pipelines,
            &targets,
            &environment,
            world.effect_sheets,
        );
        let mut fog = Fog::with_heights(device, queue, &pipelines, &targets.depth, &ground.heights);
        let paint = config.paint.map(|style| {
            fog.set_paint(device, queue, &targets.paint, &style);
            PaintResources::new(&pipelines)
        });
        let fog_mask = fog_mask::fog_mask(device, queue, &pipelines, &targets, &config.fog_look);
        let post = post::new(
            device,
            queue,
            &pipelines,
            &targets,
            bytemuck::bytes_of(&config.light.bloom()),
            bytemuck::bytes_of(&config.light.grade()),
        );
        let overlay_glow = [0.0; 4];
        let overlays = overlay::new(
            device,
            queue,
            &pipelines,
            &targets,
            bytemuck::cast_slice(&overlay_glow),
            true,
        );
        let xray = config
            .xray_min_hidden_fraction
            .map(|fraction| XrayResources::new(device, queue, &pipelines, &targets, fraction))
            .transpose()?;
        let high_air = config
            .separate_high_air
            .then(|| crate::high_air::HighAir::new(device, &pipelines, &targets));
        let frame = FrameRenderer::new(&pipelines);
        // Presentation and drawing share one local terrain receiver hierarchy.
        presentation.set_shadow(Some((
            config.light.clone(),
            ground.terrain_partition.clone(),
        )));
        let controls = config.controls;
        let mut runtime = Self {
            frame_metadata: stats::FrameMetadata::new(presentation),
            pipelines,
            targets,
            environment,
            shadow,
            ground,
            grass,
            models,
            scenery,
            effects,
            fog,
            sky,
            frame,
            post,
            fog_mask,
            overlays,
            xray,
            paint,
            high_air,
            config,
            geometry: world.geometry,
            overlay_glow,
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            refresh_statics: true,
            refresh_knowledge: true,
            prepared_frame: None,
            pending_catalog_frame: None,
        };
        runtime.set_controls(queue, presentation, controls)?;
        Ok(runtime)
    }

    /// GPU readback for authored cards never borrows canonical presentation.
    /// Complete this admission phase before preparing the first frame.
    pub async fn prepare_assets(&mut self, device: &Device, queue: &Queue) -> Result<(), String> {
        self.models.bake_cards(device, queue, &self.pipelines).await
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
    /// Retained external meshes; the source library and effect sheets are
    /// reported by their own CPU lifetime owners.
    pub fn geometry_payload_bytes(&self) -> usize {
        self.geometry.payload_bytes()
    }

    pub fn geometry(&self) -> Arc<WorldGeometry> {
        self.geometry.clone()
    }
    pub fn static_generation(&self) -> u32 {
        self.generation
    }

    /// Skipping a prepared publication means the next drawn frame cannot rely
    /// on the previous generation's held batch writes.
    pub fn invalidate_statics(&mut self) {
        self.generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        self.refresh_statics = true;
    }

    /// Prepare exactly the current canonical frame. Call only after acquiring
    /// a drawable output: this consumes incremental fog/scar knowledge.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &mut Presentation,
    ) -> Result<(), String> {
        if self.pending_catalog_frame == Some(presentation.last_frame().index) {
            return Err("publish a fresh canonical frame after appearance commit".into());
        }
        self.pending_catalog_frame = None;
        if self.refresh_statics {
            presentation.resend_static_frame(self.generation);
        }
        self.restore_knowledge(presentation);
        if presentation.scars_changed() {
            if let Some(grass) = &mut self.grass {
                grass.regrow();
            }
        }
        let frame = presentation.last_frame();
        let camera = frame.camera;
        let wrap = frame.light_wrap;
        let index = frame.index;
        let changed = self.refresh_statics || self.prepared_frame != Some(index);
        self.environment.prepare(queue, &frame.view, &camera);
        self.ground.prepare(&camera, frame.shadow.as_ref());
        if let Some(fit) = &frame.shadow {
            let cameras: Vec<&[u8]> = fit.casters.iter().map(bytemuck::bytes_of).collect();
            self.shadow
                .prepare(queue, bytemuck::bytes_of(&fit.receiver), &cameras);
        }
        if let Some(grass) = self.grass.as_mut().filter(|_| self.config.controls.grass) {
            grass.prepare(queue, presentation);
        }
        if changed {
            self.models.prepare(device, queue, frame)?;
            self.scenery.prepare(self.models.statics());
            if let Some(xray) = &mut self.xray {
                xray.prepare(device, &self.models);
            }
        }
        let bounds = self.grass.as_ref().and_then(Grass::scar_bounds);
        self.ground
            .prepare_scars(device, presentation, bounds, self.targets.size);
        self.fog.prepare(device, queue, presentation, &camera)?;
        if changed {
            self.environment.set_cast_lights(
                queue,
                if self.config.controls.cast_lights {
                    presentation.lights()
                } else {
                    &[]
                },
                wrap,
            );
            self.effects.prepare(
                device,
                queue,
                bytemuck::cast_slice(if self.config.controls.effects {
                    presentation.effects()
                } else {
                    &[]
                }),
            );
        }
        self.frame_metadata.update(presentation);
        self.prepared_frame = Some(index);
        self.refresh_statics = false;
        Ok(())
    }

    fn restore_knowledge(&mut self, presentation: &mut Presentation) {
        if self.refresh_knowledge {
            presentation.resend_scars();
            presentation.resend_fog();
            self.refresh_knowledge = false;
        }
    }

    /// Queries can be the first consumer after recovery. Restore canonical
    /// retained work before either a probe or a drawn frame consumes it.
    pub fn probe_fog(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &mut Presentation,
        points: &[crate::probes::FogPoint],
    ) -> Result<crate::probes::ProbeReadback, String> {
        let count = u32::try_from(points.len()).map_err(|e| e.to_string())?;
        self.restore_knowledge(presentation);
        let upload = presentation
            .prepare_fog(None, self.config.size[0], self.config.size[1], count)?
            .unwrap_or_default();
        crate::probes::FogProbes::new(&self.pipelines).probe(
            device,
            queue,
            &mut self.fog,
            &upload,
            points,
        )
    }

    pub fn encode(
        &self,
        device: &Device,
        queue: &Queue,
        view: &View,
        encoder: &mut CommandEncoder,
        geometry: PlayerGeometry<'_>,
    ) {
        self.frame.encode(
            &FrameInputs {
                device,
                queue,
                view,
                frame_view: self.config.controls.view,
                targets: &self.targets,
                sky: &self.sky,
                environment: &self.environment,
                shadow: &self.shadow,
                ground: &self.ground,
                grass: self.grass.as_ref().filter(|_| self.config.controls.grass),
                models: &self.models,
                scenery: &self.scenery,
                effects: &self.effects,
                fog: &self.fog,
                fog_mask: &self.fog_mask,
                post: &self.post,
                overlays: &self.overlays,
                overlay_geometry: geometry.overlay,
                paint: self.paint.as_ref().map(|resources| PaintFrame {
                    resources,
                    geometry: geometry.paint,
                    shown: self.config.controls.paint,
                }),
                xray: self.xray.as_ref(),
                high_air: self.high_air.as_ref(),
            },
            encoder,
        );
    }

    pub fn set_fog_look(&mut self, queue: &Queue, look: FogLook) {
        self.fog_mask.set_look(queue, &look);
        self.config.fog_look = look;
    }

    pub fn set_overlay_glow(&mut self, queue: &Queue, glow: [f32; 4]) {
        self.overlays.write_glow(queue, bytemuck::cast_slice(&glow));
        self.overlay_glow = glow;
    }

    pub fn resize(
        &mut self,
        device: &Device,
        queue: &Queue,
        size: [u32; 2],
        presentation: &mut Presentation,
    ) -> bool {
        let size = crate::render_size(size);
        if !self.targets.rebuild(device, size) {
            return false;
        }
        self.config.size = size;
        self.post = post::new(
            device,
            queue,
            &self.pipelines,
            &self.targets,
            bytemuck::bytes_of(&self.config.light.bloom()),
            bytemuck::bytes_of(&self.config.light.grade()),
        );
        self.fog_mask = fog_mask::fog_mask(
            device,
            queue,
            &self.pipelines,
            &self.targets,
            &self.config.fog_look,
        );
        self.fog_mask.set_mask_view(
            queue,
            match self.config.controls.view {
                crate::frame::FrameView::FogMask => fog_mask::FogMaskView::Fog,
                crate::frame::FrameView::GroundMask => fog_mask::FogMaskView::Ground,
                crate::frame::FrameView::GroundClasses => fog_mask::FogMaskView::Classes,
                _ => fog_mask::FogMaskView::None,
            },
        );
        self.overlays = overlay::new(
            device,
            queue,
            &self.pipelines,
            &self.targets,
            bytemuck::cast_slice(&self.overlay_glow),
            true,
        );
        self.fog.rebind_depth(device, &self.targets.depth);
        if let Some(style) = self.config.paint {
            self.fog
                .set_paint(device, queue, &self.targets.paint, &style);
        }
        if let Some(xray) = &mut self.xray {
            xray.rebind_depth(device, &self.targets);
            xray.set_coverage_enabled(queue, self.config.controls.xray_coverage);
        }
        if self.high_air.is_some() {
            self.high_air = Some(crate::high_air::HighAir::new(
                device,
                &self.pipelines,
                &self.targets,
            ));
        }
        self.effects
            .rebind(device, &self.pipelines, &self.targets, &self.environment);
        // The next draw derives fresh size-dependent dispatches from knowledge.
        presentation.resend_fog();
        true
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        self.targets.record_memory(memory);
        self.environment.record_memory(memory);
        self.shadow.record_memory(memory);
        self.ground.record_memory(memory);
        if let Some(grass) = &self.grass {
            grass.record_memory(memory);
        }
        self.models.record_memory(memory);
        self.scenery.record_memory(memory);
        self.effects.record_memory(memory);
        self.fog.record_memory(memory);
        self.sky.record_memory(memory);
        self.post.record_memory(memory);
        self.fog_mask.record_memory(memory);
        self.overlays.record_memory(memory);
        if let Some(xray) = &self.xray {
            xray.record_memory(memory);
        }
        if let Some(high) = &self.high_air {
            high.record_memory(memory);
        }
    }
}
