//! Debug views inspect the actual resolved fog mask; switching back restores
//! the style and its display-space rim without replacing renderer resources.
#[path = "support/readback.rs"]
mod readback;
mod support;

use presentation::records::FogLook;
use renderer_core::fog_mask::{self, FogMaskView};
use renderer_core::pipelines;
use renderer_core::{Targets, HDR_FORMAT};
use wgpu::*;

fn read(gpu: &support::Gpu, texture: &TextureView, mut encoder: CommandEncoder) -> Vec<[f32; 4]> {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("read-screen-pixels"),
        source: ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> pixels: array<vec4f>;
@compute @workgroup_size(8) fn read(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(image);
    if (id.x >= size.x * size.y) { return; }
    pixels[id.x] = textureLoad(image, vec2i(i32(id.x % size.x), i32(id.x / size.x)), 0);
}
"#
            .into(),
        ),
    });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("read-screen-pixels"),
            layout: None,
            module: &shader,
            entry_point: Some("read"),
            compilation_options: Default::default(),
            cache: None,
        });
    let output = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("screen-pixels"),
        size: 8 * 2 * 16,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = renderer_core::bind(
        &gpu.device,
        &pipeline.get_bind_group_layout(0),
        "screen-pixels",
        &[
            BindingResource::TextureView(texture),
            output.as_entire_binding(),
        ],
    );
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(2, 1, 1);
    }
    let readback = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("screen-readback"),
        size: output.size(),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, output.size());
    readback::pixels(&gpu.device, &gpu.queue, encoder, &readback)
}
fn close(actual: [f32; 4], expected: [f32; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 0.002),
        "expected {expected:?}, got {actual:?}"
    );
}
#[test]
fn fog_ground_and_class_views_use_resolved_mask_and_suppress_the_rim() {
    let gpu = support::gpu().expect("real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let mut targets = Targets::new(&gpu.device, [8, 2]);
    let texture = |label, format| {
        gpu.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: 8,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let lit = texture("controlled-lit", HDR_FORMAT);
    let mask = texture("controlled-mask", TextureFormat::Rgba8Unorm);
    gpu.queue.write_texture(
        lit.as_image_copy(),
        bytemuck::cast_slice(&[0x3400u16, 0x3800, 0x3a00, 0x3c00].repeat(16)),
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8 * 8),
            rows_per_image: None,
        },
        lit.size(),
    );
    let mask_row = [
        [255u8, 0, 255, 255],
        [0, 255, 255, 255],
        [64, 191, 0, 255],
        [0, 0, 0, 255],
    ]
    .repeat(2);
    gpu.queue.write_texture(
        mask.as_image_copy(),
        bytemuck::cast_slice(&mask_row.repeat(2)),
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8 * 4),
            rows_per_image: None,
        },
        mask.size(),
    );
    targets.lit = lit.create_view(&Default::default());
    targets.fog_mask = mask.create_view(&Default::default());
    let look = FogLook {
        dim: 0.5,
        saturation: 1.,
        line_spacing_px: 8.,
        rim_color: [1., 0., 0.],
        rim_alpha: 1.,
        rim_width_px: 2.,
        reach_px: 4.,
        ..Default::default()
    };
    let mut fog = fog_mask::fog_mask(&gpu.device, &gpu.queue, &pipelines, &targets, &look);
    for (view, expected) in [
        (
            FogMaskView::Fog,
            [
                [0., 0., 0., 1.],
                [16., 16., 16., 1.],
                [0.35, 0.35, 0.35, 1.],
                [16., 16., 16., 1.],
            ],
        ),
        (
            FogMaskView::Ground,
            [
                [16., 16., 16., 1.],
                [16., 16., 16., 1.],
                [0., 0., 0., 1.],
                [0., 0., 0., 1.],
            ],
        ),
        (
            FogMaskView::Classes,
            [
                [0.25, 0.5, 0.75, 1.],
                [0.25, 0.5, 0.75, 1.],
                [0., 0., 0., 1.],
                [0., 0., 0., 1.],
            ],
        ),
    ] {
        fog.set_mask_view(&gpu.queue, view);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        fog.encode(&mut encoder, &targets);
        let pixels = read(&gpu, &targets.hdr, encoder);
        for (actual, expected) in pixels[..4].iter().zip(expected) {
            close(*actual, expected);
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.display,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        fog.encode_rim(&mut encoder, &targets);
        assert!(
            read(&gpu, &targets.display, encoder)
                .iter()
                .all(|p| *p == [0.; 4]),
            "debug view retained a fog rim"
        );
    }
    fog.set_mask_view(&gpu.queue, FogMaskView::None);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    fog.encode(&mut encoder, &targets);
    let pixels = read(&gpu, &targets.hdr, encoder);
    close(pixels[0], [0.125, 0.25, 0.375, 1.]);
    close(pixels[1], [0.25, 0.5, 0.75, 1.]);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    fog.encode_rim(&mut encoder, &targets);
    let rim = read(&gpu, &targets.display, encoder);
    assert!(
        rim[1][0] > 0.1,
        "ordinary view failed to restore seen-side rim"
    );
}
