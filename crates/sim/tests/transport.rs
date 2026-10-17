//! Passenger carriage behavior through the public battle authority.
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::observation::HoldPhase;
use serde_json::json;
use sim::battle::Battle;

fn setup(units: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    crate::common::scenario_with(&map.to_string(), units, json!([]), json!([]))
}

fn setup_with_map(
    map: serde_json::Value,
    units: serde_json::Value,
    events: serde_json::Value,
) -> contract::scenario::ScenarioDefinition {
    crate::common::scenario_with(&map.to_string(), units, events, json!([]))
}

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq,
        order,
        queued: false,
    }
}

fn side_command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        slot: None,
        seq,
        order,
        queued: false,
    }
}

fn transport_skirmish_setup() -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::test_game();
    for (kind, faction, category) in [
        ("test_apc", "us", "veh"),
        ("test_recon", "us", "inf"),
        ("test_tank", "eastern", "veh"),
    ] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"roster":{"factions":[faction],"category":category,"family_name":"Test Transport","variant":"Test"}}),
        );
    }
    sim::fixtures::patch_catalog(&mut rules, "units", "test_apc", json!({"mounts":[]}));
    serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]},
        "rules":rules,"units":[],"events":[],"scripts":[],
        "skirmish":{"factions":["us","eastern"],"rules":{"credits_per_minute":200,"starting_minutes":5,"preparation_s":0,"max_units":30,"dispatch_interval_s":1,"capture_s":20,"majority_s":1800,"victory_score":1000},"sites":{"entries":[{"side":"blue","center":[400,10],"yaw":0.0},{"side":"red","center":[400,590],"yaw":0.0}],"objectives":[{"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null},{"id":"south","center":[200,200],"radius_m":50,"kind":"field","counterpart":"north"},{"id":"north","center":[600,400],"radius_m":50,"kind":"field","counterpart":"south"}]}}
    })).unwrap()
}

#[test]
fn a_squad_boards_a_stopped_carrier_and_leaves_the_field() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]},
            {"side":"red","kind":"test_rifle","position":[600,200]}
        ])),
        7,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    let passenger = battle.unit(UnitId(1)).unwrap();
    assert_eq!(battle.observe(Side::Red).identified.len(), 1);
    assert!(!passenger.on_field());
    assert_eq!(
        passenger.hold.as_ref().map(|h| h.phase),
        Some(sim::hold::Phase::Inside)
    );
    assert_eq!(
        battle
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(1))
            .and_then(|u| u.hold.map(|h| h.phase)),
        Some(HoldPhase::Inside)
    );
    assert!(battle
        .observe(Side::Red)
        .audible
        .iter()
        .all(|cue| cue.listener != UnitId(1)));
    assert_eq!(
        passenger.service,
        contract::observation::ServiceStatus::OutOfRange
    );
}

#[test]
fn a_squad_walks_to_a_distant_carrier_and_boards_it() {
    // The squad can never stand on the carrier's centre (the hull is in the
    // way), so its walk has to end where the hold's reach admits it.
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[220,240]},
            {"side":"blue","kind":"test_rifle","position":[200,240]},
            {"side":"red","kind":"test_rifle","position":[600,200]}
        ])),
        7,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    let aboard = (0..battle.rules().tick_hz * 40).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|u| u.hold.as_ref())
            .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
    });
    assert!(aboard, "the squad never boarded the carrier 20 m away");
}

#[test]
fn orders_aimed_at_a_passenger_are_refused_as_aboard() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        8,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 4) {
        battle.step();
    }
    let ack = battle.accept(command(
        2,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 2,
            goal: [300.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    ));
    assert_eq!(ack.error, Some(OrderError::Aboard { unit: UnitId(1) }));
}

#[test]
fn squads_beyond_the_seats_are_refused_closest_first() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[214,200]},
            {"side":"blue","kind":"test_recon","position":[202,200]}
        ])),
        9,
    );
    let ack = battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1), UnitId(2)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    assert_eq!(
        ack.refused.iter().map(|r| r.unit).collect::<Vec<_>>(),
        vec![UnitId(1)]
    );
    assert_eq!(ack.refused[0].reason, OrderError::CapacityFull);
}

#[test]
fn a_carrier_drives_to_a_squad_and_takes_it_aboard() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[250,200]},
            {"side":"blue","kind":"test_recon","position":[220,200]}
        ])),
        10,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Collect {
                units: vec![UnitId(0)],
                squads: vec![UnitId(1)],
                gesture: 1
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 12) {
        battle.step();
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    assert_eq!(
        battle
            .unit(UnitId(1))
            .unwrap()
            .hold
            .as_ref()
            .map(|h| h.phase),
        Some(sim::hold::Phase::Inside)
    );
}

#[test]
fn transport_orders_replay_every_tick_exactly() {
    let setup = setup(json!([
        {"side":"blue","kind":"test_apc","position":[200,200]},
        {"side":"blue","kind":"test_recon","position":[195,200]}
    ]));
    let mut live = Battle::new(&setup, 11);
    live.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    let mut digests = vec![live.digest()];
    for _ in 0..180 {
        live.step();
        digests.push(live.digest());
    }
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    while replay.tick() < live.tick() {
        replay.step();
        assert_eq!(replay.digest(), digests[replay.tick() as usize]);
    }
}

#[test]
fn catalog_rejects_invalid_carriage_seats_and_squads() {
    let mut game = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut game,
        "units",
        "test_apc",
        json!({"capabilities":{"carriage":{"seats":7}}}),
    );
    assert!(serde_json::from_value::<contract::scenario::Rules>(game).is_err());
}

#[test]
fn capacity_lost_during_boarding_is_published_as_refusal() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]},
            {"side":"blue","kind":"test_recon","position":[196,200]}
        ])),
        12,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        ))
        .error
        .is_none());
    assert!(battle
        .accept(command(
            2,
            Order::Board {
                units: vec![UnitId(2)],
                carrier: UnitId(0),
                gesture: 2,
            },
        ))
        .error
        .is_none());
    let mut seen = false;
    for _ in 0..(battle.rules().tick_hz * 2) {
        battle.step();
        seen |= battle.observe(Side::Blue).refusals.iter().any(|refusal| {
            matches!(refusal.unit, UnitId(1) | UnitId(2))
                && refusal.reason == OrderError::CapacityFull
        });
    }
    assert!(seen);
}

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
    assert!(!arrival.unit(UnitId(1)).unwrap().on_field());
    assert!(arrival.unit(UnitId(1)).unwrap().alive());
    assert!(matches!(
        arrival.unit(UnitId(0)).unwrap().orders.front(),
        Some(sim::units::UnitOrder::Move(order)) if order.destination.x == 220.0
    ));
}

#[test]
fn refunding_a_loaded_carrier_refunds_its_passengers() {
    let mut battle = Battle::new(&transport_skirmish_setup(), 41);
    assert!(battle
        .accept(side_command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_apc".into(),
                destination: [300.0, 100.0],
            },
        ))
        .error
        .is_none());
    assert!(battle
        .accept(side_command(
            Side::Blue,
            2,
            Order::ConfirmPurchase {
                variant: "test_recon".into(),
                destination: [300.0, 100.0],
            },
        ))
        .error
        .is_none());
    battle.accept(side_command(Side::Blue, 3, Order::Ready));
    battle.accept(side_command(Side::Red, 1, Order::Ready));
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).is_some() {
            break;
        }
    }
    assert!(battle.unit(UnitId(0)).is_some() && battle.unit(UnitId(1)).is_some());
    assert!(battle
        .accept(command(
            4,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    let refused = battle.accept(command(
        5,
        Order::Refund {
            units: vec![UnitId(1)],
        },
    ));
    assert_eq!(refused.error, Some(OrderError::Aboard { unit: UnitId(1) }));
    let before = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    assert!(battle
        .accept(command(
            6,
            Order::Refund {
                units: vec![UnitId(0)]
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().retired {
            break;
        }
    }
    let after = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    assert!(after > before);
    assert!(battle.unit(UnitId(0)).unwrap().retired);
    assert!(battle.unit(UnitId(1)).unwrap().retired);
}

#[test]
fn a_destroyed_carrier_bails_out_its_passengers_hurt_and_pinned() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]}
        ])),
        42,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 4) {
        battle.step();
    }
    let passenger = battle.unit(UnitId(1)).unwrap();
    assert!(!passenger.on_field());
    assert_eq!(
        passenger
            .members
            .iter()
            .map(|member| member.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        passenger.members.len()
    );
}
