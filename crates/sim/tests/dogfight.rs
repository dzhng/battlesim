use contract::ids::UnitId;
use contract::observation::ObservedTargetId;
use serde_json::json;
use sim::air::dogfight::{breaks, intent};
use sim::air::{FixedWingState, Intent};
use sim::knowledge::Track;
use sim::math::{v2, v3};

fn track(position: [f64; 3], velocity: [f64; 2], yaw: f64) -> Track {
    Track {
        id: ObservedTargetId(1),
        last_seen: 0,
        position: v3(position[0], position[1], position[2]),
        yaw,
        velocity: v2(velocity[0], velocity[1]),
        members: Vec::new(),
    }
}

fn own(position: [f64; 3], heading: f64) -> FixedWingState {
    FixedWingState {
        pos: v3(position[0], position[1], position[2]),
        heading,
        speed_mps: 200.0,
        burner: 0.0,
        turn_sign: 1,
    }
}

fn air() -> contract::scenario::FixedWingFlight {
    serde_json::from_value(sim::fixtures::test_game()["air"]["fixed_wing"].clone()).unwrap()
}

fn air_battle(units: serde_json::Value) -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({"mounts": [{
            "id": "air missiles", "name": "Air Missiles", "weapons": ["air_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }], "body": {"hull": {"hp": 10000}}}),
    );
    let setup = serde_json::from_value(json!({
        "map": {"size": [2400, 1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": rules, "units": units, "events": [], "scripts": []
    }))
    .unwrap();
    sim::battle::Battle::new(&setup, 1)
}

#[test]
fn firing_makes_it_your_target_and_it_adopts_you_only_if_free() {
    let mut battle = air_battle(json!([
        {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
        {"side":"red", "kind":"test_jet", "position":[900,500], "yaw":std::f64::consts::PI, "engagement":"return_fire_only"}
    ]));
    for _ in 0..120 {
        battle.step();
    }
    assert_eq!(
        battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(1))
    );
    assert_eq!(
        battle.unit(UnitId(1)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(0))
    );
}

#[test]
fn a_third_jet_joins_by_targeting_an_engaged_one() {
    let mut battle = air_battle(json!([
        {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
        {"side":"red", "kind":"test_jet", "position":[900,500], "yaw":std::f64::consts::PI, "engagement":"return_fire_only"},
        {"side":"red", "kind":"test_jet", "position":[1100,500], "yaw":std::f64::consts::PI, "engagement":"fire_at_will"}
    ]));
    for _ in 0..180 {
        battle.step();
    }
    assert_eq!(
        battle.unit(UnitId(2)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(0))
    );
}

#[test]
fn the_link_breaks_past_the_rule_distance_in_3d() {
    let rules = air();
    assert!(!breaks(rules.dogfight_break_m, &rules));
    assert!(breaks(
        (rules.dogfight_break_m.powi(2) + 1.0).sqrt(),
        &rules
    ));
}

#[test]
fn a_jet_never_dogfights_a_helicopter() {
    let mut battle = air_battle(json!([
        {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
        {"side":"red", "kind":"test_heli", "position":[900,500], "yaw":std::f64::consts::PI, "engagement":"return_fire_only"}
    ]));
    for _ in 0..120 {
        battle.step();
    }
    assert_eq!(
        battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target,
        None
    );
    assert_eq!(
        battle.unit(UnitId(1)).unwrap().air.unwrap().dogfight_target,
        None
    );
}

#[test]
fn on_a_head_on_merge_both_break_away() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [-200.0, 0.0], std::f64::consts::PI);
    let Intent::Pursue { point, .. } = intent(&own, &target, 7, &air()) else {
        panic!("pursuit")
    };
    assert!(
        point.x < own.pos.x,
        "merge point should be behind own jet: {point:?}"
    );
}

#[test]
fn pursuit_aims_where_the_target_was() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [50.0, 0.0], 0.0);
    let Intent::Pursue { point, vel, lag_s } = intent(&own, &target, 7, &air()) else {
        panic!("pursuit")
    };
    assert_eq!(point.x, target.position.x);
    assert_eq!(vel.x, target.velocity.x);
    assert!(lag_s > 0.0);
}

#[test]
fn dogfight_height_stays_in_its_band() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [0.0, 0.0], 0.0);
    let Intent::Pursue { point, .. } = intent(&own, &target, 7, &air()) else {
        panic!("pursuit")
    };
    assert!((air().dogfight_band_agl_m[0]..=air().dogfight_band_agl_m[1]).contains(&point.z));
}
