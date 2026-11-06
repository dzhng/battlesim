//! The unit info cards a capture page showed over its battle (`cards.bin`,
//! beside `capture.json`): the live battle view's cards, laid out by its
//! readout layer from the side's own view, recorded frame by frame as their
//! primitives (`web/src/battle/benchmark/cardCapture.ts` writes it). Native
//! draws exactly those, through the shared readout pass; it decides no card,
//! no word and no place itself. Little-endian:
//!
//! ```text
//! b"BCRD", u32 version (1), u32 n, n bytes of UTF-8 JSON (`CardFile`),
//! u32 atlas width, u32 atlas height, width × height premultiplied RGBA8
//! ```
//!
//! Frames are in lockstep with the capture's frame records. A stale or
//! malformed file, or one naming what it does not hold, is refused, never
//! read in part.
use std::collections::HashMap;
use std::path::Path;

use renderer_core::readouts::{Line, ReadoutDraw, Shadow};
use serde::Deserialize;

/// The sidecar's name in a scene's directory.
pub const FILE: &str = "cards.bin";
const MAGIC: &[u8; 4] = b"BCRD";
const VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CardFile {
    /// The page's size in CSS pixels.
    viewport: [f32; 2],
    /// Atlas pixels a CSS pixel.
    scale: f32,
    cards: Vec<String>,
    fonts: Vec<Font>,
    sprites: Vec<Sprite>,
    leaders: Vec<Leader>,
    parts: Vec<Vec<Primitive>>,
    frames: Vec<Vec<Placement>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Font {
    letter_spacing: f32,
    glyphs: HashMap<char, Glyph>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Glyph {
    advance: f32,
    #[serde(deserialize_with = "Option::deserialize")]
    ink: Option<usize>,
    #[serde(deserialize_with = "Option::deserialize")]
    shadow: Option<usize>,
}

/// An atlas rectangle, and where its top-left lies from its draw point.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sprite {
    at: [f32; 2],
    size: [f32; 2],
    offset: [f32; 2],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShadowSpec {
    color: [f32; 4],
    /// The CSS blur length: a Gaussian of half its sigma.
    blur: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Leader {
    width: f32,
    ink: [f32; 4],
    opacity: f32,
    dash: Vec<f32>,
    shadows: Vec<ShadowSpec>,
}

/// One primitive of a card, in CSS pixels from its top-left; an unknown
/// kind is refused by name.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Primitive {
    Rect(Rect),
    Text(Text),
    Sprite(Icon),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Rect {
    #[serde(rename = "box")]
    rect: [f32; 4],
    fill: [f32; 4],
    border: [f32; 4],
    border_px: f32,
    blur_px: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    text: String,
    font: usize,
    at: [f32; 2],
    opacity: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Icon {
    sprite: usize,
    at: [f32; 2],
    opacity: f32,
}

/// One shown card: card, part, leader (-1 for none), opacity, x, y, ring
/// x, y, r, then the leader line's three points.
#[derive(Debug, Deserialize)]
struct Placement([f32; 15]);

impl Placement {
    fn index(&self, at: usize) -> Option<usize> {
        let v = self.0[at];
        (v >= 0.0 && v.fract() == 0.0).then_some(v as usize)
    }
    fn card(&self) -> Option<usize> {
        self.index(0)
    }
    fn part(&self) -> Option<usize> {
        self.index(1)
    }
    fn leader(&self) -> Option<usize> {
        self.index(2)
    }
}

/// A recorded run's cards.
#[derive(Debug)]
pub struct RecordedCards {
    file: CardFile,
    atlas_size: [u32; 2],
    atlas: Vec<u8>,
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

    fn u32(&mut self) -> Result<u32, String> {
        let word: [u8; 4] = self.take(4)?.try_into().map_err(|_| "a short word")?;
        Ok(u32::from_le_bytes(word))
    }
}

impl RecordedCards {
    /// Reads `path`, which must hold exactly `frames` frames.
    pub fn read(path: &Path, frames: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes, frames).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The cards `bytes` hold, for a capture of `frames` frame records.
    pub fn parse(bytes: &[u8], frames: usize) -> Result<Self, String> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a recorded card file".into());
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(format!("card file version {version}, not {VERSION}"));
        }
        let n = r.u32()? as usize;
        let file: CardFile =
            serde_json::from_slice(r.take(n)?).map_err(|e| format!("the card layout: {e}"))?;
        let atlas_size = [r.u32()?, r.u32()?];
        let texels = (atlas_size[0] as usize)
            .checked_mul(atlas_size[1] as usize)
            .and_then(|t| t.checked_mul(4))
            .ok_or("the atlas's size overflows")?;
        let atlas = r.take(texels)?.to_vec();
        if r.at != bytes.len() {
            return Err(format!("{} bytes past the atlas", bytes.len() - r.at));
        }
        if file.frames.len() != frames {
            return Err(format!(
                "{} card frames for {frames} frame records",
                file.frames.len()
            ));
        }
        let cards = Self {
            file,
            atlas_size,
            atlas,
        };
        cards.check()?;
        Ok(cards)
    }

    /// Refuses the first reference that names nothing the file holds.
    fn check(&self) -> Result<(), String> {
        let f = &self.file;
        if !(f.viewport.iter().all(|&v| v > 0.0) && f.scale > 0.0) {
            return Err(format!(
                "a {}×{} page at scale {} lays out no card",
                f.viewport[0], f.viewport[1], f.scale
            ));
        }
        let [aw, ah] = self.atlas_size.map(|v| v as f32);
        for (i, s) in f.sprites.iter().enumerate() {
            if s.at[0] + s.size[0] > aw || s.at[1] + s.size[1] > ah {
                return Err(format!("sprite {i} lies past the {aw}×{ah} atlas"));
            }
        }
        let sprite = |i: Option<usize>, at: &str| match i {
            Some(i) if i >= f.sprites.len() => {
                Err(format!("{at} names sprite {i} of {}", f.sprites.len()))
            }
            _ => Ok(()),
        };
        for (i, font) in f.fonts.iter().enumerate() {
            for (ch, g) in &font.glyphs {
                sprite(g.ink, &format!("font {i} glyph {ch}"))?;
                sprite(g.shadow, &format!("font {i} glyph {ch}"))?;
            }
        }
        for (p, part) in f.parts.iter().enumerate() {
            for primitive in part {
                match primitive {
                    Primitive::Sprite(icon) => sprite(Some(icon.sprite), &format!("part {p}"))?,
                    Primitive::Text(t) => {
                        let font = f.fonts.get(t.font).ok_or_else(|| {
                            format!("part {p} names font {} of {}", t.font, f.fonts.len())
                        })?;
                        if let Some(ch) = t.text.chars().find(|c| !font.glyphs.contains_key(c)) {
                            return Err(format!("part {p} writes {ch}, which its font lacks"));
                        }
                    }
                    Primitive::Rect(_) => {}
                }
            }
        }
        for (i, frame) in f.frames.iter().enumerate() {
            for c in frame {
                if c.card().is_none_or(|n| n >= f.cards.len()) {
                    return Err(format!("frame {i} names card {}", c.0[0]));
                }
                if c.part().is_none_or(|n| n >= f.parts.len()) {
                    return Err(format!(
                        "frame {i} names part {} of {}",
                        c.0[1],
                        f.parts.len()
                    ));
                }
                if c.0[2] != -1.0 && c.leader().is_none_or(|n| n >= f.leaders.len()) {
                    return Err(format!(
                        "frame {i} names leader {} of {}",
                        c.0[2],
                        f.leaders.len()
                    ));
                }
            }
        }
        Ok(())
    }

    /// The atlas the sprites come from: its size and premultiplied pixels.
    pub fn atlas(&self) -> ([u32; 2], &[u8]) {
        (self.atlas_size, &self.atlas)
    }

    /// The cards frame `index` shows, back to front, by name.
    pub fn cards_of(&self, index: usize) -> Vec<&str> {
        self.file.frames.get(index).map_or_else(Vec::new, |frame| {
            frame
                .iter()
                .filter_map(|c| c.card().map(|n| self.file.cards[n].as_str()))
                .collect()
        })
    }

    /// Frame `index`'s cards as the readout pass draws them on a display of
    /// `size` pixels: the page's layout scaled to it.
    pub fn draw(&self, index: usize, size: [u32; 2]) -> ReadoutDraw {
        let f = &self.file;
        let mut out = ReadoutDraw::new(size, self.atlas_size);
        let k = [
            size[0] as f32 / f.viewport[0],
            size[1] as f32 / f.viewport[1],
        ];
        let Some(frame) = f.frames.get(index) else {
            return out;
        };
        for c in frame {
            let Some(part) = c.part() else {
                continue;
            };
            let [opacity, x, y] = [c.0[3], c.0[4], c.0[5]];
            for primitive in &f.parts[part] {
                self.primitive(&mut out, primitive, [x, y], opacity, k);
            }
            if let Some(leader) = c.leader() {
                let p = &c.0;
                let at = |i: usize| [p[i] * k[0], p[i + 1] * k[1]];
                let line = self.line(&f.leaders[leader], opacity, k[0]);
                out.circle(at(6), p[8] * k[0], &line);
                out.polyline(&[at(9), at(11), at(13)], &line);
            }
        }
        out
    }

    fn primitive(
        &self,
        out: &mut ReadoutDraw,
        primitive: &Primitive,
        [x, y]: [f32; 2],
        opacity: f32,
        k: [f32; 2],
    ) {
        let fade = |c: [f32; 4], by: f32| [c[0], c[1], c[2], c[3] * by];
        match primitive {
            Primitive::Rect(r) => out.rect(
                [
                    (x + r.rect[0]) * k[0],
                    (y + r.rect[1]) * k[1],
                    (x + r.rect[2]) * k[0],
                    (y + r.rect[3]) * k[1],
                ],
                fade(r.fill, opacity),
                fade(r.border, opacity),
                r.border_px * k[0],
                r.blur_px * k[0],
            ),
            Primitive::Sprite(icon) => self.sprite(
                out,
                icon.sprite,
                [x + icon.at[0], y + icon.at[1]],
                icon.opacity * opacity,
                k,
            ),
            Primitive::Text(t) => {
                let font = &self.file.fonts[t.font];
                // Every shadow of the run under every glyph's ink, as CSS
                // paints a text shadow.
                for ink in [false, true] {
                    let mut pen = [x + t.at[0], y + t.at[1]];
                    for ch in t.text.chars() {
                        let g = &font.glyphs[&ch];
                        if let Some(s) = if ink { g.ink } else { g.shadow } {
                            self.sprite(out, s, pen, t.opacity * opacity, k);
                        }
                        pen[0] += g.advance + font.letter_spacing;
                    }
                }
            }
        }
    }

    /// Sprite `index` drawn from CSS point `at` (its offset applied).
    fn sprite(&self, out: &mut ReadoutDraw, index: usize, at: [f32; 2], opacity: f32, k: [f32; 2]) {
        let s = &self.file.sprites[index];
        let scale = self.file.scale;
        let x0 = (at[0] + s.offset[0]) * k[0];
        let y0 = (at[1] + s.offset[1]) * k[1];
        out.sprite(
            [s.at[0], s.at[1], s.at[0] + s.size[0], s.at[1] + s.size[1]],
            [
                x0,
                y0,
                x0 + s.size[0] / scale * k[0],
                y0 + s.size[1] / scale * k[1],
            ],
            opacity,
        );
    }

    fn line(&self, leader: &Leader, opacity: f32, k: f32) -> Line {
        let alpha = leader.opacity * opacity;
        let shadow = |i: usize| {
            leader.shadows.get(i).map(|s| Shadow {
                color: [s.color[0], s.color[1], s.color[2], s.color[3] * alpha],
                sigma_px: s.blur / 2.0 * k,
            })
        };
        Line {
            width_px: leader.width * k,
            ink: [
                leader.ink[0],
                leader.ink[1],
                leader.ink[2],
                leader.ink[3] * alpha,
            ],
            dash: (leader.dash.len() == 2).then(|| [leader.dash[0] * k, leader.dash[1] * k]),
            shadows: [shadow(0), shadow(1)],
        }
    }
}

#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;
