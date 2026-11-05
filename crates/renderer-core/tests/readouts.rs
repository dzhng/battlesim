//! GPU regression of the unit info cards' pass over the exported shaders:
//! laid-out primitives (a filled rectangle, a sprite from the atlas, a
//! stroke) are drawn into the readout layer and laid over a finished
//! display and over the HDR look's HLG target through the overlay's
//! composite. White lands at the display's white and at HLG reference
//! white (signal 0.75); partial coverage keeps its share, premultiplied;
//! a frame without cards leaves the picture untouched. Run under the
//! repository GPU lock.
mod support;

use renderer_core::pipelines;
use renderer_core::readback::RgbaReadback;
use renderer_core::readouts::{Line, ReadoutDraw, ReadoutResources, Shadow};
use renderer_core::{DISPLAY_FORMAT, HDR_FORMAT};
use wgpu::*;

const SIZE: [u32; 2] = [64, 32];
/// The display's finished look under the cards: a mid grey.
const UNDER: f64 = 0.25;

/// A 2×2 atlas of opaque white: a sprite's ink, premultiplied.
const ATLAS: [u8; 16] = [255; 16];

fn target(device: &Device, format: TextureFormat) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("test-readout-output"),
        size: Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The primitives laid over a cleared `format` target, read back as floats.
fn drawn(
    gpu: &support::Gpu,
    pass: &mut ReadoutResources,
    draw: &ReadoutDraw,
    format: TextureFormat,
) -> Vec<[f32; 4]> {
    let output = target(&gpu.device, format);
    let view = output.create_view(&Default::default());
    pass.prepare(&gpu.device, &gpu.queue, draw);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("test-finished-look"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color {
                    r: UNDER,
                    g: UNDER,
                    b: UNDER,
                    a: 1.0,
                }),
                store: StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    let hdr = (format == HDR_FORMAT).then_some(&view);
    pass.encode(&mut encoder, &view, hdr);
    let read = RgbaReadback::encode(&gpu.device, &mut encoder, &output).unwrap();
    gpu.queue.submit([encoder.finish()]);
    let bytes = pollster::block_on(read.read(&gpu.device)).unwrap();
    if format == HDR_FORMAT {
        bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|t| {
                std::array::from_fn(|c| half::f16::from_le_bytes([t[2 * c], t[2 * c + 1]]).to_f32())
            })
            .collect()
    } else {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|t| t.map(|v| f32::from(v) / 255.0))
            .collect()
    }
}

fn at(pixels: &[[f32; 4]], x: u32, y: u32) -> [f32; 3] {
    let p = pixels[(y * SIZE[0] + x) as usize];
    [p[0], p[1], p[2]]
}

fn close(actual: [f32; 3], expected: f32, tolerance: f32, subject: &str) {
    assert!(
        actual.iter().all(|v| (v - expected).abs() <= tolerance),
        "{subject}: expected {expected} in every channel, got {actual:?}"
    );
}

/// A whole white card face (x 2…18), the atlas stretched over x 22…30 at
/// half opacity, and a whole white stroke 3 px wide along y 24.5 from x 34
/// to 60 over a black shadow.
fn primitives() -> ReadoutDraw {
    let mut draw = ReadoutDraw::new(SIZE, [2, 2]);
    draw.rect([2.0, 2.0, 18.0, 18.0], [1.0; 4], [0.0; 4], 0.0, 0.0);
    draw.sprite([0.0, 0.0, 2.0, 2.0], [22.0, 2.0, 30.0, 18.0], 0.5);
    let line = Line {
        width_px: 3.0,
        ink: [1.0; 4],
        dash: None,
        shadows: [
            Some(Shadow {
                color: [0.0, 0.0, 0.0, 1.0],
                sigma_px: 1.0,
            }),
            None,
        ],
    };
    draw.polyline(&[[34.0, 24.5], [60.0, 24.5]], &line);
    draw
}

#[test]
fn cards_lay_over_the_display_and_the_hlg_signal_at_reference_white() {
    let gpu = support::gpu().expect("a real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).expect("exported shaders");
    let mut pass =
        ReadoutResources::new(&gpu.device, &gpu.queue, &pipelines, SIZE, [2, 2], &ATLAS).unwrap();
    let draw = primitives();

    let sdr = drawn(&gpu, &mut pass, &draw, DISPLAY_FORMAT);
    let tol = 1.5 / 255.0;
    close(
        at(&sdr, 10, 10),
        1.0,
        tol,
        "a whole card face is the display's white",
    );
    close(
        at(&sdr, 40, 10),
        UNDER as f32,
        tol,
        "nothing drawn leaves the look",
    );
    close(
        at(&sdr, 26, 10),
        0.5 + 0.5 * UNDER as f32,
        2.0 / 255.0,
        "a half-opaque sprite keeps half its share, premultiplied",
    );
    close(
        at(&sdr, 26, 30),
        UNDER as f32,
        tol,
        "past the sprite, the look",
    );
    close(at(&sdr, 47, 24), 1.0, tol, "a stroke's ink at its middle");
    // Two pixels from its middle, past its ink, inside its shadow.
    let halo = at(&sdr, 47, 26);
    assert!(
        halo.iter().all(|&v| v < UNDER as f32 - 0.02),
        "the stroke's dark shadow darkens just past it: {halo:?}"
    );

    let hdr = drawn(&gpu, &mut pass, &draw, HDR_FORMAT);
    let tol = 2e-3;
    close(at(&hdr, 10, 10), 0.75, tol, "white is HLG reference white");
    close(
        at(&hdr, 40, 10),
        UNDER as f32,
        tol,
        "nothing drawn leaves the signal",
    );
    close(
        at(&hdr, 26, 10),
        0.5 * 0.75 + 0.5 * UNDER as f32,
        4e-3,
        "half coverage is half the white signal over the look",
    );
    close(
        at(&hdr, 47, 24),
        0.75,
        tol,
        "a stroke's ink at reference white",
    );
}

#[test]
fn a_frame_without_cards_leaves_the_picture_untouched() {
    let gpu = support::gpu().expect("a real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).expect("exported shaders");
    let mut pass =
        ReadoutResources::new(&gpu.device, &gpu.queue, &pipelines, SIZE, [2, 2], &ATLAS).unwrap();
    // A frame with cards, then one whose cards are gone (faded out, hidden):
    // the second must not show the first's.
    drawn(&gpu, &mut pass, &primitives(), DISPLAY_FORMAT);
    let empty = drawn(
        &gpu,
        &mut pass,
        &ReadoutDraw::new(SIZE, [2, 2]),
        DISPLAY_FORMAT,
    );
    assert!(
        empty.iter().all(|p| p[..3]
            .iter()
            .all(|v| (v - UNDER as f32).abs() <= 1.5 / 255.0)),
        "a frame without cards drew something"
    );
}
