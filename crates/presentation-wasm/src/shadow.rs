//! Installed sun-shadow receivers and their diagnostics.
use crate::Presentation;
use presentation::static_world::ground::partition::{TerrainPartition, CELL_M};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// Fit local receivers under the quality-resolved light; terrain is immutable.
    pub fn set_shadow(&mut self, light: &str, terrain: &[f32]) -> Result<(), JsError> {
        let light: presentation::light::LightSettings = serde_json::from_str(light)
            .map_err(|e| JsError::new(&format!("presentation.light: {e}")))?;
        let terrain = Arc::new(TerrainPartition::new(terrain, CELL_M));
        self.inner.set_shadow(Some((light, terrain)));
        Ok(())
    }
}
