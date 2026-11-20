//! The generator's hills hide what stands behind them: the lowest rise the
//! presets draw (`fixtures/map-presets.json` `hills`: the narrowest at the
//! gentlest grade) between a scout and a squad keeps the squad out of
//! sight, where the same ground without it shows the squad.
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

const BLUE: [f64; 2] = [380.0, 300.0];
const RED: [f64; 2] = [820.0, 300.0];

fn battle(relief: Value) -> Battle {
    let map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "forests": [], "relief": relief })
    .to_string();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": common::physical_map(&map),
        "rules": common::game(),
        "units": [
            { "side": "blue", "kind": "test_recon", "position": BLUE },
            { "side": "red", "kind": "test_rifle", "position": RED, "engagement": "return_fire_only" },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Whether blue has identified red after a few seconds.
fn seen(relief: Value) -> bool {
    let mut b = battle(relief);
    for _ in 0..common::tick_hz() * 3 {
        b.step();
    }
    !b.observe(Side::Blue).identified.is_empty()
}

#[test]
fn the_lowest_generated_hill_hides_a_squad_behind_it() {
    let presets: Value =
        serde_json::from_str(include_str!("../../../fixtures/map-presets.json")).unwrap();
    let hills = &presets["hills"];
    let radius = hills["radius_m"][0].as_f64().unwrap();
    let peak = radius * hills["peak_grade"][0].as_f64().unwrap();
    let middle = [(BLUE[0] + RED[0]) / 2.0, BLUE[1]];
    assert!(seen(json!([])), "on flat ground the scout sees the squad");
    assert!(
        !seen(json!([{ "kind": "ridge", "center": middle, "peak_m": peak, "radius_m": radius }])),
        "a {peak} m rise {radius} m across hides it"
    );
}
