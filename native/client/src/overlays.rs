//! The indicators a capture page drew over its battle (`overlays.bin`, beside
//! `capture.json`): the live game's contact glyphs, aircraft marks, orders,
//! zone, objectives and border, as the battle view's builders made them for
//! the side's own view, recorded frame by frame. Native draws them as the
//! page uploaded them; it never builds an indicator itself.
//!
//! The page rebuilds its overlay on every publication, and the map's border
//! alone is over a megabyte, so the file holds each distinct part once, and
//! each frame names the parts its overlay concatenates
//! (`combineWorldMeshes`). Little-endian:
//!
//! ```text
//! b"BOVL", u32 version (1), u32 frames, u32 parts
//! parts × { u32 floats per channel (opaque, translucent, painted,
//!           paintedMarching), each a multiple of ten; those f32s in order }
//! frames × { u32 count; count × u32 part index }
//! ```
//!
//! Frames are in lockstep with the capture's frame records. A stale or
//! malformed file is refused, never read in part.
use std::path::Path;

/// The sidecar's name in a scene's directory.
pub const FILE: &str = "overlays.bin";
/// Floats per vertex in every channel (position, normal, colour), as the
/// browser's player meshes.
pub const VERTEX_FLOATS: usize = 10;
const MAGIC: &[u8; 4] = b"BOVL";
const VERSION: u32 = 1;

/// The overlay channels, in the file's order: display-space opaque and
/// translucent marks, then still and marching ground paint.
pub type Channels = [Vec<f32>; 4];

/// A recorded run's overlays: its distinct parts and each frame's.
#[derive(Debug, Default, PartialEq)]
pub struct RecordedOverlays {
    parts: Vec<Channels>,
    frames: Vec<Vec<u32>>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let len = self.bytes.len();
        let end = self.at.checked_add(n).filter(|&end| end <= len);
        let end =
            end.ok_or_else(|| format!("ends at byte {len}, short of {n} more at {}", self.at))?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    /// The next `n` four-byte words, all there or refused.
    fn words(&mut self, n: usize) -> Result<&[[u8; 4]], String> {
        let bytes = self.take(n.checked_mul(4).ok_or("a count overflows")?)?;
        Ok(bytes.as_chunks::<4>().0)
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.words(1)?[0]))
    }

    fn u32s(&mut self, n: usize) -> Result<Vec<u32>, String> {
        Ok(self
            .words(n)?
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect())
    }

    fn floats(&mut self, n: usize) -> Result<Vec<f32>, String> {
        Ok(self
            .words(n)?
            .iter()
            .map(|w| f32::from_le_bytes(*w))
            .collect())
    }
}

impl RecordedOverlays {
    /// Reads `path`, which must hold exactly `frames` frames.
    pub fn read(path: &Path, frames: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes, frames).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The overlays `bytes` hold, for a capture of `frames` frame records.
    pub fn parse(bytes: &[u8], frames: usize) -> Result<Self, String> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a recorded overlay file".into());
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(format!("overlay file version {version}, not {VERSION}"));
        }
        let count = r.u32()? as usize;
        if count != frames {
            return Err(format!("{count} overlay frames for {frames} frame records"));
        }
        let part_count = r.u32()? as usize;
        let mut parts = Vec::new();
        for part in 0..part_count {
            let counts = [r.u32()?, r.u32()?, r.u32()?, r.u32()?].map(|n| n as usize);
            if let Some(n) = counts.iter().find(|n| !n.is_multiple_of(VERTEX_FLOATS)) {
                return Err(format!(
                    "part {part}: {n} floats is not whole {VERTEX_FLOATS}-float vertices"
                ));
            }
            let mut channels = Channels::default();
            for (channel, n) in channels.iter_mut().zip(counts) {
                *channel = r.floats(n)?;
            }
            parts.push(channels);
        }
        let mut indexed = Vec::with_capacity(count);
        for frame in 0..count {
            let n = r.u32()? as usize;
            let named = r.u32s(n)?;
            if let Some(i) = named.iter().find(|&&i| i as usize >= part_count) {
                return Err(format!("frame {frame} names part {i} of {part_count}"));
            }
            indexed.push(named);
        }
        if r.at != bytes.len() {
            return Err(format!("{} bytes past the last frame", bytes.len() - r.at));
        }
        Ok(Self {
            parts,
            frames: indexed,
        })
    }

    /// The parts frame `index` draws, in order: the same list for the same
    /// overlay.
    pub fn parts_of(&self, index: usize) -> &[u32] {
        self.frames.get(index).map_or(&[], Vec::as_slice)
    }

    /// Frame `index`'s overlay: each channel its parts concatenated, as the
    /// page combined and uploaded them.
    pub fn channels(&self, index: usize) -> Channels {
        let mut channels = Channels::default();
        for &part in self.parts_of(index) {
            for (channel, floats) in channels.iter_mut().zip(&self.parts[part as usize]) {
                channel.extend_from_slice(floats);
            }
        }
        channels
    }
}

/// What the replay draws its recorded indicators with: the overlays, and
/// the game's ground paint and overlay halo (`presentation.overlay`), as
/// every browser view sets them (`gamePaint`, `gameOverlayGlow`).
pub struct Indicators {
    pub overlays: RecordedOverlays,
    pub paint: renderer_core::paint::Paint,
    pub glow: renderer_core::records::OverlayGlow,
}

impl Indicators {
    pub fn new(
        overlays: RecordedOverlays,
        style: &presentation::settings::Overlay,
    ) -> Result<Self, String> {
        let (glow, paint) = (&style.glow, &style.paint);
        let paint = renderer_core::paint::Paint {
            albedo: paint.albedo as f32,
            emissive: glow.ground as f32,
            fog_keep: paint.fog_keep as f32,
            grass_glow: paint.grass_glow as f32,
            grass_falloff_m: paint.grass_falloff_m as f32,
            saturation: paint.saturation as f32,
        };
        paint.validate()?;
        Ok(Self {
            overlays,
            paint,
            // The browser's halo: its blur's sigma follows its radius.
            glow: renderer_core::records::OverlayGlow {
                radius_px: glow.radius_px as f32,
                sigma_px: glow.radius_px as f32 / 2.5,
                strength: glow.overlay as f32,
                pad: 0.0,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in the documented layout: `parts` (four channels each), then
    /// each frame's part indices.
    fn file(frames: &[&[u32]], parts: &[[&[f32]; 4]]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for n in [VERSION, frames.len() as u32, parts.len() as u32] {
            bytes.extend(n.to_le_bytes());
        }
        for part in parts {
            for channel in part {
                bytes.extend((channel.len() as u32).to_le_bytes());
            }
            for value in part.iter().flat_map(|c| c.iter()) {
                bytes.extend(value.to_le_bytes());
            }
        }
        for frame in frames {
            bytes.extend((frame.len() as u32).to_le_bytes());
            for i in *frame {
                bytes.extend(i.to_le_bytes());
            }
        }
        bytes
    }

    fn vertex(x: f32) -> Vec<f32> {
        (0..VERTEX_FLOATS).map(|i| x + i as f32).collect()
    }

    #[test]
    fn a_frame_concatenates_its_parts_channel_by_channel() {
        let (a, b, c) = (vertex(0.0), vertex(100.0), vertex(200.0));
        let bytes = file(
            &[&[], &[0, 1], &[1]],
            &[[&[], &a, &b, &[]], [&c, &[], &a, &b]],
        );
        let read = RecordedOverlays::parse(&bytes, 3).unwrap();
        assert_eq!(
            read.channels(0),
            Channels::default(),
            "an empty frame draws nothing"
        );
        assert_eq!(
            read.channels(1),
            [
                c.clone(),
                a.clone(),
                [b.clone(), a.clone()].concat(),
                b.clone()
            ]
        );
        assert_eq!(read.channels(2), [c, vec![], a, b]);
        assert_eq!(read.parts_of(1), &[0, 1]);
    }

    /// The bytes the browser writes for one part (an opaque vertex, 0…9)
    /// drawn by the second of two frames (`web/tests/overlayCapture.test.ts`).
    const GOLDEN: &str = concat!(
        "424f564c", "01000000", "02000000", "01000000", "0a000000", "00000000", "00000000",
        "00000000", "00000000", "0000803f", "00000040", "00004040", "00008040", "0000a040",
        "0000c040", "0000e040", "00000041", "00001041", "00000000", "01000000", "00000000",
    );

    #[test]
    fn the_browsers_file_reads_as_it_was_written() {
        let bytes: Vec<u8> = (0..GOLDEN.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&GOLDEN[i..i + 2], 16).unwrap())
            .collect();
        let read = RecordedOverlays::parse(&bytes, 2).unwrap();
        assert_eq!(read.channels(0), Channels::default());
        assert_eq!(read.channels(1)[0], vertex(0.0));
    }

    #[test]
    fn a_recording_without_marks_is_empty_frames() {
        let read = RecordedOverlays::parse(&file(&[&[], &[]], &[]), 2).unwrap();
        assert_eq!(read.channels(1), Channels::default());
    }

    #[test]
    fn a_stale_or_malformed_file_is_refused() {
        let good = file(&[&[0]], &[[&vertex(1.0), &[], &[], &[]]]);
        assert!(RecordedOverlays::parse(&good, 1).is_ok());
        let refused = |bytes: &[u8], frames: usize, says: &str| {
            let e = RecordedOverlays::parse(bytes, frames).expect_err(says);
            assert!(e.contains(says), "{e}");
        };
        let mut magic = good.clone();
        magic[0] = b'X';
        refused(&magic, 1, "not a recorded overlay file");
        let mut version = good.clone();
        version[4] = 2;
        refused(&version, 1, "version 2");
        refused(&good, 2, "1 overlay frames for 2 frame records");
        let ragged = file(&[&[0]], &[[&[1.0; 9], &[], &[], &[]]]);
        refused(&ragged, 1, "not whole 10-float vertices");
        let unknown = file(&[&[1]], &[[&vertex(1.0), &[], &[], &[]]]);
        refused(&unknown, 1, "names part 1 of 1");
        refused(&good[..good.len() - 2], 1, "ends at byte");
        let mut long = good.clone();
        long.push(0);
        refused(&long, 1, "1 bytes past the last frame");
    }
}
