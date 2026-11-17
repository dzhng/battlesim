//! Engagement eligibility from ammunition, damage and target classes.
use super::*;

#[test]
fn the_default_gun_fires_at_what_it_cannot_hurt_but_specialists_do_not() {
    let map: Value = serde_json::from_str(&map(json!([]))).unwrap();
    let mut rules = rules();
    // A grenade whose damage cuts off short of the tank's armour.
    rules["weapons"]["grenade"]["partial_from"] = json!(0.6);
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [300, 300] },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 5);
    let shots = run(&mut b, 120);
    assert!(
        !shots_by(&shots, 0, "rifle").is_empty(),
        "rifles keep firing (W09)"
    );
    assert!(
        shots_by(&shots, 0, "grenade").is_empty(),
        "the grenade cannot hurt a tank"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).reason,
        ActionReason::NoCompatibleTarget
    );
}

#[test]
fn a_default_gun_with_a_finite_supply_keeps_it_for_what_it_can_hurt() {
    // The same squad and tank as above, but the rifles carry a counted supply.
    let mut rules = rules();
    rules["weapons"]["test_counted_rifle"] = json!({ "extends": "rifle", "ammo": 200 });
    rules["weapons"]["test_counted_rifle"]["round_cost"] = json!(1);
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "mounts": [{ "id": "rifles", "weapons": ["test_counted_rifle"] }] }),
    );
    let map: Value = serde_json::from_str(&map(json!([]))).unwrap();
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [300, 300] },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 5);
    let shots = run(&mut b, 120);
    assert!(
        shots_by(&shots, 0, "test_counted_rifle").is_empty(),
        "no counted round spent on a tank it cannot hurt"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).reason,
        ActionReason::NoCompatibleTarget
    );
}

/// A weapon is fired only at the target classes its layer lists: the same
/// launcher, listing `vehicle` alone, holds its missiles while a rifle squad
/// stands in the open within reach; listing `infantry` too, it fires at it.
#[test]
fn a_weapon_fires_only_at_the_target_classes_its_layer_lists() {
    let launches = |classes: Value| {
        let mut rules = rules();
        rules["weapons"]["atgm"]["targets"]["ground"]["classes"] = classes;
        let map: Value = serde_json::from_str(&map(json!([]))).unwrap();
        let setup = serde_json::from_value(json!({
            "map": map,
            "rules": rules,
            "units": [
                { "side": "blue", "kind": "test_at", "position": [100, 300] },
                { "side": "red", "kind": "test_rifle", "position": [300, 300],
                  "engagement": "return_fire_only" },
            ],
            "events": [],
            "scripts": [],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 5);
        shots_by(&run(&mut b, ticks(20.0)), 0, "atgm").len()
    };
    assert_eq!(
        launches(json!(["vehicle"])),
        0,
        "a vehicle-only row spares soldiers"
    );
    assert!(
        launches(json!(["infantry", "vehicle"])) > 0,
        "a row listing infantry fires at them"
    );
}
