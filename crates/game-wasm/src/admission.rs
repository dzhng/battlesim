//! Upload diagnostics after the contract has checked canonical content and maps.
use crate::js_error;
use contract::bundle::{BundleDiagnostic, BundleError, ScenarioBundle};
use wasm_bindgen::prelude::*;

pub(crate) fn admit_bundle(json: &str) -> Result<ScenarioBundle, BundleError> {
    let bundle = ScenarioBundle::from_json(json)?;
    sim::admit(&bundle.scenario.rules).map_err(|error| {
        BundleError::Invalid(vec![BundleDiagnostic {
            code: "invalid_rules",
            location: format!("$.scenario.rules.{}", error.path),
            message: error.to_string(),
        }])
    })?;
    Ok(bundle)
}

/// Validate a downloaded ScenarioBundle without consulting repository
/// fixtures. The bundle module owns canonical bytes, identities, topology,
/// and embedded asset admission.
#[wasm_bindgen]
pub fn check_scenario_bundle(bundle_json: &str) -> String {
    match admit_bundle(bundle_json) {
        Ok(bundle) => serde_json::json!({
            "status": "ok",
            "identity": bundle.identity().expect("validated bundle identity"),
        })
        .to_string(),
        Err(contract::bundle::BundleError::Invalid(diagnostics)) => {
            serde_json::json!({ "status": "error", "diagnostics": diagnostics }).to_string()
        }
    }
}

/// Canonicalize a builder draft after deriving its map and rules identities.
/// This is intentionally separate from upload admission: saved files must
/// already carry the identities they claim.
#[wasm_bindgen]
pub fn author_scenario_bundle(bundle_json: &str) -> Result<String, JsError> {
    let bundle =
        contract::bundle::ScenarioBundle::from_authoring_json(bundle_json).map_err(js_error)?;
    sim::admit(&bundle.scenario.rules).map_err(js_error)?;
    bundle.to_json().map_err(js_error)
}

/// Return the canonical bytes of a validated ScenarioBundle.
#[wasm_bindgen]
pub fn canonical_scenario_bundle(bundle_json: &str) -> Result<String, JsError> {
    let bundle = admit_bundle(bundle_json).map_err(|error| js_error(error.to_string()))?;
    bundle
        .to_json()
        .map_err(|error| js_error(error.to_string()))
}
