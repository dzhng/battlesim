//! Browser device, surface and byte transport for the common Rust runtime.
#![cfg(target_arch = "wasm32")]

pub use presentation_wasm::*;
use renderer_core::bundle::Library;
use renderer_core::effects::EffectSheets;
use renderer_core::pipelines::Pipelines;
use renderer_core::runtime::{Runtime, RuntimeConfig, RuntimeWorld, WorldGeometry};
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

mod appearance;
mod asset;
pub use appearance::{AppearanceDraft, PreparedAppearances};
mod diagnostics;
mod geometry;
mod gpu;
mod ground_queries;
pub use gpu::BrowserGpu;
mod high_surface;
mod queries;
mod recovery;
mod render;
mod stats_api;
mod world;

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
    water_detail: Option<renderer_core::water::WaterDetail>,
    controls: Option<renderer_core::runtime::Controls>,
}

#[wasm_bindgen]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    high_surface: Option<high_surface::HighSurface>,
    gpu: Rc<gpu::GpuInner>,
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
    frames: u32,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
impl Renderer {
    /// Clone page ownership before awaiting, so disposal may race admission safely.
    #[wasm_bindgen(js_name = new, unchecked_return_type = "Promise<Renderer>")]
    pub fn create(
        gpu: &BrowserGpu,
        canvas: HtmlCanvasElement,
        settings: String,
        high_canvas: Option<HtmlCanvasElement>,
    ) -> Result<js_sys::Promise, JsValue> {
        let settings: Settings = serde_json::from_str(&settings).map_err(js_error)?;
        if let Some(paint) = settings.paint {
            paint.validate().map_err(js_error)?;
        }
        let gpu = gpu.inner.clone();
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            let renderer = Self::admit(
                gpu,
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
                    water_detail: settings.water_detail.unwrap_or_default(),
                    controls: settings.controls.unwrap_or_default(),
                },
            )
            .await?;
            Ok(renderer.into())
        }))
    }

    /// Rebuild targets while synchronously borrowing current knowledge.
    pub fn resize(&mut self, canvas: HtmlCanvasElement, presentation: &mut Presentation) {
        let size = renderer_core::render_size([canvas.width(), canvas.height()]);
        if size != [self.width, self.height] {
            [self.width, self.height] = size;
            self.config.size = size;
            if let Some(runtime) = &mut self.runtime {
                runtime.resize(
                    &self.gpu.device,
                    &self.gpu.queue,
                    size,
                    presentation.core_mut(),
                );
            }
            self.rebind_display();
        }
        self.configure_surface();
    }

    pub fn recover_surface(&mut self) {
        self.configure_surface();
    }

    pub fn destroy_device_for_test(&self) {
        self.gpu.device.destroy();
    }
}

impl Renderer {
    async fn admit(
        gpu: Rc<gpu::GpuInner>,
        canvas: HtmlCanvasElement,
        high_canvas: Option<HtmlCanvasElement>,
        mut config: RuntimeConfig,
    ) -> Result<Renderer, JsValue> {
        gpu.ensure_live()?;
        let device = &gpu.device;
        let queue = &gpu.queue;
        let surface = gpu
            .instance
            .create_surface(SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == TextureFormat::Bgra8Unorm)
            .ok_or_else(|| js_error("no surface format"))?;
        let [width, height] = renderer_core::render_size([canvas.width(), canvas.height()]);
        surface.configure(
            device,
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
                    &gpu.instance,
                    &gpu.adapter,
                    device,
                    canvas,
                    [width, height],
                )
            })
            .transpose()?;
        let pipeline_errors = device.push_error_scope(ErrorFilter::Validation);
        let display_pipelines =
            renderer_core::pipelines::build_display(device, DISPLAY_SHADER, format)
                .map_err(js_error)?;
        if let Some(error) = pipeline_errors.pop().await {
            return Err(js_error(format!("display pipeline validation: {error:?}")));
        }
        gpu.ensure_live()?;
        let display_sampler = device.create_sampler(&SamplerDescriptor::default());
        let player_meshes = geometry::PlayerMeshes::new(device, queue);
        let overlay_instances = renderer_core::buffer(
            device,
            queue,
            "overlay-instance",
            BufferUsages::VERTEX,
            bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0]),
        );
        config.size = [width, height];
        Ok(Self {
            surface,
            high_surface,
            gpu,
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
            frames: 0,
        })
    }

    fn configure_surface(&self) {
        if let Some(high) = &self.high_surface {
            high.configure(&self.gpu.device, [self.width, self.height]);
        }
        self.surface.configure(
            &self.gpu.device,
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
                        &self.gpu.device,
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
                &self.gpu.device,
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
