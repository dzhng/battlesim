//! One unit at rest with no battle (`presentation::presentation::showcase`)
//! and the loops it idles with.
use super::{error, Presentation};
use presentation::sound::idle;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// Shows one unit at rest (`ShowcaseInput` JSON) drawn from `fielded`
    /// (its retained Rust units and clip tables); returns the
    /// `WorldHeader`'s address, its rows changed.
    pub fn set_showcase(
        &mut self,
        fielded: &crate::fielded::Fielded,
        input: &str,
    ) -> Result<u32, JsError> {
        let units = fielded.inner.units.clone();
        let tables = fielded.inner.tables.clone();
        self.inner
            .set_showcase(units, tables, serde_json::from_str(input).map_err(error)?)
            .map_err(error)?;
        Ok(self.refresh_world_header())
    }
}

/// The loops unit type `kind` idles with, JSON `IdleLoop`s, from a battle's
/// sound setup (`SoundFrame`'s, JSON).
#[wasm_bindgen]
pub fn idle_loops(setup: &str, kind: &str) -> Result<String, JsError> {
    let loops = idle::idle_loops_of(setup, kind).map_err(error)?;
    Ok(serde_json::to_string(&loops).expect("loops serialize"))
}
