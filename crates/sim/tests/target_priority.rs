//! Automatic target ranking: return-fire threat, damage share, distance and
//! observed id decide which identified enemy a mount takes.
use contract::command::TargetRef;
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;

use crate::common;
use crate::weapons::{battle, map, mount, rules, run, scenario_with};

#[test]
fn automatic_acquisition_prefers_a_damaging_threat_over_harmless_value() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"test_rifle","position":[300,260],"engagement":"return_fire_only"},
            {"side":"red","kind":"test_tank","position":[300,340],"engagement":"return_fire_only"}
        ]),
        json!([]),
        json!([]),
    );
    let mut authored = rules();
    sim::fixtures::patch_catalog(&mut authored, "units", "test_rifle", json!({"cost":1000}));
    sim::fixtures::patch_catalog(&mut authored, "units", "test_tank", json!({"cost":100}));
    setup.rules = serde_json::from_value(authored).unwrap();
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 3);
    let threat = b
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == common::unit_kind("test_tank"))
        .unwrap()
        .id;
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: threat })
    );
}

#[test]
fn threatening_targets_use_authored_price_before_distance() {
    for (tank_cost, at_cost, expected) in [(1000, 100, "test_tank"), (100, 1000, "test_at")] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                {"side":"blue","kind":"test_tank","position":[100,300]},
                {"side":"red","kind":"test_tank","position":[400,260],"engagement":"return_fire_only"},
                {"side":"red","kind":"test_at","position":[300,340],"engagement":"return_fire_only"}
            ]),
            json!([]),
            json!([]),
        );
        let mut authored = rules();
        sim::fixtures::patch_catalog(
            &mut authored,
            "units",
            "test_tank",
            json!({"cost":tank_cost}),
        );
        sim::fixtures::patch_catalog(&mut authored, "units", "test_at", json!({"cost":at_cost}));
        setup.rules = serde_json::from_value(authored).unwrap();
        let mut b = Battle::new(&setup, 5);
        run(&mut b, 3);
        let wanted = b
            .observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind(expected))
            .unwrap()
            .id;
        assert_eq!(
            mount(&b, Side::Blue, 0, 0).target,
            Some(TargetRef::Identified { id: wanted })
        );
    }
}

#[test]
fn expensive_launcher_without_return_fire_reach_loses_threat_priority() {
    for (minimum, maximum) in [(300.0, 1800.0), (0.0, 100.0)] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                {"side":"blue","kind":"test_tank","position":[100,300]},
                {"side":"red","kind":"test_at","position":[300,260],"engagement":"return_fire_only"},
                {"side":"red","kind":"test_tank","position":[300,340],"engagement":"return_fire_only"}
            ]),
            json!([]),
            json!([]),
        );
        let mut authored = rules();
        sim::fixtures::patch_catalog(&mut authored, "units", "test_tank", json!({"cost":100}));
        sim::fixtures::patch_catalog(&mut authored, "units", "test_at", json!({"cost":1000}));
        authored["weapons"]["atgm"]["min_range_m"] = json!(minimum);
        common::json_ground_reach(&mut authored["weapons"]["atgm"], maximum);
        setup.rules = serde_json::from_value(authored).unwrap();
        let mut b = Battle::new(&setup, 5);
        run(&mut b, 3);
        let wanted = b
            .observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind("test_tank"))
            .unwrap()
            .id;
        assert_eq!(
            mount(&b, Side::Blue, 0, 0).target,
            Some(TargetRef::Identified { id: wanted })
        );
    }
}

#[test]
fn equal_threat_and_price_use_distance_then_stable_observed_id() {
    for (second, nearest) in [(json!([250, 340]), true), (json!([300, 340]), false)] {
        let mut b = battle(
            json!([]),
            json!([
                {"side":"blue","kind":"test_tank","position":[100,300]},
                {"side":"red","kind":"test_tank","position":[300,260],"engagement":"return_fire_only"},
                {"side":"red","kind":"test_tank","position":second,"engagement":"return_fire_only"}
            ]),
            json!([]),
            json!([]),
        );
        run(&mut b, 3);
        let mut observed = b.observe(Side::Blue).identified.clone();
        observed.sort_by_key(|u| u.id);
        let wanted = observed[usize::from(nearest)].id;
        assert_eq!(
            mount(&b, Side::Blue, 0, 0).target,
            Some(TargetRef::Identified { id: wanted })
        );
    }
}

#[test]
fn each_weapon_takes_the_costliest_target_it_can_damage() {
    // Blue tank faces a red tank and a red rifle squad.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [300, 260] },
            { "side": "red", "kind": "test_rifle", "position": [300, 340] },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 5);
    let ids: Vec<_> = b
        .observe(Side::Blue)
        .identified
        .iter()
        .map(|e| (e.id, e.kind))
        .collect();
    let tank_id = ids
        .iter()
        .find(|(_, k)| *k == common::unit_kind("test_tank"))
        .unwrap()
        .0;
    let rifle_id = ids
        .iter()
        .find(|(_, k)| *k == common::unit_kind("test_rifle"))
        .unwrap()
        .0;
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: tank_id }),
        "cannon on the tank"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).target,
        Some(TargetRef::Identified { id: rifle_id }),
        "HMG cannot hurt the tank"
    );
}

#[test]
fn a_gun_takes_the_target_it_hurts_most_before_the_costlier_one() {
    // Both targets threaten it, but the gunner rakes the jeep: its rounds
    // get fully through the jeep and only partly through the tank's side.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_gun_jeep", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [350, 250], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_jeep", "position": [350, 350], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 5);
    let jeep = b
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == common::unit_kind("test_jeep"))
        .expect("the jeep is not identified")
        .id;
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: jeep })
    );
}
