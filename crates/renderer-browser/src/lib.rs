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
use renderer_core::fog_mask::FogMaskResources;
use renderer_core::model::Model;
use renderer_core::pipelines::{Bound, Pipelines};
use renderer_core::post::PostResources;
use renderer_core::world::{Layer, Surface as WorldSurface, WorldInputs};
use renderer_core::{encode_grouped, Targets};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

mod pipeline_export {
    include!(concat!(env!("OUT_DIR"), "/pipeline_export.rs"));
}

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
    {"name":"worldOpaque","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"fragment":{"entryPoint":"world","targets":[{"format":"rgba16float"},{"format":"rgba8unorm"}]},"depthStencil":{"format":"depth32float","depthWriteEnabled":true,"depthCompare":"greater"},"multisample":{"count":4}},
    {"name":"displayModel","kind":"render","shader":"fixture.wgsl","layout":["camera"],"vertex":{"entryPoint":"vertex","buffers":[{"arrayStride":28,"stepMode":"vertex","attributes":[{"format":"float32x3","offset":0,"shaderLocation":0},{"format":"float32x4","offset":12,"shaderLocation":1}]}]},"primitive":{"topology":"triangle-list","cullMode":"none"},"fragment":{"entryPoint":"display","targets":[{"format":"bgra8unorm"}]}}
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
  let uv = vec2<f32>(in.position.x / 640.0, 1.0 - in.position.y / 360.0);
  let c = textureSample(lit, linear, uv);
  return vec4<f32>(1.0 - exp(-c.rgb), 1.0);
}
"#;

#[derive(Debug, Default, Clone, Copy)]
struct Stats {
    frames: u32,
    draw_calls: u32,
    model_vertices: u32,
    resizes: u32,
    shared_passes: u32,
}

fn build_exported_pipelines(device: &Device) -> Result<Pipelines, String> {
    Pipelines::build(device, pipeline_export::MANIFEST, |_, shader| {
        pipeline_export::source(shader)
            .map(str::to_owned)
            .ok_or_else(|| format!("missing embedded WGSL source: {shader}"))
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
    targets: Targets,
    pipelines: Pipelines,
    production_pipelines: Pipelines,
    shared_post: PostResources,
    shared_fog_mask: FogMaskResources,
    depth: DepthPrepass,
    camera_group: BindGroup,
    model: Model,
    width: u32,
    height: u32,
    format: TextureFormat,
    stats: Stats,
    asset_report: Option<String>,
    asset_texture: Option<Texture>,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
impl Renderer {
    /// Diagnostics for the exact production pipeline export embedded in this
    /// browser artifact. The fixture frame remains the control while its full
    /// resource graph is wired.
    #[wasm_bindgen]
    pub fn exported_pipeline_count() -> usize {
        pipeline_export::COUNT
    }

    #[wasm_bindgen]
    pub fn exported_pipeline_manifest() -> String {
        pipeline_export::MANIFEST.to_owned()
    }

    #[wasm_bindgen]
    pub fn exported_pipeline_source(name: String) -> Option<String> {
        pipeline_export::source(&name).map(str::to_owned)
    }

    /// Build the embedded production pipeline export on this browser device.
    /// The fixture frame does not call this yet; the method isolates export
    /// compatibility from the still-bounded frame comparison.
    pub async fn validate_export(&self) -> Result<String, JsValue> {
        let built = self.production_pipelines.names().count();
        let _shared_resources = (&self.shared_post, &self.shared_fog_mask);
        Ok(format!("{{\"pipelines\":{built}}}"))
    }

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
        let targets = Targets::new(&device, [width, height]);
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
        let camera = renderer_core::buffer(
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
            &[BindingResource::Buffer(camera.as_entire_buffer_binding())],
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
        Ok(Self {
            surface,
            device,
            queue,
            targets,
            pipelines,
            production_pipelines,
            shared_post,
            shared_fog_mask,
            depth,
            camera_group,
            model,
            width,
            height,
            format,
            stats: Stats::default(),
            asset_report: None,
            asset_texture: None,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resize(&mut self, canvas: HtmlCanvasElement) {
        self.width = canvas.width().max(1);
        self.height = canvas.height().max(1);
        self.targets = Targets::new(&self.device, [self.width, self.height]);
        self.stats.resizes += 1;
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
                alpha_mode: CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );
    }

    /// Parse one fetched BGAT bundle through the native client's shared asset
    /// reader and upload its first authored texture to this browser device.
    /// The returned JSON is the exact boundary accounting used by the spike.
    #[cfg(target_arch = "wasm32")]
    pub fn load_bundle(
        &mut self,
        name: String,
        bundle_bytes: js_sys::Uint8Array,
        textures: js_sys::Array,
    ) -> Result<String, JsValue> {
        let bundle_bytes = bundle_bytes.to_vec();
        let mut owned = Vec::with_capacity(textures.length() as usize);
        for value in textures.iter() {
            let bytes = js_sys::Uint8Array::new(&value).to_vec();
            owned.push((format!("texture-{}", owned.len()), bytes));
        }
        let refs: Vec<(&str, &[u8])> = owned
            .iter()
            .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
            .collect();
        let library = Library::from_bundle(&name, &bundle_bytes, &refs).map_err(js_error)?;
        let bundle = library.bundle(&name).map_err(js_error)?;
        let vertices: usize = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().map(|m| m.vertices()).sum()
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .map(|m| m.vertices())
                .sum(),
        };
        let indices: usize = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().map(|m| m.indices.len()).sum()
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .map(|m| m.indices.len())
                .sum(),
        };
        if let Some(texture) = bundle.textures.first() {
            let size = Extent3d {
                width: texture.size,
                height: texture.size,
                depth_or_array_layers: 1,
            };
            let gpu = self.device.create_texture(&TextureDescriptor {
                label: Some("shared-asset-albedo"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if texture.srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &texture.levels[0],
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(texture.size * 4),
                    rows_per_image: Some(texture.size),
                },
                size,
            );
            self.asset_texture = Some(gpu);
        }
        let mesh = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().find(|mesh| !mesh.indices.is_empty())
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .find(|mesh| !mesh.indices.is_empty()),
        };
        if let Some(mesh) = mesh {
            self.model = Model::from_indexed_fitted(
                &self.device,
                &self.queue,
                "shared-asset-model",
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
            )
            .map_err(js_error)?;
            self.stats.model_vertices = self.model.vertex_count();
        }
        let report = format!(
            r#"{{"name":"{}","bundleBytes":{},"textureBytes":{},"textures":{},"materials":{},"vertices":{},"indices":{}}}"#,
            name,
            bundle_bytes.len(),
            owned.iter().map(|(_, b)| b.len()).sum::<usize>(),
            bundle.textures.len(),
            bundle.materials.len(),
            vertices,
            indices
        );
        self.asset_report = Some(report.clone());
        Ok(report)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("shared-renderer-frame"),
            });
        let fixture = Fixture {
            depth: &self.targets.depth,
            camera: &self.camera_group,
            solid: &self.pipelines.bound("modelPrepass"),
            world: &self.pipelines.bound("worldOpaque"),
            model: &self.model,
        };
        encode_grouped(&mut encoder, "shared-depth", |e| {
            self.depth.encode(&fixture, e)
        });
        encode_grouped(&mut encoder, "shared-world", |e| {
            renderer_core::world::encode_sky(&fixture, &self.targets, e)
        });
        renderer_core::world::encode_world(
            &fixture,
            &self.targets,
            &mut encoder,
            true,
            false,
            true,
        );
        self.shared_fog_mask.encode(&mut encoder, &self.targets);
        self.shared_post.encode(&mut encoder);
        self.shared_fog_mask.encode_rim(&mut encoder, &self.targets);
        self.stats.shared_passes += 1;
        let surface = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface)
            | CurrentSurfaceTexture::Suboptimal(surface) => surface,
            other => return Err(js_error(format!("surface unavailable: {other:?}"))),
        };
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        let pipeline = self.pipelines.bound("displayModel");
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-composite"),
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
            pipeline.set(&mut pass, &[("camera", &self.camera_group)]);
            pass.set_vertex_buffer(0, self.model.vertex().slice(..));
            pass.set_index_buffer(self.model.index().slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..self.model.index_count(), 0, 0..1);
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
            r#"{{"frames":{},"drawCalls":{},"modelVertices":{},"resizes":{},"sharedPasses":{},"width":{},"height":{},"sharedTargets":true,"sharedDepth":true,"sharedWorld":true,"asset":{}}}"#,
            self.stats.frames,
            self.stats.draw_calls,
            self.stats.model_vertices,
            self.stats.resizes,
            self.stats.shared_passes,
            self.width,
            self.height,
            self.asset_report.as_deref().unwrap_or("null")
        )
    }
}
