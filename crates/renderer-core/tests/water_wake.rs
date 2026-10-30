//! Observe one production wave step on calm water, before propagation can
//! disguise the impulse's direction. Run under the shared GPU lock.
#[path = "support/readback.rs"]
mod readback;
mod support;

use presentation::{records::WaveSplat, wave::SPLATS_MAX};
use renderer_core::{bind, buffer, pipelines, records::WaveRun, HDR_FORMAT};
use wgpu::*;

const SIZE: u32 = 128;
const CENTRE: i32 = 64;
const RADIUS: f32 = 3.0;

#[derive(Debug, Default)]
struct Displacement {
    behind: f64,
    ahead: f64,
    left_arm: f64,
    right_arm: f64,
}

fn displacement(pixels: &[[u16; 4]], heading: [i32; 2]) -> Displacement {
    let mut result = Displacement::default();
    for y in 0..SIZE as i32 {
        for x in 0..SIZE as i32 {
            let along = (x - CENTRE) * heading[0] + (y - CENTRE) * heading[1];
            let across = (y - CENTRE) * heading[0] - (x - CENTRE) * heading[1];
            let height =
                half::f16::from_bits(pixels[(y as u32 * SIZE + x as u32) as usize][0]).to_f64();
            assert!(height.is_finite(), "non-finite height at ({x}, {y})");
            if along < -(RADIUS as i32) {
                result.behind += height.abs();
                if across < -(RADIUS as i32) {
                    result.left_arm += height.abs();
                }
                if across > RADIUS as i32 {
                    result.right_arm += height.abs();
                }
            }
            if along > RADIUS as i32 {
                result.ahead += height.abs();
            }
        }
    }
    result
}

#[test]
fn wave_step_trails_both_wake_arms_behind_travel_but_rings_are_symmetric() {
    let gpu = support::gpu().expect("a real GPU");
    let device = &gpu.device;
    let pipelines = pipelines::build_builtin(device).unwrap();
    let pipeline = pipelines.bound("waveStep");
    let texture = |label, usage| {
        device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage,
            view_formats: &[],
        })
    };
    // A fresh texture is zero-initialized. Every comparison reads this same
    // calm field, so production speed and damping contribute nothing.
    let calm = texture("calm-water", TextureUsages::TEXTURE_BINDING);
    let output = texture(
        "wave-step-output",
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
    );
    let run = buffer(
        device,
        &gpu.queue,
        "wave-run",
        BufferUsages::UNIFORM,
        bytemuck::bytes_of(&WaveRun {
            splats: 1,
            ..Default::default()
        }),
    );
    let splats = device.create_buffer(&BufferDescriptor {
        label: Some("wave-splats"),
        size: (SPLATS_MAX * size_of::<WaveSplat>()) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let group = bind(
        device,
        pipelines.layout("waveLayout"),
        "wave-step",
        &[
            BindingResource::TextureView(&calm.create_view(&Default::default())),
            run.as_entire_binding(),
            splats.as_entire_binding(),
        ],
    );
    let read = device.create_buffer(&BufferDescriptor {
        label: Some("wave-readback"),
        size: u64::from(SIZE * SIZE) * 8,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    for (kind, heading) in [(0.0, [0, 0]), (1.0, [1, 0]), (1.0, [0, 1])] {
        gpu.queue.write_buffer(
            &splats,
            0,
            bytemuck::bytes_of(&WaveSplat {
                // Centre on a texel, well clear of the absorbing border.
                splat: [CENTRE as f32 + 0.5, CENTRE as f32 + 0.5, RADIUS, 1.0],
                step: 0.0,
                kind,
                heading: heading.map(|v| v as f32),
            }),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output.create_view(&Default::default()),
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pipeline.set(&mut pass, &[("waveLayout", &group)]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &read,
                layout: TexelCopyBufferLayout {
                    bytes_per_row: Some(SIZE * 8),
                    ..Default::default()
                },
            },
            output.size(),
        );
        let pixels = readback::pixels::<[u16; 4]>(device, &gpu.queue, encoder, &read);
        if kind == 0.0 {
            for axis in [[1, 0], [0, 1]] {
                let d = displacement(&pixels, axis);
                assert!(d.behind > 0.0, "the ring must disturb calm water: {d:?}");
                assert!(
                    (d.behind - d.ahead).abs() < 1e-5 && (d.left_arm - d.right_arm).abs() < 1e-5,
                    "rings must spread symmetrically on axis {axis:?}: {d:?}"
                );
            }
        } else {
            let d = displacement(&pixels, heading);
            assert!(
                d.behind > d.ahead * 4.0 && d.left_arm > 0.0 && d.right_arm > 0.0,
                "V-wakes must trail behind heading {heading:?} with arms on both sides: {d:?}"
            );
        }
    }
}
