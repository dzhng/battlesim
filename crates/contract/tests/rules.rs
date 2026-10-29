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
