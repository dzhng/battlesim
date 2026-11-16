//! A carrier lets its squads out where it is sent, or refuses an unload
//! with no room for them, and publishes which of its steps lets them out.
use super::*;
use contract::observation::OrderStep;

#[test]
fn a_carrier_unloads_where_sent_and_queued_orders_follow() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        13,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());

    let unload = battle.accept(command(
        2,
        Order::Unload {
            into: None,
            units: vec![UnitId(0)],
            goal: Some([205.0, 200.0]),
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert!(unload.error.is_none());
    let mut follow_command = command(
        3,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 3,
            goal: [220.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    );
    follow_command.queued = true;
    let follow = battle.accept(follow_command);
    assert!(follow.error.is_none());
    for _ in 0..(battle.rules().tick_hz * 8) {
        battle.step();
    }
    assert!(battle.unit(UnitId(1)).unwrap().on_field());
    assert!(battle.unit(UnitId(0)).unwrap().position.x > 205.0);
}

#[test]
fn an_unload_with_no_room_is_refused_and_everyone_stays_aboard() {
    let blocked_map = json!({
        "size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[{"kind":"wall","center":[200,200],"yaw":0,"half_extents":[10,10,4]}],
        "buildings":[]
    });
    let mut in_place = Battle::new(
        &setup_with_map(
            blocked_map,
            json!([
                {"side":"blue","kind":"test_apc","position":[200,200]},
                {"side":"blue","kind":"test_rifle","position":[200,200]}
            ]),
            json!([]),
        ),
        14,
    );
    in_place.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(in_place.rules().tick_hz * 3) {
        in_place.step();
    }
    let refused = in_place.accept(command(
        2,
        Order::Unload {
            into: None,
            units: vec![UnitId(0)],
            goal: None,
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert_eq!(
        refused.refused,
        vec![contract::command::Refusal {
            unit: UnitId(0),
            reason: OrderError::NoRoomToUnload,
        }]
    );
    assert!(!in_place.unit(UnitId(1)).unwrap().on_field());
    assert!(in_place.unit(UnitId(1)).unwrap().alive());

    let arrival_map = json!({
        "size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[],"buildings":[]
    });
    let mut arrival = Battle::new(
        &setup_with_map(
            arrival_map,
            json!([
                {"side":"blue","kind":"test_apc","position":[200,200]},
                {"side":"blue","kind":"test_rifle","position":[195,200]}
            ]),
            json!([{"tick":100,"add_prop":{"kind":"wall","center":[201,200],"yaw":0,"half_extents":[10,10,4]}}]),
        ),
        15,
    );
    arrival.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(arrival.rules().tick_hz * 2) {
        arrival.step();
    }
    let unload = arrival.accept(command(
        2,
        Order::Unload {
            into: None,
            units: vec![UnitId(0)],
            goal: Some([201.0, 200.0]),
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert!(unload.error.is_none());
    let mut follow_command = command(
        3,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 3,
            goal: [220.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    );
    follow_command.queued = true;
    assert!(arrival.accept(follow_command).error.is_none());
    arrival.step();
    assert_eq!(step_kinds(&arrival, UnitId(0)), ["move", "unload", "move"]);
    let mut seen = false;
    let mut saw_exiting = false;
    for _ in 0..(arrival.rules().tick_hz * 5) {
        arrival.step();
        saw_exiting |= matches!(
            arrival
                .unit(UnitId(1))
                .unwrap()
                .hold
                .as_ref()
                .map(|hold| hold.phase),
            Some(sim::hold::Phase::Exiting(_))
        );
        seen |= arrival.observe(Side::Blue).refusals.iter().any(|refusal| {
            refusal.unit == UnitId(0) && refusal.reason == OrderError::NoRoomToUnload
        });
    }
    assert!(saw_exiting);
    assert!(seen);
    // Refused: the step that would have unloaded is gone, and nothing
    // ahead unloads.
    assert_eq!(step_kinds(&arrival, UnitId(0)), ["move"]);
    assert!(!arrival.unit(UnitId(1)).unwrap().on_field());
    assert!(arrival.unit(UnitId(1)).unwrap().alive());
    assert!(matches!(
        arrival.unit(UnitId(0)).unwrap().orders.front(),
        Some(sim::units::UnitOrder::Move(order)) if order.destination.x == 220.0
    ));
}

/// Blue's published order steps for unit `id`, by kind: a drive that lets
/// its passengers out is a move then an unload.
fn step_kinds(battle: &Battle, id: UnitId) -> Vec<&'static str> {
    let observation = battle.observe(Side::Blue);
    let own = observation.own.iter().find(|u| u.id == id).unwrap();
    (own.steps.iter())
        .map(|step| match step {
            OrderStep::Move { .. } => "move",
            OrderStep::Unload { .. } => "unload",
            other => panic!("only drives and unloads here, got {other:?}"),
        })
        .collect()
}

#[test]
fn the_unload_step_of_a_queued_drive_unload_drive_is_published() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        16,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    let drive = |seq, goal| Order::Move {
        units: vec![UnitId(0)],
        gesture: seq,
        goal,
        route: Default::default(),
        direction: Default::default(),
        facing: None,
    };
    assert!(battle
        .accept(command(2, drive(2, [240.0, 200.0])))
        .error
        .is_none());
    let mut unload = command(
        3,
        Order::Unload {
            into: None,
            units: vec![UnitId(0)],
            goal: Some([280.0, 200.0]),
            gesture: 3,
            route: Default::default(),
        },
    );
    unload.queued = true;
    assert!(battle.accept(unload).error.is_none());
    let mut onward = command(4, drive(4, [320.0, 200.0]));
    onward.queued = true;
    assert!(battle.accept(onward).error.is_none());
    battle.step();
    assert_eq!(
        step_kinds(&battle, UnitId(0)),
        ["move", "move", "unload", "move"]
    );
    // It lets them out where the drive before it ends.
    let unload = OrderStep::Unload {
        point: [280.0, 200.0],
        into: None,
    };
    assert_eq!(battle.observe(Side::Blue).own[0].steps[2], unload);

    // The unload step becomes the goal: the flag goes with it.
    let mut heading_to_unload = false;
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.observe(Side::Blue).own[0].steps.len() == 3 {
            heading_to_unload = true;
            break;
        }
    }
    assert!(heading_to_unload);
    assert_eq!(step_kinds(&battle, UnitId(0)), ["move", "unload", "move"]);

    // Unloaded and driving on: no step unloads any more.
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(battle.unit(UnitId(1)).unwrap().on_field());
    battle.step();
    assert_eq!(step_kinds(&battle, UnitId(0)), ["move"]);
}
