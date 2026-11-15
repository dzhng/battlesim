//! Authored-model impostors: one unlit bake, coverage filtering and relit cards
//! shared by native, browser and offscreen model review.
mod bake;
mod camera;
mod cards;

pub use bake::AtlasRequest;
pub(crate) use bake::{AtlasInput, BakeInput, ResidentInput};
pub(crate) use cards::{CardInstance, Cards};

#[derive(Clone)]
pub(crate) struct CardFrame {
    pub center: [f64; 3],
    pub radius: f64,
}

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub yaws: u32,
    pub pitches: Vec<f64>,
    pub cell: u32,
    pub supersample: u32,
}
/// A tripwire for temporary render-target and readback memory, not a card cap.
pub const DEFAULT_BAKE_MEMORY_BYTES: u64 = 64 * 1024 * 1024;

impl Spec {
    pub fn runtime() -> Self {
        Self {
            yaws: 8,
            pitches: vec![0.85, 0.5],
            cell: 64,
            supersample: 2,
        }
    }
    /// Temporary GPU payload for three RGBA8 targets, their aligned readbacks,
    /// and the Depth32 target. Validate dimensions before allocating anything.
    pub fn bake_memory_bytes(&self) -> Result<u64, String> {
        self.validate(u32::MAX)?;
        let px = u64::from(self.cell) * u64::from(self.supersample);
        let width = u64::from(self.yaws) * px;
        let height = self.pitches.len() as u64 * px;
        let row = width * 4;
        let read_row = row.next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
        row.checked_mul(4)
            .and_then(|targets| read_row.checked_mul(3)?.checked_add(targets))
            .and_then(|bytes| bytes.checked_mul(height))
            .ok_or_else(|| "impostor bake memory exceeds byte bounds".into())
    }

    pub fn validate(&self, max_texture_dimension: u32) -> Result<(), String> {
        if self.yaws == 0
            || self.pitches.is_empty()
            || self.cell == 0
            || self.supersample == 0
            || self.pitches.iter().any(|p| !p.is_finite())
        {
            return Err("impostor spec needs positive dimensions and finite pitches".into());
        }
        let side = self.cell.checked_mul(self.supersample);
        let dimensions = side.and_then(|side| {
            Some([
                side.checked_mul(self.yaws)?,
                side.checked_mul(self.pitches.len() as u32)?,
            ])
        });
        if dimensions.is_none_or(|d| d.into_iter().any(|n| n > max_texture_dimension)) {
            return Err("impostor atlas exceeds the device texture dimensions".into());
        }
        Ok(())
    }
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            cell: 128,
            ..Self::runtime()
        }
    }
}

/// Review can inspect the resident runtime layer, or force an independent bake.
#[derive(Clone, Copy)]
pub enum AtlasSource<'a> {
    Fresh(&'a Spec),
    PreferResident(&'a Spec),
}

#[derive(Clone)]
pub struct Atlas {
    /// Runtime routing, excluded from the cross-platform pixel hash.
    pub layer: Option<u32>,
    pub appearance: String,
    pub spec: Spec,
    pub width: u32,
    pub height: u32,
    pub center: [f64; 3],
    pub radius: f64,
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
    pub mask: Vec<u8>,
    pub hash: String,
}
impl Atlas {
    fn hash(&mut self) {
        // Match JSON.stringify's ECMAScript number formatting and property order
        // so the same GPU pixels have the same identity across both platforms.
        let number = |n| ryu_js::Buffer::new().format(n).to_owned();
        let appearance = serde_json::to_string(&self.appearance).expect("string JSON");
        let pitches = self
            .spec
            .pitches
            .iter()
            .map(|&p| number(p))
            .collect::<Vec<_>>()
            .join(",");
        let center = self.center.map(number).join(",");
        let metadata=format!("{{\"appearance\":{appearance},\"spec\":{{\"yaws\":{},\"pitches\":[{pitches}],\"cell\":{},\"supersample\":{}}},\"center\":[{center}],\"radius\":{}}}",self.spec.yaws,self.spec.cell,self.spec.supersample,number(self.radius));
        let mut hash = Sha256::new();
        for bytes in [metadata.as_bytes(), &self.albedo, &self.normal, &self.mask] {
            hash.update(bytes);
        }
        self.hash = format!("{:x}", hash.finalize());
    }
}

/// Exact coverage-weighted integer-byte filter used by the authored bake.
fn box_filter(source: &[u8], width: u32, height: u32, factor: u32) -> Vec<u8> {
    let (w, h) = (width / factor, height / factor);
    let mut out = vec![0; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u64; 4];
            for dy in 0..factor {
                for dx in 0..factor {
                    let at = (((y * factor + dy) * width + x * factor + dx) * 4) as usize;
                    let alpha = u64::from(source[at + 3]);
                    for c in 0..3 {
                        sum[c] += u64::from(source[at + c]) * alpha;
                    }
                    sum[3] += alpha;
                }
            }
            let at = ((y * w + x) * 4) as usize;
            if sum[3] > 0 {
                for c in 0..3 {
                    out[at + c] = (sum[c] as f64 / sum[3] as f64).round() as u8;
                }
            }
            out[at + 3] = (sum[3] as f64 / f64::from(factor * factor)).round() as u8;
        }
    }
    out
}
