//! Aggregate building ownership, exercised through scenario loading and Battle.
use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

fn original_setup(input: Value) -> ScenarioDefinition {
    serde_json::from_value(input).unwrap()
}
fn original_observation(frame: &contract::observation::ObservationFrame) -> String {
    let mut json = serde_json::to_string(frame).unwrap();
    // This frozen oracle predates the derived own-unit concealment readout.
    json = json
        .replace(",\"concealed\":true", "")
        .replace(",\"concealed\":false", "");
    for p in &frame.known_props {
        let optional = |id: Option<u32>| id.map_or_else(|| "null".into(), |id| id.to_string());
        let added = format!(
            ",\"id\":{},\"building\":{},\"structure_owner\":{},\"authored_prop\":{}",
            p.id,
            optional(p.building),
            optional(p.structure_owner),
            optional(p.authored_prop)
        );
        assert!(json.contains(&added));
        json = json.replace(&added, "");
    }
    json
}
fn original_props(world: &sim::world::WorldGeometry) -> String {
    let columns: Vec<_> = world
        .export_props()
        .chunks_exact(10)
        .flat_map(|r| std::iter::once(r[0] + r[1] * 65536.0).chain(r[2..].iter().copied()))
        .collect();
    serde_json::to_string(&columns).unwrap()
}

#[test]
fn buildings_match_the_parity_oracle_observations_digests_queries_and_seats() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/buildings/oracle.json"
    ))
    .unwrap();
    let mut blessed = oracle.clone();
    for arm in blessed["arms"].as_array_mut().unwrap() {
        let mut setup: ScenarioDefinition = original_setup(arm["scenario"].clone());
        setup.map = crate::common::physical_map(setup.map, &setup.rules);
        let mut b = Battle::new(&setup, arm["seed"].as_u64().unwrap());
        let slots: Vec<_> = b.world().props().filter(|p|p.body.garrison).map(|p| json!({"id":p.id,"slots":sim::garrison::slots(b.world(),p,&setup.rules).iter().map(|s|json!({"position":[s.position.x,s.position.y,s.position.z],"normal":[s.slot.normal.x,s.slot.normal.y],"facade":s.slot.facade})).collect::<Vec<_>>()})).collect();
        let queries: Vec<_>=b.world().props().filter(|p|p.body.garrison).map(|p|json!({"id":p.id,"surface":sim::world::export::surface_record(b.world().surface_at(p.center.x,p.center.y)),"hit":sim::world::export::hit_record(b.world().raycast(sim::math::v3(p.center.x-60.0,p.center.y,p.base_z+2.0),sim::math::v3(1.0,0.0,0.0),100.0))})).collect();
        let actual = [
            ("props", original_props(b.world())),
            ("slots", serde_json::to_string(&slots).unwrap()),
            ("queries", serde_json::to_string(&queries).unwrap()),
        ];
        for (key, value) in actual {
            arm[key] = json!(value);
        }
        for row in arm["rows"].as_array_mut().unwrap() {
            while b.tick() < row["tick"].as_u64().unwrap() {
                b.step();
            }
            row["digest"] = json!(format!("{:016x}", b.digest()));
            for (side, name) in [(Side::Blue, "blue"), (Side::Red, "red")] {
                row[name] = json!(original_observation(b.observe(side)));
            }
        }
        let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
        while replay.tick() < b.tick() {
            replay.step();
        }
        assert_eq!(replay.digest(), b.digest());
    }
    if crate::common::bless_parity("buildings/oracle.json", &blessed) {
        return;
    }
    for (arm, expected) in blessed["arms"]
        .as_array()
        .unwrap()
        .iter()
        .zip(oracle["arms"].as_array().unwrap())
    {
        for key in ["props", "slots", "queries"] {
            assert_eq!(arm[key], expected[key], "{} {key}", arm["name"]);
        }
        for (row, frozen) in arm["rows"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["rows"].as_array().unwrap())
        {
            for key in ["digest", "blue", "red"] {
                assert_eq!(
                    row[key], frozen[key],
                    "{} tick {} {key}",
                    arm["name"], row["tick"]
                );
            }
        }
    }
}

fn compound_setup(events: Value) -> ScenarioDefinition {
    let descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
        include_str!("../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap();
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
    let geometry = descriptor
        .materialize(contract::templates::PlacementFrame {
            translation: [400.0, 300.0, 0.0],
            yaw: 0.0,
        })
        .unwrap();
    let mut rules = crate::common::village();
    sim::fixtures::patch_catalog(&mut rules, "props", "building", json!({"body":{"hp":1000}}));
    rules["weapons"]["tank_he"]["structural_damage"] = json!(100);
    rules["weapons"]["tank_he"]["blast_radius_m"] = json!(20);
    serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "template_catalog_hash":catalogue.hash(),
            "buildings":[{"owner":0,"kind":"building","category":descriptor.category,"regional_family":descriptor.regional_family,"parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":geometry}],
            "props":[{"id":2,"kind":"tooth","center":[700,550],"yaw":0,"half_extents":[0.6,0.6,0.6]}]},
        "rules":rules,"units":[],"events":events,"scripts":[]
    })).unwrap()
}

#[test]
fn one_blast_wears_the_owner_once_by_its_nearest_part() {
    let setup = compound_setup(json!([{"tick":1,"burst":{"point":[409,301],"weapon":"tank_he"}}]));
    let mut b = Battle::new(&setup, 11);
    assert_eq!(b.structures().hp(b.world(), 0), Some(1000.0));
    assert_eq!(b.structures().hp(b.world(), 1), Some(1000.0));
    b.step();
    // Unequal distances rule out adding both physical parts' contributions:
    // wing is 1m away (95 damage), main is 5m away (75). One building takes 95.
    assert_eq!(b.structures().hp(b.world(), 0), Some(905.0));
    assert_eq!(b.structures().hp(b.world(), 1), Some(905.0));
    assert!(b.world().prop(0).is_some() && b.world().prop(1).is_some());
}

#[test]
fn owner_collapse_replaces_every_part_atomically() {
    let setup = compound_setup(json!((1..=11)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    let mut b = Battle::new(&setup, 11);
    for _ in 0..10 {
        b.step();
    }
    assert_eq!(b.structures().hp(b.world(), 1), Some(50.0));
    assert!(b.world().prop(0).is_some() && b.world().prop(1).is_some());
    b.step();
    assert!(
        b.world().prop(0).is_none() && b.world().prop(1).is_none(),
        "one terminal transition must remove both shells in this tick"
    );
    let remains: Vec<_> = b
        .world()
        .props()
        .filter(|p| b.structures().replaced_by(p.id).is_some())
        .collect();
    assert_eq!(
        remains
            .iter()
            .map(|p| (
                p.id,
                p.center.x,
                p.center.y,
                p.half.x,
                p.half.y,
                p.half.z,
                p.base_z,
                b.structures().replaced_by(p.id),
                b.world().building_of(p.id)
            ))
            .collect::<Vec<_>>(),
        vec![
            (3, 400.0, 300.0, 4.0, 3.0, 1.0, 0.0, Some(0), Some(0)),
            (4, 406.0, 301.0, 2.0, 2.0, 1.0, 0.0, Some(1), Some(0)),
        ]
    );
    assert_eq!(b.structures().hp(b.world(), 0), None);
    assert_eq!(b.structures().hp(b.world(), 1), None);
}

#[test]
fn singleton_destroyable_remains_keep_the_original_digest_trace() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/buildings/singleton-chain.json"
    ))
    .unwrap();
    let mut setup: ScenarioDefinition = original_setup(oracle["scenario"].clone());
    setup.map = crate::common::physical_map(setup.map, &setup.rules);
    let mut b = Battle::new(&setup, 11);
    for row in oracle["rows"].as_array().unwrap() {
        while b.tick() < row["tick"].as_u64().unwrap() {
            b.step();
        }
        assert_eq!(
            format!("{:016x}", b.digest()),
            row["digest"].as_str().unwrap(),
            "tick {}",
            b.tick()
        );
        for (side, name) in [(Side::Blue, "blue"), (Side::Red, "red")] {
            assert_eq!(
                original_observation(b.observe(side)),
                row[name].as_str().unwrap(),
                "tick {} {name}",
                b.tick()
            );
        }
        if b.tick() >= 11 {
            assert_eq!(b.structures().hp(b.world(), 0), None);
        }
        if (11..=13).contains(&b.tick()) {
            assert_eq!(
                b.structures().hp(b.world(), 1),
                Some(200.0 - (b.tick() - 11) as f64 * 95.0)
            );
        }
        if b.tick() >= 14 {
            assert_eq!(b.structures().hp(b.world(), 1), None);
            assert!(b.world().prop(2).is_some());
        }
    }
}

#[test]
fn compound_damageable_remains_have_one_fresh_integrity_per_state() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/buildings/singleton-chain.json"
    ))
    .unwrap();
    let mut setup = compound_setup(json!((1..=14)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    setup.rules = serde_json::from_value(oracle["scenario"]["rules"].clone()).unwrap();
    let mut b = Battle::new(&setup, 11);
    for _ in 0..11 {
        b.step();
    }
    for id in [0, 1] {
        assert_eq!(b.structures().hp(b.world(), id), None);
        assert_eq!(b.world().structure_owner(id), None);
    }
    for tick in 11..=14 {
        if tick > 11 {
            b.step();
        }
        for id in [3, 4] {
            assert_eq!(b.world().building_of(id), Some(0));
            assert_eq!(
                b.world().structure_owner(id),
                if tick < 14 { Some(3) } else { None }
            );
            assert_eq!(
                b.structures().hp(b.world(), id),
                if tick < 14 {
                    Some(200.0 - (tick - 11) as f64 * 95.0)
                } else {
                    None
                }
            );
        }
    }
    for id in [5, 6] {
        assert!(b.world().prop(id).is_some());
        assert_eq!(b.world().building_of(id), Some(0));
        assert_eq!(b.world().structure_owner(id), Some(5));
    }
    assert_eq!(b.structures().replaced_by(5), Some(3));
    assert_eq!(b.structures().replaced_by(6), Some(4));
}

#[test]
fn seating_uses_only_exposed_compound_spans() {
    let setup = compound_setup(json!([]));
    let b = Battle::new(&setup, 11);
    let slots = sim::garrison::slots(b.world(), b.world().prop(0).unwrap(), &setup.rules);
    let east: Vec<_> = slots
        .iter()
        .filter(|s| s.slot.facade == 0)
        .map(|s| [s.position.x, s.position.y])
        .collect();
    let standoff = setup.rules.garrison.slot_standoff_m;
    assert_eq!(
        east,
        vec![
            [404.0 + standoff, 297.75],
            [408.0 + standoff, 299.25],
            [408.0 + standoff, 300.75],
            [408.0 + standoff, 302.25]
        ]
    );
    assert!(slots
        .iter()
        .all(|s| s.position.x != 404.0 + standoff || s.position.y < 299.0));
}

#[test]
fn seeing_one_part_reveals_all_replacements_but_hidden_sides_keep_prior_state() {
    let mut setup = compound_setup(json!((1..=11)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[370,300],"engagement":"return_fire_only"},{"side":"red","kind":"rifle","position":[250,100],"engagement":"return_fire_only"}])).unwrap();
    let mut rules = serde_json::to_value(&setup.rules).unwrap();
    // A short-range observer sees the near part; the other side is wholly hidden.
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "rifle",
        json!({"sensors":{"ground_m":28}}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let mut b = Battle::new(&setup, 11);
    for _ in 0..10 {
        b.step();
    }
    assert!(b
        .observe(Side::Blue)
        .ground_visibility
        .visible(396.0, 300.0));
    assert!(!b
        .observe(Side::Blue)
        .ground_visibility
        .visible(407.0, 301.0));
    b.step();
    b.step();
    assert!(
        b.world().prop(0).is_none(),
        "hp {:?}",
        b.structures().hp(b.world(), 0)
    );
    let seen: Vec<_> = b
        .observe(Side::Blue)
        .known_props
        .iter()
        .map(|p| p.replaces)
        .collect();
    assert_eq!(seen, vec![Some(0), Some(1)]);
    assert!(b.observe(Side::Red).known_props.is_empty());
    crate::common::order(
        &mut b,
        Side::Red,
        1,
        contract::command::Order::Move {
            units: vec![contract::ids::UnitId(1)],
            goal: [390.0, 330.0],
            route: contract::command::RoutePolicy::Fastest,
            gesture: 1,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    for _ in 0..3600 {
        b.step();
        if !b.observe(Side::Red).known_props.is_empty() {
            break;
        }
    }
    assert_eq!(
        b.observe(Side::Red)
            .known_props
            .iter()
            .map(|p| p.replaces)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)],
        "red position {:?}, state {:?}, orders {:?}",
        b.unit(contract::ids::UnitId(1)).unwrap().position,
        b.unit(contract::ids::UnitId(1)).unwrap().state,
        b.unit(contract::ids::UnitId(1)).unwrap().orders
    );
}

#[test]
fn different_clicked_parts_claim_one_building_in_the_same_tick() {
    let mut setup = compound_setup(json!([]));
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[425,301]},{"side":"blue","kind":"rifle","position":[425,340]}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    let order = |seq, unit, building| contract::command::CommandEnvelope {
        side: Side::Blue,
        seq,
        queued: false,
        order: contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(unit)],
            building,
        },
    };
    assert_eq!(b.accept(order(1, 0, 1)).error, None);
    assert_eq!(
        b.accept(order(2, 1, 0)).error,
        Some(contract::command::OrderError::BuildingOccupied)
    );
    for _ in 0..600 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    assert_eq!(
        b.observe(Side::Blue).own[0]
            .garrison
            .as_ref()
            .unwrap()
            .building,
        0
    );
}

#[test]
fn stepped_facade_eyes_stay_outside_every_shell_and_fire_past_the_whole_owner() {
    let mut setup = compound_setup(json!([]));
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[425,301],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 1,
        },
    );
    for _ in 0..600 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    let unit = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(unit.garrisoned());
    let observed = b.observe(Side::Blue).own[0].garrison.as_ref().unwrap();
    assert_eq!(
        (observed.center, observed.half),
        ([402.0, 300.0], [6.0, 3.0]),
        "the delivered footprint must include every current physical part"
    );
    let eyes = sim::garrison::facade_eyes(unit, &setup.rules);
    for eye in &eyes {
        assert!(
            b.world()
                .props()
                .filter(|p| b.world().building_of(p.id) == Some(0))
                .all(|p| !p.footprint().contains(eye.xy(), 0.0)),
            "eye {eye:?} must stand outside every physical part"
        );
    }
    let east: Vec<_> = eyes.iter().filter(|e| e.x > 404.0 && e.y < 303.0).collect();
    assert_eq!(east.len(), 2);
    for (eye, expected) in east.iter().zip([[404.45, 297.75], [408.45, 300.75]]) {
        assert!((eye.x - expected[0]).abs() < 1e-12 && (eye.y - expected[1]).abs() < 1e-12);
    }
    for eye in east {
        assert!(b
            .world()
            .sight_clear(*eye, sim::math::v3(430.0, eye.y, eye.z)));
    }
    assert!(b
        .world()
        .raycast_past(
            sim::math::v3(390.0, 301.0, 2.0),
            sim::math::v3(1.0, 0.0, 0.0),
            30.0,
            Some(0)
        )
        .is_none());
    assert!(b.world().segment_clear_except(
        sim::math::v3(390.0, 301.0, 2.0),
        sim::math::v3(420.0, 301.0, 2.0),
        Some(1)
    ));
}

#[test]
fn public_geometry_ids_use_one_exact_limb_pair_and_keep_physical_columns() {
    let setup = compound_setup(json!([]));
    let b = Battle::new(&setup, 11);
    let layout: Value =
        serde_json::from_str(&sim::world::export::layout_json(b.world().types())).unwrap();
    assert_eq!(
        layout["propFields"],
        json!(["idLo", "idHi", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"])
    );
    let building = setup.rules.catalog.props().kind("building").0 as f32;
    let tooth = setup.rules.catalog.props().kind("tooth").0 as f32;
    assert_eq!(
        b.world()
            .export_props()
            .chunks_exact(10)
            .map(|p| p.to_vec())
            .collect::<Vec<_>>(),
        vec![
            vec![0., 0., building, 400., 300., 0., 4., 3., 4., 0.],
            vec![1., 0., building, 406., 301., 0., 2., 2., 4., 0.],
            vec![2., 0., tooth, 700., 550., 0., 0.6, 0.6, 0.6, 0.]
        ]
    );
    let mut wide = b.world().prop(1).unwrap().clone();
    wide.id = u32::MAX;
    assert_eq!(
        sim::world::export::prop_record(&wide),
        [65535., 65535., building, 406., 301., 0., 2., 2., 4., 0.]
    );
    let metadata: Value = serde_json::from_str(&b.world().export_buildings()).unwrap();
    assert_eq!(
        metadata,
        json!({"catalogueHash":setup.map.template_catalog_hash,"buildings":[{"owner":0,"kind":"building","templateId":setup.map.buildings[0].geometry.template_id,"category":"attached_home","regionalFamily":"api_fixture","parts":[{"part":"main","prop":0},{"part":"wing","prop":1}]}]})
    );
}

#[test]
fn garrison_capability_requires_an_authored_aggregate_and_an_exposed_span() {
    let rules = crate::common::rules();
    let ordinary:contract::map::MapDefinition=serde_json::from_value(json!({"size":[100,100],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[{"kind":"building","center":[50,50],"yaw":0,"half_extents":[4,3,4]}]})).unwrap();
    assert!(
        std::panic::catch_unwind(|| sim::world::WorldGeometry::new(&ordinary, &rules)).is_err(),
        "a garrison body must not bypass the physical aggregate owner"
    );
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].geometry.edges.clear();
    setup.map.buildings[0].geometry.entrances = None;
    assert!(
        std::panic::catch_unwind(|| Battle::new(&setup, 11)).is_err(),
        "unresolved exposure cannot claim a squad with no seats or eyes"
    );
}

#[test]
fn building_membership_and_added_physical_facts_are_authoritative_digest_state() {
    let setup = compound_setup(json!([]));
    let baseline = Battle::new(&setup, 11).digest();
    let mut other = setup.clone();
    other.map.buildings[0].owner = 1;
    assert_ne!(
        Battle::new(&other, 11).digest(),
        baseline,
        "equal physical poses cannot hide a different integrity owner"
    );
    other = setup.clone();
    other.map.buildings[0].geometry.floor_z = Some(vec![0.0, 4.0]);
    assert_ne!(
        Battle::new(&other, 11).digest(),
        baseline,
        "complete physical floor facts must not disappear from identity"
    );
    other = setup.clone();
    other.map.buildings[0].geometry.frame.yaw = 0.9;
    assert_ne!(
        Battle::new(&other, 11).digest(),
        baseline,
        "the frame that groups exposed firing directions is authoritative"
    );
}

#[test]
fn aggregate_motion_is_rejected_while_ordinary_movable_bodies_keep_working() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].kind = "crate".into();
    assert!(
        std::panic::catch_unwind(|| Battle::new(&setup, 11)).is_err(),
        "a one-prop shove cannot tear apart a placed aggregate"
    );
    let world = crate::common::flat(
        [100.0, 100.0],
        r#","props":[{"kind":"crate","center":[50,50],"yaw":0,"half_extents":[1,1,1]}]"#,
    );
    let mut world = world;
    world.move_prop(0, sim::math::v2(55.0, 52.0), 0.5, 1);
    let p = world.prop(0).unwrap();
    assert_eq!((p.center.x, p.center.y, p.yaw), (55.0, 52.0, 0.5));
    assert_eq!(world.structure_owner(0), Some(0));
    assert_eq!(world.building_of(0), None);
}

#[test]
fn ordinary_authored_remains_keep_their_source_while_dynamic_remains_have_none() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/buildings/singleton-chain.json"
    ))
    .unwrap();
    let mut rules = original["scenario"]["rules"].clone();
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "wall",
        json!({"body":{"hp":1000},"destroyed":{"into":{"height_m":2,"prop":"damaged_shell"}}}),
    );
    let mut events = vec![
        json!({"tick":1,"add_prop":{"kind":"wall","center":[500,300],"yaw":0,"half_extents":[2,2,4]}}),
    ];
    for tick in 2..=15 {
        for x in [403, 503] {
            events.push(json!({"tick":tick,"burst":{"point":[x,300],"weapon":"tank_he"}}));
        }
    }
    let setup:ScenarioDefinition=serde_json::from_value(json!({"map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[{"kind":"wall","center":[400,300],"yaw":0,"half_extents":[2,2,4]}]},"rules":rules,"units":[{"side":"blue","kind":"rifle","position":[450,340],"engagement":"return_fire_only"}],"events":events,"scripts":[]})).unwrap();
    let mut b = Battle::new(&setup, 11);
    for _ in 0..18 {
        b.step();
    }
    assert_eq!(
        (b.world().authored_prop(4), b.world().authored_prop(5)),
        (Some(0), None)
    );
    assert_eq!(
        (b.structures().replaced_by(4), b.structures().replaced_by(5)),
        (Some(2), Some(3))
    );
    let known = b
        .observe(Side::Blue)
        .known_props
        .iter()
        .map(|p| {
            (
                p.id,
                p.authored_prop,
                p.replaces,
                p.building,
                p.structure_owner,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        known,
        vec![
            (4, Some(0), Some(2), None, None),
            (5, None, Some(3), None, None)
        ]
    );
}

#[test]
fn authored_parts_share_one_bounded_dense_namespace_with_ordinary_props() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].parts[1].prop = 2;
    setup.map.props[0].id = None;
    let rows = setup.map.authored_props().unwrap();
    assert_eq!(
        rows.iter()
            .map(|(id, p)| (*id, p.center))
            .collect::<Vec<_>>(),
        vec![(0, [400., 300.]), (1, [700., 550.]), (2, [406., 301.])]
    );
    let mut invalid = setup.map.clone();
    invalid.props[0].id = Some(0);
    assert!(invalid
        .authored_props()
        .unwrap_err()
        .contains("unique and dense"));
    invalid = setup.map.clone();
    invalid.buildings[0].parts[1].prop = u32::MAX;
    assert!(invalid
        .authored_props()
        .unwrap_err()
        .contains("unique and dense"));
    invalid = setup.map.clone();
    invalid.buildings[0].parts[1].part = "main".into();
    assert!(invalid
        .authored_props()
        .unwrap_err()
        .contains("uniquely cover"));
}

#[test]
fn a_holdable_replacement_uses_current_parts_and_its_fresh_owner() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/buildings/singleton-chain.json"
    ))
    .unwrap();
    let mut setup = compound_setup(json!((1..=11)
        .chain(400..=402)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    let mut rules = original["scenario"]["rules"].clone();
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "damaged_shell",
        json!({"body":{"garrison":true}}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[425,301],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    let expected = sim::garrison::slots(b.world(), b.world().prop(0).unwrap(), &setup.rules)
        .iter()
        .map(|s| s.position)
        .collect::<Vec<_>>();
    for _ in 0..12 {
        b.step();
    }
    assert_eq!(b.world().structure_owner(4), Some(3));
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 4,
        },
    );
    for _ in 0..300 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    let unit = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(unit.garrisoned());
    assert_eq!(
        unit.garrison
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .map(|s| s.position)
            .collect::<Vec<_>>(),
        expected,
        "an admitted replacement cannot enter an empty garrison"
    );
    assert_eq!(
        b.observe(Side::Blue).own[0]
            .garrison
            .as_ref()
            .unwrap()
            .building,
        3
    );
    while b.tick() < 403 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrison.is_none());
    assert!(b.world().prop(3).is_none() && b.world().prop(4).is_none());
    assert_eq!(
        (b.world().structure_owner(5), b.world().structure_owner(6)),
        (Some(5), Some(5))
    );
}
