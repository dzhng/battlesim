//! Aggregate building ownership, exercised through scenario loading and Battle.
use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

/// The shipped rules with round numbers: a building of 1000 integrity, and
/// a shell that takes 95 from it bursting a metre away.
fn compound_rules() -> Value {
    let mut rules = crate::common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "building",
        json!({"body":{"hp":1000,"hp_scale":"fixed"}}),
    );
    rules["weapons"]["tank_he"]["structural_damage"] = json!(100);
    rules["weapons"]["tank_he"]["blast_radius_m"] = json!(20);
    rules
}

/// `rules` with remains that can be destroyed in their turn. The shipped
/// ruin has no integrity, so `kind` falls to a 200-integrity shell first.
fn with_damageable_remains(mut rules: Value, kind: &str) -> Value {
    rules["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({"props":{"damaged_shell":{
            "extends":"wall",
            "body":{"hp":200},
            "destroyed":{"into":{"prop":"ruin","height_m":1}},
            "appearance":{"drawn_by":"ruin","remains_state":"ruin"}
        }}}));
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        kind,
        json!({"body":{"hp":1000},"destroyed":{"into":{"prop":"damaged_shell","height_m":2}}}),
    );
    rules
}

fn compound_setup(events: Value) -> ScenarioDefinition {
    let descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
        include_str!("../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap();
    compound_with_descriptor(descriptor, events)
}

fn compound_with_descriptor(
    descriptor: contract::templates::BuildingTemplateDescriptor,
    events: Value,
) -> ScenarioDefinition {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
    let geometry = descriptor
        .materialize(contract::templates::PlacementFrame {
            translation: [400.0, 300.0, 0.0],
            yaw: 0.0,
        })
        .unwrap();
    let rules = compound_rules();
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
fn compound_damageable_remains_have_one_fresh_integrity_per_state() {
    let mut setup = compound_setup(json!((1..=14)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    setup.rules =
        serde_json::from_value(with_damageable_remains(compound_rules(), "building")).unwrap();
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
    let slots = sim::garrison::building_seats(b.world().building(0).unwrap(), &setup.rules);
    let east: Vec<_> = slots
        .iter()
        .filter(|s| s.slot.facade == 0)
        .map(|s| [s.position.x, s.position.y])
        .collect();
    let standoff = setup.rules.garrison.slot_standoff_m;
    assert_eq!(
        east,
        vec![[408.0 + standoff, 301.0], [408.0 + standoff, 301.0]]
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
    assert_eq!(
        east.len(),
        1,
        "parallel exposed spans share one directional eye"
    );
    for (eye, expected) in east.iter().zip([[408.45, 301.0]]) {
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
        json!({"catalogueHash":setup.map.template_catalog_hash,"buildings":[{"owner":0,"kind":"building","templateId":setup.map.buildings[0].geometry.template_id,"category":"attached_home","regionalFamily":"api_fixture","frame":{"translation":[400.0,300.0,0.0],"yaw":0.0},"parts":[{"part":"main","prop":0},{"part":"wing","prop":1}]}]})
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
    let mut setup = compound_setup(json!([]));
    let mut rows = serde_json::to_value(&setup.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rows,
        "props",
        "building",
        json!({"destroyed":{"into":{"prop":"crate"}}}),
    );
    setup.rules = serde_json::from_value(rows).unwrap();
    assert!(
        std::panic::catch_unwind(|| Battle::new(&setup, 11)).is_err(),
        "a movable replacement cannot tear apart an aggregate after collapse"
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
    let rules = with_damageable_remains(compound_rules(), "wall");
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
    let mut setup = compound_setup(json!((1..=11)
        .chain(400..=402)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    let mut rules = with_damageable_remains(compound_rules(), "building");
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "damaged_shell",
        json!({"body":{"garrison":true}}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[425,301],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
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
    assert!(
        unit.garrison
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.position.z < b.world().prop(3).unwrap().top_z()),
        "a holdable replacement cannot seat soldiers above its live roof"
    );

    assert!(
        unit.garrison
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.position.z == 0.0),
        "only the original ground-floor bays remain below this two-metre shell"
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

#[test]
fn floor_band_seats_follow_exposed_bays_and_stop_at_the_third_floor() {
    let mut setup = compound_setup(json!([]));
    setup.rules.buildings.capacity_soldiers = 32;
    setup.map.buildings[0].geometry.floor_z = Some(vec![0.0, 3.0, 6.0, 7.0]);
    let b = Battle::new(&setup, 11);
    let definition = &setup.map.buildings[0].geometry;
    let seats = sim::garrison::building_seats(b.world().building(0).unwrap(), &setup.rules);
    assert!(
        seats.iter().any(|s| s.position.z == 6.0),
        "third floor has seats"
    );
    assert!(seats
        .iter()
        .all(|s| [0.0, 3.0, 6.0].contains(&s.position.z)));
    for seat in &seats {
        let edge = &definition.edges[seat.edge];
        assert!(edge.exposed);
        let bay = seat.slot.position
            - sim::math::v2(edge.normal[0], edge.normal[1]) * setup.rules.garrison.slot_standoff_m;
        assert!(
            edge.bays
                .as_ref()
                .unwrap()
                .iter()
                .any(|p| (bay - sim::math::v2(p[0], p[1])).length() < 1e-8),
            "seat lies at a physical bay: {seat:?}"
        );
    }
    assert_eq!(seats.len(), 30);
}

#[test]
fn known_floor_geometry_does_not_invent_unresolved_facade_bays() {
    let mut setup = compound_setup(json!([]));
    let geometry = &mut setup.map.buildings[0].geometry;
    geometry.floor_z = Some(vec![0.0, 3.0, 6.0]);
    let unresolved = geometry
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.exposed)
        .max_by(|(_, a), (_, b)| {
            (a.span_m[1] - a.span_m[0]).total_cmp(&(b.span_m[1] - b.span_m[0]))
        })
        .unwrap()
        .0;
    geometry.edges[unresolved].bays = None;
    let b = Battle::new(&setup, 11);
    let seats = sim::garrison::building_seats(b.world().building(0).unwrap(), &setup.rules);
    assert!(!seats.is_empty(), "resolved facades still supply seats");
    assert!(
        seats.iter().all(|s| s.edge != unresolved),
        "unresolved bays cannot supply physical seats"
    );
}

#[test]
fn abundant_facade_bays_never_admit_more_than_32_seats() {
    let mut setup = compound_setup(json!([]));
    setup.rules.buildings.capacity_soldiers = 100;
    let geometry = &mut setup.map.buildings[0].geometry;
    geometry.floor_z = Some(vec![0.0, 3.0, 6.0]);
    for edge in geometry.edges.iter_mut().filter(|e| e.exposed) {
        let [a, b] = edge.span;
        edge.bays = Some(
            (1..=20)
                .map(|j| {
                    let t = j as f64 / 21.0;
                    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
                })
                .collect(),
        );
    }
    let b = Battle::new(&setup, 11);
    let seats = sim::garrison::building_seats(b.world().building(0).unwrap(), &setup.rules);
    assert_eq!(seats.len(), 32);
    for f in 0..4 {
        assert_eq!(seats.iter().filter(|s| s.slot.facade == f).count(), 8);
    }
}

#[test]
fn each_directional_eye_is_an_occupied_seat_on_its_highest_held_floor() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].geometry.floor_z = Some(vec![0.0, 3.0, 6.0]);
    setup.units = serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[380,300],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
        },
    );
    for _ in 0..1000 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    let u = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(u.garrisoned());
    let g = u.garrison.as_ref().unwrap();
    let eyes = sim::garrison::facade_eyes(u, &setup.rules);
    assert!(eyes.len() <= 4);
    for f in 0..4 {
        let held: Vec<_> = u
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .filter_map(|(k, _)| g.seat(k))
            .filter(|s| s.slot.facade == f)
            .collect();
        if held.is_empty() {
            continue;
        }
        let highest = held
            .iter()
            .map(|s| s.position.z)
            .max_by(f64::total_cmp)
            .unwrap();
        assert!(
            eyes.iter()
                .any(|eye| held.iter().any(|s| s.position.z == highest
                    && *eye
                        == s.position
                            + sim::math::v3(0.0, 0.0, setup.rules.physics.infantry_eye_m))),
            "group {f} has an occupied highest-band eye, rather than an average: {eyes:?}"
        );
    }
}

#[test]
fn a_third_floor_garrison_sees_over_a_two_storey_obstacle() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].geometry.floor_z = Some(vec![0.0, 3.0, 6.0]);
    setup.map.props.push(
        serde_json::from_value(
            json!({"id":3,"kind":"wall","center":[440,301],"yaw":0,"half_extents":[3,20,2.5]}),
        )
        .unwrap(),
    );
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"rifle","position":[380,300],"engagement":"return_fire_only"},
        {"side":"red","kind":"tank","position":[580,301],"engagement":"return_fire_only"}
    ]))
    .unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
        },
    );
    for _ in 0..1000 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    for _ in 0..30 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrisoned());
    assert_eq!(
        b.observe(Side::Blue).identified.len(),
        1,
        "highest occupied facade sees the tank over the intervening five-metre shop"
    );
    crate::common::order(
        &mut b,
        Side::Blue,
        2,
        contract::command::Order::Attack {
            units: vec![contract::ids::UnitId(0)],
            target: contract::command::TargetRef::Ground {
                point: [580.0, 301.0, 0.0],
            },
        },
    );
    let mut fired = false;
    for _ in 0..400 {
        b.step();
        if b.rounds().any(|(flight, round)| {
            round.unit == contract::ids::UnitId(0)
                && b.arsenal().weapons[round.weapon].id == "rifle"
                && flight.position.z > 6.0
        }) {
            assert_eq!(
                b.world().prop(3).unwrap().top_z(),
                5.0,
                "the shop still stands when the upper-floor rifle fires"
            );
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "a gun held on floor three fires over the shop from its real muzzle"
    );
}

#[test]
fn building_integrity_scales_with_its_footprint_and_floor_bands() {
    let mut setup = compound_setup(json!([]));
    let mut rows = serde_json::to_value(&setup.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rows,
        "props",
        "building",
        json!({"body":{"hp":2,"hp_scale":"building_floor_bands","garrison":false}}),
    );
    setup.rules = serde_json::from_value(rows).unwrap();
    let b = Battle::new(&setup, 11);
    assert_eq!(
        b.structures().hp(b.world(), 0),
        Some(256.0),
        "64 square metres times two floor bands times coefficient two"
    );
    assert_eq!(
        b.structures().hp(b.world(), 1),
        Some(256.0),
        "parts share that integrity"
    );
}

#[test]
fn low_rise_collapse_height_scales_and_clamps_from_the_destroyed_row() {
    for (height, expected) in [(8.0, 2.0), (16.0, 4.0), (32.0, 6.0)] {
        let mut setup = compound_setup(json!((1..=11)
            .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
            .collect::<Vec<_>>()));
        let geometry = &mut setup.map.buildings[0].geometry;
        geometry.height_m = height;
        geometry.floor_z = Some(vec![0.0, height / 2.0]);
        for part in &mut geometry.parts {
            part.half_extents[2] = height / 2.0;
        }
        for edge in &mut geometry.edges {
            edge.top_z = height;
        }
        let mut rows = serde_json::to_value(&setup.rules).unwrap();
        sim::fixtures::patch_catalog(
            &mut rows,
            "props",
            "building",
            json!({"destroyed":{"into":{"building":{"height_fraction":0.25,"max_height_m":6,"collapse_max_floors":6,"gutted_prop":"gutted"}}}}),
        );
        setup.rules = serde_json::from_value(rows).unwrap();
        let mut b = Battle::new(&setup, 11);
        for _ in 0..11 {
            b.step();
        }
        assert!(b.world().prop(0).is_none());
        for prop in b
            .world()
            .props()
            .filter(|p| b.structures().replaced_by(p.id).is_some())
        {
            assert_eq!(2.0 * prop.half.z, expected);
        }
    }
}

#[test]
fn tall_buildings_leave_a_terminal_ungarrisonable_shell() {
    let mut setup = compound_setup(json!((400..=410)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    let geometry = &mut setup.map.buildings[0].geometry;
    geometry.height_m = 24.0;
    geometry.floor_z = Some((0..7).map(|n| n as f64 * 3.0).collect());
    for part in &mut geometry.parts {
        part.half_extents[2] = 12.0;
    }
    for edge in &mut geometry.edges {
        edge.top_z = 24.0;
    }
    let mut rows = serde_json::to_value(&setup.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rows,
        "props",
        "building",
        json!({"destroyed":{"into":{"building":{"height_fraction":0.25,"max_height_m":6,"collapse_max_floors":6,"gutted_prop":"gutted"}}}}),
    );
    setup.rules = serde_json::from_value(rows).unwrap();
    setup.rules.garrison.survival_probability_on_collapse = 1.0;
    setup.units = serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[380,300],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
        },
    );
    for _ in 0..300 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrisoned());
    while b.tick() < 410 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrison.is_none());
    let shells: Vec<_> = b
        .world()
        .props()
        .filter(|p| b.structures().replaced_by(p.id).is_some())
        .cloned()
        .collect();
    assert_eq!(shells.len(), 2);
    for shell in &shells {
        assert_eq!(2.0 * shell.half.z, 24.0);
        assert_eq!(b.world().types().id(shell.kind), "gutted");
        assert!(shell.body.occludes && shell.body.stops_rounds && !shell.body.garrison);
        assert_eq!(
            shell.body.cover_tier,
            Some(contract::scenario::CoverTier::Medium)
        );
        assert_eq!(b.structures().hp(b.world(), shell.id), None);
        let mut integrity = b.structures().clone();
        assert!(
            !integrity.damage(b.world(), shell.id, 1e9),
            "no second terminal transition"
        );
    }
    let ack = b.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        seq: 2,
        queued: false,
        order: contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: shells[0].id,
        },
    });
    assert_eq!(ack.error, Some(contract::command::OrderError::NotABuilding));
}

#[test]
fn a_squad_reinforced_during_entry_never_enters_only_partly_seated() {
    let mut setup = compound_setup(json!([]));
    setup.rules.buildings.capacity_soldiers = 7;
    setup.rules.garrison.enter_exit_s = 15.0;
    setup.rules.service.soldier_replacement_s = 0.1;
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"rifle","position":[380,300],"engagement":"return_fire_only","condition":{"casualties":2}},
        {"side":"blue","kind":"supply","position":[380,330],"engagement":"return_fire_only"}
    ])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
        },
    );
    for _ in 0..1400 {
        b.step();
    }
    let u = b.unit(contract::ids::UnitId(0)).unwrap();
    assert_eq!(
        u.members.iter().filter(|m| m.alive()).count(),
        8,
        "the truck restored the squad before seating"
    );
    assert!(!u.garrisoned(), "the whole reinforced squad no longer fits");
    assert!(u.orders.is_empty(), "refusal ends the entry attempt");
}

#[test]
fn seeing_the_near_part_learns_replacements_beyond_the_eyes_reach() {
    let mut descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
        include_str!("../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap();
    descriptor.parts[0].half_extents[0] = 50.0;
    descriptor.parts[1].center[0] = 52.0;
    for edge in &mut descriptor.edges {
        if edge.id == "main-north" || edge.id == "main-south" {
            edge.span_m = [-50.0, 50.0];
        }
    }
    let mut setup = compound_with_descriptor(
        descriptor,
        json!((1..=12)
            .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
            .collect::<Vec<_>>()),
    );
    setup.units = serde_json::from_value(json!([{
        "side":"blue","kind":"rifle","position":[328,300],
        "engagement":"return_fire_only"
    }]))
    .unwrap();
    let mut rules = serde_json::to_value(&setup.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "rifle",
        json!({"sensors":{"ground_m":30}}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let mut b = Battle::new(&setup, 11);
    for _ in 0..18 {
        b.step();
    }
    assert!(b.world().prop(0).is_none());
    assert!(!b
        .observe(Side::Blue)
        .ground_visibility
        .visible(452.0, 301.0));
    assert_eq!(
        b.observe(Side::Blue)
            .known_props
            .iter()
            .map(|p| p.replaces)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)],
        "the whole building's remains are learned"
    );
}

#[test]
fn upper_floor_collapse_deaths_land_on_the_remaining_physical_surface() {
    let mut setup = compound_setup(json!((400..=410)
        .map(|tick| json!({"tick":tick,"burst":{"point":[409,301],"weapon":"tank_he"}}))
        .collect::<Vec<_>>()));
    setup.map.relief = vec![contract::map::Relief::Mesa {
        rect: [0.0, 0.0, 800.0, 600.0],
        height_m: 2.0,
        side_degrees: 45.0,
    }];
    let geometry = &mut setup.map.buildings[0].geometry;
    geometry.frame.translation[2] = 2.0;
    geometry.floor_z = Some(vec![2.0, 5.0, 8.0]);
    for part in &mut geometry.parts {
        part.base_z += 2.0;
    }
    for edge in &mut geometry.edges {
        edge.base_z += 2.0;
        edge.top_z += 2.0;
    }
    for entrance in geometry.entrances.iter_mut().flatten() {
        entrance.position[2] += 2.0;
    }
    setup.rules.garrison.survival_probability_on_collapse = 0.0;
    setup.rules.weapons.get_mut("tank_he").unwrap().damage = 0.0;
    setup.units = serde_json::from_value(json!([{"side":"blue","kind":"rifle","position":[380,300],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::Garrison {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
        },
    );
    while b.tick() < 390 {
        b.step();
    }
    let seated = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(seated.garrisoned());
    assert!(seated.members.iter().any(|s| s.position.z == 8.0));
    let positions: Vec<_> = seated
        .members
        .iter()
        .map(|s| (s.id, s.position.xy()))
        .collect();
    while b.tick() < 410 {
        b.step();
    }
    assert!(b.world().prop(0).is_none());
    let observed = b.observe(Side::Blue);
    assert_eq!(observed.corpses.len(), positions.len());
    for (id, xy) in positions {
        let fallen = observed.corpses.iter().find(|f| f.soldier == id).unwrap();
        assert_eq!([fallen.position[0], fallen.position[1]], [xy.x, xy.y]);
        assert_eq!(
            fallen.position[2],
            b.world().surface_at(xy.x, xy.y).unwrap().z,
            "the old upper floor no longer supports soldier {id}"
        );
    }
}

#[test]
fn ordinary_destruction_cannot_enter_an_aggregate_only_state() {
    let mut raw = crate::common::game();
    sim::fixtures::patch_catalog(
        &mut raw,
        "props",
        "crate",
        json!({"destroyed":{"into":{"prop":"building","height_m":2}}}),
    );
    let rules: contract::scenario::Rules = serde_json::from_value(raw).unwrap();
    let map = serde_json::from_value(json!({"size":[100,100],"fog_cell_m":8,
    "height_grid_m":4,"slope_cutoff_deg":35,"props":[{
        "kind":"crate","center":[50,50],"yaw":0,"half_extents":[1,1,1]
    }]}))
    .unwrap();
    assert!(
        std::panic::catch_unwind(|| sim::world::WorldGeometry::new(&map, &rules)).is_err(),
        "a later scaled/garrison state needs an aggregate, just like its initial state"
    );
}

#[test]
fn later_ordinary_placement_cannot_create_an_aggregate_only_body() {
    let rules = crate::common::rules();
    let map = serde_json::from_value(json!({"size":[100,100],"fog_cell_m":8,
        "height_grid_m":4,"slope_cutoff_deg":35}))
    .unwrap();
    // Aggregate-capable rows may remain unused in the same catalog.
    let mut world = sim::world::WorldGeometry::new(&map, &rules);
    let body = serde_json::from_value(json!({"kind":"building","center":[50,50],
        "yaw":0,"half_extents":[4,3,4]}))
    .unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.add_prop(&body))).is_err(),
        "dynamic ordinary placement cannot invent missing building facts"
    );
}

#[test]
fn ordinary_movable_destruction_chains_remain_placeable() {
    let rules = crate::common::rules();
    let map = serde_json::from_value(json!({"size":[100,100],"fog_cell_m":8,
    "height_grid_m":4,"slope_cutoff_deg":35,"props":[{
        "kind":"parked_car","center":[50,50],"yaw":0,"half_extents":[2,1,1]
    }]}))
    .unwrap();
    let mut world = sim::world::WorldGeometry::new(&map, &rules);
    world.move_prop(0, sim::math::v2(55.0, 52.0), 0.5, 1);
    let body = world.prop(0).unwrap();
    assert_eq!((body.center.x, body.center.y, body.yaw), (55.0, 52.0, 0.5));
    let remains = serde_json::from_value(json!({"kind":"car_wreck","center":[55,52],
        "yaw":0.5,"half_extents":[2,1,0.35]}))
    .unwrap();
    let id = world.add_prop(&remains);
    assert_eq!(world.building_of(id), None);
}

#[test]
fn generated_tree_and_bridge_bodies_cannot_require_building_bulk() {
    let map: contract::map::MapDefinition = serde_json::from_value(json!({
        "size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "rivers":[{"points":[{"xy":[200,0],"width_m":12,"depth_m":1.5},
            {"xy":[200,300],"width_m":12,"depth_m":1.5}],"surface_z":-0.5}],
        "bridges":[{"deck":"bridge_deck","center":[200,150],"half_extents":[18,5],
            "yaw":0,"deck_z":0.1,"thickness_m":0.8}],
        "forests":[{"shape":{"kind":"polygon","ring":[[20,20],[80,20],[80,80],[20,80]]}}]
    }))
    .unwrap();
    let ordinary_rules = crate::common::rules();
    sim::world::WorldGeometry::new(&map, &ordinary_rules);
    let mut wrongly_accepted = Vec::new();
    for kind in [ordinary_rules.forests.tree.as_str(), "bridge_deck"] {
        let mut raw = crate::common::game();
        sim::fixtures::patch_catalog(
            &mut raw,
            "props",
            kind,
            json!({"body":{"hp":100,"hp_scale":"building_floor_bands"},"destroyed":"removed"}),
        );
        let rules = serde_json::from_value(raw).unwrap();
        if std::panic::catch_unwind(|| sim::world::WorldGeometry::new(&map, &rules)).is_ok() {
            wrongly_accepted.push(kind);
        }
    }
    assert!(
        wrongly_accepted.is_empty(),
        "generated {wrongly_accepted:?} have no aggregate floor/footprint facts"
    );
}
