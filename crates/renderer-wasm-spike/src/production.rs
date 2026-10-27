//! A bounded production-shaped pass graph for the one-renderer spike.
//!
//! This owner deliberately takes plain presentation records at its wasm
//! boundary. It allocates and retires every target together, then executes the
//! same high-level order as BattleFrame: sun cascades, reverse-Z world/depth,
//! effects, fog mask, and post/composite. The shadow sampler and terrain
//! material table remain explicit unsupported counters rather than silently
//! pretending to be parity.

use crate::production_shaders::{COMPOSITE, EFFECTS, FOG, SHADOW, WORLD};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Color, CommandEncoderDescriptor, Device, Extent3d, LoadOp,
    Operations, Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, Sampler, StoreOp, Surface, Texture, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

const CASCADES: usize = 4;
const SHADOW_SIZE: u32 = 512;

#[derive(Default, Clone, Copy)]
pub struct ProductionStats {
    pub frames: u32,
    pub shadow_passes: u32,
    pub world_passes: u32,
    pub effect_passes: u32,
    pub fog_passes: u32,
    pub post_passes: u32,
    pub draw_calls: u32,
    pub uploaded_bytes: u64,
    pub buffer_bytes: u64,
    pub texture_bytes: u64,
    pub target_bytes: u64,
    pub unsupported_material_channels: u32,
    pub unsupported_shadow_sampling: u32,
}

impl ProductionStats {
    pub fn json(self, last_ms: f64) -> String {
        format!(
            "{{\"frames\":{},\"shadowPasses\":{},\"worldPasses\":{},\"effectPasses\":{},\"fogPasses\":{},\"postPasses\":{},\"drawCalls\":{},\"uploadedBytes\":{},\"bufferBytes\":{},\"textureBytes\":{},\"targetBytes\":{},\"unsupportedMaterialChannels\":{},\"unsupportedShadowSampling\":{},\"lastFrameMs\":{last_ms:.3}}}",
            self.frames,
            self.shadow_passes,
            self.world_passes,
            self.effect_passes,
            self.fog_passes,
            self.post_passes,
            self.draw_calls,
            self.uploaded_bytes,
            self.buffer_bytes,
            self.texture_bytes,
            self.target_bytes,
            self.unsupported_material_channels,
            self.unsupported_shadow_sampling,
        )
    }
}

pub struct ProductionRenderer {
    camera: Buffer,
    shadow_camera: Buffer,
    environment: Buffer,
    fog: Buffer,
    terrain: Option<Buffer>,
    model_vertex: Option<Buffer>,
    model_index: Option<Buffer>,
    model_instance: Option<Buffer>,
    effects: Option<Buffer>,
    albedo: Option<Texture>,
    albedo_width: u32,
    albedo_height: u32,
    sampler: Sampler,
    camera_bind: BindGroup,
    shadow_camera_bind: BindGroup,
    environment_bind: BindGroup,
    fog_bind: BindGroup,
    effect_bind: BindGroup,
    material_bind: Option<BindGroup>,
    terrain_shadow: RenderPipeline,
    model_shadow: RenderPipeline,
    terrain_world: RenderPipeline,
    model_world: RenderPipeline,
    effects_pipeline: RenderPipeline,
    fog_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,
    target_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    depth: Texture,
    world: Texture,
    fog_mask: Texture,
    shadows: Texture,
    world_view: TextureView,
    fog_view: TextureView,
    depth_view: TextureView,
    shadow_views: [TextureView; CASCADES],
    composite_bind: BindGroup,
    stats: ProductionStats,
    last_ms: f64,
}

fn buffer(device: &Device, label: &str, size: u64, usage: wgpu::BufferUsages) -> Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn layout_camera(device: &Device, label: &str) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
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
    })
}

fn layout_uniform(
    device: &Device,
    label: &str,
    size: u64,
    stages: wgpu::ShaderStages,
) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: stages,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(std::num::NonZeroU64::new(size).unwrap()),
            },
            count: None,
        }],
    })
}

#[allow(clippy::too_many_arguments)]
fn pipeline(
    device: &Device,
    module: &wgpu::ShaderModule,
    label: &str,
    layout: &wgpu::PipelineLayout,
    vertex_entry: &str,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    fragment_entry: Option<&str>,
    format: wgpu::TextureFormat,
    depth: bool,
    blend: bool,
) -> RenderPipeline {
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: VertexState {
            module,
            entry_point: Some(vertex_entry),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: Default::default(),
        depth_stencil: depth.then_some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: fragment_entry.map(|entry_point| wgpu::FragmentState {
            module,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn targets(
    device: &Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (
    Texture,
    Texture,
    Texture,
    Texture,
    TextureView,
    TextureView,
    TextureView,
    [TextureView; CASCADES],
    u64,
) {
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let make = |label: &str, usage: wgpu::TextureUsages, fmt: wgpu::TextureFormat| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage,
            view_formats: &[],
        })
    };
    let depth = make(
        "one-renderer-production-depth",
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        wgpu::TextureFormat::Depth32Float,
    );
    let world = make(
        "one-renderer-production-world",
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        format,
    );
    let fog = make(
        "one-renderer-production-fog-mask",
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let shadows = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("one-renderer-production-sun-cascades"),
        size: Extent3d {
            width: SHADOW_SIZE,
            height: SHADOW_SIZE,
            depth_or_array_layers: CASCADES as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = |texture: &Texture| texture.create_view(&TextureViewDescriptor::default());
    let shadow_views = std::array::from_fn(|layer| {
        shadows.create_view(&TextureViewDescriptor {
            label: Some("one-renderer-production-shadow-cascade"),
            dimension: Some(TextureViewDimension::D2),
            base_array_layer: layer as u32,
            array_layer_count: Some(1),
            ..Default::default()
        })
    });
    let bytes = (width as u64 * height as u64 * 4 * 3)
        + (SHADOW_SIZE as u64 * SHADOW_SIZE as u64 * 4 * CASCADES as u64);
    let world_view = view(&world);
    let fog_view = view(&fog);
    let depth_view = view(&depth);
    (
        depth,
        world,
        fog,
        shadows,
        world_view,
        fog_view,
        depth_view,
        shadow_views,
        bytes,
    )
}

impl ProductionRenderer {
    pub fn new(device: &Device, format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let camera_layout = layout_camera(device, "one-renderer-production-camera-layout");
        let environment_layout = layout_uniform(
            device,
            "one-renderer-production-environment-layout",
            64,
            wgpu::ShaderStages::FRAGMENT,
        );
        let fog_layout = layout_uniform(
            device,
            "one-renderer-production-fog-layout",
            32,
            wgpu::ShaderStages::FRAGMENT,
        );
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("one-renderer-production-material-layout"),
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
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("one-renderer-production-post-layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let camera = buffer(
            device,
            "one-renderer-production-camera",
            64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let shadow_camera = buffer(
            device,
            "one-renderer-production-shadow-camera",
            64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let environment = buffer(
            device,
            "one-renderer-production-environment",
            64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let fog = buffer(
            device,
            "one-renderer-production-fog",
            32,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-camera-bind"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let shadow_camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-shadow-camera-bind"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_camera.as_entire_binding(),
            }],
        });
        let environment_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-environment-bind"),
            layout: &environment_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: environment.as_entire_binding(),
            }],
        });
        let fog_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-fog-bind"),
            layout: &fog_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: fog.as_entire_binding(),
            }],
        });
        let effect_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-effect-bind"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("one-renderer-production-linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let (
            depth,
            world,
            fog_mask,
            shadows,
            world_view,
            fog_view,
            depth_view,
            shadow_views,
            target_bytes,
        ) = targets(device, width, height, format);
        let composite_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-composite-bind"),
            layout: &post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&world_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&fog_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let world_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-production-world-shader"),
            source: wgpu::ShaderSource::Wgsl(WORLD.into()),
        });
        let shadow_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-production-shadow-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW.into()),
        });
        let fog_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-production-fog-shader"),
            source: wgpu::ShaderSource::Wgsl(FOG.into()),
        });
        let post_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-production-composite-shader"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
        });
        let effect_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("one-renderer-production-effect-shader"),
            source: wgpu::ShaderSource::Wgsl(EFFECTS.into()),
        });
        let terrain_buffers = [Some(wgpu::VertexBufferLayout {
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
        })];
        let model_buffers = [
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
        ];
        let effect_buffers = [
            None,
            Some(wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                ],
            }),
        ];
        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-production-world-layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&environment_layout)],
            immediate_size: 0,
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-production-model-layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&environment_layout),
                Some(&material_layout),
            ],
            immediate_size: 0,
        });
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-production-shadow-layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });
        let effect_layout = shadow_layout.clone();
        let fog_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-production-fog-layout"),
            bind_group_layouts: &[Some(&fog_layout)],
            immediate_size: 0,
        });
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("one-renderer-production-post-layout"),
            bind_group_layouts: &[Some(&post_layout), Some(&fog_layout)],
            immediate_size: 0,
        });
        let terrain_shadow = pipeline(
            device,
            &shadow_module,
            "one-renderer-production-shadow-terrain",
            &shadow_layout,
            "terrain_vertex",
            &terrain_buffers,
            None,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
            false,
        );
        let model_shadow = pipeline(
            device,
            &shadow_module,
            "one-renderer-production-shadow-model",
            &shadow_layout,
            "model_vertex",
            &model_buffers,
            None,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
            false,
        );
        let terrain_world = pipeline(
            device,
            &world_module,
            "one-renderer-production-world-terrain",
            &world_layout,
            "terrain_vertex",
            &terrain_buffers,
            Some("world_fragment"),
            format,
            true,
            false,
        );
        let model_world = pipeline(
            device,
            &world_module,
            "one-renderer-production-world-model",
            &model_layout,
            "model_vertex",
            &model_buffers,
            Some("model_fragment"),
            format,
            true,
            false,
        );
        let effects_pipeline = pipeline(
            device,
            &effect_module,
            "one-renderer-production-effects",
            &effect_layout,
            "vertex",
            &effect_buffers,
            Some("fragment"),
            format,
            false,
            true,
        );
        let fog_pipeline = pipeline(
            device,
            &fog_module,
            "one-renderer-production-fog",
            &fog_pipeline_layout,
            "vertex",
            &[],
            Some("fragment"),
            wgpu::TextureFormat::Rgba8Unorm,
            false,
            false,
        );
        let composite_pipeline = pipeline(
            device,
            &post_module,
            "one-renderer-production-post",
            &post_pipeline_layout,
            "vertex",
            &[],
            Some("fragment"),
            format,
            false,
            false,
        );
        Self {
            camera,
            shadow_camera,
            environment,
            fog,
            terrain: None,
            model_vertex: None,
            model_index: None,
            model_instance: None,
            effects: None,
            albedo: None,
            albedo_width: 0,
            albedo_height: 0,
            sampler,
            camera_bind,
            shadow_camera_bind,
            environment_bind,
            fog_bind,
            effect_bind,
            material_bind: None,
            terrain_shadow,
            model_shadow,
            terrain_world,
            model_world,
            effects_pipeline,
            fog_pipeline,
            composite_pipeline,
            target_format: format,
            width,
            height,
            depth,
            world,
            fog_mask,
            shadows,
            world_view,
            fog_view,
            depth_view,
            shadow_views,
            composite_bind,
            stats: ProductionStats {
                target_bytes,
                unsupported_material_channels: 3,
                unsupported_shadow_sampling: 1,
                ..Default::default()
            },
            last_ms: 0.0,
        }
    }
}

mod render;
