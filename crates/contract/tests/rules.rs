//! Whole-rules admission: what loading the game's rules refuses.
use contract::scenario::Rules;
use serde_json::{json, Value};

/// The game's rules with an empty catalog: nothing here reads a unit.
fn game() -> Value {
    let mut game: Value =
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    game["catalog"] = json!([]);
    game
}

#[test]
fn rules_without_a_bail_out_row_for_every_host_are_refused() {
    serde_json::from_value::<Rules>(game()).expect("the full rules load");
    for host in ["building", "ground", "air"] {
        let mut game = game();
        game["bail_out"].as_object_mut().unwrap().remove(host);
        let error = serde_json::from_value::<Rules>(game)
            .expect_err(&format!("rules without a {host} bail-out loaded"))
            .to_string();
        assert!(
            error.contains("bail_out") && error.contains(host),
            "{error}"
        );
    }
}

#[test]
fn application_settings_cannot_enter_battle_rules() {
    for (key, value) in [
        ("presentation", json!({})),
        (
            "controls",
            json!({"reverse_zone_length_m": 10, "reverse_zone_margin_m": 1}),
        ),
        ("seed", json!(1)),
    ] {
        let mut rules = game();
        rules[key] = value;
        let error = serde_json::from_value::<Rules>(rules)
            .unwrap_err()
            .to_string();
        assert!(error.contains(key), "{error}");
    }
}

#[test]
fn zero_wander_period_is_refused_before_soldiers_can_move() {
    let mut rules = game();
    rules["infantry_movement"]["wander_period_s"] = json!(0);
    let error = serde_json::from_value::<Rules>(rules)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("infantry_movement.wander_period_s"),
        "{error}"
    );
}

#[test]
fn unsafe_section_ranges_are_refused_by_the_rules_reader() {
    for (pointer, value, path) in [
        ("/cover/tiers/light", json!(0.5), "cover.tiers.light"),
        ("/ground/cell_m", json!(0), "ground.cell_m"),
        (
            "/movement/drive/acceleration_s",
            json!(0),
            "movement.drive.acceleration_s",
        ),
        ("/ricochet/speed_kept", json!(0), "ricochet.speed_kept"),
        (
            "/guided/release_coast_s",
            json!(0),
            "guided.release_coast_s",
        ),
        (
            "/physics/vehicle_aim_height_fraction",
            json!(2),
            "physics.vehicle_aim_height_fraction",
        ),
        (
            "/bail_out/ground/hp_lost",
            json!([-1, 0]),
            "bail_out.ground.hp_lost",
        ),
        ("/bail_out/air/death", json!(2), "bail_out.air.death"),
        ("/garrison/enter_exit_s", json!(-1), "garrison.enter_exit_s"),
        ("/pushing/relearn_m", json!(-1), "pushing.relearn_m"),
        ("/launch/clearance_m", json!(-1), "launch.clearance_m"),
        (
            "/infantry_movement/pace_variation",
            json!(1.1),
            "infantry_movement.pace_variation",
        ),
        (
            "/movement/drive/turn_speed_factor",
            json!(1.1),
            "movement.drive.turn_speed_factor",
        ),
    ] {
        let mut rules = game();
        *rules.pointer_mut(pointer).unwrap() = value;
        let error = serde_json::from_value::<Rules>(rules)
            .expect_err(path)
            .to_string();
        assert!(error.contains(path), "{path}: {error}");
    }
}

#[test]
fn a_derived_weapon_range_survives_resolution_and_encounter_overrides() {
    let mut raw = game();
    raw["weapons"] = json!({ "test_gun": {
        "name": "Test Gun", "description": "A test row.", "icon": "rifle",
        "stationary": false, "speed_mps": 800,
        "aim_s": 1, "reload_s": 2, "ammo": "unlimited",
        "penetration": 5, "partial_from": 0.6, "damage": 10,
        "near_miss_suppression": 0, "suppression_radius_m": 0, "blast_radius_m": 0,
        "targets": {
            "ground": { "classes": ["vehicle"], "range_m": 300, "scatter_mrad": 5 },
            "low_air": { "classes": ["air_rotor"], "range_m": 150, "scatter_mrad": 8 }
        }
    }});
    let rules: Rules = serde_json::from_value(raw).expect("layers supply the reach");
    assert_eq!(rules.weapons["test_gun"].ballistics.range_m, 300.0);
    let resolved = serde_json::to_value(&rules).unwrap();
    assert_eq!(resolved["weapons"]["test_gun"]["range_m"], 300.0);
    let round_trip: Rules = serde_json::from_value(resolved.clone()).unwrap();
    assert_eq!(serde_json::to_value(round_trip).unwrap(), resolved);
    let overrides = json!({ "weapons": { "test_gun": { "targets": {
        "low_air": { "range_m": 400 }
    }}}});
    let changed = rules.overridden(overrides.as_object().unwrap()).unwrap();
    assert_eq!(changed.weapons["test_gun"].ballistics.range_m, 400.0);
    let again: Rules = serde_json::from_value(serde_json::to_value(&changed).unwrap()).unwrap();
    assert_eq!(again.weapons["test_gun"].ballistics.range_m, 400.0);
}

#[test]
fn a_building_cannot_admit_more_than_the_supported_soldier_capacity() {
    let mut raw = game();
    raw["buildings"]["capacity_soldiers"] = json!(32);
    serde_json::from_value::<Rules>(raw.clone()).expect("the supported capacity loads");
    raw["buildings"]["capacity_soldiers"] = json!(33);
    let error = serde_json::from_value::<Rules>(raw)
        .expect_err("a building above the supported capacity is refused")
        .to_string();
    assert!(error.contains("buildings.capacity_soldiers"), "{error}");
}
