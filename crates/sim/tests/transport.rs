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

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq,
        order,
        queued: false,
    }
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
    for _ in 0..(battle.rules().tick_hz * 3) {
        battle.step();
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
