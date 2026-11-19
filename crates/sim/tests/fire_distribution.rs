//! Functional tests for automatic target distribution.

use contract::ids::UnitId;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::weapons::Target;

use crate::common;

fn map() -> Value {
    json!({
        "size": [1200, 600],
        "fog_cell_m": 8,
        "height_grid_m": 4,
        "slope_cutoff_deg": 35
    })
}

fn rules() -> Value {
    let mut rules = common::scenario_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({"sensors": {"ground_m": 800, "low_air_m": 800, "high_air_m": 800}}),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({"body": {"hull": {"hp": 1.0e9}}}),
    );
    rules
}

fn battle(units: Value, rules: Value, seed: u64) -> Battle {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map(), "rules": rules, "units": units, "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, seed)
}

fn lock_target(battle: &Battle, unit: u32) -> Option<UnitId> {
    battle.unit(UnitId(unit)).and_then(|u| {
        u.mounts.iter().find_map(|m| match m.lock.as_ref()?.target {
            Target::Unit(id) => Some(id),
            _ => None,
        })
    })
}

fn fanout(seed: u64) -> Battle {
    battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 302]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "red", "kind": "test_supply", "position": [350, 280]},
            {"side": "red", "kind": "test_supply", "position": [350, 320]}
        ]),
        rules(),
        seed,
    )
}

#[test]
fn identical_shooters_fan_out_across_nearly_equal_targets() {
    for seed in 1..8 {
        let mut battle = fanout(seed);
        battle.step();
        let first = lock_target(&battle, 0).expect("the first shooter locks a target");
        let second = lock_target(&battle, 1).expect("the second shooter locks a target");
        assert_ne!(first, second, "seed {seed}: both shooters took one target");
    }
}

#[test]
fn an_engaged_nearer_target_loses_to_a_bit_farther_one() {
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 302]},
            {"side": "red", "kind": "test_supply", "position": [350, 280]},
            {"side": "red", "kind": "test_supply", "position": [400, 320]}
        ]),
        rules(),
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(2)));
    assert_eq!(lock_target(&battle, 1), Some(UnitId(3)));
}

#[test]
fn a_far_target_still_loses_when_distance_exceeds_penalty_and_spread() {
    let mut rules = rules();
    common::json_ground_reach(&mut rules["weapons"]["hmg"], 800.0);
    let penalty = rules["targeting"]["engaged_distance_penalty"]
        .as_f64()
        .unwrap();
    let spread = rules["targeting"]["distance_spread"].as_f64().unwrap();
    let near = 250.0;
    let far = near * (1.0 + penalty) * (1.0 + spread) / (1.0 - spread) + 1.0;
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 320]},
            {"side": "red", "kind": "test_supply", "position": [350, 300]},
            {"side": "red", "kind": "test_supply", "position": [100.0 + far, 300]}
        ]),
        rules,
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(2)));
    assert_eq!(lock_target(&battle, 1), Some(UnitId(2)));
}

#[test]
fn purchase_cost_does_not_out_rank_nearer_distance() {
    let mut rules = rules();
    rules["catalog"].as_array_mut().unwrap().push(json!({
        "units": {"expensive_target": {"extends": "test_supply", "cost": 100000}}
    }));
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "red", "kind": "test_supply", "position": [350, 300]},
            {"side": "red", "kind": "expensive_target", "position": [500, 300]}
        ]),
        rules,
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(1)));
}

#[test]
fn a_fresh_choice_is_stable_across_replays_and_ticks() {
    let units = json!([
        {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
        {"side": "red", "kind": "test_supply", "position": [350, 280]},
        {"side": "red", "kind": "test_supply", "position": [350, 320]}
    ]);
    let mut first = battle(units.clone(), rules(), 19);
    let mut second = battle(units, rules(), 19);
    let mut first_targets = Vec::new();
    let mut second_targets = Vec::new();
    for _ in 0..20 {
        first.step();
        second.step();
        first_targets.push(lock_target(&first, 0));
        second_targets.push(lock_target(&second, 0));
    }
    assert_eq!(first_targets, second_targets);
    assert!(first_targets
        .iter()
        .all(|target| *target == first_targets[0]));
}
