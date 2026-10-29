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
