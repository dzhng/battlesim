//! Automatic target ranking: return-fire threat, damage share, distance and
//! observed id decide which identified enemy a mount takes, and a mount keeps
//! it until it can no longer shoot it.
use contract::command::TargetRef;
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

use crate::common;
use crate::weapons::{battle, map, mount, rules, run, scenario_with, ticks};

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
fn threatening_targets_use_distance_after_return_fire_priority() {
    for (tank_cost, at_cost) in [(1000, 100), (100, 1000)] {
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
            .find(|e| e.kind == common::unit_kind("test_at"))
            .unwrap()
            .id;
        assert_eq!(
            mount(&b, Side::Blue, 0, 0).target,
            Some(TargetRef::Identified { id: wanted })
        );
    }
}

#[test]
fn a_launcher_without_return_fire_reach_loses_threat_priority() {
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
fn equal_threat_uses_distance_then_stable_observed_id() {
    for (second, nearest) in [(json!([250, 340]), true), (json!([300, 340]), false)] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                {"side":"blue","kind":"test_tank","position":[100,300]},
                {"side":"red","kind":"test_tank","position":[300,260],"engagement":"return_fire_only"},
                {"side":"red","kind":"test_tank","position":second,"engagement":"return_fire_only"}
            ]),
            json!([]),
            json!([]),
        );
        // Without the seeded spread, an exact tie falls to the observed id.
        setup.rules.targeting.distance_spread = 0.0;
        let mut b = Battle::new(&setup, 5);
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
fn each_weapon_takes_a_target_it_can_damage() {
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

#[test]
fn a_loaded_weapon_drops_a_target_it_can_no_longer_reach() {
    // The nearer red squad walks out of rifle range but stays identified by
    // blue's scout; the rifles take the remaining scout-class target instead
    // of holding a loaded round on the unreachable one (W07).
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [1100, 300], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "blue", "kind": "test_recon", "position": [100, 340] },
            { "side": "red", "kind": "test_rifle", "position": [200, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 590], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    // The squad must live to walk out of range: this is about which target a
    // loaded weapon holds, not how fast rifles kill or pin at long range.
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    setup.rules.targeting.distance_spread = 0.0;
    setup.rules.targeting.engaged_distance_penalty = 0.0;
    let mut b = Battle::new(&setup, 5);
    let handle = |b: &Battle, kind: &str| {
        b.observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind(kind))
            .map(|e| e.id)
    };
    run(&mut b, 5);
    let squad = handle(&b, "test_rifle").expect("the squad is identified");
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: squad })
    );
    let mut switched = false;
    for _ in 0..10000 {
        b.step();
        if mount(&b, Side::Blue, 0, 0).target
            == handle(&b, "test_recon").map(|id| TargetRef::Identified { id })
        {
            switched = handle(&b, "test_rifle") == Some(squad);
            break;
        }
    }
    assert!(
        switched,
        "replaced while the out-of-range squad was still identified"
    );
}

#[test]
fn a_weapon_keeps_its_target_while_it_can_still_shoot_it() {
    // Blue's rifles take the nearer of two red squads; it then walks back past
    // the other but stays in range and in sight. The rifles stay on it: a
    // squad that swaps targets every time the ranking shifts flickers on
    // screen and wastes its aim (W07).
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [550, 300], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "blue", "kind": "test_recon", "position": [100, 340] },
            { "side": "red", "kind": "test_rifle", "position": [300, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [340, 380], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    // Nobody dies or is pinned: this is about which target the rifles hold.
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 5);
    let first = mount(&b, Side::Blue, 0, 0).target;
    assert!(first.is_some(), "the rifles take a target");
    let mut swaps = 0;
    for _ in 0..ticks(40.0) {
        b.step();
        if mount(&b, Side::Blue, 0, 0).target != first {
            swaps += 1;
        }
    }
    let at = |id: u32| b.unit(UnitId(id)).unwrap().position.xy();
    assert!(
        (at(2) - at(0)).length() > 300.0,
        "the first target walked back past the other"
    );
    assert_eq!(swaps, 0, "the rifles never left their first target");
}
