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

/// A weapon row's `name` is what the player reads on a card and its `icon`
/// names a generated drawing: an identifier, a slug, a long description or
/// a missing icon is refused at load, naming the row.
#[test]
fn a_weapon_row_needs_a_short_label_and_an_icon_id() {
    let load = |field: &str, value: Value| {
        let mut game: Value =
            serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
        // A derived row: its own label and icon are checked, not only its parent's.
        game["weapons"]["autocannon"][field] = value;
        contract::weapons::resolve_weapons(game["weapons"].clone())
    };
    for name in [
        "",
        "MAIN_GUN",
        "ground_tank_ap",
        "heavy sniper",
        "Spike (direct guidance)",
        "Heavy sniper rifle, scoped",
        "Autocannon Autocannon",
        "A very long autocannon",
    ] {
        let error = load("name", json!(name))
            .err()
            .unwrap_or_else(|| panic!("accepted name {name:?}"))
            .to_string();
        assert!(
            error.contains("weapons.autocannon") && error.contains("name"),
            "{name:?}: {error}"
        );
    }
    for name in ["Autocannon", "AP Shell", "RPG-29", "TOW 2A", "Heavy Sniper"] {
        if let Err(error) = load("name", json!(name)) {
            panic!("refused {name:?}: {error}");
        }
    }
    for icon in ["", "Autocannon", "auto cannon", "../hmg"] {
        let error = load("icon", json!(icon))
            .err()
            .unwrap_or_else(|| panic!("accepted icon {icon:?}"))
            .to_string();
        assert!(
            error.contains("weapons.autocannon") && error.contains("icon"),
            "{icon:?}: {error}"
        );
    }
}
