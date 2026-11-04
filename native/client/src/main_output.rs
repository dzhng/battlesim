use super::*;
use renderer_core::post::PostMode;

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

/// Writes 8-bit RGBA, or with `Sixteen`, big-endian 16-bit RGB.
pub(super) fn write_png(
    path: &std::path::Path,
    size: [u32; 2],
    depth: png::BitDepth,
    pixels: &[u8],
) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size[0], size[1]);
    encoder.set_color(match depth {
        png::BitDepth::Sixteen => png::ColorType::Rgb,
        _ => png::ColorType::Rgba,
    });
    encoder.set_depth(depth);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(pixels).map_err(|e| e.to_string())
}

/// An RGBA16F readback (little-endian half floats) of a signal in [0, 1], as
/// the big-endian 16-bit RGB a PNG stores.
fn rgb16(rgba16f: &[u8]) -> Vec<u8> {
    let channel = |half: &[u8; 2]| {
        let value = half::f16::from_le_bytes(*half).to_f32().clamp(0.0, 1.0);
        ((value * 65535.0).round() as u16).to_be_bytes()
    };
    let halves = rgba16f.as_chunks::<2>().0;
    halves
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|texel| texel[..3].iter().flat_map(channel))
        .collect()
}

impl super::Run {
    /// Draws the prepared frame and writes it to `path` as a PNG: the display,
    /// or the HDR look's signal at 16 bits. Then presents it.
    pub(super) fn write_frame(&mut self, path: &Path) -> Result<(), String> {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let mut encoder = device.create_command_encoder(&Default::default());
        self.scene
            .encode(device, queue, &self.view, &mut encoder, Default::default());
        let hdr = self.scene.post.hdr_output(PostMode::Look);
        let source = hdr.map_or(&self.scene.targets.display_texture, |view| view.texture());
        let read = renderer_core::readback::RgbaReadback::encode(device, &mut encoder, source)?;
        queue.submit([encoder.finish()]);
        let mut texels = pollster::block_on(read.read(device))?;
        let size = self.scene.targets.size;
        if hdr.is_some() {
            output::write_png(path, size, png::BitDepth::Sixteen, &rgb16(&texels))?;
        } else {
            // Native plates are opaque exports, independent of stored display alpha.
            for pixel in texels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
            output::write_png(path, size, png::BitDepth::Eight, &texels)?;
        }
        self.written += 1;
        let encoder = device.create_command_encoder(&Default::default());
        self.present(encoder);
        Ok(())
    }
}
