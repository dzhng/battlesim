//! Which squad enters, and by which entry: the route searches admission
//! runs within its work allowance.
use super::*;

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
        player: None,
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
        player: None,
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
fn work_ending_before_any_entry_route_keeps_useful_gathering() {
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
    // The nearer western squad's search round the long wall is first and
    // spends all the work unfinished, so no entry route completes.
    setup.rules.navigation.move_validation_work = 40000;
    let mut b = Battle::new(&setup, 1);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: occupy(&[0, 1], 1),
        queued: false,
    });
    assert_eq!(ack.error, None);
    let plan = ack.building.unwrap();
    assert!(plan.entrant.is_none(), "no entry route completed: {plan:?}");
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
        phase(b, Side::Blue, 0) == Some(HoldPhase::Entering)
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

/// A building a block away, with a row of buildings between it and the
/// squad: the squad walks round the row to the far building and garrisons it.
/// Every entry search has to round the row, and together they cost more than
/// the admission's whole route allowance.
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
        order: occupy(&[0], 1),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let entrant = ack.building.as_ref().and_then(|p| p.entrant.clone());
    assert_eq!(entrant.map(|e| e.unit), Some(UnitId(0)), "{ack:?}");
    until(&mut b, 5000, "the squad inside the far building", |b| {
        inside(b, 0)
    });
}
