//! One unit at rest with no battle (`presentation::presentation::showcase`)
//! and the loops it idles with.
use presentation::sound::idle;
use presentation::world::UnitsSection;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::{error, Presentation};

/// The `units` section `Fielding::resolve` returned for the shown unit, its
/// clip tables added with `add_clip_table`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sections {
    units: UnitsSection,
}

#[wasm_bindgen]
impl Presentation {
    /// Shows one unit at rest (`ShowcaseInput` JSON) drawn from `fielded`
    /// (`{units}` JSON) and the clip tables added before; returns the
    /// `WorldHeader`'s address, its rows changed.
    pub fn set_showcase(&mut self, fielded: &str, input: &str) -> Result<u32, JsError> {
        let Sections { units } = serde_json::from_str(fielded).map_err(error)?;
        let tables = std::mem::take(&mut self.tables);
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
