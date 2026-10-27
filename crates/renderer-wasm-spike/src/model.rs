use wgpu::{
    BindGroup, BindGroupLayout, Device, Extent3d, Queue, RenderPipeline, Sampler, Texture,
    TextureView, TextureViewDescriptor,
};

/// Owns the authored model workload and the depth target shared with the
/// terrain pass. The parent renderer supplies the surface and command encoder;
/// this owner keeps model resources together so resize/rebuild lifetimes do
/// not drift from the pass that consumes them.
pub struct ModelRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    instance_buffer: Option<wgpu::Buffer>,
    texture: Option<Texture>,
    sampler: Sampler,
    depth_texture: Option<Texture>,
    depth_width: u32,
    depth_height: u32,
    texture_width: u32,
    texture_height: u32,
    vertices: u32,
    indices: u32,
    instances: u32,
}

impl ModelRenderer {
    pub fn new(
        device: &Device,
        camera_layout: &BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-authored-model-shader"),
            source: wgpu::ShaderSource::Wgsl(MODEL_SHADER.into()),
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("one-renderer-authored-model-material-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("one-renderer-authored-model-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-authored-model-layout"),
            bind_group_layouts: &[Some(camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("one-renderer-authored-model-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 36,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 12,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 28,
                                shader_location: 2,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 32,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 4,
                            },
                        ],
                    }),
                ],
            },
            primitive: Default::default(),
            multisample: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            vertex_buffer: None,
            index_buffer: None,
            instance_buffer: None,
            texture: None,
            sampler,
            depth_texture: None,
            depth_width: 0,
            depth_height: 0,
            texture_width: 0,
            texture_height: 0,
            vertices: 0,
            indices: 0,
            instances: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        vertices: &[f32],
        indices: &[u32],
        instances: &[f32],
        texture: &[u8],
        texture_width: u32,
        texture_height: u32,
    ) {
        if self.vertices != (vertices.len() / 9) as u32 {
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("one-renderer-authored-model-vertices"),
                size: (vertices.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertices = (vertices.len() / 9) as u32;
        }
        if self.indices != indices.len() as u32 {
            self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("one-renderer-authored-model-indices"),
                size: (indices.len() * 4) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.indices = indices.len() as u32;
        }
        if self.instances != (instances.len() / 8) as u32 {
            self.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("one-renderer-authored-model-instances"),
                size: (instances.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.instances = (instances.len() / 8) as u32;
        }
        queue.write_buffer(
            self.vertex_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(vertices),
        );
        queue.write_buffer(
            self.index_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(indices),
        );
        queue.write_buffer(
            self.instance_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(instances),
        );
        if self.texture.is_none()
            || self.texture_width != texture_width
            || self.texture_height != texture_height
        {
            self.texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("one-renderer-authored-model-albedo"),
                size: Extent3d {
                    width: texture_width,
                    height: texture_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            self.texture_width = texture_width;
            self.texture_height = texture_height;
        }
        queue.write_texture(
            self.texture.as_ref().unwrap().as_image_copy(),
            texture,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_width * 4),
                rows_per_image: Some(texture_height),
            },
            Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn depth_view(&mut self, device: &Device, width: u32, height: u32) -> TextureView {
        if self.depth_texture.is_none() || self.depth_width != width || self.depth_height != height
        {
            self.depth_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("one-renderer-authored-model-depth"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
            self.depth_width = width;
            self.depth_height = height;
        }
        self.depth_texture
            .as_ref()
            .unwrap()
            .create_view(&TextureViewDescriptor::default())
    }

    pub fn camera_bind_group(&self, device: &Device, camera_buffer: &wgpu::Buffer) -> BindGroup {
        let layout = self.pipeline.get_bind_group_layout(0);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-authored-model-camera-bind"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        })
    }

    pub fn material_bind_group(&self, device: &Device) -> BindGroup {
        let layout = self.pipeline.get_bind_group_layout(1);
        let view = self
            .texture
            .as_ref()
            .unwrap()
            .create_view(&TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-authored-model-material-bind"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        self.vertex_buffer.as_ref().unwrap()
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        self.index_buffer.as_ref().unwrap()
    }

    pub fn instance_buffer(&self) -> &wgpu::Buffer {
        self.instance_buffer.as_ref().unwrap()
    }

    pub fn indices(&self) -> u32 {
        self.indices
    }

    pub fn instances(&self) -> u32 {
        self.instances
    }
}

const MODEL_SHADER: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var linear: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) uv: vec2<f32> };
@vertex fn vertex(
  @location(0) position: vec3<f32>,
  @location(1) color: vec4<f32>,
  @location(2) uv: vec2<f32>,
  @location(3) placement: vec4<f32>,
  @location(4) tint: vec4<f32>,
) -> Out {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let world = vec3<f32>(
    c * position.x - s * position.y + placement.x,
    s * position.x + c * position.y + placement.y,
    position.z + placement.z,
  );
  var out: Out;
  out.position = camera.view_proj * vec4<f32>(world, 1.0);
  out.color = color * tint;
  out.uv = uv;
  return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> { return in.color * textureSample(albedo, linear, in.uv); }
"#;
