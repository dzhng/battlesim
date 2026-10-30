use serde_json::{json, Value};

#[test]
fn weapon_values_reject_negative_damage_with_a_named_error() {
    let mut rows: Value =
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
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
        ("rifle", "suppression_radius_m", -1.0),
        ("atgm", "turn_deg_s", 0.0),
        ("grenade", "min_range_m", -1.0),
        ("grenade", "partial_from", 1.0),
    ] {
        let mut game: Value =
            serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
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
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    game["weapons"]["grenade"]["targets"]["ground"]["range_m"] = json!(10.0);
    game["weapons"]["grenade"]["min_range_m"] = json!(11.0);
    let error = contract::weapons::resolve_weapons(game["weapons"].clone())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.grenade") && error.contains("min_range_m"),
        "{error}"
    );
}

#[test]
fn an_unpowered_row_with_range_is_refused() {
    let mut game: Value =
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    game["weapons"]["air_bomb"]["range_m"] = json!(1000.0);
    let error = contract::weapons::resolve_weapons(game["weapons"].clone())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.air_bomb") && error.contains("range_m"),
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
            serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
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

/// Each layer authors valid reach and scatter; the flight envelope follows
/// the longest layer. Other firing values belong to the weapon.
#[test]
fn a_height_band_requires_valid_range_and_scatter() {
    for (band, named) in [
        (json!({ "range_m": -1.0 }), "targets.low_air.range_m"),
        (
            json!({ "scatter_mrad": -1.0 }),
            "targets.low_air.scatter_mrad",
        ),
        // Only range and spread change with the band: the gun, its
        // magazine and its round are the same whatever it aims at.
        (json!({ "aim_s": 1.0 }), "aim_s"),
    ] {
        let mut game: Value =
            serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
        game["weapons"]["heavy_sniper"]["targets"]["low_air"] = band.clone();
        let error = contract::weapons::resolve_weapons(game["weapons"].clone())
            .err()
            .unwrap_or_else(|| panic!("accepted targets.low_air = {band}"))
            .to_string();
        assert!(
            error.contains("weapons.heavy_sniper") && error.contains(named),
            "{error}"
        );
    }
}

#[test]
fn seeker_fov_is_typed_and_bounded() {
    let mut game: Value =
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    game["weapons"]["stinger"]["seeker"] = json!("infrared");
    game["weapons"]["stinger"]["seeker_fov_deg"] = json!(70.0);
    let rows = contract::weapons::resolve_weapons(game["weapons"].clone()).unwrap();
    assert_eq!(rows["stinger"].seeker_fov_deg, Some(70.0));

    game["weapons"]["stinger"]["seeker_fov_deg"] = json!(361.0);
    let error = contract::weapons::resolve_weapons(game["weapons"].clone())
        .unwrap_err()
        .to_string();
    assert!(error.contains("seeker_fov_deg"), "{error}");
}

#[test]
fn anti_radiation_is_distinct_from_a_radar_seeker() {
    let mut game: Value =
        serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    game["weapons"]["sead_missile"]["anti_radiation"] = json!(true);
    game["weapons"]["air_missile"]["anti_radiation"] = json!(false);
    let rows = contract::weapons::resolve_weapons(game["weapons"].clone()).unwrap();
    assert!(rows["sead_missile"].anti_radiation);
    assert!(!rows["air_missile"].anti_radiation);
    assert_eq!(
        rows["sead_missile"].seeker,
        Some(contract::weapons::SeekerSupport::Radar)
    );
}

#[test]
fn every_air_target_row_reaches_its_range_within_its_lifetime() {
    let game: Value = serde_json::from_str(include_str!("../../../fixtures/rules.json")).unwrap();
    let rows = contract::weapons::resolve_weapons(game["weapons"].clone()).unwrap();
    for (id, row) in rows {
        if row
            .targets
            .contains_key(&contract::catalog::AltitudeLayer::LowAir)
            || row
                .targets
                .contains_key(&contract::catalog::AltitudeLayer::HighAir)
        {
            let Some(accel) = row.ballistics.accel_mps2 else {
                continue;
            };
            let top = row.ballistics.top_speed_mps.expect("motor has a top speed");
            assert_eq!(top, 290.0, "{id} aircraft missile top speed");
            let lifetime = row
                .ballistics
                .lifetime_s
                .expect("guided air row has a lifetime");
            let motor_s = (top - row.ballistics.speed_mps) / accel;
            assert!(
                (motor_s - 2.0).abs() < 1e-9,
                "{id} motor reaches cap in {motor_s:.3}s"
            );
            let distance = row.ballistics.speed_mps * motor_s
                + 0.5 * accel * motor_s * motor_s
                + top * (lifetime - motor_s).max(0.0);
            assert!(
                distance >= row.ballistics.range_m,
                "{id} expires after {distance:.0} m, short of {:.0} m",
                row.ballistics.range_m
            );
        }
    }
}

/// A fake direct-fire row whose `targets` each test writes, so no roster
/// row's eligibility is pinned.
fn test_rows(targets: Value) -> Value {
    let mut targets = targets;
    for fire in targets.as_object_mut().unwrap().values_mut() {
        let fire = fire.as_object_mut().unwrap();
        fire.entry("range_m").or_insert(json!(300));
        fire.entry("scatter_mrad").or_insert(json!(5));
    }
    json!({ "test_gun": {
        "name": "Test Gun", "description": "A test row.", "icon": "rifle", "appearance": "rifle",
        "stationary": false, "speed_mps": 800,
        "aim_s": 1, "reload_s": 2, "ammo": "unlimited",
        "penetration": 5, "partial_from": 0.6, "damage": 10,
        "near_miss_suppression": 0, "suppression_radius_m": 0, "blast_radius_m": 0,
        "targets": targets
    }})
}

/// Each layer names the kinds of target it engages there: a soldier or a
/// vehicle on the ground, an aircraft by how it flies. A point on the ground
/// is no unit, so no class bars it.
#[test]
fn a_layer_engages_only_the_target_classes_it_lists() {
    use contract::catalog::AltitudeLayer::{Ground, HighAir, LowAir};
    use contract::weapons::TargetClass::{AirFixedWing, AirRotor, Infantry, Vehicle};
    let rows = contract::weapons::resolve_weapons(test_rows(json!({
        "ground": { "classes": ["vehicle"] },
        "low_air": { "classes": ["air_rotor"], "range_m": 100 },
        "high_air": { "classes": ["air_fixed_wing", "air_rotor"] }
    })))
    .unwrap();
    let gun = &rows["test_gun"];
    let reach = |layer, class| gun.engages(layer, class).map(|fire| fire.range_m);
    assert_eq!(reach(Ground, Some(Vehicle)), Some(300.0));
    assert_eq!(reach(Ground, Some(Infantry)), None);
    assert_eq!(reach(Ground, None), Some(300.0), "a ground point");
    assert_eq!(reach(LowAir, Some(AirRotor)), Some(100.0));
    assert_eq!(reach(LowAir, Some(AirFixedWing)), None);
    assert_eq!(reach(HighAir, Some(AirFixedWing)), Some(300.0));
    assert_eq!(reach(HighAir, Some(AirRotor)), Some(300.0));
}

/// A layer must say what it engages, in classes found at that height: none,
/// an unknown name, a repeat, or a soldier in the air is refused, naming the
/// row and the layer.
#[test]
fn a_layer_with_missing_unknown_or_misplaced_classes_is_refused() {
    for ground in [
        json!({}),
        json!({ "classes": [] }),
        json!({ "classes": ["tank"] }),
        json!({ "classes": ["vehicle", "vehicle"] }),
        json!({ "classes": ["air_rotor"] }),
    ] {
        let error = contract::weapons::resolve_weapons(test_rows(json!({ "ground": ground })))
            .err()
            .unwrap_or_else(|| panic!("accepted targets.ground = {ground}"))
            .to_string();
        assert!(
            error.contains("weapons.test_gun") && error.contains("classes"),
            "{ground}: {error}"
        );
    }
    let error = contract::weapons::resolve_weapons(test_rows(
        json!({ "low_air": { "classes": ["infantry"] } }),
    ))
    .unwrap_err()
    .to_string();
    assert!(error.contains("targets.low_air.classes"), "{error}");
}

/// Fields a cutover removed are refused by name, never silently ignored: a
/// row still saying `anti_armor` would otherwise lose its meaning unnoticed.
#[test]
fn a_removed_or_unknown_weapon_field_is_refused_by_name() {
    for (field, value) in [
        ("anti_armor", json!(true)),
        ("interception", json!("missile")),
        ("can_intercept", json!("missile")),
        ("aa_tier", json!("short")),
        ("fov_deg", json!(60)),
        ("scatter_mrad", json!(8)),
    ] {
        let mut rows = test_rows(json!({ "ground": { "classes": ["vehicle"] } }));
        rows["test_gun"][field] = value;
        let error = contract::weapons::resolve_weapons(rows)
            .err()
            .unwrap_or_else(|| panic!("accepted {field}"))
            .to_string();
        assert!(
            error.contains("weapons.test_gun") && error.contains(field),
            "{field}: {error}"
        );
    }
}

/// A row may author a field at its default, or null an inherited optional
/// one, and still load; a key that is no field is refused by name.
#[test]
fn a_field_at_its_default_or_null_is_a_field() {
    let mut rows = test_rows(json!({ "ground": { "classes": ["vehicle"] } }));
    rows["test_gun"]["delivery"] = json!("powered");
    rows["test_gun"]["proximity_fuse_m"] = json!(2);
    rows["test_child"] = json!({ "extends": "test_gun", "proximity_fuse_m": null, "seeker": null });
    let loaded = contract::weapons::resolve_weapons(rows.clone()).unwrap();
    assert_eq!(loaded["test_gun"].proximity_fuse_m, Some(2.0));
    assert_eq!(loaded["test_child"].proximity_fuse_m, None);
    rows["test_child"]["proximity_fuze_m"] = json!(2);
    let error = contract::weapons::resolve_weapons(rows)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.test_child") && error.contains("proximity_fuze_m"),
        "{error}"
    );
}

/// Interception is a target class like any other: `missile`, a round fired
/// from that layer, may be listed in any layer, guided or not, and is
/// matched by the one `engages`.
#[test]
fn any_layer_may_list_missile() {
    use contract::catalog::AltitudeLayer::{Ground, HighAir, LowAir};
    use contract::weapons::TargetClass::{AirFixedWing, Missile};
    let targets = json!({
        "ground": { "classes": ["missile"], "range_m": 100 },
        "high_air": { "classes": ["air_fixed_wing", "missile"], "range_m": 200 }
    });
    for guided in [false, true] {
        let mut rows = test_rows(targets.clone());
        rows["test_gun"]["proximity_fuse_m"] = json!(3);
        if guided {
            rows["test_gun"]["turn_deg_s"] = json!(180);
            rows["test_gun"]["guidance"] = json!("stationary");
        }
        let rows = contract::weapons::resolve_weapons(rows).unwrap();
        let gun = &rows["test_gun"];
        let reach = |layer, class| gun.engages(layer, class).map(|fire| fire.range_m);
        assert_eq!(reach(Ground, Some(Missile)), Some(100.0));
        assert_eq!(reach(HighAir, Some(Missile)), Some(200.0));
        assert_eq!(reach(HighAir, Some(AirFixedWing)), Some(200.0));
        assert_eq!(
            reach(LowAir, Some(Missile)),
            None,
            "a layer it does not list"
        );
    }
}

/// A round destroys another only by bursting near it, so a row that lists
/// `missile` must author the proximity fuse that reaches it: without one it
/// would fire at rounds it can never destroy.
#[test]
fn a_row_listing_missile_needs_a_proximity_fuse() {
    let rows = test_rows(json!({ "high_air": { "classes": ["missile"] } }));
    let error = contract::weapons::resolve_weapons(rows.clone())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("weapons.test_gun") && error.contains("proximity_fuse_m"),
        "{error}"
    );
    let mut fused = rows;
    fused["test_gun"]["proximity_fuse_m"] = json!(3);
    contract::weapons::resolve_weapons(fused).expect("a fused interceptor loads");
}

/// Every resolved row states its damage fade and each layer's fire values.
#[test]
fn resolved_weapons_require_explicit_damage_and_layer_values() {
    let base = || {
        test_rows(json!({
            "ground": { "classes": ["vehicle"], "range_m": 300, "scatter_mrad": 5 }
        }))
    };
    let mut inherited = base();
    inherited["child"] = json!({ "extends": "test_gun" });
    let resolved = contract::weapons::resolve_weapons(inherited).unwrap();
    assert_eq!(
        resolved["child"]
            .against(contract::catalog::AltitudeLayer::Ground)
            .unwrap()
            .range_m,
        300.0
    );

    let mut bomb = base();
    bomb["test_gun"]["delivery"] = json!("unpowered");
    bomb["test_gun"].as_object_mut().unwrap().remove("range_m");
    bomb["test_gun"]
        .as_object_mut()
        .unwrap()
        .remove("speed_mps");
    bomb["test_gun"]["targets"]["ground"]["range_m"] = json!(0.0);
    contract::weapons::resolve_weapons(bomb).expect("a bomb carries its former zero reach");

    for (field, path) in [
        ("partial_from", "partial_from"),
        ("range_m", "targets.ground.range_m"),
    ] {
        let mut rows = base();
        if field == "partial_from" {
            rows["test_gun"].as_object_mut().unwrap().remove(field);
        } else {
            rows["test_gun"]["targets"]["ground"]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }
        let error = contract::weapons::resolve_weapons(rows)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("weapons.test_gun")
                && error.contains(path.split('.').next_back().unwrap()),
            "{error}"
        );
    }
}

/// A serialized resolved row may carry its computed reach, but authors cannot
/// give it a second reach that disagrees with its target layers.
#[test]
fn an_authored_range_must_match_the_largest_target_layer() {
    let mut rows = test_rows(json!({
        "ground": { "classes": ["vehicle"], "range_m": 300 },
        "low_air": { "classes": ["air_rotor"], "range_m": 150 }
    }));
    rows["test_gun"]["range_m"] = json!(301);
    let error = contract::weapons::resolve_weapons(rows)
        .expect_err("a second, disagreeing reach must be refused")
        .to_string();
    assert!(
        error.contains("weapons.test_gun") && error.contains("range_m"),
        "{error}"
    );
}
