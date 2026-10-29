//! Complete authored catalog inputs shared by admission tests.
use serde_json::{json, Value};

pub fn roles() -> Value {
    json!({ "roles": {
        "mbt": { "name": "Main battle tank", "description": "", "symbol": ["armour"] },
        "infantry": { "name": "Infantry", "description": "", "symbol": ["infantry"] },
    } })
}

/// A complete tracked tank entry, abstract so leaves extend it.
pub fn base_tank() -> Value {
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

/// The prop types the test units leave behind: a tank's wreck that burns
/// down to a lighter one.
fn wrecks() -> Value {
    json!({ "props": {
        "wreck": { "abstract": true, "wreck": true,
            "body": { "blocks": { "infantry": true, "vehicle": true }, "stops_rounds": true,
                "occludes": false, "armor": 0.5 },
            "appearance": { "drawn_by": "wreck" } },
        "light_wreck": { "extends": "wreck",
            "body": { "weight_class": "light", "cover_tier": "light", "hp": 150 },
            "destroyed": "removed" },
        "heavy_wreck": { "extends": "wreck",
            "body": { "weight_class": "heavy", "cover_tier": "heavy", "hp": 400 },
            "destroyed": { "into": { "prop": "light_wreck", "height_m": 1.2 } } },
    } })
}

pub fn units(entries: Value) -> Vec<Value> {
    vec![roles(), wrecks(), json!({ "units": entries })]
}

pub fn rotor_air(cruise_kmh: u32, turn_deg_s: u32, climb_mps: u32) -> Value {
    json!({ "air": { "cruise_kmh": cruise_kmh, "turn_deg_s": turn_deg_s, "climb_mps": climb_mps, "layer": "low_air", "flight": "rotor" } })
}
