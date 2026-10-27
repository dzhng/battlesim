//! Browser shell for the shared renderer core.
//!
//! This is deliberately a small, real frame checkpoint. It uses the shared
//! target allocation, pipeline parser, reverse-Z depth pass and world-pass
//! attachment/order code. The shell owns only WebGPU surface setup and a tiny
//! fixture adapter; native's production resource owners remain in
//! `native-client`.

#![cfg(target_arch = "wasm32")]

use native_client::bundle::Library;
use renderer_core::depth::{DepthInputs, DepthPrepass};
use renderer_core::fog::FogComputeOwner;
use renderer_core::fog_mask::FogMaskResources;
use renderer_core::fog_sight::FogSightResources;
use renderer_core::model::Model;
use renderer_core::model_material::ModelMaterialBindings;
use renderer_core::pipelines::{Bound, Pipelines};
use renderer_core::post::PostResources;
use renderer_core::shadow::{CasterRouting, ShadowResources, CAMERA_BYTES, RECEIVER_BYTES};
use renderer_core::world::{Layer, Surface as WorldSurface, WorldInputs};
use renderer_core::{encode_grouped, TargetOwner};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

pub(crate) mod pipeline_export {
    include!(concat!(env!("OUT_DIR"), "/pipeline_export.rs"));
}
mod asset;
mod diagnostic;
mod environment;
mod export;
mod presentation_upload;
mod production_model;
mod recovery;
mod shadow_upload;
mod state;
use asset::AssetSnapshot;
use environment::BrowserEnvironment;
use state::{PresentationSnapshot, Stats};

const PIPELINES: &str = r#"{
  "sizes":{"bloomLevels":0,"skyLut":[1,1],"effectSheet":1,"grassClump":1},
  "layouts":{
    "camera":[{"binding":0,"buffer":{"type":"uniform"},"visibility":["vertex"]}],
    "sampled":[
      {"binding":0,"texture":{"sampleType":"float","viewDimension":"2d","multisampled":false},"visibility":["fragment"]},
      {"binding":1,"sampler":{"type":"filtering"},"visibility":["fragment"]}
    ]
  },
  "pipelines":[
    {"name":"modelPrepass","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"depthStencil":{"format":"depth32float","depthWriteEnabled":true,"depthCompare":"greater"},"multisample":{"count":4}},
    {"name":"modelPrepassCutout","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"depthStencil":{"format":"depth32float","depthWriteEnabled":true,"depthCompare":"greater"},"multisample":{"count":4}},
    {"name":"worldOpaque","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"fragment":{"entryPoint":"world","targets":[{"format":"rgba16float"},{"format":"rgba8unorm"}]},"depthStencil":{"format":"depth32float","depthWriteEnabled":false,"depthCompare":"always"},"multisample":{"count":4}},
    {"name":"displayModel","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"fragment":{"entryPoint":"display","targets":[{"format":"bgra8unorm"}]}},
    {"name":"composite","kind":"render","shader":"fixture.wgsl","layout":["sampled"],"vertex":{"entryPoint":"fullscreen","buffers":[]},"primitive":{"topology":"triangle-list"},"fragment":{"entryPoint":"composite","targets":[{"format":"bgra8unorm"}]}},
    {"name":"overlayDisplay","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"fragment":{"entryPoint":"display","targets":[{"format":"bgra8unorm","blend":{"color":{"srcFactor":"src-alpha","dstFactor":"one-minus-src-alpha"},"alpha":{"srcFactor":"one","dstFactor":"one-minus-src-alpha"}}}]}}
  ]
}"#;

const SHADER: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct V { @builtin(position) position: vec4<f32>, @location(0) colour: vec4<f32> };
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) colour: vec4<f32>) -> V {
  var out: V; out.position = camera.view_proj * vec4<f32>(position, 1.0); out.colour = colour; return out;
}
struct W { @location(0) colour: vec4<f32>, @location(1) fog: vec4<f32> };
@fragment fn world(in: V) -> W { var out: W; out.colour = in.colour; out.fog = vec4<f32>(0.0); return out; }
@fragment fn display(in: V) -> @location(0) vec4<f32> { return in.colour; }
@vertex fn fullscreen(@builtin(vertex_index) i: u32) -> V {
  var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
  var out: V; out.position = vec4<f32>(p[i], 0.0, 1.0); out.colour = vec4<f32>(1.0); return out;
}
@group(0) @binding(0) var lit: texture_2d<f32>;
@group(0) @binding(1) var linear: sampler;
@fragment fn composite(in: V) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.position.x / 640.0, in.position.y / 360.0);
  let c = textureSample(lit, linear, uv);
  return vec4<f32>(c.rgb, 1.0);
}
"#;

fn build_exported_pipelines(device: &Device) -> Result<Pipelines, String> {
    Pipelines::build(device, pipeline_export::MANIFEST, |_, shader| {
        pipeline_export::source(shader)
            .map(str::to_owned)
            .ok_or_else(|| format!("missing embedded WGSL source: {shader}"))
    })
}

fn array_view(texture: &Texture) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    })
}

struct Fixture<'a> {
    depth: &'a TextureView,
    camera: &'a BindGroup,
    solid: &'a Bound,
    world: &'a Bound,
    model: &'a Model,
}

impl Fixture<'_> {
    fn draw(&self, pass: &mut RenderPass<'_>, pipeline: &Bound) {
        pipeline.set(pass, &[("camera", self.camera)]);
        pass.set_vertex_buffer(0, self.model.vertex().slice(..));
        pass.set_index_buffer(self.model.index().slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.model.index_count(), 0, 0..1);
    }
}

impl DepthInputs for Fixture<'_> {
    fn depth_view(&self) -> &TextureView {
        self.depth
    }
    fn camera_group(&self) -> (&'static str, &BindGroup) {
        ("camera", self.camera)
    }
    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>) {
        self.draw(pass, self.solid);
    }
    fn draw_scenery_depth(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_models_depth(&self, pass: &mut RenderPass<'_>, pipeline: &Bound, _cutout: bool) {
        self.draw(pass, pipeline);
    }
}

impl WorldInputs for Fixture<'_> {
    fn draw_sky(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_ground(&self, pass: &mut RenderPass<'_>, _layer: Layer) {
        self.draw(pass, self.world);
    }
    fn draw_skirt(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn bind_models(&self, pass: &mut RenderPass<'_>, _layer: Layer, _surface: WorldSurface) {
        self.world.set(pass, &[("camera", self.camera)]);
        pass.set_vertex_buffer(0, self.model.vertex().slice(..));
    }
    fn draw_models(&self, pass: &mut RenderPass<'_>, _layer: Layer, _surface: WorldSurface) {
        pass.set_index_buffer(self.model.index().slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.model.index_count(), 0, 0..1);
    }
    fn draw_scenery(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn draw_grass(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn draw_backdrop(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn draw_water(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn bind_glass(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_glass(&self, _pass: &mut RenderPass<'_>) {}
}

#[wasm_bindgen]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: Device,
    queue: Queue,
    targets: TargetOwner,
    pipelines: Pipelines,
    production_pipelines: Pipelines,
    shared_post: PostResources,
    shared_fog_mask: FogMaskResources,
    shared_fog_sight: FogSightResources,
    material_bindings: ModelMaterialBindings,
    material_palette: Buffer,
    material_rows: Buffer,
    material_surface: Texture,
    material_sampler: Sampler,
    shared_shadow: ShadowResources,
    shadow_routing: CasterRouting,
    shared_fog_compute: FogComputeOwner,
    environment: BrowserEnvironment,
    /// The presentation camera record bound by the shared model prepass.
    presentation_camera: Buffer,
    /// CPU presentation records staged for the remaining production passes.
    /// The fog look is owned and bound by `shared_fog_mask`; only its CPU
    /// snapshot remains here so a device or target rebuild can replay it.
    presentation_environment: Buffer,
    presentation_overlay: Buffer,
    presentation_snapshot: Option<PresentationSnapshot>,
    production_camera_group: BindGroup,
    production_model_vertices: Buffer,
    production_model_instances: Buffer,
    production_model_vertex_count: u32,
    depth: DepthPrepass,
    camera_buffer: Buffer,
    camera_group: BindGroup,
    model: Model,
    shared_ground: Option<Model>,
    shared_overlay: Option<Model>,
    ground_snapshot: Option<Vec<f32>>,
    display_sampler: Sampler,
    display_group: BindGroup,
    width: u32,
    height: u32,
    format: TextureFormat,
    alpha_mode: CompositeAlphaMode,
    device_lost: Arc<AtomicU32>,
    stats: Stats,
    asset_report: Option<String>,
    asset_texture: Option<Texture>,
    asset_snapshot: Option<AssetSnapshot>,
    model_opaque_diagnostic: bool,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
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
                required_features: Features::empty(),
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
        let pipeline_export = PIPELINES.replace(
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
        let pipelines = Pipelines::build(&device, &pipeline_export, |_name, _shader| {
            Ok(SHADER.to_owned())
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
        let shared_shadow = ShadowResources::new(
            &device,
            &production_pipelines,
            64,
            &[0; RECEIVER_BYTES],
            CAMERA_BYTES,
        );
        let shared_fog_compute = FogComputeOwner::new(&production_pipelines);
        let (
            material_palette,
            material_rows,
            _material_albedo,
            material_surface,
            material_sampler,
            material_bindings,
        ) = diagnostic::material_probe(&device, &queue, production_pipelines.layout("modelLayout"));
        let production_camera = renderer_core::buffer(
            &device,
            "shared-production-camera",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 160],
        );
        let presentation_environment = renderer_core::buffer(
            &device,
            "shared-presentation-environment",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 256],
        );
        let presentation_overlay = renderer_core::buffer(
            &device,
            "shared-presentation-overlay",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 256],
        );
        let environment = BrowserEnvironment::new(
            &device,
            &queue,
            &production_pipelines,
            &presentation_environment,
            &shared_shadow,
        );
        let production_camera_group = renderer_core::bind(
            &device,
            production_pipelines.layout("typegpuCameraLayout"),
            "shared-production-camera-group",
            &[BindingResource::Buffer(
                production_camera.as_entire_buffer_binding(),
            )],
        );
        let (production_vertices, production_model_vertex_count) =
            production_model::fixture_vertices();
        let production_model_vertices = renderer_core::buffer(
            &device,
            "shared-production-model-vertices",
            BufferUsages::VERTEX,
            &production_vertices,
        );
        let production_model_instances = renderer_core::buffer(
            &device,
            "shared-production-model-instances",
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            bytemuck::cast_slice(&[
                0.0f32, 0.0, 0.0, 0.0, // placement
                0.0, 0.0, 0.0, 0.0, // data
                1.0, 1.0, 1.0, 1.0, // tint
                1.0, 1.0, 1.0, 1.0, // scale
            ]),
        );
        let camera_buffer = renderer_core::buffer(
            &device,
            "shared-camera",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            bytemuck::cast_slice(&[
                1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
        );
        let camera_group = renderer_core::bind(
            &device,
            pipelines.layout("camera"),
            "shared-camera-group",
            &[BindingResource::Buffer(
                camera_buffer.as_entire_buffer_binding(),
            )],
        );
        let depth = DepthPrepass::new(&pipelines);
        let model = Model::from_indexed_fitted(
            &device,
            &queue,
            "shared-fixture-model",
            &[
                -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0,
                -1.0, 1.0, 0.0,
            ],
            &[
                31, 107, 184, 255, 31, 107, 184, 255, 209, 122, 31, 255, 31, 107, 184, 255, 209,
                122, 31, 255, 31, 107, 184, 255,
            ],
            &[0, 1, 2, 3, 4, 5],
        )
        .map_err(js_error)?;
        let display_sampler = device.create_sampler(&SamplerDescriptor::default());
        let display_group = renderer_core::bind(
            &device,
            pipelines.layout("sampled"),
            "shared-display-composite-group",
            &[
                BindingResource::TextureView(&targets.lit),
                BindingResource::Sampler(&display_sampler),
            ],
        );
        Ok(Self {
            surface,
            device,
            queue,
            targets,
            pipelines,
            production_pipelines,
            shared_post,
            shared_fog_mask,
            shared_fog_sight,
            material_bindings,
            material_palette,
            material_rows,
            material_surface,
            material_sampler,
            shared_shadow,
            shadow_routing: CasterRouting::default(),
            shared_fog_compute,
            environment,
            presentation_camera: production_camera,
            presentation_environment,
            presentation_overlay,
            presentation_snapshot: None,
            production_camera_group,
            production_model_vertices,
            production_model_instances,
            production_model_vertex_count,
            depth,
            camera_buffer,
            camera_group,
            model,
            shared_ground: None,
            shared_overlay: None,
            ground_snapshot: None,
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
                shared_fog_sight: true,
                ..Stats::default()
            },
            asset_report: None,
            asset_texture: None,
            asset_snapshot: None,
            model_opaque_diagnostic: false,
        })
    }

    /// Upload the presentation fog sight records into the shared compute
    /// owner. The browser scene currently sends an inactive zero record; a
    /// production bridge can pass the packed `FogUpload` arrays directly.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_fog_sight(
        &mut self,
        params: &[f32],
        eyes: &[f32],
        foliage: &[f32],
        occluders: &[f32],
        rebuild: &[u32],
        map_words: &[u32],
        tiles: &[u32],
        wholes: &[u32],
        whole_items: &[u32],
        whole_uniform: &[f32],
    ) -> Result<(), JsValue> {
        if params.len() < 64 || map_words.len() < 2 || tiles.len() < 3 {
            return Err(js_error("invalid fog sight record"));
        }
        self.shared_fog_sight
            .upload(
                &self.device,
                &self.queue,
                &renderer_core::fog_sight::FogSightUpload {
                    params: bytemuck::cast_slice(&params[..64]),
                    eyes: bytemuck::cast_slice(eyes),
                    foliage: bytemuck::cast_slice(foliage),
                    occluders: bytemuck::cast_slice(occluders),
                    rebuild: bytemuck::cast_slice(rebuild),
                    map_words: [map_words[0], map_words[1]],
                    tiles: [tiles[0], tiles[1], tiles[2]],
                    wholes,
                    whole_items: bytemuck::cast_slice(whole_items),
                    whole_uniform: bytemuck::cast_slice(whole_uniform),
                },
            )
            .map_err(js_error)?;
        self.stats.fog_sight_uploads = self.stats.fog_sight_uploads.saturating_add(1);
        Ok(())
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
            self.display_group = renderer_core::bind(
                &self.device,
                self.pipelines.layout("sampled"),
                "shared-display-composite-group",
                &[
                    BindingResource::TextureView(&self.targets.lit),
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

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_shared_model_instances(&self, instances: &[f32]) -> Result<(), JsValue> {
        if !instances.len().is_multiple_of(16) {
            return Err(js_error("shared model instances need 16 floats"));
        }
        self.queue.write_buffer(
            &self.production_model_instances,
            0,
            bytemuck::cast_slice(instances),
        );
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("shared-renderer-frame"),
            });
        let ground = self.shared_ground.as_ref().unwrap_or(&self.model);
        let fixture = Fixture {
            depth: &self.targets.depth,
            camera: &self.camera_group,
            solid: &self.pipelines.bound("modelPrepass"),
            world: &self.pipelines.bound("worldOpaque"),
            model: ground,
        };
        encode_grouped(&mut encoder, "shared-depth", |e| {
            self.depth.encode(&fixture, e)
        });
        encode_grouped(&mut encoder, "shared-world", |e| {
            renderer_core::world::encode_sky(&fixture, &self.targets, e)
        });
        renderer_core::world::encode_world(&fixture, &self.targets, &mut encoder, true, true, true);
        production_model::encode_prepass(
            &mut encoder,
            &self.targets,
            &self.production_pipelines,
            &self.production_camera_group,
            self.material_bindings.bind_group(),
            &self.production_model_vertices,
            &self.production_model_instances,
            self.production_model_vertex_count,
        );
        self.stats.shared_model_prepass = true;
        if self.model_opaque_diagnostic {
            production_model::encode_opaque(
                &mut encoder,
                &self.targets,
                &self.production_pipelines,
                &self.production_camera_group,
                self.material_bindings.bind_group(),
                self.shared_fog_sight.group(0),
                self.environment.bindings.group(),
                &self.production_model_vertices,
                &self.production_model_instances,
                self.production_model_vertex_count,
            );
        }
        self.shared_fog_mask.encode(&mut encoder, &self.targets);
        self.shared_post.encode(&mut encoder);
        self.stats.shared_passes += 1;
        let surface = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface)
            | CurrentSurfaceTexture::Suboptimal(surface) => surface,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.recover_surface();
                return Ok(());
            }
            other => return Err(js_error(format!("surface unavailable: {other:?}"))),
        };
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        let pipeline = self.pipelines.bound("composite");
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-display-composite"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pipeline.set(&mut pass, &[("sampled", &self.display_group)]);
            pass.draw(0..3, 0..1);
        }
        if let Some(overlay) = &self.shared_overlay {
            let pipeline = self.pipelines.bound("overlayDisplay");
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-overlay-diagnostic"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pipeline.set(&mut pass, &[("camera", &self.camera_group)]);
            pass.set_vertex_buffer(0, overlay.vertex().slice(..));
            pass.set_index_buffer(overlay.index().slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..overlay.index_count(), 0, 0..1);
            self.stats.draw_calls = self.stats.draw_calls.saturating_add(1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface);
        self.stats.frames += 1;
        self.stats.draw_calls += 1;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn stats(&self) -> String {
        format!(
            r#"{{"frames":{},"drawCalls":{},"modelVertices":{},"resizes":{},"sharedPasses":{},"targetGeneration":{},"sharedMaterials":{},"sharedShadow":{},"sharedEnvironment":{},"sharedFogSight":{},"shadowCasterBatches":[{},{},{},{}],"sharedFogCompute":{},"sharedModelPrepass":{},"surfaceRecoveries":{},"deviceRecoveries":{},"deviceLost":{},"presentationUploads":{},"fogSightUploads":{},"presentationBound":{},"presentationCameraWords":{},"presentationEnvironmentWords":{},"presentationFogWords":{},"presentationOverlayWords":{},"width":{},"height":{},"sharedTargets":true,"sharedDepth":true,"sharedWorld":true,"asset":{}}}"#,
            self.stats.frames,
            self.stats.draw_calls,
            self.stats.model_vertices,
            self.stats.resizes,
            self.stats.shared_passes,
            self.stats.target_generation,
            self.stats.shared_materials,
            self.stats.shared_shadow,
            self.stats.shared_environment,
            self.stats.shared_fog_sight,
            self.stats.shadow_caster_batches[0],
            self.stats.shadow_caster_batches[1],
            self.stats.shadow_caster_batches[2],
            self.stats.shadow_caster_batches[3],
            true,
            self.stats.shared_model_prepass,
            self.stats.surface_recoveries,
            self.stats.device_recoveries,
            self.device_lost.load(Ordering::Relaxed),
            self.stats.presentation_uploads,
            self.stats.fog_sight_uploads,
            self.stats.presentation_bound,
            self.stats.presentation_camera_words,
            self.stats.presentation_environment_words,
            self.stats.presentation_fog_words,
            self.stats.presentation_overlay_words,
            self.width,
            self.height,
            self.asset_report.as_deref().unwrap_or("null")
        )
    }
}
