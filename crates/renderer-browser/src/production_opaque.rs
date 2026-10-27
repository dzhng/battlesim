//! Browser-side bind groups for the exported modelOpaque pipeline.

use renderer_core::pipelines::Pipelines;
use renderer_core::shadow::ShadowResources;
use wgpu::*;

pub struct OpaqueResources {
    pub environment: BindGroup,
    pub fog: BindGroup,
    _buffers: Vec<Buffer>,
    _textures: Vec<Texture>,
    _views: Vec<TextureView>,
    _samplers: Vec<Sampler>,
}

pub fn encode(
    encoder: &mut CommandEncoder,
    targets: &renderer_core::Targets,
    pipelines: &Pipelines,
    camera: &BindGroup,
    materials: &BindGroup,
    resources: &OpaqueResources,
    vertices: &Buffer,
    instances: &Buffer,
    vertex_count: u32,
) {
    let pipeline = pipelines.bound("modelOpaque");
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("shared-production-model-opaque"),
        color_attachments: &[
            Some(RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.lit),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &targets.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.fog_mask),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
        ],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    pipeline.set(
        &mut pass,
        &[
            ("typegpuCameraLayout", camera),
            ("modelLayout", materials),
            ("fogLayout", &resources.fog),
            ("environment", &resources.environment),
        ],
    );
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    pass.draw(0..vertex_count, 0..1);
}

fn texture(device: &Device, format: TextureFormat, usage: TextureUsages) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("shared-opaque-placeholder"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: usage | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

pub fn new(device: &Device, pipelines: &Pipelines, shadow: &ShadowResources) -> OpaqueResources {
    let make_buffer = |label: &str, size| {
        device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size,
            usage: BufferUsages::UNIFORM | BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    };
    let buffers = vec![
        make_buffer("shared-opaque-environment", 2048),
        make_buffer("shared-opaque-lights", 2048),
        make_buffer("shared-opaque-sun", 1024),
        make_buffer("shared-opaque-fog-params", 512),
        make_buffer("shared-opaque-fog-layer", 64),
        make_buffer("shared-opaque-paint-style", 64),
        make_buffer("shared-opaque-fog-storage-0", 4096),
        make_buffer("shared-opaque-fog-storage-1", 4096),
        make_buffer("shared-opaque-fog-storage-2", 4096),
        make_buffer("shared-opaque-fog-storage-3", 4096),
    ];
    let environment_data = &buffers[0];
    let lights = &buffers[1];
    let sun = &buffers[2];
    let params = &buffers[3];
    let layer = &buffers[4];
    let paint_style = &buffers[5];
    let empty_storage = &buffers[6..10];
    let sky = texture(device, TextureFormat::Rgba16Float, TextureUsages::COPY_DST);
    let pmrem = texture(device, TextureFormat::Rgba16Float, TextureUsages::COPY_DST);
    let dfg = texture(device, TextureFormat::Rgba16Float, TextureUsages::COPY_DST);
    let paint = texture(device, TextureFormat::Rgba8Unorm, TextureUsages::COPY_DST);
    let wholes = texture(device, TextureFormat::R32Uint, TextureUsages::COPY_DST);
    let sampler = device.create_sampler(&SamplerDescriptor::default());
    let views = vec![
        sky.create_view(&TextureViewDescriptor::default()),
        pmrem.create_view(&TextureViewDescriptor::default()),
        dfg.create_view(&TextureViewDescriptor::default()),
        paint.create_view(&TextureViewDescriptor::default()),
        wholes.create_view(&TextureViewDescriptor::default()),
    ];
    let environment = renderer_core::bind(
        device,
        pipelines.layout("environment"),
        "shared-opaque-environment-group",
        &[
            environment_data.as_entire_binding(),
            BindingResource::Buffer(lights.as_entire_buffer_binding()),
            BindingResource::TextureView(&views[0]),
            BindingResource::TextureView(&views[1]),
            BindingResource::TextureView(&views[2]),
            BindingResource::Sampler(&sampler),
            sun.as_entire_binding(),
            BindingResource::TextureView(&shadow.depth),
            BindingResource::Sampler(&shadow.compare),
        ],
    );
    let fog = renderer_core::bind(
        device,
        pipelines.layout("fogLayout"),
        "shared-opaque-fog-group",
        &[
            params.as_entire_binding(),
            layer.as_entire_binding(),
            empty_storage[0].as_entire_binding(),
            empty_storage[1].as_entire_binding(),
            empty_storage[2].as_entire_binding(),
            empty_storage[3].as_entire_binding(),
            paint_style.as_entire_binding(),
            BindingResource::TextureView(&views[3]),
            BindingResource::TextureView(&views[4]),
        ],
    );
    OpaqueResources {
        environment,
        fog,
        _buffers: buffers,
        _textures: vec![sky, pmrem, dfg, paint, wholes],
        _views: views,
        _samplers: vec![sampler],
    }
}
