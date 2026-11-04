//! Atomic external asset admission and descriptive tooling model requests.
use super::{error, Presentation};
use presentation::presentation::lab::Asset;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    pub fn canonical_appearance_rows(&self) -> u32 {
        self.inner.canonical_appearance_rows() as u32
    }

    /// Starts a replacement catalog. The active catalog remains usable until
    /// installation succeeds; a failed fetch or decode cannot clear it.
    pub fn begin_lab_assets(&mut self) {
        self.lab_assets.clear();
    }

    pub fn add_lab_asset(
        &mut self,
        name: String,
        bundle: &[u8],
        skeleton: Option<Vec<u8>>,
    ) -> Result<(), JsError> {
        let asset = Asset::from_bundle(bundle, skeleton.as_deref()).map_err(error)?;
        self.lab_assets.insert(name, asset);
        Ok(())
    }

    /// Commits the staged catalog and returns the current world header.
    pub fn install_lab_assets(&mut self, detail: &str) -> Result<u32, JsError> {
        let detail = serde_json::from_str(detail).map_err(error)?;
        self.inner
            .install_lab_assets(std::mem::take(&mut self.lab_assets), detail);
        Ok(self.refresh_world_header())
    }

    pub fn set_lab_models(&mut self, json: &str) -> Result<(), JsError> {
        self.inner
            .set_lab_models(serde_json::from_str(json).map_err(error)?)
            .map_err(error)
    }
}
