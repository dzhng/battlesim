use super::*;

/// Interleaved stereo float samples as a 32-bit float WAV.
pub(super) fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<(), String> {
    let rate = audio::SAMPLE_RATE;
    let data = (samples.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    for (v, n) in [
        (16u32, 4),
        (3, 2),
        (2, 2),
        (rate, 4),
        (rate * 8, 4),
        (8, 2),
        (32, 2),
    ] {
        bytes.extend_from_slice(&v.to_le_bytes()[..n]);
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub(super) fn write_png(path: &std::path::Path, size: [u32; 2], rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size[0], size[1]);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}

pub(super) fn write_exr(
    path: &std::path::Path,
    size: [u32; 2],
    rgba16f: &[u8],
) -> Result<(), String> {
    use exr::prelude::*;
    use half::f16;

    let width = size[0] as usize;
    let height = size[1] as usize;

    let pixels: Vec<f16> = rgba16f
        .chunks_exact(2)
        .map(|b| f16::from_le_bytes([b[0], b[1]]))
        .collect();

    let create_channel = |offset: usize| -> Vec<f16> {
        (0..width * height)
            .map(|i| pixels[i * 4 + offset])
            .collect()
    };

    let red = create_channel(0);
    let green = create_channel(1);
    let blue = create_channel(2);

    let layer = Layer::new(
        (width, height),
        LayerAttributes::named("main"),
        Encoding::SMALL_LOSSLESS,
        SpecificChannels::rgb(|pos: Vec2<usize>| {
            let idx = pos.y() * width + pos.x();
            (red[idx], green[idx], blue[idx])
        }),
    );

    let image = Image::from_layer(layer);
    image
        .write()
        .to_file(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}
