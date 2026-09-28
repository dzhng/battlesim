//! The unit catalog's resolution: inheritance, parts, and the named errors
//! a broken catalog fails with.
use contract::catalog::{resolve, Body, CatalogError, Mobility};
use serde_json::{json, Value};

fn roles() -> Value {
    json!({ "roles": {
        "mbt": { "name": "Main battle tank", "description": "", "symbol": ["armour"] },
        "infantry": { "name": "Infantry", "description": "", "symbol": ["infantry"] },
    } })
}

/// A complete tracked tank entry, abstract so leaves extend it.
fn base_tank() -> Value {
    json!({
        "abstract": true,
        "name": "Tank", "description": "", "faction": "test", "family": "tanks",
        "roles": ["mbt"], "cost": 200,
        "body": { "hull": {
            "half_extents_m": [3.5, 1.8, 1.2], "eye_m": 2.3, "hp": 100,
            "armor": { "front": 140, "side": 100, "rear": 60, "roof": 40,
                "ricochet": { "front": 0.5, "side": 0.3, "rear": 0.2, "roof": 0.6 } },
            "weight_class": "heavy", "push_class": "heavy", "wreck": "tank_wreck"
        } },
        "mobility": { "tracked": { "mps": 6, "road_mps": 12, "turn_deg_s": 45, "reverse_fraction": 0.4 } },
        "sensors": { "ground_m": 350, "sight_shape": { "front": 1, "side": 0.5, "rear": 0.3 }, "on": "cannon" },
        "mounts": [
            { "name": "cannon", "weapons": ["ap", "he"], "turret": true, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55] },
            { "name": "HMG", "weapons": ["hmg"], "turret": true, "on": "cannon", "pivot_m": [-0.25, -0.58, 2.35], "muzzle_m": [1.4, 0, 0.3] }
        ],
        "sound": { "profile": "vehicle", "loudness_m": 650 },
        "appearance": "tank"
    })
}

fn units(entries: Value) -> Vec<Value> {
    vec![roles(), json!({ "units": entries })]
}

#[test]
fn a_variant_gives_only_what_differs_and_inherits_the_rest() {
    let catalog = resolve(&units(json!({
        "base": base_tank(),
        "m1": { "extends": "base", "name": "M1" },
        "m1a1": {
            "extends": "m1", "name": "M1A1",
            "body": { "hull": { "armor": { "front": 180 } } },
            // A mount merges by name: the cannon swaps its weapon, the HMG stays.
            "mounts": [{ "name": "cannon", "weapons": ["ap_120"] }]
        },
    })))
    .unwrap();
    // The abstract base is never a type.
    assert_eq!(catalog.ids(), ["m1", "m1a1"]);
    let (m1, m1a1) = (catalog.by_id("m1"), catalog.by_id("m1a1"));
    let (h1, h2) = (m1.hull().unwrap(), m1a1.hull().unwrap());
    assert_eq!((h1.armor.front, h2.armor.front), (140.0, 180.0));
    assert_eq!(h2.armor.side, 100.0, "untouched armour faces are inherited");
    assert_eq!(m1a1.mounts[0].weapons, ["ap_120"]);
    assert_eq!(
        m1a1.mounts[0].muzzle_m,
        Some([5.9, 0.0, 0.55]),
        "and so is its geometry"
    );
    assert_eq!(m1a1.mounts[1], m1.mounts[1]);
    assert_eq!(m1a1.name, "M1A1");
    assert!(matches!(m1a1.mobility, Mobility::Tracked { mps, .. } if mps == 6.0));
}

#[test]
fn a_resolved_catalog_resolves_to_itself() {
    let parts = json!({ "parts": { "era": {
        "name": "ERA", "description": "", "nodes": ["era_*"],
        "patch": { "body": { "hull": { "armor": { "side": 150 } } }, "cost": 230 }
    } } });
    let mut docs = units(json!({
        "base": base_tank(),
        "m1": { "extends": "base", "name": "M1", "parts": ["era"] },
    }));
    docs.push(parts);
    let once = resolve(&docs).unwrap();
    let doc = serde_json::to_value(&once).unwrap();
    let twice = resolve(doc.as_array().unwrap()).unwrap();
    assert_eq!(once, twice);
    // The part applied once, after inheritance, and its list is kept.
    let m1 = twice.by_id("m1");
    assert_eq!((m1.hull().unwrap().armor.side, m1.cost), (150.0, 230));
    assert_eq!(m1.parts, ["era"]);
}

#[test]
fn a_squad_carries_its_soldiers_mounts_by_slot() {
    let soldiers = json!({ "soldiers": {
        "rifleman": { "name": "Rifleman", "description": "", "hp": 100, "appearance": ["r"],
            "mounts": [{ "name": "rifles", "weapons": ["rifle"], "squad": true }] },
        "gunner": { "extends": "rifleman", "name": "Gunner",
            "mounts": [{ "name": "launcher", "weapons": ["atgm"], "special": true }] },
    } });
    let mut docs = units(json!({ "team": {
        "name": "Team", "description": "", "faction": "test", "family": "infantry",
        "roles": ["infantry"], "cost": 100,
        "body": { "squad": { "slots": ["rifleman", "gunner", "rifleman"] } },
        "mobility": { "foot": { "mps": 3, "road_multiplier": 1.3 } },
        "sensors": { "ground_m": 600, "sight_shape": { "front": 1, "side": 1, "rear": 1 } },
        "sound": { "profile": "infantry", "loudness_m": 200 }
    } }));
    docs.push(soldiers);
    let catalog = resolve(&docs).unwrap();
    let t = catalog.index("team").unwrap();
    let carried: Vec<(&str, &[usize])> = catalog
        .mounts(t)
        .iter()
        .map(|m| (m.def.name.as_str(), m.carriers.as_slice()))
        .collect();
    assert_eq!(
        carried,
        [("rifles", &[0, 1, 2][..]), ("launcher", &[1][..])]
    );
    assert!(matches!(&catalog.get(t).body, Body::Squad { slots } if slots.len() == 3));
}

fn error(entries: Value) -> CatalogError {
    resolve(&units(entries)).unwrap_err()
}

#[test]
fn a_broken_catalog_fails_at_load_naming_the_entry() {
    let cycle = error(json!({
        "a": { "extends": "b", "name": "A" },
        "b": { "extends": "a", "name": "B" },
    }));
    assert!(matches!(cycle, CatalogError::Cycle { .. }), "{cycle}");

    let orphan = error(json!({ "m1": { "extends": "nothing" } }));
    assert_eq!(
        orphan,
        CatalogError::UnknownParent {
            section: "units",
            id: "m1".into(),
            parent: "nothing".into()
        }
    );

    // A leaf missing a component.
    let mut partial = base_tank();
    partial.as_object_mut().unwrap().remove("abstract");
    partial.as_object_mut().unwrap().remove("sensors");
    let incomplete = error(json!({ "t": partial }));
    assert!(
        matches!(&incomplete, CatalogError::Invalid { id, error, .. } if id == "t" && error.contains("sensors")),
        "{incomplete}"
    );

    let unknown_role = error(json!({
        "base": base_tank(),
        "t": { "extends": "base", "roles": ["artillery"] },
    }));
    assert_eq!(
        unknown_role,
        CatalogError::UnknownRole {
            id: "t".into(),
            role: "artillery".into()
        }
    );
}

#[test]
fn a_part_needing_an_unbuilt_capability_is_refused() {
    let mut docs = units(json!({
        "base": base_tank(),
        "sepv3": { "extends": "base", "parts": ["trophy"] },
    }));
    docs.push(json!({ "parts": { "trophy": {
        "name": "Trophy APS", "description": "", "nodes": ["trophy_*"],
        "patch": { "capabilities": { "aps": { "charges": 4 } } }
    } } }));
    let refused = resolve(&docs).unwrap_err();
    assert!(
        matches!(&refused, CatalogError::Invalid { id, error, .. } if id == "sepv3" && error.contains("aps")),
        "{refused}"
    );
}
