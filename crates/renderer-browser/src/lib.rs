//! Browser device, surface and byte transport for the common Rust runtime.
#![cfg(target_arch = "wasm32")]

pub use presentation_wasm::*;
use renderer_core::bundle::Library;
use renderer_core::effects::EffectSheets;
use renderer_core::pipelines::Pipelines;
use renderer_core::runtime::{Runtime, RuntimeConfig, RuntimeWorld, WorldGeometry};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

pub(crate) mod pipeline_export {
    include!(concat!(env!("OUT_DIR"), "/pipeline_export.rs"));
}
mod asset;
mod diagnostics;
mod export;
mod geometry;
mod high_surface;
mod queries;
mod recovery;
mod render;
mod state;
mod stats_api;
mod world;
use state::Stats;

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
  return textureSample(display, linear, in.uv);
}
"#;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    light: presentation::light::LightSettings,
    fog: presentation::fog::FogStyle,
    grass_density: Option<f64>,
    paint: Option<renderer_core::paint::Paint>,
    xray_min_hidden_fraction: Option<f32>,
    controls: Option<renderer_core::runtime::Controls>,
}

#[wasm_bindgen]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    high_surface: Option<high_surface::HighSurface>,
    device: Device,
    queue: Queue,
    runtime: Option<Runtime>,
    config: RuntimeConfig,
    geometry: Option<Arc<WorldGeometry>>,
    effect_sheets: Option<Arc<EffectSheets>>,
    model_library: Arc<Library>,
    display_pipelines: Pipelines,
    display_sampler: Sampler,
    display_group: Option<BindGroup>,
    gpu_timer: Option<renderer_core::timing::FrameTimer>,
    player_meshes: geometry::PlayerMeshes,
    overlay_instances: Buffer,
    overlay_glow: [f32; 4],
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

#[wasm_bindgen]
impl Renderer {
    /// Device admission is asynchronous; canonical presentation borrows are not.
    pub async fn new(
        canvas: HtmlCanvasElement,
        settings: String,
        high_canvas: Option<HtmlCanvasElement>,
    ) -> Result<Renderer, JsValue> {
        let settings: Settings = serde_json::from_str(&settings).map_err(js_error)?;
        if let Some(paint) = settings.paint {
            paint.validate().map_err(js_error)?;
        }
        Self::admit(
            canvas,
            high_canvas.clone(),
            RuntimeConfig {
                size: [1, 1],
                light: settings.light,
                fog_look: settings.fog.look().map_err(js_error)?,
                grass_density: settings.grass_density,
                paint: settings.paint,
                xray_min_hidden_fraction: settings.xray_min_hidden_fraction,
                separate_high_air: high_canvas.is_some(),
                controls: settings.controls.unwrap_or_default(),
            },
        )
        .await
    }

    /// Rebuild targets while synchronously borrowing current knowledge.
    pub fn resize(&mut self, canvas: HtmlCanvasElement, presentation: &mut Presentation) {
        let size = [canvas.width().max(1), canvas.height().max(1)];
        if size != [self.width, self.height] {
            [self.width, self.height] = size;
            self.config.size = size;
            if let Some(runtime) = &mut self.runtime {
                runtime.resize(&self.device, &self.queue, size, presentation.core_mut());
            }
            self.rebind_display();
            self.stats.resizes += 1;
        }
        self.configure_surface();
    }

    pub fn recover_surface(&mut self) {
        self.configure_surface();
        self.stats.surface_recoveries += 1;
    }

    pub fn destroy_device_for_test(&self) {
        self.device.destroy();
    }
}

impl Renderer {
    async fn admit(
        canvas: HtmlCanvasElement,
        high_canvas: Option<HtmlCanvasElement>,
        mut config: RuntimeConfig,
    ) -> Result<Renderer, JsValue> {
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
        let high_surface = high_canvas
            .map(|canvas| {
                high_surface::HighSurface::new(
                    &instance,
                    &adapter,
                    &device,
                    canvas,
                    [width, height],
                )
            })
            .transpose()?;
        let pipeline_errors = device.push_error_scope(ErrorFilter::Validation);
        let display_pipelines = Pipelines::build(&device, DISPLAY_PIPELINES, |_, _| {
            Ok(DISPLAY_SHADER.to_owned())
        })
        .map_err(js_error)?;
        if let Some(error) = pipeline_errors.pop().await {
            return Err(js_error(format!("display pipeline validation: {error:?}")));
        }
        let display_sampler = device.create_sampler(&SamplerDescriptor::default());
        let player_meshes = geometry::PlayerMeshes::new(&device);
        let overlay_instances = renderer_core::buffer(
            &device,
            "overlay-instance",
            BufferUsages::VERTEX,
            bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0]),
        );
        config.size = [width, height];
        Ok(Self {
            surface,
            high_surface,
            device,
            queue,
            runtime: None,
            config,
            geometry: None,
            effect_sheets: None,
            model_library: Arc::new(Library::default()),
            display_pipelines,
            display_sampler,
            display_group: None,
            gpu_timer: None,
            player_meshes,
            overlay_instances,
            overlay_glow: [0.0; 4],
            width,
            height,
            format,
            alpha_mode: caps.alpha_modes[0],
            device_lost,
            stats: Stats::default(),
            asset_report: None,
        })
    }

    fn configure_surface(&self) {
        if let Some(high) = &self.high_surface {
            high.configure(&self.device, [self.width, self.height]);
        }
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

    fn rebind_display(&mut self) {
        if let Some(high) = &mut self.high_surface {
            high.group = self
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.high_air.as_ref())
                .map(|air| {
                    renderer_core::bind(
                        &self.device,
                        self.display_pipelines.layout("sampled"),
                        "high-air-surface",
                        &[
                            BindingResource::TextureView(air.output()),
                            BindingResource::Sampler(&self.display_sampler),
                        ],
                    )
                });
        }

        self.display_group = self.runtime.as_ref().map(|runtime| {
            renderer_core::bind(
                &self.device,
                self.display_pipelines.layout("sampled"),
                "display-surface",
                &[
                    BindingResource::TextureView(&runtime.targets.display),
                    BindingResource::Sampler(&self.display_sampler),
                ],
            )
        });
    }
}
