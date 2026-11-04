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

/// Writes an RGBA16F readback (little-endian half floats) as an RGB EXR.
pub(super) fn write_exr(
    path: &std::path::Path,
    size: [u32; 2],
    rgba16f: &[u8],
) -> Result<(), String> {
    use exr::prelude::*;
    let width = size[0] as usize;
    let half = |pixel: usize, channel: usize| {
        let at = (pixel * 4 + channel) * 2;
        f16::from_le_bytes([rgba16f[at], rgba16f[at + 1]])
    };
    let channels = SpecificChannels::rgb(|pos: Vec2<usize>| {
        let pixel = pos.y() * width + pos.x();
        (half(pixel, 0), half(pixel, 1), half(pixel, 2))
    });
    let layer = Layer::new(
        (width, size[1] as usize),
        LayerAttributes::named("main"),
        Encoding::SMALL_LOSSLESS,
        channels,
    );
    Image::from_layer(layer)
        .write()
        .to_file(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

impl super::Run {
    /// Draws the prepared frame and writes it to `path`: the display as a
    /// PNG, or the HDR look as an EXR beside it. Then presents it.
    pub(super) fn write_frame(&mut self, path: &Path) -> Result<(), String> {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let mut encoder = device.create_command_encoder(&Default::default());
        self.scene
            .encode(device, queue, &self.view, &mut encoder, Default::default());
        let hdr = self.scene.post.hdr_output();
        let source = hdr.map_or(&self.scene.targets.display_texture, |view| view.texture());
        let read = renderer_core::readback::RgbaReadback::encode(device, &mut encoder, source)?;
        queue.submit([encoder.finish()]);
        let mut texels = pollster::block_on(read.read(device))?;
        let size = self.scene.targets.size;
        if hdr.is_some() {
            output::write_exr(&path.with_extension("exr"), size, &texels)?;
        } else {
            // Native plates are opaque exports, independent of stored display alpha.
            for pixel in texels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
            output::write_png(path, size, &texels)?;
        }
        self.plates += 1;
        let encoder = device.create_command_encoder(&Default::default());
        self.present(encoder);
        Ok(())
    }
}
