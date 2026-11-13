//! A wisp (a ribbon broken up by its `misc.w` amount) leaves gaps where it
//! was laid and a plain ribbon stays even; a wisp drawn as two stretches
//! meets itself without a seam.
#[path = "support/effects.rs"]
mod effects;
#[path = "support/readback.rs"]
mod readback;
mod support;

use presentation::view::{Camera as Eye, View as Framed};
use wgpu::*;

const SIZE: [u32; 2] = [256, 128];

fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let frac = f32::from(h & 0x3ff);
    match exp {
        0 => sign * frac * 2f32.powi(-24),
        31 => f32::INFINITY,
        _ => sign * (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// One ribbon (shape 0, an opacity in alpha) from `a` to `b` on the ground's
/// plane, 3 m wide, its ends `along` its trail and broken by `amount`.
fn ribbon(a: [f32; 3], b: [f32; 3], along: [f32; 2], amount: f32) -> [f32; 16] {
    [
        a[0], a[1], a[2], 3.0, b[0], b[1], b[2], 0.0, 0.9, 0.9, 0.9, 0.8, 0.0, along[0], along[1],
        amount,
    ]
}

/// The lit target's brightness along the ribbon's middle row after drawing
/// `records` over black.
fn draw(gpu: &support::Gpu, records: &[[f32; 16]]) -> Vec<f32> {
    let eye = Eye {
        eye: [0., 0., 40.],
        target: [0., 0., 0.],
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 0.1,
        size: SIZE,
    };
    let framed = Framed::new(&eye);
    let (_, targets, _, mut effects, _) =
        effects::stage(gpu, SIZE, &framed.record(&eye), &framed.camera(&eye, 0.));
    effects.prepare(&gpu.device, &gpu.queue, bytemuck::cast_slice(records));
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &targets.lit,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    effects.encode(&mut encoder, &targets);
    let row = (SIZE[0] * 8).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("lit-readback"),
        size: u64::from(row * SIZE[1]),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        targets.lit_texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        targets.lit_texture.size(),
    );
    let halves: Vec<u16> = readback::pixels(&gpu.device, &gpu.queue, encoder, &buffer);
    let y = SIZE[1] / 2;
    (0..SIZE[0])
        .map(|x| {
            let at = ((y * row / 2) + x * 4) as usize;
            (0..3).map(|c| half_to_f32(halves[at + c])).sum::<f32>() / 3.0
        })
        .collect()
}

#[test]
fn a_wisp_breaks_up_where_it_was_laid_and_a_plain_ribbon_stays_even() {
    let gpu = support::gpu().expect("real GPU");
    let (a, m, b) = ([-12., 0., 0.], [0., 0., 0.], [12., 0., 0.]);
    // Its middle (24 m across about 90 px), clear of its ends.
    let middle = |v: Vec<f32>| v[94..162].to_vec();
    let plain = middle(draw(&gpu, &[ribbon(a, b, [0., 6.], 0.)]));
    let wisp = middle(draw(&gpu, &[ribbon(a, b, [0., 6.], 0.85)]));
    let range = |v: &[f32]| {
        v.iter()
            .fold((f32::MAX, 0.0f32), |(lo, hi), &x| (lo.min(x), hi.max(x)))
    };
    let (plain_lo, plain_hi) = range(&plain);
    let (wisp_lo, wisp_hi) = range(&wisp);
    assert!(plain_hi > 0.05, "the ribbon is drawn: {plain_hi}");
    assert!(
        plain_lo > 0.9 * plain_hi,
        "a plain ribbon is even: {plain_lo} {plain_hi}"
    );
    assert!(
        wisp_hi > 0.5 * plain_hi,
        "a wisp still shows: {wisp_hi} {plain_hi}"
    );
    assert!(
        wisp_lo < 0.35 * wisp_hi,
        "and breaks up along its length: {wisp_lo} {wisp_hi}"
    );
    // The same wisp as two stretches meeting at its middle, as a trail lays it.
    let split = middle(draw(
        &gpu,
        &[ribbon(a, m, [0., 3.], 0.85), ribbon(m, b, [3., 6.], 0.85)],
    ));
    let seam = SIZE[0] as usize / 2 - 94;
    for (k, (x, y)) in wisp.iter().zip(&split).enumerate() {
        if k.abs_diff(seam) > 1 {
            assert!(
                (x - y).abs() < 0.03 * wisp_hi,
                "two stretches draw as one wisp: {x} vs {y} at {k}"
            );
        }
    }
}

/// An engine plume drawn as one ribbon or as two meeting at its middle, as
/// a trail lays it, is the same plume: its noise runs on across the joint.
#[test]
fn a_plume_drawn_in_two_stretches_draws_as_one() {
    let gpu = support::gpu().expect("real GPU");
    let (a, m, b) = ([-12., 0., 0.], [0., 0., 0.], [12., 0., 0.]);
    let middle = |v: Vec<f32>| v[94..162].to_vec();
    // misc.w 2 + its optical depth; b.w its far end's width: 2 m to 4 m.
    let plume = |a, b, along, w: [f32; 2]| {
        let mut r = ribbon(a, b, along, 5.0);
        (r[3], r[7]) = (w[0], w[1]);
        r
    };
    let whole = middle(draw(&gpu, &[plume(a, b, [0., 2.], [2., 4.])]));
    let split = middle(draw(
        &gpu,
        &[
            plume(a, m, [0., 1.], [2., 3.]),
            plume(m, b, [1., 2.], [3., 4.]),
        ],
    ));
    let top = whole.iter().copied().fold(0.0f32, f32::max);
    assert!(top > 0.05, "the plume is drawn: {top}");
    let seam = SIZE[0] as usize / 2 - 94;
    for (k, (x, y)) in whole.iter().zip(&split).enumerate() {
        if k.abs_diff(seam) > 1 {
            assert!(
                (x - y).abs() < 0.03 * top,
                "two stretches draw as one plume: {x} vs {y} at {k}"
            );
        }
    }
}
