//! Combined building intents, through public preview and admission.
use super::*;

#[test]
fn a_building_group_enters_one_squad_and_gathers_its_selected_companions() {
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "test_recon", "position": [350, 300] },
            { "side": "blue", "kind": "test_rifle", "position": [320, 320] },
            { "side": "blue", "kind": "test_tank", "position": [300, 280] }
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
fn replacing_a_hold_cancels_departure_behind_an_expiring_contact_attack() {
    let mut setup = common::scenario_with(
        &map(json!([
            {"kind":"wall", "center":[425,300], "yaw":0, "half_extents":[1,60,10]}
        ])),
        json!([
            {"side":"blue", "kind":"test_recon", "position":[350,300], "engagement":"return_fire_only"},
            {"side":"red", "kind":"test_rifle", "position":[450,300], "engagement":"return_fire_only"}
        ]),
        json!([{"tick":1000, "fire":{"unit":1}}]),
        json!([]),
    );
    setup.rules.sensors.contact_lifetime_s = 1.0;
    let mut b = Battle::new(&setup, 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 900, "selected holder inside", |b| inside(b, 0));
    until(&mut b, 1000, "hidden shooter reported", |b| {
        !b.observe(Side::Blue).contacts.is_empty()
    });
    assert!(b.observe(Side::Blue).identified.is_empty());
    let contact = b.observe(Side::Blue).contacts[0].clone();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id: contact.id },
        },
    );
    assert_eq!(c.send(&mut b, Side::Blue, exit(&[0]), true), None);
    b.step();
    assert!(matches!(
        b.unit(UnitId(0)).unwrap().orders.back(),
        Some(sim::units::UnitOrder::Exit)
    ));
    c.ok(&mut b, Side::Blue, occupy(&[0], 1));
    b.step();
    until(&mut b, 100, "contact expires", |b| {
        b.observe(Side::Blue).contacts.is_empty()
    });
    run(&mut b, ticks(5.0));
    assert!(
        inside(&b, 0),
        "expired attack cannot release the old departure"
    );
}

#[test]
fn queued_reentry_after_an_entering_squads_attack_and_exit_remains_unproven() {
    let mut setup = common::scenario_with(
        &map(json!([
            {"kind":"wall", "center":[425,300], "yaw":0, "half_extents":[1,60,10]}
        ])),
        json!([
            {"side":"blue", "kind":"test_recon", "position":[350,300], "engagement":"return_fire_only"},
            {"side":"blue", "kind":"test_tank", "position":[300,280], "engagement":"return_fire_only"},
            {"side":"red", "kind":"test_rifle", "position":[450,300], "engagement":"return_fire_only"}
        ]),
        json!([{"tick":5, "fire":{"unit":2}}]),
        json!([]),
    );
    setup.rules.sensors.contact_lifetime_s = 20.0;
    let mut b = Battle::new(&setup, 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 500, "selected squad entering", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    let contact = b.observe(Side::Blue).contacts[0].clone();
    for order in [
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id: contact.id },
        },
        exit(&[0]),
    ] {
        assert_eq!(c.send(&mut b, Side::Blue, order, true), None);
    }
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: occupy(&[0, 1], 1),
        queued: true,
    });
    assert_eq!(ack.error, None, "useful companion gathering still succeeds");
    let plan = ack.building.unwrap();
    assert!(
        plan.entrant.is_none() && plan.unproven,
        "queued attack can expire and expose the departure: {plan:?}"
    );
    b.step();
    assert!(matches!(
        b.unit(UnitId(0)).unwrap().orders.back(),
        Some(sim::units::UnitOrder::Exit)
    ));
    until(&mut b, 1200, "original queue leaves the building", |b| {
        b.observe(Side::Blue).contacts.is_empty() && b.unit(UnitId(0)).unwrap().garrison.is_none()
    });
}

#[test]
fn a_building_group_ranks_reachable_route_distance_instead_of_distance_to_the_wall() {
    let mut b = battle_with(
        json!([{ "kind": "wall", "center": [375, 300], "yaw": 0, "half_extents": [1, 20, 2] }]),
        json!([
            { "side": "blue", "kind": "test_recon", "position": [350, 300] },
            { "side": "blue", "kind": "test_recon", "position": [365, 260] }
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
    let mut b = battle(west_squads(&["test_recon", "test_recon"]), 1);
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
    let mut b = battle(west_squads(&["test_recon", "test_recon"]), 1);
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
        json!([{ "side": "blue", "kind": "test_recon", "position": [365, 300] }]),
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
        let mut b = battle(west_squads(&["test_recon"]), 1);
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
            { "side": "blue", "kind": "test_recon", "position": [30, 30] },
            { "side": "blue", "kind": "test_recon", "position": [350, 300] }
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
            "buildings":[common::building(0, TEMPLATE, [330.0, 40.0], 0.0)],
            "forests":[{"shape":{"kind":"polygon","ring":[[70,0],[130,0],[130,80],[70,80]]}}]
        })
        .to_string(),
        json!([{"side":"blue","kind":"test_tank","position":[15,40],"yaw":0,"engagement":"return_fire_only"}]),
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

#[test]
fn queued_building_entry_ranks_predicted_origins_and_preserves_an_indefinite_companion() {
    let mut b = battle(
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300]},
            {"side":"blue","kind":"test_recon","position":[300,350]},
            {"side":"blue","kind":"test_rifle","position":[250,200]}
        ]),
        1,
    );
    let mut c = Commander::new();
    let waypoints = [[300.0, 250.0], [380.0, 330.0]];
    for (id, goal) in waypoints.iter().enumerate() {
        c.ok(
            &mut b,
            Side::Blue,
            Order::Move {
                units: vec![UnitId(id as u32)],
                gesture: id as u64 + 1,
                goal: *goal,
                route: RoutePolicy::Shortest,
                direction: Default::default(),
                facing: None,
            },
        );
    }
    c.ok(&mut b, Side::Blue, ground(2, [600.0, 300.0]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: occupy(&[0, 1, 2], 3),
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
        Some(UnitId(1)),
        "rank from executable waypoint origins"
    );
    let places = ack.placement.unwrap().destinations;
    assert!(places.iter().find(|d| d.unit == UnitId(0)).unwrap().placed);
    assert!(!places.iter().find(|d| d.unit == UnitId(2)).unwrap().placed);
    assert!(ack.building.unwrap().unproven);
    let mut visited = [false; 2];
    until(
        &mut b,
        4000,
        "finite prefixes then entry and gathering",
        |b| {
            for (id, goal) in waypoints.iter().enumerate() {
                visited[id] |= (b.unit(UnitId(id as u32)).unwrap().position.xy()
                    - v2(goal[0], goal[1]))
                .length()
                    < 2.0;
            }
            inside(b, 1) && b.unit(UnitId(0)).unwrap().orders.is_empty()
        },
    );
    assert!(visited.into_iter().all(|v| v));
    assert_eq!(b.unit(UnitId(2)).unwrap().orders.len(), 1);
    assert!(matches!(
        b.unit(UnitId(2)).unwrap().orders.front(),
        Some(sim::units::UnitOrder::Attack { .. })
    ));
}

#[test]
fn combined_building_queries_are_pure_and_queued_exit_orders_replay_every_tick() {
    let setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300]},
            {"side":"blue","kind":"test_recon","position":[330,330]},
            {"side":"blue","kind":"test_tank","position":[310,280]}
        ]),
        json!([]),
        json!([]),
    );
    let (mut queried, mut quiet) = (Battle::new(&setup, 13), Battle::new(&setup, 13));
    let mut scripted_setup = setup.clone();
    scripted_setup.scripts = vec![
        contract::scenario::ScriptedOrder {
            tick: 1,
            side: Side::Blue,
            order: garrison(&[0]),
            queued: false,
        },
        contract::scenario::ScriptedOrder {
            tick: 601,
            side: Side::Blue,
            order: exit(&[0]),
            queued: false,
        },
        contract::scenario::ScriptedOrder {
            tick: 601,
            side: Side::Blue,
            order: occupy(&[0, 1, 2], 1),
            queued: true,
        },
    ];
    let mut scripted = Battle::new(&scripted_setup, 13);
    let request = contract::command::BuildingPreviewRequest {
        units: vec![UnitId(0), UnitId(1), UnitId(2)],
        building: BUILDING,
        ..Default::default()
    };
    let mut digests = Vec::new();
    let mut seq = 0;
    for t in 0..1800 {
        if t % 71 == 0 {
            let before = (
                queried.digest(),
                queried.route_searches(Side::Blue),
                queried.navigation_revision(Side::Blue),
            );
            let expected = queried.preview_building(Side::Blue, &request).unwrap();
            let reversed = contract::command::BuildingPreviewRequest {
                units: vec![UnitId(2), UnitId(1), UnitId(0)],
                ..request.clone()
            };
            assert_eq!(
                queried.preview_building(Side::Blue, &reversed).unwrap(),
                expected
            );
            assert_eq!(
                (
                    queried.digest(),
                    queried.route_searches(Side::Blue),
                    queried.navigation_revision(Side::Blue)
                ),
                before
            );
        }
        let orders = match t {
            0 => vec![(garrison(&[0]), false)],
            600 => vec![(exit(&[0]), false), (occupy(&[0, 1, 2], 1), true)],
            _ => Vec::new(),
        };
        for (order, queued) in orders {
            seq += 1;
            let command = CommandEnvelope {
                side: Side::Blue,
                seq,
                order,
                queued,
            };
            let expected = quiet.accept(command.clone());
            assert_eq!(expected.error, None, "{expected:?}");
            assert_eq!(queried.accept(command), expected);
        }
        queried.step();
        quiet.step();
        scripted.step();
        assert_eq!(
            queried.digest(),
            quiet.digest(),
            "query purity at tick {}",
            quiet.tick()
        );
        assert_eq!(
            scripted.digest(),
            quiet.digest(),
            "authored command at tick {}",
            quiet.tick()
        );
        digests.push(quiet.digest());
    }
    assert!(inside(&quiet, 0));
    assert!(quiet.unit(UnitId(1)).unwrap().orders.is_empty());
    let mut replay = Battle::from_replay(&setup, &quiet.replay()).unwrap();
    for expected in digests {
        replay.step();
        assert_eq!(
            replay.digest(),
            expected,
            "building replay at tick {}",
            replay.tick()
        );
    }
}

#[test]
fn a_replacement_building_group_can_replace_an_unreachable_selected_entry_reservation() {
    let mut b = battle_with(
        json!([
            {"kind":"wall","center":[340,300],"yaw":0,"half_extents":[1,11,2]},
            {"kind":"wall","center":[360,300],"yaw":0,"half_extents":[1,11,2]},
            {"kind":"wall","center":[350,290],"yaw":0,"half_extents":[11,1,2]},
            {"kind":"wall","center":[350,310],"yaw":0,"half_extents":[11,1,2]}
        ]),
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300]},
            {"side":"blue","kind":"test_recon","position":[370,270]}
        ]),
        json!([]),
        1,
    );
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 2,
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
        "the retained path cannot enter; choose the reachable squad"
    );
    assert!(!ack.placement.unwrap().destinations[0].placed);
    until(&mut b, 1400, "the replacement entrant inside", |b| {
        inside(b, 1)
    });
    assert!(
        b.unit(UnitId(0)).unwrap().orders.is_empty(),
        "replacement clears the unreachable old reservation"
    );
}

#[test]
fn replacement_entry_cancels_a_pending_exit_but_does_not_preserve_an_actual_exit() {
    let mut b = battle(west_squads(&["test_recon", "test_tank"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "holder inside", |b| inside(b, 0));
    c.ok(&mut b, Side::Blue, exit(&[0]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 3,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.error, None);
    assert_eq!(ack.building.unwrap().entrant.unwrap().unit, UnitId(0));
    for _ in 0..600 {
        b.step();
        assert_eq!(
            phase(&b, Side::Blue, 0),
            Some(GarrisonPhase::Inside),
            "superseded pending exit must not run"
        );
    }
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: exit(&[0]),
        queued: false,
    });
    assert_eq!(ack.error, None);
    b.step();
    assert_eq!(phase(&b, Side::Blue, 0), Some(GarrisonPhase::Exiting));
    let plan = b
        .preview_building(
            Side::Blue,
            &contract::command::BuildingPreviewRequest {
                units: vec![UnitId(0)],
                building: BUILDING,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        plan.entrant.is_none(),
        "an active exit is not an already-held entry"
    );
}

#[test]
fn an_unseen_enemy_occupant_cannot_change_a_building_group_plan() {
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_tank","position":[320,320],"engagement":"return_fire_only"},
            {"side":"red","kind":"test_rifle","position":[450,300],"engagement":"return_fire_only"}
        ]),
        json!([]),
        json!([]),
    );
    let mut rules = serde_json::to_value(&setup.rules).unwrap();
    for kind in ["test_recon", "test_tank"] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"sensors":{"ground_m":10}}),
        );
    }
    setup.rules = serde_json::from_value(rules).unwrap();
    let (mut occupied, mut empty) = (Battle::new(&setup, 1), Battle::new(&setup, 1));
    let ack = occupied.accept(CommandEnvelope {
        side: Side::Red,
        seq: 1,
        order: garrison(&[2]),
        queued: false,
    });
    assert_eq!(ack.error, None);
    for _ in 0..1000 {
        occupied.step();
        empty.step();
    }
    assert!(inside(&occupied, 2));
    let visible = occupied.observe(Side::Blue);
    assert!(
        visible.identified.is_empty() && visible.contacts.is_empty(),
        "the fixture must keep occupancy unseen"
    );
    assert_eq!(visible.own, empty.observe(Side::Blue).own);
    let request = contract::command::BuildingPreviewRequest {
        units: vec![UnitId(0), UnitId(1)],
        building: BUILDING,
        ..Default::default()
    };
    let before = occupied.digest();
    let plan = occupied.preview_building(Side::Blue, &request).unwrap();
    assert_eq!(plan, empty.preview_building(Side::Blue, &request).unwrap());
    assert_eq!(before, occupied.digest());
    assert!(plan.entrant.is_some() && plan.destinations.iter().all(|d| d.placed));
    let ack = occupied.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.building, Some(plan));
    until(&mut occupied, 1600, "entrant discovering the enemy", |b| {
        b.unit(UnitId(0)).unwrap().orders.is_empty()
    });
    assert!(!inside(&occupied, 0));
    assert!(inside(&occupied, 2));
}

#[test]
fn unfinished_shorter_routes_withhold_entry_and_keep_useful_gathering() {
    let mut setup = common::scenario_with(
        &map(json!([
            {"kind":"wall","center":[375,300],"yaw":0,"half_extents":[1,80,2]}
        ])),
        json!([
            {"side":"blue","kind":"test_recon","position":[370,300]},
            {"side":"blue","kind":"test_recon","position":[420,335]}
        ]),
        json!([]),
        json!([]),
    );
    // The eastern route completes, while the nearer western squad's searches
    // around the long wall remain unfinished. Its lower bound can still win.
    setup.rules.navigation.move_validation_work = 40000;
    let mut b = Battle::new(&setup, 1);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.error, None);
    let plan = ack.building.unwrap();
    assert!(plan.entrant.is_none(),"a completed route cannot promise the shortest while a shorter candidate is unproven: {plan:?}");
    assert!(plan.unproven);
    assert!(
        plan.destinations.iter().all(|d| d.placed),
        "reserved fallback remains useful: {plan:?}"
    );
    b.step();
    until(
        &mut b,
        2200,
        "gathering while entry remains unproven",
        |b| (0..2).all(|id| b.unit(UnitId(id)).unwrap().orders.is_empty()),
    );
    assert!((0..2).all(|id| !b.unit(UnitId(id)).unwrap().garrisoned()));
    for d in plan.destinations {
        assert!((b.unit(d.unit).unwrap().position.xy() - v2(d.goal[0], d.goal[1])).length() < 2.0);
    }
}

#[test]
fn known_occupied_and_too_small_buildings_gather_the_whole_selection() {
    for occupied in [true, false] {
        let mut setup = common::scenario_with(
            &map(json!([])),
            json!([
                {"side":"blue","kind":"test_recon","position":[350,300]},
                {"side":"blue","kind":"test_rifle","position":[325,325]},
                {"side":"blue","kind":"test_tank","position":[310,275]}
            ]),
            json!([]),
            json!([]),
        );
        if !occupied {
            setup.rules.buildings.capacity_soldiers = 1;
        }
        let mut b = Battle::new(&setup, 1);
        let seq = if occupied {
            let ack = b.accept(CommandEnvelope {
                side: Side::Blue,
                seq: 1,
                order: garrison(&[0]),
                queued: false,
            });
            assert_eq!(ack.error, None);
            until(&mut b, 1200, "nonselected owner inside", |b| inside(b, 0));
            2
        } else {
            1
        };
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq,
            order: occupy(&[1, 2], 1),
            queued: false,
        });
        assert_eq!(
            ack.error, None,
            "unavailable entry still accepts useful gathering"
        );
        let plan = ack.building.unwrap();
        assert!(plan.entrant.is_none(), "{plan:?}");
        assert!(!plan.unproven, "known ineligibility is conclusive");
        assert!(plan.destinations.iter().all(|d| d.placed), "{plan:?}");
        assert_eq!(
            plan.destinations.iter().map(|d| d.unit).collect::<Vec<_>>(),
            [UnitId(1), UnitId(2)]
        );
        b.step();
        until(&mut b, 1800, "the whole selection gathered", |b| {
            plan.destinations
                .iter()
                .all(|d| b.unit(d.unit).unwrap().orders.is_empty())
        });
        for d in plan.destinations {
            assert!(!inside(&b, d.unit.0));
            assert!(
                (b.unit(d.unit).unwrap().position.xy() - v2(d.goal[0], d.goal[1])).length() < 2.0
            );
        }
        if occupied {
            assert!(inside(&b, 0));
        }
    }
}

#[test]
fn a_selected_entering_squad_keeps_its_entry_before_an_indefinite_attack_suffix() {
    let mut b = battle(west_squads(&["test_recon", "test_tank"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "selected squad already entering", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 2,
        order: ground(0, [600.0, 300.0]),
        queued: true,
    });
    assert_eq!(ack.error, None);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 3,
        order: occupy(&[0, 1], 1),
        queued: true,
    });
    assert_eq!(ack.error, None);
    assert_eq!(
        ack.building.unwrap().entrant.unwrap().unit,
        UnitId(0),
        "existing entry precedes the indefinite suffix"
    );
    until(&mut b, 1200, "selected squad enters", |b| inside(b, 0));
    assert!(
        matches!(
            b.unit(UnitId(0)).unwrap().orders.front(),
            Some(sim::units::UnitOrder::Attack { .. })
        ),
        "original armed work retained"
    );
}

#[test]
fn an_entering_squad_with_a_leaving_suffix_is_unproven_for_queued_reentry() {
    let mut b = battle(west_squads(&["test_recon", "test_tank"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "selected squad already entering", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    let walk = Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal: [300.0, 330.0],
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    for (seq, order) in [(2, walk), (3, ground(0, [600.0, 300.0]))] {
        assert_eq!(
            b.accept(CommandEnvelope {
                side: Side::Blue,
                seq,
                order,
                queued: true
            })
            .error,
            None
        );
    }
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: occupy(&[0, 1], 2),
        queued: true,
    });
    assert_eq!(ack.error, None);
    let plan = ack.building.unwrap();
    assert!(
        plan.entrant.is_none() && plan.unproven,
        "finite leaving work precedes an indefinite attack: {plan:?}"
    );
    b.step();
    assert_eq!(
        b.unit(UnitId(0)).unwrap().orders.len(),
        3,
        "unproven queued reentry preserves predecessors"
    );
    until(
        &mut b,
        1800,
        "original leaving work reaches its attack",
        |b| {
            matches!(
                b.unit(UnitId(0)).unwrap().orders.front(),
                Some(sim::units::UnitOrder::Attack { .. })
            )
        },
    );
    assert!(!inside(&b, 0));
    assert!((b.unit(UnitId(0)).unwrap().position.xy() - v2(300.0, 330.0)).length() < 2.0);
}

#[test]
fn an_inside_holder_with_finite_leaving_work_before_an_attack_cannot_promise_queued_reentry() {
    let mut b = battle(west_squads(&["test_recon", "test_tank"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "selected holder already inside", |b| {
        inside(b, 0)
    });
    let walk = Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal: [300.0, 330.0],
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    for (seq, order) in [
        (2, garrison(&[0])),
        (3, walk),
        (4, ground(0, [600.0, 300.0])),
    ] {
        assert_eq!(
            b.accept(CommandEnvelope {
                side: Side::Blue,
                seq,
                order,
                queued: true
            })
            .error,
            None
        );
    }
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 5,
        order: occupy(&[0, 1], 2),
        queued: true,
    });
    assert_eq!(ack.error, None);
    let plan = ack.building.unwrap();
    assert!(
        plan.entrant.is_none() && plan.unproven,
        "the finite leave precedes an indefinite attack: {plan:?}"
    );
    b.step();
    until(
        &mut b,
        1800,
        "original leaving work reaches its attack",
        |b| {
            matches!(
                b.unit(UnitId(0)).unwrap().orders.front(),
                Some(sim::units::UnitOrder::Attack { .. })
            )
        },
    );
    assert!(!inside(&b, 0));
    assert!((b.unit(UnitId(0)).unwrap().position.xy() - v2(300.0, 330.0)).length() < 2.0);
}

#[test]
fn exhausted_entry_proof_remains_unproven_when_gathering_succeeds() {
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[380,300]}
        ]),
        json!([]),
        json!([]),
    );
    setup.rules.garrison.enter_exit_s = 30.0;
    setup.rules.navigation.move_validation_work = 1000;
    let mut b = Battle::new(&setup, 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 500, "current entry began", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    let plan = b
        .preview_building(
            Side::Blue,
            &contract::command::BuildingPreviewRequest {
                units: vec![UnitId(0)],
                building: BUILDING,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(plan.entrant.is_none());
    assert!(
        plan.unproven,
        "work ended before the physical entry was decided: {plan:?}"
    );
    assert!(
        plan.destinations[0].placed,
        "reserved fallback still gathers: {plan:?}"
    );
}

#[test]
fn an_unfinished_finite_predecessor_cannot_be_skipped_for_a_different_entrant() {
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[30,30]},
            {"side":"blue","kind":"test_recon","position":[350,300]}
        ]),
        json!([]),
        json!([]),
    );
    setup.rules.navigation.move_validation_work = 60000;
    let mut b = Battle::new(&setup, 1);
    let walk = Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal: [384.0, 300.0],
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    assert_eq!(
        b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: 1,
            order: walk,
            queued: false
        })
        .error,
        None
    );
    let plan = b
        .preview_building(
            Side::Blue,
            &contract::command::BuildingPreviewRequest {
                units: vec![UnitId(0), UnitId(1)],
                building: BUILDING,
                queued: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(plan.entrant.is_none(),"the farther proven origin must not win while the nearer future origin is unproven: {plan:?}");
    assert!(plan.unproven);
    assert!(
        plan.destinations.iter().any(|d| d.placed),
        "useful queued gathering remains: {plan:?}"
    );
}

#[test]
fn replacement_entry_reasserts_an_inside_hold_before_its_finite_leaving_suffix() {
    let mut b = battle(west_squads(&["test_recon", "test_tank"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "holder inside", |b| inside(b, 0));
    let walk = Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal: [300.0, 330.0],
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    for (seq, order) in [(2, garrison(&[0])), (3, walk)] {
        assert_eq!(
            b.accept(CommandEnvelope {
                side: Side::Blue,
                seq,
                order,
                queued: true
            })
            .error,
            None
        );
    }
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: occupy(&[0, 1], 2),
        queued: false,
    });
    assert_eq!(ack.error, None);
    assert_eq!(ack.building.unwrap().entrant.unwrap().unit, UnitId(0));
    for _ in 0..600 {
        b.step();
        assert_eq!(
            phase(&b, Side::Blue, 0),
            Some(GarrisonPhase::Inside),
            "replacement supersedes the leaving suffix"
        );
    }
    assert!(b.unit(UnitId(0)).unwrap().orders.is_empty());
}
