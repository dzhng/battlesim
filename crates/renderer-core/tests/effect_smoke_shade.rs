//! Dark smoke shades what is behind it: a lit smoke puff of a near-black
//! albedo darkens a bright world, and a white one still lightens a dark
//! one, through the shared effect pass; an engine's billow (a sheet sprite
//! flagged to draw noise, not its sheet) hides by optical depth, so dense
//! black smoke nearly blacks out even a bright HDR sky.
#[path = "support/effects.rs"]
mod effects;
#[path = "support/readback.rs"]
mod readback;
mod support;

use presentation::view::{Camera as Eye, View as Framed};
use wgpu::*;

const SIZE: [u32; 2] = [128, 64];

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

/// A lit smoke-sheet puff (shape 2, layer 1) 4 m across at the middle.
fn puff(albedo: f32, opacity: f32) -> [f32; 16] {
    [
        0., 0., 0., 4., 0., 10., 1., 2., albedo, albedo, albedo, opacity, 2., 0., 1., 0.,
    ]
}

/// The lit target's middle after drawing `records` over an even `ground`.
fn middle(gpu: &support::Gpu, ground: f64, records: &[[f32; 16]]) -> f32 {
    let eye = Eye {
        eye: [0., 0., 40.],
        target: [0., 0., 0.],
        up: [0., 1., 0.],
        fov_y: 0.6,
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
                load: LoadOp::Clear(Color {
                    r: ground,
                    g: ground,
                    b: ground,
                    a: 1.0,
                }),
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
    let y = SIZE[1] / 2 * row / 2;
    let at = |x: u32| (y + x * 4) as usize;
    let luma = |x: u32| (0..3).map(|c| half_to_f32(halves[at(x) + c])).sum::<f32>() / 3.0;
    ROW.with(|r| *r.borrow_mut() = (0..SIZE[0]).map(luma).collect());
    luma(SIZE[0] / 2)
}

thread_local! {
    /// The middle row the last `middle` call read.
    static ROW: std::cell::RefCell<Vec<f32>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn black_smoke_darkens_a_bright_world_and_white_smoke_lightens_a_dark_one() {
    let gpu = support::gpu().expect("real GPU");
    let bright = middle(&gpu, 2.0, &[]);
    let blackened = middle(&gpu, 2.0, &[puff(0.035, 0.85)]);
    assert!(
        blackened < 0.35 * bright,
        "black smoke darkens what is behind it: {blackened} over {bright}"
    );
    let dark = middle(&gpu, 0.02, &[]);
    let whitened = middle(&gpu, 0.02, &[puff(0.95, 0.6)]);
    assert!(
        whitened > dark + 0.2,
        "white smoke lightens: {whitened} over {dark}"
    );
    // Black smoke over white smoke stays dark: the later puff is on top.
    let mixed = middle(&gpu, 2.0, &[puff(0.95, 0.8), puff(0.035, 0.85)]);
    let white = middle(&gpu, 2.0, &[puff(0.95, 0.8)]);
    assert!(
        mixed < 0.5 * white,
        "black over white reads black: {mixed} vs {white}"
    );
}

/// An engine billow (shape 2, misc.w 1) 4 m across at the middle: optical
/// depth `density` × `opacity`.
fn billow(albedo: f32, opacity: f32, density: f32) -> [f32; 16] {
    [
        0., 0., 0., 4., 0., 3., 1., 2., albedo, albedo, albedo, opacity, 2., density, 1., 1.,
    ]
}

#[test]
fn a_dense_black_billow_blacks_out_a_bright_sky_and_a_thin_white_one_does_not_hide_it() {
    let gpu = support::gpu().expect("real GPU");
    let bright = middle(&gpu, 4.0, &[]);
    let black = middle(&gpu, 4.0, &[billow(0.03, 1.0, 10.0)]);
    assert!(
        black < 0.05 * bright,
        "dense black smoke darkens HDR sky: {black} over {bright}"
    );
    // The same albedo at a thin optical depth only dims it.
    let thin = middle(&gpu, 4.0, &[billow(0.03, 1.0, 0.3)]);
    assert!(
        thin > 0.5 * bright && thin < bright,
        "thin smoke dims: {thin}"
    );
    let dark = middle(&gpu, 0.02, &[]);
    let white = middle(&gpu, 0.02, &[billow(0.95, 1.0, 2.0)]);
    assert!(
        white > dark + 0.2,
        "white billow lightens: {white} over {dark}"
    );
    // Its edge is soft and broken: the middle is denser than near its rim.
    let none = middle(&gpu, 4.0, &[billow(0.03, 0.0, 10.0)]);
    assert!((none - bright).abs() < 1e-3, "no opacity, no smoke");
}

/// An engine plume (shape 0 ribbon, misc.w 2 + density) 6 m wide across the
/// middle, its noise at `along`.
fn plume(albedo: f32, density: f32, along: [f32; 2]) -> [f32; 16] {
    [
        -10.,
        0.,
        0.,
        6.,
        10.,
        0.,
        0.,
        6.,
        albedo,
        albedo,
        albedo,
        1.,
        0.,
        along[0],
        along[1],
        2. + density,
    ]
}

#[test]
fn a_dense_dark_plume_darkens_a_bright_sky_and_a_white_one_lightens_a_dark_one() {
    let gpu = support::gpu().expect("real GPU");
    let bright = middle(&gpu, 4.0, &[]);
    let dark = middle(&gpu, 4.0, &[plume(0.03, 8.0, [0., 3.])]);
    assert!(
        dark < 0.25 * bright,
        "dense dark plume darkens HDR sky: {dark} over {bright}"
    );
    let thin = middle(&gpu, 4.0, &[plume(0.03, 0.2, [0., 3.])]);
    assert!(thin > 0.7 * bright, "a thin plume only dims: {thin}");
    let night = middle(&gpu, 0.02, &[]);
    let white = middle(&gpu, 0.02, &[plume(0.95, 3.0, [0., 3.])]);
    assert!(
        white > night + 0.2,
        "a white plume lightens: {white} over {night}"
    );
}

#[test]
fn a_dark_plume_thinner_than_a_pixel_hides_less_and_never_glows() {
    let gpu = support::gpu().expect("real GPU");
    let bright = middle(&gpu, 4.0, &[]);
    // A 1 cm wide dark plume, far below a pixel at this view: drawn at the
    // least width, it must not brighten what is behind it (no zoom boost).
    let mut thin = plume(0.03, 8.0, [0., 3.]);
    (thin[3], thin[7]) = (0.01, 0.01);
    let seen = middle(&gpu, 4.0, &[thin]);
    assert!(
        seen <= bright + 1e-3,
        "a subpixel dark plume glowed: {seen} over {bright}"
    );
    let full = middle(&gpu, 4.0, &[plume(0.03, 8.0, [0., 3.])]);
    assert!(
        seen > full,
        "and hides less than a whole one: {seen} vs {full}"
    );
}

#[test]
fn a_mixed_plume_holds_both_black_and_white_smoke_inside_it() {
    let gpu = support::gpu().expect("real GPU");
    let grey = middle(&gpu, 1.0, &[]);
    // A mixed plume (misc.w 1000 + depth; red and green its greys) along x.
    let mut mixed = plume(0.5, 6.0, [0., 6.]);
    (mixed[8], mixed[9], mixed[10], mixed[15]) = (0.02, 0.95, 0.0, 1006.0);
    middle(&gpu, 1.0, &[mixed]);
    let row = ROW.with(|r| r.borrow().clone());
    let inside = &row[24..104];
    let darkest = inside.iter().copied().fold(f32::MAX, f32::min);
    let lightest = inside.iter().copied().fold(0.0f32, f32::max);
    assert!(
        darkest < 0.5 * grey,
        "black smoke within it: {darkest} on {grey}"
    );
    assert!(
        lightest > 1.3 * grey,
        "and white smoke: {lightest} on {grey}"
    );
}
