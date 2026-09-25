//! Scenario building shared by the authority tests: rules always come from
//! the one fixture owner, maps from the lab fixtures.
#![allow(dead_code)]
use contract::scenario::ScenarioDefinition;

pub const GEOMETRY_LAB: &str = include_str!("../../../../fixtures/geometry-lab.json");
pub const MOVEMENT_LAB: &str = include_str!("../../../../fixtures/movement-lab.json");
const VILLAGE: &str = include_str!("../../../../fixtures/village.json");

pub fn village() -> serde_json::Value {
    serde_json::from_str(VILLAGE).unwrap()
}

/// The runtime rules section, picked from the village fixture.
pub fn rules() -> serde_json::Value {
    let v = village();
    serde_json::json!({
        "tick_hz": v["tick_hz"],
        "movement": v["movement"],
        "physics": v["physics"],
        "health": v["health"],
        "sensors": v["sensors"],
        "cost_priority": v["cost_priority"],
    })
}

pub fn scenario(
    map: &str,
    units: serde_json::Value,
    events: serde_json::Value,
) -> ScenarioDefinition {
    scenario_with(map, units, events, serde_json::json!([]))
}

pub fn scenario_with(
    map: &str,
    units: serde_json::Value,
    events: serde_json::Value,
    scripts: serde_json::Value,
) -> ScenarioDefinition {
    let map: serde_json::Value = serde_json::from_str(map).unwrap();
    serde_json::from_value(serde_json::json!({
        "map": map, "rules": rules(), "units": units, "events": events, "scripts": scripts,
    }))
    .unwrap()
}
