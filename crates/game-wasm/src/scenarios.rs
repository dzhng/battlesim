//! The battles the worker prepares on a resolved map: a saved encounter's, an
//! authored scenario's, the synthetic endurance field's and the city stress
//! recipe's, each made as the native readers make it.
use contract::ids::Side;
use contract::map::MapDefinition;
use contract::scenario::Rules;
use wasm_bindgen::prelude::*;

use crate::js_error;

/// The scenario a saved encounter (`encounter_json`, its file's text) makes on
/// its resolved map under `rules_json`, its overrides merged in, as
/// `EncounterDefinition::on` makes it natively; or the refusal naming the
/// encounter's field at fault.
#[wasm_bindgen]
pub fn encounter_scenario(
    map_json: &str,
    encounter_json: &str,
    rules_json: &str,
) -> Result<String, JsError> {
    let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let encounter: contract::scenario::EncounterDefinition =
        serde_json::from_str(encounter_json).map_err(js_error)?;
    let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
    let setup = encounter.on(map, rules).map_err(js_error)?;
    sim::admit(&setup.rules).map_err(js_error)?;
    serde_json::to_string(&setup).map_err(js_error)
}

/// The battle an authored scenario (`document_json`, a
/// `ScenarioDocument`) makes on its resolved `scenario` map under
/// `rules_json` and the shipped match rules, as `ScenarioDocument::on`
/// makes it natively: `{"battle_seed": u64, "scenario": ScenarioDefinition}`,
/// the scenario's text exactly as a battle takes it; or the refusal naming
/// the document's field at fault.
#[wasm_bindgen]
pub fn authored_scenario(
    map_json: &str,
    document_json: &str,
    rules_json: &str,
) -> Result<String, JsError> {
    let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let document: contract::scenario_document::ScenarioDocument =
        serde_json::from_str(document_json).map_err(js_error)?;
    let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
    let battle_seed = document.battle_seed.value();
    let setup = document
        .on(map, rules, &sim::skirmish::starter_rules())
        .map_err(js_error)?;
    sim::admit(&setup.rules).map_err(js_error)?;
    let scenario = serde_json::to_string(&setup).map_err(js_error)?;
    Ok(format!(
        r#"{{"battle_seed":{battle_seed},"scenario":{scenario}}}"#
    ))
}

/// The synthetic endurance battle for `seed` on its resolved field
/// (`map_json`, the catalogue's `endurance`), with the late state's remains
/// when `late`; rules from the one fixture.
#[wasm_bindgen]
pub fn endurance_scenario(
    map_json: &str,
    fixture_json: &str,
    seed: u64,
    late: bool,
) -> Result<String, JsError> {
    let field: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::endurance::scenario(&field, &fixture, seed, late).map_err(js_error)?;
    serde_json::to_string(&setup).map_err(js_error)
}

/// The full generated world with the simulation-owned central contact stress recipe.
#[wasm_bindgen]
pub fn city_stress_preparation(
    map_json: &str,
    fixture_json: &str,
    seed: u64,
    late: bool,
) -> Result<String, JsError> {
    let field: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::endurance::city_scenario(&field, &fixture, seed, late).map_err(js_error)?;
    let first = setup
        .units
        .iter()
        .find(|u| u.side == Side::Blue)
        .ok_or_else(|| js_error("the city stress fixture has no blue unit"))?;
    let report = serde_json::json!({
        "start": { "at": first.position, "yaw": first.yaw },
        "livingUnits": {
            "blue": setup.units.iter().filter(|u| u.side == Side::Blue && u.condition.is_none()).count(),
            "red": setup.units.iter().filter(|u| u.side == Side::Red && u.condition.is_none()).count(),
        },
    });
    // Carry canonical scenario bytes without materializing the full map again
    // in JavaScript merely to obtain its small preparation metadata.
    Ok(format!(
        "{{\"scenario\":{},\"report\":{}}}",
        serde_json::to_string(&setup).map_err(js_error)?,
        serde_json::to_string(&report).map_err(js_error)?
    ))
}
