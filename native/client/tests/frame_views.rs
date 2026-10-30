//! Inspector output uses the production post and overlay shaders: class data
//! bypasses grading, mask views bypass bloom, and paint retains stored colour.
use native_client::{gpu, pipelines};
use presentation::records::{Bloom, Grade};
use renderer_core::post::{self, PostMode};
use renderer_core::{overlay, Targets, HDR_FORMAT};
use wgpu::*;
const SIZE: [u32; 2] = [128, 64];
fn pixels(gpu: &gpu::Gpu, targets: &Targets, mut encoder: CommandEncoder) -> Vec<[u8; 4]> {
    let row = SIZE[0] * 4;
    let buffer = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("inspector-readback"),
        size: u64::from(row * SIZE[1]),
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        targets.display_texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        targets.display_texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    buffer.map_async(MapMode::Read, .., |r| r.unwrap());
    gpu.device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = buffer.get_mapped_range(..).unwrap();
    let pixels = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    buffer.unmap();
    pixels
}
fn backdrop(encoder: &mut CommandEncoder, targets: &Targets, value: f64) {
    encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &targets.display,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color {
                    r: value,
                    g: value,
                    b: value,
                    a: 1.,
                }),
                store: StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}
#[test]
fn post_modes_and_paint_inspection_preserve_their_output_contracts() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("real GPU");
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let mut targets = Targets::new(&gpu.device, SIZE);
    let texture = |label, size: [u32; 2], format| {
        gpu.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size[0],
                height: size[1],
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
    let hdr = texture("controlled-hdr", SIZE, HDR_FORMAT);
    gpu.queue.write_texture(
        hdr.as_image_copy(),
        bytemuck::cast_slice(
            &[0x3400u16, 0x3800, 0x3a00, 0x3c00].repeat((SIZE[0] * SIZE[1]) as usize),
        ),
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE[0] * 8),
            rows_per_image: None,
        },
        hdr.size(),
    );
    targets.hdr = hdr.create_view(&Default::default());
    let bloom = Bloom {
        weights: [0.25; 4],
        strength: 4.,
        threshold: 0.,
        knee: 0.1,
        ..Default::default()
    };
    let grade = Grade {
        saturation: 0.,
        contrast: 1.,
        exposure: 1.,
        sky_grade_mix: 1.,
        shadow_tint: [1.; 4],
        highlight_tint: [1.; 4],
        ..Default::default()
    };
    let post = post::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        bytemuck::bytes_of(&bloom),
        bytemuck::bytes_of(&grade),
    );
    let mut results = vec![];
    for mode in [PostMode::Raw, PostMode::Ungraded, PostMode::Look] {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        post.encode(&mut encoder, mode);
        results.push(pixels(&gpu, &targets, encoder)[0]);
    }
    assert_eq!(
        results[0],
        [191, 128, 64, 255],
        "raw class bytes were tone mapped or graded"
    );
    assert_ne!(results[1], results[0], "mask output bypassed the tone map");
    assert_ne!(results[2], results[1], "look output lost bloom and grading");
    // An ungraded view after the bloom pass must still ignore that bright bloom.
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    post.encode(&mut encoder, PostMode::Ungraded);
    assert_eq!(
        pixels(&gpu, &targets, encoder)[0],
        results[1],
        "mask view inherited world bloom"
    );
    let paint = texture("controlled-paint", SIZE, TextureFormat::Rgba8Unorm);
    gpu.queue.write_texture(
        paint.as_image_copy(),
        &[102u8, 26, 0, 128].repeat((SIZE[0] * SIZE[1]) as usize),
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE[0] * 4),
            rows_per_image: None,
        },
        paint.size(),
    );
    targets.paint = paint.create_view(&Default::default());
    let halo = texture(
        "controlled-halo",
        [SIZE[0] / 2, SIZE[1] / 2],
        TextureFormat::Rgba8Unorm,
    );
    gpu.queue.write_texture(
        halo.as_image_copy(),
        &vec![255; (SIZE[0] * SIZE[1]) as usize],
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE[0] * 2),
            rows_per_image: None,
        },
        halo.size(),
    );
    targets.overlay_glow = halo.create_view(&Default::default());
    let overlay = overlay::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        &renderer_core::records::OverlayGlow {
            radius_px: 4.0,
            sigma_px: 4.0,
            ..Default::default()
        },
    );
    for (background, expected) in [(0., [0u8, 26, 102, 255]), (1., [127u8, 153, 229, 255])] {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        backdrop(&mut encoder, &targets, background);
        overlay.encode_paint(&mut encoder, &targets.display);
        let actual = pixels(&gpu, &targets, encoder)[0];
        assert!(
            actual
                .into_iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 1),
            "paint inspector changed stored hue, alpha or added halo: {actual:?} vs {expected:?}"
        );
    }
}
