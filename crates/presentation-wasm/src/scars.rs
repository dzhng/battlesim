//! The scar pool's upload, as the renderer takes it.
use wasm_bindgen::prelude::*;

/// What a renderer uploads before drawing the scars
/// (`presentation::scars::ScarUpload`).
#[wasm_bindgen]
pub struct ScarUpload(pub(crate) presentation::scars::ScarUpload);

#[wasm_bindgen]
impl ScarUpload {
    /// The directory's words (two a `rg32uint` texel) when it changed.
    pub fn directory(&self) -> Option<Vec<u32>> {
        self.0.directory.as_ref().map(|(words, _)| words.clone())
    }
    /// The directory texture's width and height, when it changed.
    pub fn directory_size(&self) -> Vec<u32> {
        self.0
            .directory
            .as_ref()
            .map_or_else(Vec::new, |(_, s)| s.to_vec())
    }
    /// Pool writes, (offset, texels) pairs, each within one row.
    pub fn writes(&self) -> Vec<u32> {
        self.0.writes.concat()
    }
    /// The writes' texels in order, four bytes each.
    pub fn texels(&self) -> Vec<u8> {
        self.0.texels.clone()
    }
    /// `ScarParams`' words before the look.
    pub fn params(&self) -> Vec<u32> {
        self.0.params.to_vec()
    }
}

/// The scar pool's first texels: the normalization table, written once.
#[wasm_bindgen]
pub fn scar_normalization() -> Vec<u8> {
    presentation::scars::normalization()
}
