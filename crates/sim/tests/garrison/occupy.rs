//! Combined building intents, through public preview and admission.
use super::*;

#[test]
fn a_building_group_enters_one_squad_and_gathers_its_selected_companions() {
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "recon", "position": [350, 300] },
            { "side": "blue", "kind": "rifle", "position": [320, 320] },
            { "side": "blue", "kind": "tank", "position": [300, 280] }
        ]),
        1,
    );
    let order: Order = serde_json::from_value(json!({
        "kind": "occupy_building", "units": [0, 1, 2], "building": BUILDING,
        "gesture": 1, "facing": null
    }))
    .expect("combined building intent");
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let outcome = serde_json::to_value(&ack).unwrap();
    assert_eq!(outcome["building"]["entrant"]["unit"], 0);
    let destinations = ack.placement.unwrap().destinations;
    assert_eq!(
        destinations.iter().map(|d| d.unit.0).collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(destinations.iter().all(|d| d.placed), "{destinations:?}");
    until(&mut b, 1600, "entry and companion arrival", |b| {
        inside(b, 0)
            && destinations.iter().all(|d| {
                let u = b.unit(d.unit).unwrap();
                u.orders.is_empty() && (u.position.xy() - v2(d.goal[0], d.goal[1])).length() < 2.0
            })
    });
    assert!(!inside(&b, 1));
    assert!(destinations.iter().all(|d| d.goal[0] < CENTRE[0] - HALF[0]));
}

fn occupy(units: &[u32], gesture: u64) -> Order {
    Order::OccupyBuilding {
        units: units.iter().copied().map(UnitId).collect(),
        building: BUILDING,
        gesture,
        facing: None,
    }
}

#[test]
fn a_building_group_ranks_reachable_route_distance_instead_of_distance_to_the_wall() {
    let mut b = battle_with(
        json!([{ "kind": "wall", "center": [375, 300], "yaw": 0, "half_extents": [1, 20, 2] }]),
        json!([
            { "side": "blue", "kind": "recon", "position": [350, 300] },
            { "side": "blue", "kind": "recon", "position": [365, 260] }
        ]),
        json!([]),
        1,
    );
    let request = contract::command::BuildingPreviewRequest {
        units: vec![UnitId(0), UnitId(1)],
        building: BUILDING,
        ..Default::default()
    };
    let plan = b.preview_building(Side::Blue, &request).unwrap();
    assert_eq!(
        plan.entrant.as_ref().map(|e| e.unit),
        Some(UnitId(1)),
        "{plan:?}"
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.building, Some(plan));
    until(&mut b, 1800, "the shorter-route squad inside", |b| {
        inside(b, 1)
    });
    assert!(!inside(&b, 0));
}

#[test]
fn a_combined_building_admission_reserves_its_entry_before_the_next_tick() {
    let mut b = battle(west_squads(&["recon", "recon"]), 1);
    let first = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0], 1),
        queued: false,
    });
    assert_eq!(first.error, None);
    assert_eq!(first.building.unwrap().entrant.unwrap().unit, UnitId(0));
    let second = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 2,
        order: garrison(&[1]),
        queued: false,
    });
    assert_eq!(second.error, Some(OrderError::BuildingOccupied));
    let third = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 3,
        order: occupy(&[1], 2),
        queued: false,
    });
    assert_eq!(third.error, None, "the other squad can still gather");
    assert!(third.building.as_ref().unwrap().entrant.is_none());
    let goal = third.placement.as_ref().unwrap().destinations[0].goal;
    until(
        &mut b,
        1600,
        "reserved entrant inside and other squad outside",
        |b| inside(b, 0) && b.unit(UnitId(1)).unwrap().orders.is_empty(),
    );
    assert!(!inside(&b, 1));
    assert!((b.unit(UnitId(1)).unwrap().position.xy() - v2(goal[0], goal[1])).length() < 2.0);
}

#[test]
fn a_selected_holder_keeps_its_work_while_a_queued_building_group_gathers_others() {
    let mut b = battle(west_squads(&["recon", "recon"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "the selected holder inside", |b| inside(b, 0));
    c.ok(&mut b, Side::Blue, ground(0, [600.0, 300.0]));
    b.step();
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 3,
        order: occupy(&[0, 1], 1),
        queued: true,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    assert_eq!(
        ack.building
            .as_ref()
            .unwrap()
            .entrant
            .as_ref()
            .map(|e| e.unit),
        Some(UnitId(0))
    );
    let goals = ack.placement.unwrap().destinations;
    assert_eq!(goals.iter().map(|d| d.unit.0).collect::<Vec<_>>(), [1]);
    assert!(goals[0].placed);
    b.step();
    assert!(matches!(
        b.unit(UnitId(0)).unwrap().orders.front(),
        Some(sim::units::UnitOrder::Attack { .. })
    ));
    until(&mut b, 1200, "the companion arrived", |b| {
        b.unit(UnitId(1)).unwrap().orders.is_empty()
    });
    assert!(inside(&b, 0));
}

#[test]
fn a_building_entry_can_use_the_rest_of_a_facade_when_its_nearest_projection_is_blocked() {
    let mut b = battle_with(
        json!([{ "kind": "wall", "center": [385, 300], "yaw": 0, "half_extents": [1, 5, 2] }]),
        json!([{ "side": "blue", "kind": "recon", "position": [365, 300] }]),
        json!([]),
        1,
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0], 1),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let entry = ack
        .building
        .unwrap()
        .entrant
        .expect("an exposed alternative entry");
    assert!((entry.approach[1] - 300.0).abs() > 5.0, "{entry:?}");
    until(&mut b, 1400, "arrival at the exposed entry", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    assert!(
        (b.unit(UnitId(0)).unwrap().position.xy() - v2(entry.approach[0], entry.approach[1]))
            .length()
            < 1.0,
        "entry starts at its accepted approach"
    );
    until(&mut b, 1400, "entry beyond the blocked projection", |b| {
        inside(b, 0)
    });
}

#[test]
fn a_future_garrison_behind_an_indefinite_attack_is_unproven_until_replaced() {
    for queued in [true, false] {
        let mut b = battle(west_squads(&["recon"]), 1);
        let mut c = Commander::new();
        c.ok(&mut b, Side::Blue, ground(0, [600.0, 300.0]));
        assert_eq!(c.send(&mut b, Side::Blue, garrison(&[0]), true), None);
        b.step();
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: 3,
            order: occupy(&[0], 1),
            queued,
        });
        assert_eq!(
            ack.building.as_ref().unwrap().entrant.is_some(),
            !queued,
            "queued {queued}: {ack:?}"
        );
        if queued {
            assert_eq!(ack.error, Some(OrderError::NoValidDestination));
            assert!(
                ack.building.as_ref().unwrap().unproven,
                "an indefinite prefix is not a proof of impossibility"
            );
            b.step();
            assert_eq!(
                b.unit(UnitId(0)).unwrap().orders.len(),
                2,
                "the failed queued intent preserves its prefix"
            );
        } else {
            assert_eq!(ack.error, None);
            until(&mut b, 1600, "replacement entry", |b| inside(b, 0));
        }
    }
}

#[test]
fn a_nearby_entry_is_not_starved_by_farther_candidates_earlier_in_unit_order() {
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "recon", "position": [30, 30] },
            { "side": "blue", "kind": "recon", "position": [350, 300] }
        ]),
        json!([]),
        json!([]),
    );
    setup.rules.navigation.move_validation_work = 60000;
    let mut b = Battle::new(&setup, 1);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    assert_eq!(
        ack.building
            .as_ref()
            .unwrap()
            .entrant
            .as_ref()
            .map(|e| e.unit),
        Some(UnitId(1)),
        "{ack:?}"
    );
    until(&mut b, 1500, "the nearer squad inside", |b| inside(b, 1));
}

#[test]
fn a_building_preview_after_live_forest_clearance_rebases_its_scratch_navigation() {
    let setup = common::scenario(
        &json!({"size":[400,80],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[{"kind":"building","center":[330,40],"yaw":0,"half_extents":[12,9,4]}],
            "forests":[{"shape":{"kind":"polygon","ring":[[70,0],[130,0],[130,80],[70,80]]}}]
        })
        .to_string(),
        json!([{"side":"blue","kind":"tank","position":[15,40],"yaw":0,"engagement":"return_fire_only"}]),
        json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [185.0, 40.0],
            route: RoutePolicy::Shortest,
            direction: Default::default(),
            facing: None,
        },
    );
    until(&mut b, 3000, "the tank cleared the forest", |b| {
        b.world().cleared_cells() > 0 && b.unit(UnitId(0)).unwrap().position.x > 150.0
    });
    let before = b.digest();
    let plan = b
        .preview_building(
            Side::Blue,
            &contract::command::BuildingPreviewRequest {
                units: vec![UnitId(0)],
                building: 0,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(plan.entrant.is_none());
    assert!(plan.destinations[0].placed, "{plan:?}");
    assert_eq!(b.digest(), before);
}
