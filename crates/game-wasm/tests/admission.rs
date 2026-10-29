//! External rules must be refused before a battle or world can be built.
use contract::bundle::{BundleMapIdentity, ScenarioBundle, FORMAT_VERSION};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};

fn scenario() -> ScenarioDefinition {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let mut scenario: ScenarioDefinition =
        serde_json::from_value(fixture["scenario"].clone()).unwrap();
    scenario.rules = serde_json::from_value(sim::fixtures::test_game()).unwrap();
    scenario
}

#[test]
fn uploaded_weapon_flight_is_refused_before_preparation() {
    let mut scenario = scenario();
    scenario.skirmish = None;
    scenario
        .rules
        .weapons
        .get_mut("rifle")
        .unwrap()
        .ballistics
        .lifetime_s = Some(1e6);
    let bundle: ScenarioBundle = serde_json::from_value(json!({
        "format_version": FORMAT_VERSION,
        "runtime_build": "admission-test",
        "map": BundleMapIdentity {
            map_hash: contract::identity::json_hash(&scenario.map).unwrap(),
            template_catalog_hash: None,
        },
        "rules_hash": contract::identity::json_hash(&scenario.rules).unwrap(),
        "scenario": scenario,
        "mode": "sandbox",
        "victory": {"source": "return true"},
        "authored_scripts": [],
        "assets": [],
    }))
    .unwrap();
    // Serialize directly: the upload boundary must reject independently of
    // any checks an authoring tool already made.
    let result: Value = serde_json::from_str(&game_wasm::check_scenario_bundle(
        &serde_json::to_string(&bundle).unwrap(),
    ))
    .unwrap();
    assert_eq!(result["status"], "error", "{result}");
    assert_eq!(
        result["diagnostics"][0]["location"],
        "$.scenario.rules.weapons.rifle.lifetime_s"
    );
}

#[test]
fn rules_need_a_crash_row_and_missiles_that_can_reach_their_range() {
    let mut rules = scenario().rules;
    rules.weapons.remove("helicopter_crash");
    let error = sim::admit(&rules).unwrap_err();
    assert_eq!(error.path, "weapons.helicopter_crash");

    let mut rules = scenario().rules;
    let row = &mut rules.weapons.get_mut("atgm").unwrap().ballistics;
    row.speed_mps = 100.0;
    row.accel_mps2 = Some(50.0);
    row.top_speed_mps = Some(200.0);
    row.lifetime_s = Some(1.0);
    row.range_m = 300.0;
    for fire in rules.weapons.get_mut("atgm").unwrap().targets.values_mut() {
        fire.range_m = 300.0;
    }
    let error = sim::admit(&rules).unwrap_err();
    assert_eq!(error.path, "weapons.atgm.lifetime_s");
    rules.weapons.get_mut("atgm").unwrap().ballistics.lifetime_s = Some(3.0);
    sim::admit(&rules).expect("the motor carries this missile past its range");
}

#[test]
fn fixtures_refuse_flight_and_movement_before_resolving_a_catalog() {
    for (section, patch, path) in [
        (
            "weapons",
            json!({"test_bad_flight": {"extends": "rifle", "name": "Bad Flight", "lifetime_s": 1e6}}),
            "weapons.test_bad_flight.lifetime_s",
        ),
        (
            "infantry_movement",
            json!({"wander_period_s": 0}),
            "infantry_movement.wander_period_s",
        ),
    ] {
        let mut fixture = sim::fixtures::test_game();
        contract::catalog::merge(&mut fixture[section], &patch);
        let docs = fixture["catalog"].as_array().unwrap().clone();
        let error = sim::fixtures::admit(fixture, docs).unwrap_err();
        assert!(error.contains(path), "{error}");
    }
}

#[test]
fn ground_only_rules_do_not_require_an_aircraft_crash_row() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let rules = serde_json::from_value(fixture["scenario"]["rules"].clone()).unwrap();
    sim::admit(&rules).expect("a ground-only catalog has nothing that can crash");
}
