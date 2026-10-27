#![cfg(target_arch = "wasm32")]

//! Browser-only `wgpu` proof of life for the one-renderer decision.
//!
//! This deliberately renders the same tiny WGSL scene through the Rust API and
//! leaves the battle renderer untouched. The seam is the one a future port
//! needs: a canvas, a WebGPU device, exported WGSL, and a frame submission.

use wasm_bindgen::prelude::*;

mod model;
use model::ModelRenderer;
mod production;
mod production_shaders;
use production::ProductionRenderer;
use web_sys::HtmlCanvasElement;
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, LoadOp, MemoryHints, Operations, PowerPreference, PresentMode,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RequestAdapterOptions,
    StoreOp, Surface, SurfaceConfiguration, SurfaceTarget, TextureUsages, TextureViewDescriptor,
    Trace, VertexState,
};

const SHADER: &str = r#"
struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0),
  );
  var out: VertexOut;
  out.position = vec4<f32>(positions[index], 0.0, 1.0);
  out.uv = vec2<f32>(positions[index].x * 0.5 + 0.5, -positions[index].y * 0.5 + 0.5);
  return out;
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
  let glow = 0.04 + 0.18 * in.uv.x + 0.22 * in.uv.y;
  return vec4<f32>(0.06 + glow, 0.16 + glow * 0.65, 0.28 + glow * 0.35, 1.0);
}
"#;

#[wasm_bindgen]
pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: wgpu::Queue,
    pipeline: RenderPipeline,
    mesh_pipeline: RenderPipeline,
    mesh_depth_pipeline: RenderPipeline,
    model: ModelRenderer,
    production: ProductionRenderer,
    mesh_buffer: Option<wgpu::Buffer>,
    camera_buffer: wgpu::Buffer,
    mesh_vertices: u32,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    present_mode: PresentMode,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl Renderer {
    /// Creates a browser WebGPU surface through `wgpu`'s BrowserWebGpu backend.
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
                label: Some("one-renderer-spike"),
                required_features: Features::empty(),
                required_limits: adapter.limits(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: MemoryHints::Performance,
                trace: Trace::Off,
            })
            .await
            .map_err(js_error)?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .first()
            .copied()
            .ok_or_else(|| js_error("wgpu returned no canvas format"))?;
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
                present_mode: if caps.present_modes.contains(&PresentMode::Fifo) {
                    PresentMode::Fifo
                } else {
                    PresentMode::AutoVsync
                },
                desired_maximum_frame_latency: 2,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
            },
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-spike-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("one-renderer-spike-pipeline"),
            layout: None,
            vertex: VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mesh_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-real-mesh-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) uv: vec2<f32> };
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Out {
  var out: Out;
  out.position = camera.view_proj * vec4<f32>(position, 1.0);
  out.color = color;
  return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> { return in.color; }
"#
                .into(),
            ),
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("one-renderer-real-camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(std::num::NonZeroU64::new(64).unwrap()),
                },
                count: None,
            }],
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("one-renderer-real-mesh-pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("one-renderer-real-mesh-layout"),
                    bind_group_layouts: &[Some(&camera_layout)],
                    immediate_size: 0,
                }),
            ),
            vertex: wgpu::VertexState {
                module: &mesh_module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 40,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 24,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mesh_depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("one-renderer-real-mesh-depth-pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("one-renderer-real-mesh-depth-layout"),
                    bind_group_layouts: &[Some(&camera_layout)],
                    immediate_size: 0,
                }),
            ),
            vertex: wgpu::VertexState {
                module: &mesh_module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 40,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 24,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let model = ModelRenderer::new(&device, &camera_layout, format);
        let production = ProductionRenderer::new(&device, format, width, height);
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("one-renderer-real-camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Renderer {
            surface,
            device,
            queue,
            pipeline,
            mesh_pipeline,
            mesh_depth_pipeline,
            model,
            production,
            mesh_buffer: None,
            camera_buffer,
            mesh_vertices: 0,
            format,
            alpha_mode: caps.alpha_modes[0],
            present_mode: if caps.present_modes.contains(&PresentMode::Fifo) {
                PresentMode::Fifo
            } else {
                PresentMode::AutoVsync
            },
            width,
            height,
        })
    }

    /// Renders one frame and returns the elapsed Rust-side encoding time in ms.
    pub fn render(&mut self) -> Result<f64, JsValue> {
        let started = js_sys::Date::now();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            other => return Err(js_error(format!("wgpu surface unavailable: {other:?}"))),
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("one-renderer-spike-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-spike-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.015,
                            g: 0.025,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(js_sys::Date::now() - started)
    }

    /// Draws the real presentation terrain mesh and camera records supplied by
    /// the browser's `presentation-wasm` frame. The shader is intentionally
    /// minimal; the important proof is the Rust backend receiving the same
    /// authored geometry and camera that the TypeGPU control receives.
    pub fn render_real_mesh(&mut self, vertices: &[f32], camera: &[f32]) -> Result<f64, JsValue> {
        if camera.len() < 16 || !vertices.len().is_multiple_of(10) {
            return Err(js_error(
                "real mesh needs a 40-byte vertex stream and a camera record",
            ));
        }
        let started = js_sys::Date::now();
        if self.mesh_vertices != (vertices.len() / 10) as u32 {
            self.mesh_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("one-renderer-real-mesh"),
                size: (vertices.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.mesh_vertices = (vertices.len() / 10) as u32;
        }
        let mesh = self.mesh_buffer.as_ref().unwrap();
        self.queue
            .write_buffer(mesh, 0, bytemuck::cast_slice(vertices));
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera[..16]));
        let layout = self.mesh_pipeline.get_bind_group_layout(0);
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-real-camera-bind"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            }],
        });
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            other => return Err(js_error(format!("wgpu surface unavailable: {other:?}"))),
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("one-renderer-real-mesh-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-real-mesh-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_vertex_buffer(0, mesh.slice(..));
            pass.draw(0..self.mesh_vertices, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(js_sys::Date::now() - started)
    }

    /// Draws the authored terrain and an authored model mesh with instanced
    /// placements. The browser control sends the same packed vertex/index/
    /// placement records; this keeps the comparison at the renderer seam.
    // The flat arguments are the wasm-bindgen upload boundary: each slice is a
    // distinct browser-owned array, so a Rust struct would add a second codec.
    #[allow(clippy::too_many_arguments)]
    pub fn render_real_model(
        &mut self,
        terrain: &[f32],
        model_vertices: &[f32],
        model_indices: &[u32],
        instances: &[f32],
        texture: &[u8],
        texture_width: u32,
        texture_height: u32,
        camera: &[f32],
    ) -> Result<f64, JsValue> {
        if camera.len() < 16
            || !terrain.len().is_multiple_of(10)
            || !model_vertices.len().is_multiple_of(9)
            || model_indices.is_empty()
            || !model_indices.len().is_multiple_of(3)
            || !instances.len().is_multiple_of(8)
            || texture.is_empty()
            || texture_width == 0
            || texture_height == 0
            || texture.len() != (texture_width as usize) * (texture_height as usize) * 4
        {
            return Err(js_error("invalid authored model workload"));
        }
        let started = js_sys::Date::now();
        if self.mesh_vertices != (terrain.len() / 10) as u32 {
            self.mesh_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("one-renderer-real-model-terrain"),
                size: (terrain.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.mesh_vertices = (terrain.len() / 10) as u32;
        }
        self.queue.write_buffer(
            self.mesh_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(terrain),
        );
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera[..16]));
        self.model.upload(
            &self.device,
            &self.queue,
            model_vertices,
            model_indices,
            instances,
            texture,
            texture_width,
            texture_height,
        );
        let depth_view = self.model.depth_view(&self.device, self.width, self.height);
        // Bind groups belong to their pipeline layout object even when the
        // entries are structurally identical across the terrain and model
        // pipelines.
        let terrain_layout = self.mesh_depth_pipeline.get_bind_group_layout(0);
        let terrain_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-authored-model-terrain-camera-bind"),
            layout: &terrain_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            }],
        });
        let model_bind = self
            .model
            .camera_bind_group(&self.device, &self.camera_buffer);
        let material_bind = self.model.material_bind_group(&self.device);
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            other => return Err(js_error(format!("wgpu surface unavailable: {other:?}"))),
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("one-renderer-authored-model-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-authored-model-terrain-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.mesh_depth_pipeline);
            pass.set_bind_group(0, &terrain_bind, &[]);
            pass.set_vertex_buffer(0, self.mesh_buffer.as_ref().unwrap().slice(..));
            pass.draw(0..self.mesh_vertices, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-authored-model-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(self.model.pipeline());
            pass.set_bind_group(0, &model_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_vertex_buffer(0, self.model.vertex_buffer().slice(..));
            pass.set_vertex_buffer(1, self.model.instance_buffer().slice(..));
            pass.set_index_buffer(
                self.model.index_buffer().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.model.indices(), 0, 0..self.model.instances());
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(js_sys::Date::now() - started)
    }

    pub fn resize(&mut self, canvas: HtmlCanvasElement) {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                color_space: Default::default(),
                width,
                height,
                present_mode: self.present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode: self.alpha_mode,
                view_formats: vec![],
            },
        );
        self.production.resize(&self.device, width, height);
        self.production.rebind_composite(&self.device);
    }

    /// Executes the production-shaped BattleFrame pass order over plain
    /// presentation records. The arrays are deliberately the same packed
    /// records used by the smaller terrain/model seam, while the extra shadow,
    /// environment, fog and effect inputs make the Rust comparison honest.
    #[allow(clippy::too_many_arguments)]
    pub fn render_production(
        &mut self,
        terrain: &[f32],
        model_vertices: &[f32],
        model_indices: &[u32],
        instances: &[f32],
        effects: &[f32],
        texture: &[u8],
        texture_width: u32,
        texture_height: u32,
        camera: &[f32],
        shadow_cameras: &[f32],
        environment: &[f32],
        fog_params: &[f32],
    ) -> Result<f64, JsValue> {
        self.production
            .render(
                &self.device,
                &self.queue,
                &self.surface,
                terrain,
                model_vertices,
                model_indices,
                instances,
                effects,
                texture,
                texture_width,
                texture_height,
                camera,
                shadow_cameras,
                environment,
                fog_params,
            )
            .map_err(js_error)
    }

    /// Returns stable JSON counters for the production-shaped pass graph.
    /// Unsupported features are counted explicitly so a fast result cannot be
    /// mistaken for a complete BattleFrame port.
    pub fn production_stats(&self) -> String {
        self.production.stats_json()
    }

    pub fn shader_source() -> String {
        SHADER.to_owned()
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
