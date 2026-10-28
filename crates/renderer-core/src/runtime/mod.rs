//! Resource composition and canonical presentation preparation shared by all shells.
mod world;
pub use world::WorldGeometry;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use presentation::light::LightSettings;
use presentation::presentation::Presentation;
use presentation::records::{Camera, FogLook, View};
use presentation::shadow::SunShadowFrame;
use presentation::static_world::ground::grass::Quality;
use wgpu::*;

use crate::bundle::BundleSource;
use crate::effects::{EffectSheets, EffectsResources};
use crate::environment::Environment;
use crate::fog::Fog;
use crate::fog_mask::{self, FogMaskResources};
use crate::frame::{FrameInputs, FrameRenderer, OverlayFrame};
use crate::grass::{Grass, GrassData};
use crate::ground::{Ground, GroundData};
use crate::models::Models;
use crate::overlay::{self, OverlayResources};
use crate::pipelines::Pipelines;
use crate::post::{self, PostResources};
use crate::scenery::Scenery;
use crate::shadow::ShadowResources;
use crate::sky::Sky;
use crate::terrain::TerrainUpload;
use crate::TargetOwner;

static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);

#[derive(Clone)]
pub struct RuntimeConfig {
    pub size: [u32; 2],
    pub light: LightSettings,
    pub fog_look: FogLook,
    /// None disables grass; otherwise density scales the authored field.
    pub grass_density: Option<f64>,
}

/// CPU inputs rather than platform-assembled GPU resources. Presentation is
/// borrowed only during synchronous admission and preparation.
pub struct RuntimeWorld<'a, S: BundleSource> {
    pub presentation: &'a mut Presentation,
    pub assets: &'a S,
    pub geometry: Arc<WorldGeometry>,
    pub effect_sheets: Arc<EffectSheets>,
}

/// Shell-owned UI vertex streams; their resources and pass policy are shared.
pub struct OverlayGeometry<'a> {
    pub vertices: &'a Buffer,
    pub instances: &'a Buffer,
    pub vertex_count: u32,
}

/// Actual preparation calls, independent of source record sizes or draw counts.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationStats {
    pub frame_preparations: u64,
    pub model_preparations: u64,
    pub effect_preparations: u64,
}

pub struct Runtime {
    pub pipelines: Pipelines,
    pub targets: TargetOwner,
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
    config: RuntimeConfig,
    geometry: Arc<WorldGeometry>,
    overlay_glow: [f32; 4],
    generation: u32,
    refresh_statics: bool,
    refresh_knowledge: bool,
    prepared_frame: Option<u32>,
    preparation_stats: PreparationStats,
}

impl Runtime {
    pub fn new<S: BundleSource>(
        device: &Device,
        queue: &Queue,
        config: RuntimeConfig,
        world: RuntimeWorld<'_, S>,
        pipeline_manifest: &str,
        source: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let pipelines = Pipelines::build(device, pipeline_manifest, source)?;
        let targets = TargetOwner::new(device, config.size);
        let sky = Sky::new(device, queue, &pipelines, &config.light.sky());
        let cold = SunShadowFrame::cold(&config.light);
        let shadow = ShadowResources::new(
            device,
            &pipelines,
            config.light.cascades.map_size,
            bytemuck::bytes_of(&cold.receiver),
            size_of::<Camera>(),
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
        let grass = config
            .grass_density
            .map(|density| {
                let packed = crate::grass::pack_shapes(world.assets, &resolved.grass_kinds())?;
                let params = resolved.grass_params(
                    &packed.facts,
                    &presentation::static_world::ground::grass::LIMITS,
                    &Quality { density },
                )?;
                Ok::<_, String>(Grass::from_data(
                    device,
                    &pipelines,
                    &ground.heights,
                    GrassData {
                        params: &params,
                        props: &resolved.grass_props(),
                        shapes: &packed.shapes,
                        max_blades: packed.max_blades,
                        capacity: resolved.biome.grass.capacity,
                    },
                ))
            })
            .transpose()?;
        let models = Models::new(device, queue, &pipelines, world.assets, presentation)?;
        let scenery = Scenery::new(device, &pipelines, world.assets, presentation)?;
        let effects = EffectsResources::new(
            device,
            queue,
            &pipelines,
            &targets,
            &environment,
            world.effect_sheets,
        );
        let fog = Fog::with_heights(device, &pipelines, &targets.depth, &ground.heights);
        let fog_mask = fog_mask::fog_mask(device, &pipelines, &targets, &config.fog_look);
        let post = post::new(
            device,
            &pipelines,
            &targets,
            bytemuck::bytes_of(&config.light.bloom()),
            bytemuck::bytes_of(&config.light.grade()),
        );
        let overlay_glow = [0.0; 4];
        let overlays = overlay::new(
            device,
            &pipelines,
            &targets,
            bytemuck::cast_slice(&overlay_glow),
            true,
        );
        let frame = FrameRenderer::new(&pipelines);
        // Presentation and drawing share one local terrain receiver hierarchy.
        presentation.set_shadow(Some((
            config.light.clone(),
            ground.terrain_partition.clone(),
        )));
        Ok(Self {
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
            config,
            geometry: world.geometry,
            overlay_glow,
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            refresh_statics: true,
            refresh_knowledge: true,
            prepared_frame: None,
            preparation_stats: PreparationStats::default(),
        })
    }

    pub fn preparation_stats(&self) -> PreparationStats {
        self.preparation_stats
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
        if self.refresh_statics {
            presentation.resend_static_frame(self.generation);
        }
        if self.refresh_knowledge {
            presentation.resend_scars();
            presentation.resend_fog();
        }
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
        if let Some(grass) = &mut self.grass {
            grass.prepare(queue, presentation);
        }
        if changed {
            self.models.prepare(device, queue, frame)?;
            self.scenery.prepare(self.models.statics());
            self.preparation_stats.model_preparations += 1;
        }
        let bounds = self.grass.as_ref().and_then(Grass::scar_bounds);
        self.ground
            .prepare_scars(device, presentation, bounds, self.targets.size);
        self.fog.prepare(device, queue, presentation, &camera)?;
        if changed {
            self.environment
                .set_cast_lights(queue, presentation.lights(), wrap);
            self.effects
                .prepare(device, queue, bytemuck::cast_slice(presentation.effects()));
            self.preparation_stats.effect_preparations += 1;
        }
        self.prepared_frame = Some(index);
        self.preparation_stats.frame_preparations += 1;
        self.refresh_statics = false;
        self.refresh_knowledge = false;
        Ok(())
    }

    pub fn encode(
        &self,
        device: &Device,
        queue: &Queue,
        view: &View,
        encoder: &mut CommandEncoder,
        geometry: Option<OverlayGeometry<'_>>,
    ) {
        let overlays = geometry.map(|geometry| OverlayFrame {
            resources: &self.overlays,
            vertices: geometry.vertices,
            instances: geometry.instances,
            vertex_count: geometry.vertex_count,
        });
        self.frame.encode(
            &FrameInputs {
                device,
                queue,
                view,
                targets: &self.targets,
                sky: &self.sky,
                environment: &self.environment,
                shadow: &self.shadow,
                ground: &self.ground,
                grass: self.grass.as_ref(),
                models: &self.models,
                scenery: &self.scenery,
                effects: &self.effects,
                fog: &self.fog,
                fog_mask: &self.fog_mask,
                post: &self.post,
                overlays,
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
        size: [u32; 2],
        presentation: &mut Presentation,
    ) -> bool {
        if !self.targets.rebuild(device, size) {
            return false;
        }
        self.config.size = size;
        self.post = post::new(
            device,
            &self.pipelines,
            &self.targets,
            bytemuck::bytes_of(&self.config.light.bloom()),
            bytemuck::bytes_of(&self.config.light.grade()),
        );
        self.fog_mask = fog_mask::fog_mask(
            device,
            &self.pipelines,
            &self.targets,
            &self.config.fog_look,
        );
        self.overlays = overlay::new(
            device,
            &self.pipelines,
            &self.targets,
            bytemuck::cast_slice(&self.overlay_glow),
            true,
        );
        self.fog.rebind_depth(device, &self.targets.depth);
        self.effects
            .rebind(device, &self.pipelines, &self.targets, &self.environment);
        // The next draw derives fresh size-dependent dispatches from knowledge.
        presentation.resend_fog();
        true
    }

    /// Recreate all device resources from retained immutable CPU inputs and the
    /// current presentation, never replaying an earlier frame or knowledge delta.
    pub fn recreate<S: BundleSource>(
        &mut self,
        device: &Device,
        queue: &Queue,
        world: (&S, &mut Presentation),
        pipeline_manifest: &str,
        source: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> Result<(), String> {
        let mut replacement = Self::new(
            device,
            queue,
            self.config.clone(),
            RuntimeWorld {
                presentation: world.1,
                assets: world.0,
                geometry: self.geometry.clone(),
                effect_sheets: self.effects.sheets(),
            },
            pipeline_manifest,
            source,
        )?;
        replacement.set_overlay_glow(queue, self.overlay_glow);
        *self = replacement;
        Ok(())
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
    }
}
