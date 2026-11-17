//! The attack-ground cursor's marks (`GroundStrikes`): how far from an aim
//! a unit's strikes can do damage, and the reach it strikes from where it
//! stands, as the simulation's own footprint and reach say
//! (`sim::strike::footprint_m`, `sim::strike::reach_m`), so the page never
//! re-derives scatter, a jet's release, a salvo's walk or a minimum range.
use contract::scenario::Rules;
use sim::weapons::Arsenal;
use wasm_bindgen::prelude::*;

use crate::js_error;

/// A scenario's admitted rules and weapons, asked about ground strikes:
/// their footprints and reach.
#[wasm_bindgen]
pub struct GroundStrikes {
    rules: Rules,
    arsenal: Arsenal,
}

#[wasm_bindgen]
impl GroundStrikes {
    /// `rules_json` is a scenario's rules (the fixture with its catalog),
    /// admitted once.
    #[wasm_bindgen(constructor)]
    pub fn new(rules_json: &str) -> Result<GroundStrikes, JsError> {
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        sim::admit(&rules).map_err(js_error)?;
        Ok(GroundStrikes {
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

    /// `[nearest, farthest]`, in metres, a ground aim may lie from a unit of
    /// `kind` for it to strike there from where it stands; none for an
    /// unknown kind, a jet, or one with no row that strikes an area.
    pub fn reach_m(&self, kind: &str) -> Option<Vec<f64>> {
        let kind = self.rules.catalog.index(kind)?;
        sim::strike::reach_m(&self.rules, &self.arsenal, kind).map(|(near, far)| vec![near, far])
    }
}
