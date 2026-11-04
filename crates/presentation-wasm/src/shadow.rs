//! Installed sun-shadow receivers and their diagnostics.
use crate::Presentation;
use presentation::static_world::ground::partition::{TerrainPartition, CELL_M};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// Executed receiver work for the latest frame, including cache reuse.
    pub fn shadow_stats(&self) -> String {
        serde_json::to_string(&self.inner.shadow_stats()).expect("receiver stats serialize")
    }

    /// Fit local receivers under the quality-resolved light; terrain is immutable.
    pub fn set_shadow(&mut self, light: &str, terrain: &[f32]) -> Result<(), JsError> {
        let light = presentation::light::LightSettings::from_value(
            &serde_json::from_str(light).map_err(crate::error)?,
        )
        .map_err(crate::error)?;
        let terrain = Arc::new(TerrainPartition::new(terrain, CELL_M));
        self.inner.set_shadow(Some((light, terrain)));
        Ok(())
    }
}
