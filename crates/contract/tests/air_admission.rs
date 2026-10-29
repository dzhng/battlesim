//! Air admission data contracts: explicit altitude layers and afterburner values.
use contract::catalog::resolve;
use serde_json::{json, Value};

fn roles() -> Value {
    json!({ "roles": {
        "mbt": { "name": "Main battle tank", "description": "", "symbol": ["armour"] },
        "infantry": { "name": "Infantry", "description": "", "symbol": ["infantry"] },
    } })
}

fn base_tank() -> Value {
    json!({
        "abstract": true,
        "name": "Tank", "description": "", "faction": "test", "family": "tanks",
        "roles": ["mbt"], "cost": 200,
        "body": { "hull": {
            "half_extents_m": [3.5, 1.8, 1.2], "eye_m": 2.3, "hp": 100,
            "armor": { "front": 140, "side": 100, "rear": 60, "roof": 40,
                "ricochet": { "front": 0.5, "side": 0.3, "rear": 0.2, "roof": 0.6 } },
            "weight_class": "heavy", "push_class": "heavy", "wreck": "heavy_wreck"
        } },
        "mobility": { "tracked": { "offroad_kmh": 22, "road_kmh": 43, "turn_deg_s": 45, "reverse_speed_factor": 0.4 } },
        "sensors": { "ground_m": 350, "low_air_m": 350, "high_air_m": 300, "sight_shape": { "front": 1, "side": 0.5, "rear": 0.3 }, "on": "cannon" },
        "mounts": [
            { "id": "cannon", "name": "Main Gun", "weapons": ["ap", "he"], "turret": true, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55] },
            { "id": "HMG", "name": "HMG", "weapons": ["hmg"], "turret": true, "on": "cannon", "pivot_m": [-0.25, -0.58, 2.35], "muzzle_m": [1.4, 0, 0.3] }
        ],
        "concealment": 2.8,
        "sound": { "profile": "vehicle", "loudness_m": 650 },
        "appearance": "tank"
    })
}

fn wrecks() -> Value {
    json!({ "props": {
        "wreck": { "abstract": true, "wreck": true,
            "body": { "blocks": { "infantry": true, "vehicle": true }, "stops_rounds": true,
                "occludes": false, "armor": 0.5 }, "appearance": { "drawn_by": "wreck" } },
        "light_wreck": { "extends": "wreck", "body": { "weight_class": "light", "cover_tier": "light", "hp": 150 }, "destroyed": "removed" },
        "heavy_wreck": { "extends": "wreck", "body": { "weight_class": "heavy", "cover_tier": "heavy", "hp": 400 }, "destroyed": { "into": { "prop": "light_wreck", "height_m": 1.2 } } },
    } })
}

fn units(entries: Value) -> Vec<Value> {
    vec![roles(), wrecks(), json!({ "units": entries })]
}
fn rotor_mobility(cruise_kmh: u32, turn_deg_s: u32, climb_mps: u32) -> Value {
    json!({ "air": { "cruise_kmh": cruise_kmh, "turn_deg_s": turn_deg_s, "climb_mps": climb_mps, "layer": "low_air", "flight": "rotor" } })
}
fn afterburner() -> Value {
    json!({ "speed_factor": 1.5, "fuel_factor": 2.0 })
}

#[test]
fn air_mobility_names_its_layer_and_its_flight_model() {
    use contract::catalog::{AltitudeLayer, FlightModel};
    let mut rotor = base_tank();
    rotor["abstract"] = json!(false);
    rotor["mobility"] = rotor_mobility(180, 30, 8);
    let mut wing = rotor.clone();
    wing["mobility"]["air"]["layer"] = json!("high_air");
    wing["mobility"]["air"]["flight"] = json!("fixed_wing");
    let catalog = resolve(&units(json!({ "rotor": rotor, "wing": wing }))).unwrap();
    let mobility = |id| catalog.by_id(id).mobility;
    assert_eq!(mobility("rotor").layer(), AltitudeLayer::LowAir);
    assert_eq!(mobility("rotor").flight(), Some(FlightModel::Rotor));
    assert_eq!(mobility("wing").layer(), AltitudeLayer::HighAir);
    assert_eq!(mobility("wing").flight(), Some(FlightModel::FixedWing));

    // The flight model and altitude layer are both authored, never derived.
    let mut unnamed = rotor.clone();
    unnamed["mobility"]["air"]
        .as_object_mut()
        .unwrap()
        .remove("flight");
    let error = resolve(&units(json!({ "unnamed": unnamed })))
        .unwrap_err()
        .to_string();
    assert!(error.contains("flight"), "{error}");

    let mut unlayered = rotor.clone();
    unlayered["mobility"]["air"]
        .as_object_mut()
        .unwrap()
        .remove("layer");
    let error = resolve(&units(json!({ "unlayered": unlayered })))
        .unwrap_err()
        .to_string();
    assert!(error.contains("layer"), "{error}");

    // A pair no flight law flies yet is refused, naming what is missing.
    let mut blimp = rotor.clone();
    blimp["mobility"]["air"]["layer"] = json!("high_air");
    let error = resolve(&units(json!({ "blimp": blimp })))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("blimp") && error.contains("rotor") && error.contains("high air"),
        "{error}"
    );
    let mut drone = wing.clone();
    drone["mobility"]["air"]["layer"] = json!("low_air");
    let error = resolve(&units(json!({ "drone": drone })))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("drone") && error.contains("fixed-wing") && error.contains("low air"),
        "{error}"
    );
}

#[test]
fn aircraft_capability_carries_air_ranges_afterburner_and_refund() {
    let mut aircraft = base_tank();
    aircraft["abstract"] = json!(false);
    aircraft["mobility"] = json!({
        "air": { "cruise_kmh": 280, "turn_deg_s": 60, "climb_mps": 12, "layer": "high_air", "flight": "fixed_wing" }
    });
    aircraft["sensors"] = json!({
        "ground_m": 500, "low_air_m": 1500, "high_air_m": 3000,
        "sight_shape": { "front": 1, "side": 0.8, "rear": 0.5 }
    });
    aircraft["capabilities"] = json!({
        "flares": { "capacity": 6, "reload_s": 18 },
        "air": {
            "fuel_capacity_s": 180,
            "afterburner": afterburner(),
            "refund": { "health": 0.5, "ammunition": 0.3, "fuel": 0.2 }
        }
    });
    let catalog = resolve(&units(json!({ "aircraft": aircraft }))).unwrap();
    let t = catalog.by_id("aircraft");
    assert_eq!(t.sensors.high_air_m, 3000.0);
    assert_eq!(
        t.capabilities
            .air
            .as_ref()
            .unwrap()
            .afterburner
            .unwrap()
            .speed_factor,
        1.5
    );

    let mut empty = aircraft.clone();
    empty["capabilities"]["air"]["afterburner"] = json!({});
    let error = resolve(&units(json!({ "empty": empty })))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("speed_factor") || error.contains("fuel_factor"),
        "{error}"
    );
}

/// The afterburner is a jet's: no rotor flight law flies it, so a rotor
/// row carrying one is refused, naming the rule.
#[test]
fn an_afterburner_needs_fixed_wing_flight() {
    let mut rotor = base_tank();
    rotor["abstract"] = json!(false);
    rotor["mobility"] = rotor_mobility(180, 30, 8);
    rotor["capabilities"] = json!({ "air": {
        "fuel_capacity_s": 180,
        "afterburner": afterburner(),
        "refund": { "health": 0.5, "ammunition": 0.3, "fuel": 0.2 }
    }});
    let error = resolve(&units(json!({ "rotor": rotor })))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("afterburner") && error.contains("fixed-wing"),
        "{error}"
    );
}
