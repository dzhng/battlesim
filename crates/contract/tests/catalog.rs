//! The catalog's resolution: inheritance, parts, prop types, and the named
//! errors a broken catalog fails with.
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
            "weight_class": "heavy", "push_class": "heavy", "wreck": "heavy_wreck"
        } },
        "mobility": { "tracked": { "offroad_kmh": 22, "road_kmh": 43, "turn_deg_s": 45, "reverse_fraction": 0.4 } },
        "sensors": { "ground_m": 350, "sight_shape": { "front": 1, "side": 0.5, "rear": 0.3 }, "on": "cannon" },
        "mounts": [
            { "name": "cannon", "weapons": ["ap", "he"], "turret": true, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55] },
            { "name": "HMG", "weapons": ["hmg"], "turret": true, "on": "cannon", "pivot_m": [-0.25, -0.58, 2.35], "muzzle_m": [1.4, 0, 0.3] }
        ],
        "sound": { "profile": "vehicle", "loudness_m": 650 },
        "appearance": "tank"
    })
}

/// The prop types the test units leave behind: a tank's wreck that burns
/// down to a lighter one.
fn wrecks() -> Value {
    json!({ "props": {
        "wreck": { "abstract": true,
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

fn units(entries: Value) -> Vec<Value> {
    vec![roles(), wrecks(), json!({ "units": entries })]
}

#[test]
fn concealed_detection_bonus_cannot_reduce_ordinary_sight() {
    for multiplier in [0.5, 0.0, -1.0] {
        let mut base = base_tank();
        base["sensors"]["concealed_range_multiplier"] = json!(multiplier);
        let error = resolve(&units(json!({ "base": base, "t": { "extends": "base" } })))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("sensors.concealed_range_multiplier"),
            "{error}"
        );
    }
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
    assert!(matches!(m1a1.mobility, Mobility::Tracked { offroad_kmh, .. } if offroad_kmh == 22.0));
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
        "mobility": { "foot": { "offroad_kmh": 11, "road_kmh": 14 } },
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

#[test]
fn a_prop_type_extends_another_and_resolves_into_its_body_row() {
    let catalog = resolve(&units(
        json!({ "base": base_tank(), "m1": { "extends": "base" } }),
    ))
    .unwrap();
    let props = catalog.props();
    // The abstract base is never a type; ids are in rank order.
    assert_eq!(props.ids(), ["heavy_wreck", "light_wreck"]);
    let tank = props.by_id("heavy_wreck");
    assert!(tank.body.blocks.vehicle && tank.body.stops_rounds && !tank.body.occludes);
    assert_eq!((tank.body.hp, tank.body.armor), (Some(400.0), 0.5));
    assert_eq!(tank.appearance.drawn_by, "wreck", "inherited");
    assert_eq!(catalog.by_id("m1").hull().unwrap().wreck, "heavy_wreck");
}

#[test]
fn a_broken_prop_type_fails_at_load_naming_it() {
    let with = |props: Value| {
        let mut docs = units(json!({ "base": base_tank(), "m1": { "extends": "base" } }));
        docs.push(json!({ "props": props }));
        resolve(&docs).unwrap_err()
    };
    let body = json!({ "blocks": { "infantry": true, "vehicle": true }, "stops_rounds": true,
        "occludes": true, "weight_class": "immovable" });
    let app = json!({ "drawn_by": "wall" });
    let prop = |id: &str, error: &str| CatalogError::Prop {
        id: id.into(),
        error: error.into(),
    };
    let mut hp = body.clone();
    hp["hp"] = json!(100);
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app } })),
        prop("wall", "hp and destroyed go together")
    );
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app,
            "destroyed": { "into": { "prop": "nothing", "height_m": 1 } } } })),
        prop(
            "wall",
            "destroyed into \"nothing\", which is not a prop type"
        )
    );
    assert_eq!(
        with(json!({
            "a": { "body": hp, "appearance": app, "destroyed": { "into": { "prop": "b", "height_m": 1 } } },
            "b": { "body": hp, "appearance": app, "destroyed": { "into": { "prop": "a", "height_m": 1 } } },
        })),
        prop("a", "its destroyed states loop")
    );
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app, "destroyed": "cleared" } })),
        prop(
            "wall",
            "only a toppling body's destroyed state is cleared ground"
        )
    );
    // A unit's wreck must be a prop type.
    let orphan = resolve(&[
        roles(),
        json!({ "units": { "base": base_tank(), "m1": { "extends": "base" } } }),
    ])
    .unwrap_err();
    assert_eq!(
        orphan,
        CatalogError::Rule {
            id: "m1".into(),
            error: "its wreck \"heavy_wreck\" is not a prop type".into()
        }
    );
}

/// A type's structure the simulation relies on is checked at load, naming
/// the entry: where each mount sits, a wreck's cover, and every number in
/// its range.
#[test]
fn a_structurally_broken_type_fails_at_load_naming_it() {
    let rule = |error: &str| CatalogError::Rule {
        id: "t".into(),
        error: error.into(),
    };
    let tank = |patch: Value| {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        contract::catalog::merge(&mut t, &patch);
        error(json!({ "t": t }))
    };
    // The HMG rides a turret no earlier mount is.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][1]["on"] = json!("roof");
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"HMG\" is on \"roof\", which is not an earlier turret mount")
    );
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"].as_array_mut().unwrap().reverse();
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"HMG\" is on \"cannon\", which is not an earlier turret mount")
    );
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][0].as_object_mut().unwrap().remove("muzzle_m");
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"cannon\" on a hull needs muzzle_m")
    );
    // Hull weapons have no infantry operator to wear equipment.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][0]["operator_appearance"] = json!({"active":["launcher"],"carried":["carried"]});
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"cannon\" on a hull cannot name operator_appearance")
    );
    // Its wreck keeps its cover tier (Q24): a heavy hull's wreck is heavy cover.
    let light = json!({ "body": { "hull": { "wreck": "light_wreck" } } });
    assert_eq!(
        tank(light),
        rule("its wreck \"light_wreck\" gives light cover, not its heavy weight's heavy (Q24)")
    );
    let stopped = json!({ "mobility": { "tracked": { "turn_deg_s": 0 } } });
    assert_eq!(tank(stopped), rule("mobility: turn_deg_s must be positive"));
    let hollow = json!({ "body": { "hull": { "hp": 0 } } });
    assert_eq!(
        tank(hollow),
        rule("a hull needs finite positive extents, eye height and hp")
    );
}

/// A soldier kind's weapons are hand weapons: never on a turret or a pivot,
/// and never both a squad's and a special one.
#[test]
fn a_broken_soldier_kind_fails_at_load_naming_it() {
    let with = |soldier: Value| {
        let mut docs = units(json!({}));
        docs.push(json!({ "soldiers": { "s": soldier } }));
        resolve(&docs).unwrap_err()
    };
    let invalid = |error: &str| CatalogError::Invalid {
        section: "soldiers",
        id: "s".into(),
        error: error.into(),
    };
    let soldier = |mount: Value| json!({ "name": "S", "description": "", "hp": 100, "appearance": ["s"], "mounts": [mount] });
    assert_eq!(
        with(soldier(
            json!({ "name": "gun", "weapons": ["rifle"], "squad": true, "special": true })
        )),
        invalid("mount \"gun\" is a squad weapon or a special one, not both")
    );
    for placed in [
        json!({ "turret": true }),
        json!({ "on": "gun" }),
        json!({ "pivot_m": [0, 0, 1] }),
        json!({ "muzzle_m": [1, 0, 0] }),
    ] {
        let mut mount = json!({ "name": "gun", "weapons": ["rifle"] });
        contract::catalog::merge(&mut mount, &placed);
        assert_eq!(
            with(soldier(mount)),
            invalid("mount \"gun\" is a hand weapon: no turret, on, pivot_m or muzzle_m"),
        );
    }
    let mut dead = soldier(json!({ "name": "gun", "weapons": ["rifle"] }));
    dead["hp"] = json!(0);
    assert_eq!(with(dead), invalid("hp must be positive"));
}

/// Parts gather along the `extends` chain, and a variant that moves on
/// other wheels replaces its parent's mobility rather than merging two.
#[test]
fn a_variant_adds_parts_and_swaps_a_component_variant() {
    let parts = json!({ "parts": {
        "era": { "name": "ERA", "description": "", "patch": { "cost": 230 } },
        "aps": { "name": "APS", "description": "", "patch": { "body": { "hull": { "hp": 120 } } } },
    } });
    let mut docs = units(json!({
        "base": base_tank(),
        "m1": { "extends": "base", "parts": ["era"] },
        "m1a2": { "extends": "m1", "parts": ["aps"],
            "mobility": { "wheeled": { "offroad_kmh": 32, "road_kmh": 72, "turn_deg_s": 40,
                                       "turning_radius_m": 8, "reverse_fraction": 0.3 } } },
    }));
    docs.push(parts);
    let catalog = resolve(&docs).unwrap();
    let a2 = catalog.by_id("m1a2");
    assert_eq!(a2.parts, ["era", "aps"]);
    assert_eq!((a2.cost, a2.hull().unwrap().hp), (230, 120.0));
    assert!(
        matches!(a2.mobility, Mobility::Wheeled { offroad_kmh, .. } if offroad_kmh == 32.0),
        "{:?}",
        a2.mobility
    );
}

/// A key written twice inside one catalog file is refused by name: a JSON
/// reader would keep only the second, silently dropping an entry.
#[test]
fn a_key_repeated_in_one_document_is_refused() {
    let text = "{ \"units\": {\n  \"tank\": { \"cost\": 1 },\n  \"tank\": { \"cost\": 2 }\n} }";
    let e = contract::catalog::parse_document(text).unwrap_err();
    assert!(
        matches!(&e, CatalogError::DuplicateKey { key, line: 3, .. } if key == "tank"),
        "{e}"
    );
    assert!(e.to_string().contains("\"tank\""), "{e}");
    let fine = contract::catalog::parse_document("{ \"units\": { \"a\": [1, {\"b\": 2}] } }");
    assert_eq!(fine.unwrap(), json!({ "units": { "a": [1, { "b": 2 }] } }));
}

/// Every mover names its own off-road and road top speeds, in km/h, and no
/// type may be faster than the 130 km/h cap or slower on a road than off it.
#[test]
fn a_mover_states_two_top_speeds_within_the_cap() {
    let load = |offroad: f64, road: f64| {
        let mut tank = base_tank();
        tank.as_object_mut().unwrap().remove("abstract");
        tank["mobility"] = json!({ "tracked": { "offroad_kmh": offroad, "road_kmh": road,
            "turn_deg_s": 45, "reverse_fraction": 0.4 } });
        resolve(&units(json!({ "t": tank })))
    };
    assert!(load(22.0, 43.0).is_ok());
    assert!(load(32.0, 130.0).is_ok());
    assert!(load(32.0, 131.0).is_err(), "over the cap");
    assert!(load(40.0, 30.0).is_err(), "a road slower than open ground");
    assert!(load(0.0, 30.0).is_err(), "no off-road speed");
}

#[test]
fn hull_armor_and_ricochet_are_admitted_in_their_physical_ranges() {
    for (field, value) in [
        ("front", -1.0),
        ("side", -1.0),
        ("rear", -1.0),
        ("roof", -1.0),
        ("ricochet.front", 1.1),
        ("ricochet.side", -0.1),
        ("ricochet.rear", 1.1),
        ("ricochet.roof", -0.1),
    ] {
        let mut tank = base_tank();
        tank["abstract"] = json!(false);
        if let Some(face) = field.strip_prefix("ricochet.") {
            tank["body"]["hull"]["armor"]["ricochet"][face] = json!(value);
        } else {
            tank["body"]["hull"]["armor"][field] = json!(value);
        }
        let error = match resolve(&units(json!({"tank": tank}))) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("invalid hull armor must fail"),
        };
        assert!(error.contains("tank") && error.contains("armor"), "{error}");
    }
}

#[test]
fn a_single_operator_has_paired_active_and_carried_appearances() {
    let mut docs = units(json!({}));
    docs.push(json!({"soldiers":{"s":{
        "name":"S", "description":"", "hp":100, "appearance":["rifle"],
        "mounts":[{"name":"launcher", "weapons":["atgm"],
            "operator_appearance":{"active":["launcher","launcher_b"],
                "carried":["carried","carried_b"]}}]
    }}}));
    let catalog = resolve(&docs).expect("paired equipment appearances must load");
    let view = serde_json::to_value(catalog.soldier("s")).unwrap();
    assert_eq!(
        view["mounts"][0]["operator_appearance"],
        json!({"active":["launcher","launcher_b"],"carried":["carried","carried_b"]})
    );
}

#[test]
fn operator_equipment_refuses_unpaired_variants_and_shared_operators() {
    let load = |appearance: Value, squad: bool| {
        let mut docs = units(json!({}));
        docs.push(json!({"soldiers":{"s":{
            "name":"S", "description":"", "hp":100, "appearance":["rifle"],
            "mounts":[{"name":"launcher", "weapons":["atgm"], "squad":squad,
                "operator_appearance":appearance}]
        }}}));
        resolve(&docs)
    };
    for appearance in [
        json!({"active":[],"carried":["carried"]}),
        json!({"active":["launcher"],"carried":[]}),
        json!({"active":["launcher","launcher_b"],"carried":["carried"]}),
    ] {
        assert_eq!(load(appearance, false).unwrap_err(), CatalogError::Invalid {
            section:"soldiers", id:"s".into(),
            error:"mount \"launcher\": operator_appearance needs nonempty active and carried sets with equal variant counts".into()
        });
    }
    assert_eq!(
        load(json!({"active":["launcher"],"carried":["carried"]}), true).unwrap_err(),
        CatalogError::Invalid {
            section: "soldiers",
            id: "s".into(),
            error: "mount \"launcher\": operator_appearance needs a single operator".into()
        }
    );
}
