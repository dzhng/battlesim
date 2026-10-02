use serde_json::{json, Value};

#[test]
fn weapon_values_reject_negative_damage_with_a_named_error() {
    let mut rows: Value =
        serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
    rows["weapons"]["rifle"]["damage"] = json!(-1);
    let error = contract::weapons::resolve_weapons(rows["weapons"].clone())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.rifle") && error.contains("damage"),
        "{error}"
    );
}

#[test]
fn weapon_values_reject_invalid_scalar_settings() {
    for (id, field, value) in [
        ("rifle", "penetration", -1.0),
        ("rifle", "aim_s", -1.0),
        ("rifle", "reload_s", -1.0),
        ("rifle", "blast_radius_m", -1.0),
        ("rifle", "structural_damage", -1.0),
        ("rifle", "near_miss_suppression", -1.0),
        ("rifle", "armor_fraction", 1.1),
        ("rifle", "suppression_radius_m", -1.0),
        ("atgm", "turn_deg_s", 0.0),
        ("grenade", "min_range_m", -1.0),
    ] {
        let mut game: Value =
            serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
        game["weapons"][id][field] = json!(value);
        let result = contract::weapons::resolve_weapons(game["weapons"].clone());
        assert!(result.is_err(), "accepted weapons.{id}.{field} = {value}");
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains(&format!("weapons.{id}")) && error.contains(field),
            "{error}"
        );
    }
}

#[test]
fn minimum_range_cannot_exceed_maximum_range() {
    let mut game: Value =
        serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
    game["weapons"]["grenade"]["range_m"] = json!(10.0);
    game["weapons"]["grenade"]["min_range_m"] = json!(11.0);
    let error = contract::weapons::resolve_weapons(game["weapons"].clone())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.grenade") && error.contains("min_range_m"),
        "{error}"
    );
}
