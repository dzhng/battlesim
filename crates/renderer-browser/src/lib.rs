//! Browser shell for the shared renderer core.
//!
//! This shell supplies a browser surface and copied presentation records to
//! the shared resource and pass owners. Authored models use the same admission,
//! indexed geometry, materials and draw runs as the native client.

#![cfg(target_arch = "wasm32")]

use renderer_core::bundle::Library;
use renderer_core::fog_mask::FogMaskResources;
use renderer_core::fog_sight::FogSightResources;
use renderer_core::frame::FrameRenderer;
use renderer_core::grass::Grass;
use renderer_core::ground::Ground;
use renderer_core::models::Models;
use renderer_core::pipelines::Pipelines;
use renderer_core::post::PostResources;
use renderer_core::shadow::{ShadowResources, CAMERA_BYTES, RECEIVER_BYTES};
use renderer_core::TargetOwner;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

pub(crate) mod pipeline_export {
    include!(concat!(env!("OUT_DIR"), "/pipeline_export.rs"));
}
mod asset;
mod browser_effects;
mod diagnostics;
mod export;
mod ground_upload;
mod presentation_upload;
mod recovery;
mod render;
mod scenery;
mod shadow_upload;
mod state;
mod stats_api;
use asset::{ModelFrameSnapshot, ModelWorldSnapshot};
use browser_effects::BrowserEffects;
use ground_upload::{GrassSnapshot, GroundSnapshot};
use renderer_core::environment::Environment;
use renderer_core::sky::Sky;
use scenery::SceneryWorldSnapshot;
use state::LightingSnapshot;
use state::{PresentationSnapshot, Stats};

const DISPLAY_PIPELINES: &str = r#"{
  "sizes":{"bloomLevels":0,"skyLut":[1,1],"effectSheet":1,"grassClump":1},
  "layouts":{"sampled":[
    {"binding":0,"texture":{"sampleType":"float","viewDimension":"2d","multisampled":false},"visibility":["fragment"]},
    {"binding":1,"sampler":{"type":"filtering"},"visibility":["fragment"]}
  ]},
  "pipelines":[{"name":"composite","kind":"render","shader":"display.wgsl","layout":["sampled"],"vertex":{"entryPoint":"fullscreen","buffers":[]},"primitive":{"topology":"triangle-list"},"fragment":{"entryPoint":"composite","targets":[{"format":"bgra8unorm"}]}}]
}"#;

const DISPLAY_SHADER: &str = r#"
struct V { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn fullscreen(@builtin(vertex_index) i: u32) -> V {
  var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
  var out: V; out.position = vec4<f32>(p[i], 0.0, 1.0); out.uv = p[i] * vec2<f32>(0.5,-0.5) + vec2<f32>(0.5); return out;
}
@group(0) @binding(0) var display: texture_2d<f32>;
@group(0) @binding(1) var linear: sampler;
@fragment fn composite(in: V) -> @location(0) vec4<f32> {
  return vec4<f32>(textureSample(display, linear, in.uv).rgb, 1.0);
}
"#;

fn build_exported_pipelines(device: &Device) -> Result<Pipelines, String> {
    Pipelines::build(device, pipeline_export::MANIFEST, |_, shader| {
        pipeline_export::source(shader)
            .map(str::to_owned)
            .ok_or_else(|| format!("missing embedded WGSL source: {shader}"))
    })
}

#[wasm_bindgen]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: Device,
    queue: Queue,
    targets: TargetOwner,
    display_pipelines: Pipelines,
    production_pipelines: Pipelines,
    shared_post: PostResources,
    shared_effects: BrowserEffects,
    shared_fog_mask: FogMaskResources,
    shared_fog_sight: FogSightResources,
    shared_overlay_resources: renderer_core::overlay::OverlayResources,
    shared_shadow: ShadowResources,
    ground: Option<Ground>,
    grass: Option<Grass>,
    ground_snapshot: Option<Arc<GroundSnapshot>>,
    grass_snapshot: Option<Arc<GrassSnapshot>>,
    environment: Environment,
    sky: Sky,
    lighting: LightingSnapshot,
    presentation_snapshot: Option<PresentationSnapshot>,
    frame: FrameRenderer,
    gpu_timer: Option<renderer_core::timing::FrameTimer>,
    models: Option<Models>,
    scenery: Option<renderer_core::scenery::Scenery>,
    scenery_world: Option<Arc<SceneryWorldSnapshot>>,
    model_library: Arc<Library>,
    model_world: Option<Arc<ModelWorldSnapshot>>,
    model_frame: Option<Arc<ModelFrameSnapshot>>,
    overlay_vertices: Buffer,
    overlay_instances: Buffer,
    overlay_vertex_count: u32,
    post_bloom: Vec<f32>,
    post_grade: Vec<f32>,
    display_sampler: Sampler,
    display_group: BindGroup,
    width: u32,
    height: u32,
    format: TextureFormat,
    alpha_mode: CompositeAlphaMode,
    device_lost: Arc<AtomicU32>,
    stats: Stats,
    asset_report: Option<String>,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn record<T: bytemuck::Pod>(words: &[f32], label: &str) -> Result<T, JsValue> {
    bytemuck::try_from_bytes(bytemuck::cast_slice(words))
        .copied()
        .map_err(|error| js_error(format!("{label}: {error}")))
}

#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub async fn new(
        canvas: HtmlCanvasElement,
        sky: Vec<f32>,
        sun: Vec<f32>,
        aerial: Vec<f32>,
    ) -> Result<Renderer, JsValue> {
        let lighting = LightingSnapshot {
            sky: record(&sky, "sky light")?,
            sun: record(&sun, "sun light")?,
            aerial: record(&aerial, "aerial light")?,
        };
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::BROWSER_WEBGPU,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("shared-renderer-browser"),
                required_features: adapter.features() & Features::TIMESTAMP_QUERY,
                required_limits: adapter.limits(),
                experimental_features: ExperimentalFeatures::disabled(),
                memory_hints: MemoryHints::Performance,
                trace: Trace::Off,
            })
            .await
            .map_err(js_error)?;
        device.on_uncaptured_error(Arc::new(|error| {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "shared renderer wgpu error: {error:?}"
            )));
        }));
        let device_lost = Arc::new(AtomicU32::new(0));
        let device_lost_callback = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            device_lost_callback.fetch_add(1, Ordering::Relaxed);
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "shared renderer device lost ({reason:?}): {message}"
            )));
        });
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == TextureFormat::Bgra8Unorm)
            .ok_or_else(|| js_error("no surface format"))?;
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        surface.configure(
            &device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: Default::default(),
                width,
                height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
            },
        );
        let targets = TargetOwner::new(&device, [width, height]);
        let pipeline_export = DISPLAY_PIPELINES.replace(
            "\"bgra8unorm\"",
            &format!(
                "\"{}\"",
                match format {
                    TextureFormat::Bgra8UnormSrgb => "bgra8unorm-srgb",
                    _ => "bgra8unorm",
                }
            ),
        );
        let pipeline_errors = device.push_error_scope(ErrorFilter::Validation);
        let display_pipelines = Pipelines::build(&device, &pipeline_export, |_name, _shader| {
            Ok(DISPLAY_SHADER.to_owned())
        })
        .map_err(js_error)?;
        if let Some(error) = pipeline_errors.pop().await {
            return Err(js_error(format!("shared pipeline validation: {error:?}")));
        }
        let production_errors = device.push_error_scope(ErrorFilter::Validation);
        let production_pipelines = build_exported_pipelines(&device).map_err(js_error)?;
        if let Some(error) = production_errors.pop().await {
            return Err(js_error(format!(
                "production pipeline validation: {error:?}"
            )));
        }
        let shared_post =
            renderer_core::post::new(&device, &production_pipelines, &targets, &[0; 32], &[0; 80]);
        let shared_fog_mask = renderer_core::fog_mask::fog_mask(
            &device,
            &production_pipelines,
            &targets,
            &[0; 96],
            true,
            true,
        );
        let shared_fog_sight =
            FogSightResources::new(&device, &production_pipelines, &targets.depth);
        let shared_overlay_resources =
            renderer_core::overlay::new(&device, &production_pipelines, &targets, &[0; 16], true);
        // Match the presentation light's authored 2048px cascade maps. The
        // earlier 64px diagnostic target made receiver sampling visibly
        // alias across the terrain even when the caster pass was correct.
        let shared_shadow = ShadowResources::new(
            &device,
            &production_pipelines,
            2048,
            &[0; RECEIVER_BYTES],
            CAMERA_BYTES,
        );
        let overlay_vertices = renderer_core::buffer(
            &device,
            "shared-overlay-vertices",
            BufferUsages::VERTEX,
            &[0; 40],
        );
        let overlay_instances = renderer_core::buffer(
            &device,
            "shared-overlay-instance",
            BufferUsages::VERTEX,
            bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0]),
        );
        let sky = Sky::new(&device, &queue, &production_pipelines, &lighting.sky);
        let environment = Environment::new(
            &device,
            &queue,
            &production_pipelines,
            &sky.lut,
            &lighting.sun,
            &lighting.aerial,
            &shared_shadow,
        );
        let shared_effects = BrowserEffects::new(
            &device,
            &targets,
            &production_pipelines,
            &environment.camera_buffer,
            &environment.data,
            &environment.pmrem,
            &environment.cast_lights,
        );
        let frame = FrameRenderer::new(&production_pipelines);
        let display_sampler = device.create_sampler(&SamplerDescriptor::default());
        let display_group = renderer_core::bind(
            &device,
            display_pipelines.layout("sampled"),
            "shared-display-composite-group",
            &[
                BindingResource::TextureView(&targets.display),
                BindingResource::Sampler(&display_sampler),
            ],
        );
        Ok(Self {
            surface,
            device,
            queue,
            targets,
            display_pipelines,
            production_pipelines,
            shared_post,
            shared_effects,
            shared_fog_mask,
            shared_fog_sight,
            shared_overlay_resources,
            shared_shadow,
            ground: None,
            grass: None,
            ground_snapshot: None,
            grass_snapshot: None,
            environment,
            sky,
            lighting,
            presentation_snapshot: None,
            frame,
            gpu_timer: None,
            models: None,
            scenery: None,
            scenery_world: None,
            model_library: Arc::new(Library::default()),
            model_world: None,
            model_frame: None,
            overlay_vertices,
            overlay_instances,
            overlay_vertex_count: 0,
            post_bloom: Vec::new(),
            post_grade: Vec::new(),
            display_sampler,
            display_group,
            width,
            height,
            format,
            alpha_mode: caps.alpha_modes[0],
            device_lost,
            stats: Stats {
                shared_shadow: true,
                shared_environment: true,
                shared_effects: true,
                shared_fog_sight: true,
                ..Stats::default()
            },
            asset_report: None,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resize(&mut self, canvas: HtmlCanvasElement) {
        self.width = canvas.width().max(1);
        self.height = canvas.height().max(1);
        if self
            .targets
            .rebuild(&self.device, [self.width, self.height])
        {
            self.shared_post = renderer_core::post::new(
                &self.device,
                &self.production_pipelines,
                &self.targets,
                &[0; 32],
                &[0; 80],
            );
            self.shared_fog_mask = renderer_core::fog_mask::fog_mask(
                &self.device,
                &self.production_pipelines,
                &self.targets,
                &[0; 96],
                true,
                true,
            );
            self.shared_overlay_resources = renderer_core::overlay::new(
                &self.device,
                &self.production_pipelines,
                &self.targets,
                &[0; 16],
                true,
            );
            self.shared_fog_sight
                .rebind_depth(&self.device, &self.targets.depth);
            self.rebuild_effects();
            if !self.post_bloom.is_empty() || !self.post_grade.is_empty() {
                self.shared_post
                    .write(
                        &self.queue,
                        bytemuck::cast_slice(&self.post_bloom),
                        bytemuck::cast_slice(&self.post_grade),
                    )
                    .expect("stored post records fit rebuilt targets");
            }
            if let Some(presentation) = &self.presentation_snapshot {
                if presentation.overlay.len() >= 2 {
                    let glow = [
                        presentation.overlay[0],
                        presentation.overlay[0] / 2.5,
                        presentation.overlay[1],
                        0.0,
                    ];
                    self.shared_overlay_resources
                        .write_glow(&self.queue, bytemuck::cast_slice(&glow));
                }
            }

            self.display_group = renderer_core::bind(
                &self.device,
                self.display_pipelines.layout("sampled"),
                "shared-display-composite-group",
                &[
                    BindingResource::TextureView(&self.targets.display),
                    BindingResource::Sampler(&self.display_sampler),
                ],
            );
            if let Some(presentation) = &self.presentation_snapshot {
                self.shared_fog_mask
                    .write_look(&self.queue, bytemuck::cast_slice(&presentation.fog))
                    .expect("presentation fog look fits shared fog owner");
            }
            self.stats.resizes += 1;
            self.stats.target_generation = self.targets.generation();
        }
        self.configure_surface();
    }

    fn rebuild_effects(&mut self) {
        let effects = self
            .presentation_snapshot
            .as_ref()
            .map(|snapshot| snapshot.effects.as_slice())
            .unwrap_or_default();
        self.shared_effects.rebuild(
            &self.device,
            &self.queue,
            &self.production_pipelines,
            browser_effects::EffectsFrame {
                targets: &self.targets,
                camera: &self.environment.camera_buffer,
                environment: &self.environment.data,
                pmrem: &self.environment.pmrem,
                cast_lights: &self.environment.cast_lights,
            },
            effects,
        );
    }

    fn configure_surface(&self) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                color_space: Default::default(),
                width: self.width,
                height: self.height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: self.alpha_mode,
                view_formats: vec![],
            },
        );
    }

    /// Reconfigure the existing browser surface after an outdated/lost
    /// surface status. The device and shared resources remain valid.
    #[wasm_bindgen]
    pub fn recover_surface(&mut self) {
        self.configure_surface();
        self.stats.surface_recoveries += 1;
    }

    /// Deliberately destroy the device so the browser scene can prove that
    /// `recover_device` handles a real device-loss callback, rather than only
    /// rebuilding an outdated surface.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn destroy_device_for_test(&self) {
        self.device.destroy();
    }
}
