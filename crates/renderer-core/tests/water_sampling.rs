//! Continuous water-bed blur samples the actual fog owner's scene sampler.
mod support;

use renderer_core::pipelines;
use renderer_core::{fog::Fog, Targets, HDR_FORMAT};
use wgpu::*;

#[test]
fn water_scene_colour_interpolates_fractional_mip_levels() {
    let gpu = support::gpu().expect("a real GPU");
    let device = &gpu.device;
    let pipelines = pipelines::build_builtin(device).unwrap();
    let mut targets = Targets::new(device, [64, 1]);
    let scene = device.create_texture(&TextureDescriptor {
        label: Some("controlled-water-bed"),
        size: Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 2,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: HDR_FORMAT,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, edge, value) in [(0, 2, 0u16), (1, 1, 0x3c00u16)] {
        gpu.queue.write_texture(
            TexelCopyTextureInfo {
                mip_level: level,
                ..scene.as_image_copy()
            },
            bytemuck::cast_slice(&[value; 4].repeat((edge * edge) as usize)),
            TexelCopyBufferLayout {
                bytes_per_row: Some(edge * 8),
                ..Default::default()
            },
            Extent3d {
                width: edge,
                height: edge,
                depth_or_array_layers: 1,
            },
        );
    }
    targets.scene_colour = scene.create_view(&Default::default());
    let fog = Fog::new(device, &gpu.queue, &pipelines, &targets);
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("water-fractional-blur"),
        source: ShaderSource::Wgsl(
            r#"
@group(2) @binding(10) var sceneColour: texture_2d<f32>;
@group(2) @binding(11) var sceneTaps: sampler;
@vertex fn vs(@builtin(vertex_index) i:u32) -> @builtin(position) vec4f {
    let p = array<vec2f,3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
    return vec4f(p[i],0,1);
}
@fragment fn fs(@builtin(position) p:vec4f) -> @location(0) vec4f {
    return textureSampleLevel(sceneColour,sceneTaps,vec2f(0.5),p.x/64.0);
}
"#
            .into(),
        ),
    });
    let empty = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[],
    });
    let empty_group = renderer_core::bind(device, &empty, "empty", &[]);
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            Some(&empty),
            Some(&empty),
            Some(pipelines.layout("fogLayout")),
        ],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(TextureFormat::Rgba8Unorm.into())],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let image = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: 64,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.create_buffer(&BufferDescriptor {
        label: None,
        size: 256,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &image.create_view(&Default::default()),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &empty_group, &[]);
        pass.set_bind_group(1, &empty_group, &[]);
        pass.set_bind_group(2, fog.water_group().1, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        image.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &readback,
            layout: TexelCopyBufferLayout {
                bytes_per_row: Some(256),
                ..Default::default()
            },
        },
        image.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    readback.map_async(MapMode::Read, .., |r| r.unwrap());
    device.poll(PollType::wait_indefinitely()).unwrap();
    let pixels = readback.get_mapped_range(..).unwrap();
    let ramp: Vec<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
    assert!(
        ramp[0] < 5 && ramp[63] > 250,
        "control must span both mip colours: {ramp:?}"
    );
    assert!(
        ramp.windows(2).all(|p| p[1] > p[0] && p[1] - p[0] <= 5),
        "fractional blur must interpolate without mip steps: {ramp:?}"
    );
}
