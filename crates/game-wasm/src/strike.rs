//! The attack-ground cursor's circle (`StrikeFootprints`): how far from an aim
//! a unit's strikes can do damage, as the simulation's own footprint says
//! (`sim::strike::footprint_m`), so the page never re-derives scatter, a
//! jet's release or a salvo's walk.
use contract::scenario::Rules;
use sim::weapons::Arsenal;
use wasm_bindgen::prelude::*;

use crate::js_error;

/// A scenario's admitted rules and weapons, asked about strike footprints.
#[wasm_bindgen]
pub struct StrikeFootprints {
    rules: Rules,
    arsenal: Arsenal,
}

#[wasm_bindgen]
impl StrikeFootprints {
    /// `rules_json` is a scenario's rules (the fixture with its catalog),
    /// admitted once.
    #[wasm_bindgen(constructor)]
    pub fn new(rules_json: &str) -> Result<StrikeFootprints, JsError> {
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        sim::admit(&rules).map_err(js_error)?;
        Ok(StrikeFootprints {
            arsenal: Arsenal::new(&rules),
            rules,
        })
    }

    /// The footprint radius, in metres, of a unit of `kind` striking a ground
    /// aim `distance_m` away; none for an unknown kind or one with no row that
    /// strikes an area.
    pub fn radius_m(&self, kind: &str, distance_m: f64) -> Option<f64> {
        let kind = self.rules.catalog.index(kind)?;
        sim::strike::footprint_m(&self.rules, &self.arsenal, kind, distance_m)
    }
}
