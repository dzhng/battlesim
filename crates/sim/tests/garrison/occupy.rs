//! The building order, through public preview and admission: each listed
//! squad walks to its approach and steps in.
use super::*;

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
    until(&mut b, 900, "the squad inside", |b| inside(b, 0));
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
    c.ok(&mut b, Side::Blue, garrison(&[0]));
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

/// A queued building order plans its approach from where the queue ends:
/// a squad told to leave, walk round to the far side and come back in
/// steps out, makes the walk, and re-enters by the far facade.
#[test]
fn a_queued_reentry_starts_from_where_the_queue_ends() {
    let mut b = battle(west_squads(&["test_recon"]), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "the squad inside", |b| inside(b, 0));
    let east = [CENTRE[0] + HALF[0] + 30.0, CENTRE[1]];
    let walk = Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal: east,
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    assert_eq!(c.send(&mut b, Side::Blue, exit(&[0]), false), None);
    assert_eq!(c.send(&mut b, Side::Blue, walk, true), None);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 4,
        order: garrison(&[0]),
        queued: true,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let approach = ack.building.unwrap().entries[0].approach;
    assert!(
        approach[0] > CENTRE[0] + HALF[0],
        "the approach faces the queue's end: {approach:?}"
    );
    let mut reached = false;
    until(&mut b, 3000, "the squad back inside after its walk", |b| {
        let at = b.unit(UnitId(0)).unwrap().position.xy();
        reached |= (at - v2(east[0], east[1])).length() < 3.0;
        reached && inside(b, 0)
    });
}

#[test]
fn building_previews_are_pure_and_queued_building_orders_replay_every_tick() {
    let setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300]},
            {"side":"blue","kind":"test_recon","position":[330,330]},
            {"side":"blue","kind":"test_recon","position":[310,280]}
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
            order: garrison(&[0, 1, 2]),
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
            600 => vec![(exit(&[0]), false), (garrison(&[0, 1, 2]), true)],
            _ => Vec::new(),
        };
        for (order, queued) in orders {
            seq += 1;
            let command = CommandEnvelope {
                side: Side::Blue,
                player: None,
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
    assert!((0..3).any(|u| inside(&quiet, u)));
    assert!((0..3).all(|u| quiet.unit(UnitId(u)).unwrap().orders.is_empty()));
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
fn an_unseen_enemy_occupant_cannot_change_a_building_plan() {
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            {"side":"blue","kind":"test_recon","position":[350,300],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_recon","position":[320,320],"engagement":"return_fire_only"},
            {"side":"red","kind":"test_rifle","position":[450,300],"engagement":"return_fire_only"}
        ]),
        json!([]),
        json!([]),
    );
    let mut rules = serde_json::to_value(&setup.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_recon",
        json!({"sensors":{"ground_m":10}}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let (mut occupied, mut empty) = (Battle::new(&setup, 1), Battle::new(&setup, 1));
    let ack = occupied.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
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
    assert_eq!(plan.entries.len(), 2, "{plan:?}");
    let ack = occupied.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: garrison(&[0, 1]),
        queued: false,
    });
    assert_eq!(ack.building, Some(plan));
    until(
        &mut occupied,
        1600,
        "both squads discovering the enemy",
        |b| (0..2).all(|u| b.unit(UnitId(u)).unwrap().orders.is_empty()),
    );
    assert!(!inside(&occupied, 0) && !inside(&occupied, 1));
    assert!(inside(&occupied, 2));
}

/// A building a block away, with a row of buildings between it and the
/// squad: the squad walks round the row to the far building and garrisons it.
#[test]
fn a_squad_routes_round_a_row_of_buildings_to_garrison_the_one_behind_it() {
    let row: Vec<_> = (0..3)
        .map(|k| {
            common::building(
                1 + k,
                FELLED,
                [CENTRE[0] - 50.0, CENTRE[1] - 36.0 + 36.0 * k as f64],
                std::f64::consts::FRAC_PI_2,
            )
        })
        .collect();
    let mut b = battle_with(
        Value::Array(row),
        json!([{ "side": "blue", "kind": "test_recon", "position": [CENTRE[0] - 140.0, CENTRE[1]] }]),
        json!([]),
        1,
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: garrison(&[0]),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let entries = ack.building.as_ref().map(|p| p.entries.clone());
    assert_eq!(
        entries.map(|e| e.iter().map(|e| e.unit).collect::<Vec<_>>()),
        Some(vec![UnitId(0)]),
        "{ack:?}"
    );
    until(&mut b, 5000, "the squad inside the far building", |b| {
        inside(b, 0)
    });
}
