//! Aircraft: a hull in the air is no ground body. Traffic drives under it,
//! soldiers neither step out of it nor lean on it, forest ground does not hide
//! it, and it is admitted against its own hull limits.

use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

/// A test helicopter: a utility airframe with a turreted HMG.
fn heli() -> Value {
    json!({
        "units": {
            "test_heli": {
                "name": "Helicopter",
                "description": "Test only: a utility helicopter with a door HMG.",
                "faction": "generic",
                "family": "aircraft",
                "roles": ["light_vehicle"],
                "cost": 180,
                "body": {
                    "hull": {
                        "half_extents_m": [7.0, 1.2, 1.6],
                        "eye_m": 2.0,
                        "hp": 200,
                        "armor": {
                            "front": 15, "side": 15, "rear": 15, "roof": 15,
                            "ricochet": { "front": 0.1, "side": 0.1, "rear": 0.1, "roof": 0.1 }
                        },
                        "weight_class": "light",
                        "push_class": "none",
                        "wreck": "light_wreck"
                    }
                },
                "mobility": { "air": { "cruise_kmh": 220, "turn_deg_s": 90, "climb_mps": 6 } },
                "sensors": { "ground_m": 700, "sight_shape": { "front": 1.0, "side": 1.0, "rear": 1.0 } },
                "mounts": [
                    { "id": "HMG", "name": "HMG", "weapons": ["hmg"], "turret": true,
                      "pivot_m": [2.0, 0, 0.4], "muzzle_m": [1.0, 0, 0] }
                ],
                "sound": { "profile": "vehicle", "loudness_m": 900 },
                "appearance": "test_jeep"
            }
        }
    })
}

pub(crate) fn fixture() -> Value {
    let mut fixture = sim::fixtures::test_game();
    fixture["catalog"].as_array_mut().unwrap().push(heli());
    fixture
}

pub(crate) fn battle_with(fixture: &Value, map: Value, units: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": map, "rules": fixture, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

pub(crate) fn open_map() -> Value {
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

pub(crate) fn order(units: &[u32], goal: [f64; 2]) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    }
}

#[test]
fn a_helicopter_enters_the_battle_at_cruise_height() {
    let fixture = fixture();
    let b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let heli = b.unit(UnitId(0)).unwrap();
    assert!(heli.airborne());
    assert!(heli.ground_footprint().is_none());
    let cruise = b.rules().air.cruise_agl_m;
    assert!(
        (heli.position.z - cruise).abs() < 1e-9,
        "z {}",
        heli.position.z
    );
}

#[test]
fn a_tank_drives_under_a_hovering_helicopter() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [150, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    assert_eq!(b.accept(order(&[0], [450.0, 200.0])).error, None);
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
    }
    let tank = b.unit(UnitId(0)).unwrap().position.xy();
    let short = (tank - sim::math::v2(450.0, 200.0)).length();
    assert!(
        short < 5.0,
        "the tank waited for the helicopter: {short:.1} m short"
    );
}

#[test]
fn soldiers_are_not_shoved_from_under_it() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [300, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    let before: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
    }
    let after: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for (a, z) in before.iter().zip(&after) {
        assert!(
            (a.xy() - z.xy()).length() < 1e-9,
            "a soldier was shoved from {a:?} to {z:?}"
        );
    }
}

#[test]
fn a_helicopter_over_forest_is_not_concealed() {
    let fixture = fixture();
    let forest_map = json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "forests": [{ "shape": { "kind": "polygon",
            "ring": [[250, 150], [350, 150], [350, 250], [250, 250]] } }] });
    let b = battle_with(
        &fixture,
        forest_map,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
            { "side": "blue", "kind": "test_tank", "position": [300, 180] },
        ]),
    );
    let rules = b.rules();
    assert!(!sim::sensing::concealed(
        b.world(),
        b.unit(UnitId(0)).unwrap(),
        rules
    ));
    assert!(
        sim::sensing::concealed(b.world(), b.unit(UnitId(1)).unwrap(), rules),
        "the same forest hides a tank on its ground"
    );
}

#[test]
fn an_air_hull_past_its_limits_is_refused() {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "body": { "hull": { "half_extents_m": [9.0, 1.2, 1.6] } } }),
    );
    let error = serde_json::from_value::<Rules>(fixture).expect_err("past the air limit");
    assert!(error.to_string().contains("air half length"), "{error}");
}

#[test]
fn an_aircraft_cruising_past_its_limit_is_refused() {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "mobility": { "air": { "cruise_kmh": 400 } } }),
    );
    let error = serde_json::from_value::<Rules>(fixture).expect_err("past the air speed");
    assert!(error.to_string().contains("cruise_kmh"), "{error}");
}

#[test]
fn a_battle_with_a_helicopter_replays_to_the_same_digest() {
    let fixture = fixture();
    let run = || {
        let mut b = battle_with(
            &fixture,
            open_map(),
            json!([
                { "side": "blue", "kind": "test_heli", "position": [200, 200] },
                { "side": "red", "kind": "test_rifle", "position": [400, 200] },
            ]),
        );
        for _ in 0..20 * b.rules().tick_hz {
            b.step();
        }
        b.digest()
    };
    assert_eq!(run(), run());
}
