//! Read-only battle rules for browser presentation, admitted by simulation.
use crate::js_error;
use contract::catalog::Mobility;
use contract::scenario::{AirRules, Rules};
use wasm_bindgen::prelude::*;

/// Parse, resolve and admit the rules before a client reads their values.
#[wasm_bindgen]
pub fn resolve_rules(json: &str) -> Result<String, JsError> {
    let rules: Rules = serde_json::from_str(json).map_err(js_error)?;
    sim::admit(&rules).map_err(js_error)?;
    serde_json::to_string(&rules).map_err(js_error)
}

/// Resolved weapon rows (`resolve_rules`' `weapons`) as the catalog view
/// publishes them, each with the facts its contract derives under `derived`
/// (`contract::weapons::published`). A reader's rows, never rules: the
/// simulation refuses `derived` as an authored field.
#[wasm_bindgen]
pub fn publish_weapons(weapons_json: &str) -> Result<String, JsError> {
    let mut rows = serde_json::Deserializer::from_str(weapons_json);
    let weapons = contract::weapons::resolve_weapons(&mut rows).map_err(js_error)?;
    Ok(contract::weapons::published(&weapons).to_string())
}

/// A preview stands at the same authored cruise height as the simulation.
#[wasm_bindgen]
pub fn cruise_agl_m(air: &str, mobility: &str) -> Result<f64, JsError> {
    let air: AirRules = serde_json::from_str(air).map_err(js_error)?;
    let mobility: Mobility = serde_json::from_str(mobility).map_err(js_error)?;
    Ok(air.cruise_agl_m(&mobility))
}

/// A unit catalog's documents (`fixtures/units/**`, or a test's own)
/// resolved by the simulation's one resolver: its view
/// (`contract::catalog::Catalog::view`), or the named error it fails with.
#[wasm_bindgen]
pub fn resolve_catalog(documents_json: &str) -> Result<String, JsError> {
    // Parsed by the catalog's own reader, which refuses a repeated key.
    let documents = contract::catalog::parse_document(documents_json).map_err(js_error)?;
    let documents = documents
        .as_array()
        .ok_or_else(|| js_error("the catalog is a list of documents"))?;
    let catalog = contract::catalog::resolve(documents).map_err(js_error)?;
    Ok(catalog.view().to_string())
}
