//! The catalog's resolution: inheritance, parts, prop types, and the named
//! errors a broken catalog fails with.
use contract::catalog::{resolve, Body, CatalogError, Mobility};
use serde_json::{json, Value};

#[path = "common/catalog.rs"]
mod inputs;
use inputs::{base_tank, roles, rotor_air, units};

#[test]
fn planned_aircraft_is_a_visible_card_but_not_a_physical_type() {
    let catalog = resolve(&units(json!({
        "base": base_tank(),
        "tank": { "extends": "base" },
        "f22": {
            "name": "F-22 Raptor", "description": "Air superiority fighter",
            "faction": "us", "family": "f22", "roles": [], "cost": 400,
            "roster": { "factions": ["us"], "family_name": "F-22 Raptor", "category": "air", "variant": "F-22A" },
            "planned": { "reason": "Aircraft mechanics are not implemented", "profile": "stealth-fighter", "weapons": [{ "name": "AA Missile", "icon": "aa_missile" }] }
        }
    }))).unwrap();
    assert!(catalog.index("f22").is_none());
    assert_eq!(catalog.ids(), ["tank"]);
    let card = catalog.card("f22").unwrap();
    assert_eq!(card.name, "F-22 Raptor");
    assert_eq!(card.cost, 400);
    assert_eq!(
        card.disabled_reason,
        Some("Aircraft mechanics are not implemented")
    );
    assert_eq!(catalog.view()["cards"][0]["id"], "f22");
    let document = serde_json::to_value(&catalog).unwrap();
    assert_eq!(resolve(document.as_array().unwrap()).unwrap(), catalog);
}

#[test]
fn aircraft_refund_weights_must_sum_to_one() {
    let mut aircraft = base_tank();
    aircraft["abstract"] = json!(false);
    aircraft["mobility"] = json!({
        "air": { "cruise_kmh": 280, "turn_deg_s": 60, "climb_mps": 12, "layer": "high_air", "flight": "fixed_wing" }
    });
    aircraft["capabilities"] = json!({ "air": {
        "fuel_capacity_s": 180,
        "refund": { "health": 0.6, "ammunition": 0.3, "fuel": 0.3 }
    }});
    let error = resolve(&units(json!({ "aircraft": aircraft })))
        .unwrap_err()
        .to_string();
    assert!(error.contains("refund weights"), "{error}");
}

#[test]
fn spotting_must_be_positive() {
    for multiplier in [0.0, -1.0] {
        let mut base = base_tank();
        base["sensors"]["spotting"] = json!(multiplier);
        let error = resolve(&units(json!({ "base": base, "t": { "extends": "base" } })))
            .unwrap_err()
            .to_string();
        assert!(error.contains("sensors.spotting"), "{error}");
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
            // A mount merges by id: the cannon swaps its weapon and keeps
            // its label, the HMG stays.
            "mounts": [{ "id": "cannon", "weapons": ["ap_120"] }]
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
    assert_eq!(m1a1.mounts[0].name, "Main Gun");
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
            "mounts": [{ "id": "rifles", "name": "Rifle", "weapons": ["rifle"], "squad": true }] },
        "gunner": { "extends": "rifleman", "name": "Gunner",
            "mounts": [{ "id": "launcher", "name": "Launcher", "weapons": ["atgm"], "special": true }] },
    } });
    let mut docs = units(json!({ "team": {
        "name": "Team", "description": "", "faction": "test", "family": "infantry",
        "roles": ["infantry"], "cost": 100,
        "body": { "squad": { "slots": ["rifleman", "gunner", "rifleman"] } },
        "mobility": { "foot": { "offroad_kmh": 11, "road_kmh": 14 } },
        "sensors": { "ground_m": 600, "low_air_m": 600, "high_air_m": 600, "sight_shape": { "front": 1, "side": 1, "rear": 1 } },
        "concealment": 1, "sound": { "profile": "infantry", "loudness_m": 200 }
    } }));
    docs.push(soldiers);
    let catalog = resolve(&docs).unwrap();
    let t = catalog.index("team").unwrap();
    let carried: Vec<(&str, &[usize])> = catalog
        .mounts(t)
        .iter()
        .map(|m| (m.def.id.as_str(), m.carriers.as_slice()))
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
fn a_physical_unit_resolving_without_concealment_is_refused() {
    let mut base = base_tank();
    base.as_object_mut().unwrap().remove("concealment");
    let error = error(json!({ "base": base, "t": { "extends": "base" } }));
    assert!(
        matches!(&error, CatalogError::Invalid { id, error, .. }
        if id == "t" && error.contains("concealment")),
        "{error}"
    );
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
    // A gun on the roof gun that rides the turret: one level of carriage only.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    let mut stacked = t["mounts"][1].clone();
    stacked["id"] = json!("stacked");
    stacked["on"] = json!("HMG");
    t["mounts"].as_array_mut().unwrap().push(stacked);
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"stacked\" is on \"HMG\", which is itself carried by a turret")
    );
    // An arc limits a turret's swing off the hull's heading: on a gun fixed
    // in the hull, or on one another turret carries, it means nothing, and
    // it must leave the gun some swing and not all of it.
    for (turret, on) in [(json!(false), Value::Null), (json!(true), json!("cannon"))] {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        t["mounts"][1]["turret"] = turret;
        t["mounts"][1]["on"] = on;
        t["mounts"][1]["arc"] = json!({ "centre_deg": 90, "width_deg": 160 });
        assert_eq!(
            error(json!({ "t": t })),
            rule("mount \"HMG\" has an arc but no turret on the hull to swing through it")
        );
    }
    for width in [0.0, 360.0] {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        t["mounts"][1]["on"] = Value::Null;
        t["mounts"][1]["arc"] = json!({ "centre_deg": 90, "width_deg": width });
        assert_eq!(
            error(json!({ "t": t })),
            rule("mount \"HMG\" arc width must lie between 0 and 360 degrees")
        );
    }
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

/// Infantry cannot have turret chains or a shared squad bore; a single
/// operator may declare a physical mount offset.
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
            json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"], "squad": true, "special": true })
        )),
        invalid("mount \"gun\" is a squad weapon or a special one, not both")
    );
    for placed in [
        json!({ "turret": true }),
        json!({ "on": "gun" }),
        json!({ "pivot_m": [0, 0, 1] }),
        json!({ "squad": true, "muzzle_m": [1, 0, 0] }),
    ] {
        let mut mount = json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"] });
        contract::catalog::merge(&mut mount, &placed);
        assert_eq!(
            with(soldier(mount)),
            invalid("mount \"gun\": infantry cannot use turret/on, share bore offsets, or declare a pivot without a muzzle"),
        );
    }
    let mut dead = soldier(json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"] }));
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
                                       "turning_radius_m": 8, "reverse_speed_factor": 0.3 } } },
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
        let speeds = json!({ "tracked": { "offroad_kmh": offroad, "road_kmh": road } });
        resolve(&units(
            json!({ "base": base_tank(), "t": { "extends": "base", "mobility": speeds } }),
        ))
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
        "mounts":[{"id":"launcher", "name":"Launcher", "weapons":["atgm"],
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
fn a_single_operator_can_declare_a_grounded_bore_and_supported_pose() {
    let mut docs = units(json!({}));
    docs.push(json!({"soldiers":{"s":{
        "name":"S", "description":"", "hp":100, "appearance":["rifle"],
        "mounts":[{"id":"launcher", "name":"Launcher", "weapons":["atgm"],
            "pivot_m":[0.2,0,0.75], "muzzle_m":[0.6,0,0],
            "operator_appearance":{"active":["tripod"],"carried":["packed"],
                "active_pose":{"clip":"kneel_fire","phase":0.25}}}]
    }}}));
    let catalog = resolve(&docs).expect("single-operator grounded mount must load");
    let view = serde_json::to_value(catalog.soldier("s")).unwrap();
    assert_eq!(view["mounts"][0]["muzzle_m"], json!([0.6, 0.0, 0.0]));
    assert_eq!(
        view["mounts"][0]["operator_appearance"]["active_pose"],
        json!({"clip":"kneel_fire","phase":0.25})
    );
}

#[test]
fn supported_operator_poses_refuse_locomotion_and_out_of_clip_phases() {
    for pose in [
        json!({"clip":"walk","phase":0}),
        json!({"clip":"kneel_fire","phase":-0.1}),
        json!({"clip":"stand_aim","phase":1.1}),
    ] {
        let mut docs = units(json!({}));
        docs.push(json!({"soldiers":{"s":{
            "name":"S", "description":"", "hp":100, "appearance":["rifle"],
            "mounts":[{"id":"launcher", "name":"Launcher", "weapons":["atgm"],
                "operator_appearance":{"active":["active"],"carried":["packed"],"active_pose":pose}}]
        }}}));
        let error = resolve(&docs).unwrap_err().to_string();
        assert!(
            error.contains("active_pose") && error.contains("finite phase"),
            "{error}"
        );
    }
}

#[test]
fn operator_equipment_refuses_unpaired_variants_and_shared_operators() {
    let load = |appearance: Value, squad: bool| {
        let mut docs = units(json!({}));
        docs.push(json!({"soldiers":{"s":{
            "name":"S", "description":"", "hp":100, "appearance":["rifle"],
            "mounts":[{"id":"launcher", "name":"Launcher", "weapons":["atgm"], "squad":squad,
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

/// A playable entry of `faction`'s roster in `family`, in the vehicle
/// category.
fn card(family: &str, factions: &[&str]) -> Value {
    json!({
        "extends": "base", "family": family,
        "roster": { "factions": factions, "family_name": "Tank", "category": "veh", "variant": "A" }
    })
}

#[test]
fn one_variant_serves_every_faction_it_names() {
    let catalog = resolve(&units(json!({
        "base": base_tank(),
        "shared": card("shared", &["us", "europe"]),
    })))
    .unwrap();
    let factions = &catalog.card("shared").unwrap().roster.factions;
    assert_eq!(
        serde_json::to_value(factions).unwrap(),
        json!(["us", "europe"])
    );
    let error = resolve(&units(json!({
        "base": base_tank(),
        "twice": card("twice", &["us", "us"]),
    })))
    .unwrap_err()
    .to_string();
    assert!(error.contains("distinct factions"), "{error}");
}

/// A faction's deck groups variants into family cards, at most ten to a
/// category: an eleventh family is refused, wherever its variants come from.
#[test]
fn a_faction_holds_at_most_ten_families_in_a_category() {
    let mut entries = json!({ "base": base_tank() });
    for k in 0..10 {
        entries[format!("own{k}")] = card(&format!("family{k}"), &["us"]);
        // A second variant of the same family adds no family.
        entries[format!("own{k}b")] = card(&format!("family{k}"), &["us"]);
    }
    entries["other"] = card("other", &["europe"]);
    assert!(resolve(&units(entries.clone())).is_ok());
    entries["shared"] = card("shared", &["europe", "us"]);
    let error = resolve(&units(entries)).unwrap_err().to_string();
    assert!(error.contains("ten families"), "{error}");
}

/// Everything a card shows is a concise label authored beside its id
/// (`contract::labels`): a type's name, its family's, each mount's, a
/// planned unit's and its planned weapons', and an active protection's.
/// One that reads as an identifier or a sentence is refused at load,
/// naming the entry and the field.
#[test]
fn every_card_label_is_concise_and_separate_from_its_id() {
    let tank = |patch: Value| {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        contract::catalog::merge(&mut t, &patch);
        resolve(&units(json!({ "t": t })))
    };
    let refused = |patch: Value, field: &str| {
        let error = tank(patch.clone())
            .err()
            .unwrap_or_else(|| panic!("accepted {patch}"));
        assert!(
            matches!(&error, CatalogError::Invalid { id, error, .. }
                if id == "t" && error.contains(field)),
            "{patch}: {error}"
        );
    };
    for name in [
        "",
        "BMP IFV Family BMP-3",
        "T-72 T-72B3",
        "Tigr Tigr-M",
        "Challenger Challenger 2 TES",
        "Menu tank (Abrams look)",
        "test_tank",
        "tank",
        "Tank ",
    ] {
        refused(json!({ "name": name }), "name");
    }
    for name in [
        "BMP-3",
        "T-72B3",
        "Rifle Squad",
        "CAESAR 8×8",
        "F/A-18E",
        "M1A2 SEP v3 Trophy",
    ] {
        if let Err(error) = tank(json!({ "name": name })) {
            panic!("refused {name:?}: {error}");
        }
    }
    let roster = |family: &str| {
        json!({ "roster": { "factions": ["us"], "category": "veh",
        "family_name": family, "variant": "A" } })
    };
    refused(roster("BMP IFV Family BMP"), "family_name");
    tank(roster("BMP")).unwrap();

    // A mount's label is its own field: its id is what other fields name.
    let mounts = |cannon: Value| json!({ "mounts": [cannon] });
    refused(
        mounts(json!({ "id": "cannon", "name": "main_gun" })),
        "mounts.cannon",
    );
    let mut unnamed = base_tank();
    unnamed.as_object_mut().unwrap().remove("abstract");
    unnamed["mounts"][1].as_object_mut().unwrap().remove("name");
    let error = resolve(&units(json!({ "t": unnamed }))).unwrap_err();
    assert!(error.to_string().contains("name"), "{error}");
    let catalog = tank(json!({})).unwrap();
    let cannon = &catalog.by_id("t").mounts[0];
    assert_eq!(
        (cannon.id.as_str(), cannon.name.as_str()),
        ("cannon", "Main Gun")
    );

    // Active protection shows on the card as a weapon: a label and an icon.
    let aps = |name: &str, icon: &str| {
        json!({ "capabilities": { "active_protection": {
        "name": name, "icon": icon, "capacity": 2, "cooldown_s": 1, "standoff_m": 10,
        "service_s": 5, "stock_per_charge": 1 } } })
    };
    tank(aps("Trophy", "trophy")).unwrap();
    refused(aps("trophy_aps", "trophy"), "active_protection");
    refused(aps("Trophy", "Trophy APS"), "active_protection");

    // A planned unit's card: its name and each weapon it will carry.
    let planned = |name: &str, weapon: Value| {
        resolve(&units(json!({ "t": {
            "name": name, "description": "", "faction": "us", "family": "f22",
            "roles": [], "cost": 400,
            "roster": { "factions": ["us"], "family_name": "F-22 Raptor", "category": "air", "variant": "F-22A" },
            "planned": { "reason": "Aircraft are not built", "profile": "fighter", "weapons": [weapon] }
        } })))
    };
    let gun = json!({ "name": "Cannon", "icon": "autocannon" });
    planned("F-22A Raptor", gun.clone()).unwrap();
    for (name, weapon, field) in [
        ("F-22 Raptor F-22A", gun.clone(), "name"),
        (
            "F-22A Raptor",
            json!({ "name": "fighter-gun", "icon": "autocannon" }),
            "planned.weapons",
        ),
        (
            "F-22A Raptor",
            json!({ "name": "Cannon", "icon": "" }),
            "planned.weapons",
        ),
    ] {
        let error = planned(name, weapon).unwrap_err();
        assert!(
            matches!(&error, CatalogError::Invalid { id, error, .. } if id == "t" && error.contains(field)),
            "{error}"
        );
    }
}

/// A vehicle's presentation class comes from its physics (how it moves, how
/// heavy its hull is, whether it hauls supply), never its unit id or role, so
/// a new vehicle sounds and articulates as its kind with no table edit. The view
/// carries it for every reader; a squad has none.
#[test]
fn the_view_gives_each_vehicle_the_class_its_physics_make_it() {
    let roles = json!({ "roles": {
        "recon": { "name": "Recon", "description": "", "symbol": [] },
        "logistics": { "name": "Logistics", "description": "", "symbol": [] },
    } });
    let mut wheeled = base_tank();
    wheeled["abstract"] = json!(false);
    wheeled["body"]["hull"]["weight_class"] = json!("medium");
    wheeled["body"]["hull"]["wreck"] = json!("medium_wreck");
    wheeled["mobility"] = json!({ "wheeled": {
        "offroad_kmh": 30, "road_kmh": 60, "turn_deg_s": 30,
        "turning_radius_m": 8, "reverse_speed_factor": 0.3 } });
    let mut recon = wheeled.clone();
    recon["roles"] = json!(["recon"]);
    let mut tagged = wheeled.clone();
    tagged["roles"] = json!(["logistics"]);
    let mut truck = recon.clone();
    truck["capabilities"] =
        json!({ "deploy": { "seconds": 3, "pack_seconds": 3 }, "supply": { "stock": 100 } });
    let mut heli = base_tank();
    heli["abstract"] = json!(false);
    heli["body"]["hull"]["weight_class"] = json!("light");
    heli["body"]["hull"]["wreck"] = json!("light_wreck");
    heli["mobility"] = rotor_air(220, 90, 6);
    let mut fighter = heli.clone();
    fighter["mobility"]["air"]["cruise_kmh"] = json!(300);
    fighter["mobility"]["air"]["layer"] = json!("high_air");
    fighter["mobility"]["air"]["flight"] = json!("fixed_wing");
    let mut docs = units(json!({
        "base": base_tank(),
        "tank": { "extends": "base" },
        "recon": recon,
        "truck": truck,
        "tagged": tagged,
        "heli": heli,
        "fighter": fighter,
        "team": {
            "name": "Team", "description": "", "faction": "test", "family": "infantry",
            "roles": ["infantry"], "cost": 100,
            "body": { "squad": { "slots": ["rifleman"] } },
            "mobility": { "foot": { "offroad_kmh": 11, "road_kmh": 14 } },
            "sensors": { "ground_m": 600, "low_air_m": 600, "high_air_m": 600, "sight_shape": { "front": 1, "side": 1, "rear": 1 } },
            "concealment": 1, "sound": { "profile": "infantry", "loudness_m": 200 }
        }
    }));
    docs.push(roles);
    docs.push(json!({ "props": {
        "medium_wreck": { "extends": "wreck",
            "body": { "weight_class": "medium", "cover_tier": "medium", "hp": 250 },
            "destroyed": "removed" },
    } }));
    docs.push(json!({ "soldiers": {
        "rifleman": { "name": "Rifleman", "description": "", "hp": 100, "appearance": ["r"],
            "mounts": [{ "id": "rifles", "name": "Rifle", "weapons": ["rifle"], "squad": true }] },
    } }));
    let view = resolve(&docs).unwrap().view();
    let classes: Vec<(&str, &Value)> = view["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| (u["id"].as_str().unwrap(), &u["vehicle_class"]))
        .collect();
    let want = [
        ("fighter", json!("high_air_light")),
        ("heli", json!("air_light")),
        ("recon", json!("wheeled_medium")),
        ("tagged", json!("wheeled_medium")),
        ("tank", json!("tracked_heavy")),
        ("team", Value::Null),
        ("truck", json!("wheeled_medium_logistics")),
    ];
    let mut classes = classes;
    classes.sort_by_key(|(id, _)| *id);
    assert_eq!(
        classes,
        want.iter().map(|(id, c)| (*id, c)).collect::<Vec<_>>()
    );
}

/// Flares are dispensed from a vehicle or an airframe: a squad carrying
/// them is refused, naming the rule.
#[test]
fn flares_need_a_hull() {
    let soldiers = json!({ "soldiers": {
        "rifleman": { "name": "Rifleman", "description": "", "hp": 100, "appearance": ["r"],
            "mounts": [{ "id": "rifles", "name": "Rifle", "weapons": ["rifle"], "squad": true }] },
    } });
    let mut docs = units(json!({ "team": {
        "name": "Team", "description": "", "faction": "test", "family": "infantry",
        "roles": ["infantry"], "cost": 100,
        "body": { "squad": { "slots": ["rifleman"] } },
        "mobility": { "foot": { "offroad_kmh": 11, "road_kmh": 14 } },
        "sensors": { "ground_m": 600, "low_air_m": 600, "high_air_m": 600, "sight_shape": { "front": 1, "side": 1, "rear": 1 } },
        "concealment": 1, "sound": { "profile": "infantry", "loudness_m": 200 },
        "capabilities": { "flares": { "capacity": 2, "reload_s": 3 } }
    } }));
    docs.push(soldiers);
    let error = resolve(&docs).unwrap_err().to_string();
    assert!(
        error.contains("capabilities.flares needs a hull"),
        "{error}"
    );
}

/// A flight model's name is the one it is authored by.
#[test]
fn a_flight_models_name_is_its_authored_name() {
    use contract::catalog::FlightModel;
    for model in [FlightModel::Rotor, FlightModel::FixedWing] {
        assert_eq!(serde_json::to_value(model).unwrap(), model.name());
    }
}

#[path = "catalog/prop_admission.rs"]
mod prop_admission;
