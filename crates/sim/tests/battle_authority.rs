//! The authority's command, tick and replay contracts.
use contract::command::{CommandEnvelope, Order, OrderError, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use sim::battle::{Battle, ReplayError};

use crate::common;

fn scenario() -> ScenarioDefinition {
    common::scenario(
        common::saved_map("geometry"),
        serde_json::json!([
            { "side": "blue", "kind": "tank", "position": [40, 150] },
            { "side": "blue", "kind": "rifle", "position": [40, 170] },
            { "side": "red", "kind": "rifle", "position": [360, 150] }
        ]),
        serde_json::json!([]),
    )
}

fn mv(side: Side, seq: u64, unit: u32, goal: [f64; 2], queued: bool) -> CommandEnvelope {
    CommandEnvelope {
        side,
        seq,
        order: Order::Move {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
        queued,
    }
}

#[test]
fn commands_are_acknowledged_in_order_and_apply_on_the_next_tick() {
    let mut b = Battle::new(&scenario(), 7);
    b.step();
    let ack = b.accept(mv(Side::Blue, 1, 0, [120.0, 150.0], false));
    assert_eq!((ack.seq, ack.applied_tick, ack.error), (1, 2, None));
    let before = b.observe(Side::Blue).own[0].position;
    assert_eq!(
        b.observe(Side::Blue).own[0].goal,
        None,
        "not applied before its tick"
    );
    b.step();
    let own = &b.observe(Side::Blue).own[0];
    assert_eq!(own.goal, Some([120.0, 150.0]));
    assert!(own.position[0] > before[0]);
}

#[test]
fn out_of_sequence_and_foreign_commands_are_rejected_without_effect() {
    let mut b = Battle::new(&scenario(), 7);
    let skip = b.accept(mv(Side::Blue, 2, 0, [100.0, 150.0], false));
    assert_eq!(skip.error, Some(OrderError::OutOfSequence { expected: 1 }));
    let foreign = b.accept(mv(Side::Blue, 1, 2, [100.0, 150.0], false));
    assert_eq!(
        foreign.error,
        Some(OrderError::NotOwnUnit { unit: UnitId(2) })
    );
    let unknown = b.accept(mv(Side::Blue, 2, 99, [100.0, 150.0], false));
    assert_eq!(
        unknown.error,
        Some(OrderError::UnknownUnit { unit: UnitId(99) })
    );
    let outside = b.accept(mv(Side::Blue, 3, 0, [-5.0, 150.0], false));
    assert_eq!(outside.error, Some(OrderError::OutOfBounds));
    let digest = b.digest();
    b.step();
    assert!(b.observe(Side::Blue).own.iter().all(|u| u.goal.is_none()));
    assert_ne!(digest, b.digest(), "the tick still advanced");
    // Red's own sequence is independent.
    assert_eq!(
        b.accept(mv(Side::Red, 1, 2, [300.0, 150.0], false)).error,
        None
    );
}

#[test]
fn shift_queues_and_plain_orders_replace_and_stop_clears() {
    let mut b = Battle::new(&scenario(), 7);
    b.accept(mv(Side::Blue, 1, 0, [80.0, 150.0], false));
    b.accept(mv(Side::Blue, 2, 0, [80.0, 190.0], true));
    b.step();
    assert_eq!(b.observe(Side::Blue).own[0].queue, vec![[80.0, 190.0]]);
    b.accept(mv(Side::Blue, 3, 0, [50.0, 100.0], false));
    b.step();
    let own = &b.observe(Side::Blue).own[0];
    assert_eq!((own.goal, own.queue.len()), (Some([50.0, 100.0]), 0));
    b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: Order::Stop {
            units: vec![UnitId(0)],
        },
        queued: false,
    });
    b.step();
    assert_eq!(b.observe(Side::Blue).own[0].goal, None);
}

#[test]
fn sides_observe_only_their_own_units() {
    let b = Battle::new(&scenario(), 7);
    assert_eq!(b.observe(Side::Blue).own.len(), 2);
    assert_eq!(b.observe(Side::Red).own.len(), 1);
    assert_eq!(b.observe(Side::Red).own[0].id, UnitId(2));
}

#[test]
fn same_seed_and_commands_replay_to_identical_tick_digests() {
    let setup = scenario();
    let mut live = Battle::new(&setup, 42);
    let mut digests = Vec::new();
    for t in 0..240u64 {
        if t == 3 {
            live.accept(mv(Side::Blue, 1, 0, [200.0, 60.0], false));
            live.accept(mv(Side::Red, 1, 2, [250.0, 250.0], false));
        }
        if t == 50 {
            live.accept(mv(Side::Blue, 2, 1, [90.0, 90.0], true));
            live.accept(mv(Side::Blue, 3, 7, [90.0, 90.0], false)); // rejected content, still recorded
        }
        live.step();
        digests.push(live.digest());
    }
    let record = live.replay();
    let json = serde_json::to_string(&record).unwrap();
    let mut replay = Battle::from_replay(&setup, &serde_json::from_str(&json).unwrap()).unwrap();
    for (t, expected) in digests.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.digest(),
            *expected,
            "first mismatch at tick {}",
            t + 1
        );
    }
    assert_eq!(
        replay.replay().accepted,
        record.accepted,
        "replay records the same commands"
    );
    // Input is refused while replaying.
    let ack = replay.accept(mv(Side::Blue, 4, 0, [10.0, 10.0], false));
    assert_eq!(ack.error, Some(OrderError::ReplayInProgress));
}

#[test]
fn a_replay_refuses_a_different_scenario_or_config() {
    let setup = scenario();
    let record = Battle::new(&setup, 1).replay();
    let mut moved = setup.clone();
    moved.units[0].position[0] += 1.0;
    assert_eq!(
        Battle::from_replay(&moved, &record).err(),
        Some(ReplayError::ScenarioMismatch)
    );
    let mut faster = setup.clone();
    faster.rules.movement.forest_vehicle_multiplier += 0.1;
    assert_eq!(
        Battle::from_replay(&faster, &record).err(),
        Some(ReplayError::ConfigMismatch)
    );
}

/// A soldier's slot (which soldier kind he is, what he carries) is state the
/// digest holds: a squad alike but for one soldier's slot hashes apart.
#[test]
fn the_digest_holds_each_soldiers_slot() {
    let b = Battle::new(&scenario(), 1);
    let squad = b.unit(UnitId(1)).unwrap();
    let mut other = squad.clone();
    other.members[0].slot = 1;
    let digest = |u: &sim::units::Unit| {
        let mut d = sim::digest::Digest::default();
        u.digest(&mut d);
        d.finish()
    };
    assert_ne!(digest(squad), digest(&other));
}

/// A map's fog resolution cuts both its observed visibility and its foliage.
#[test]
fn map_resolution_cuts_visibility_and_foliage_together() {
    for (cell, expected) in [(4.0, [5, 3]), (10.0, [2, 2])] {
        let map = serde_json::json!({
            "size": [19, 12], "height_grid_m": 4, "slope_cutoff_deg": 35,
            "fog_cell_m": cell,
            "forests": [{ "shape":{"kind":"polygon","ring":[[0.0,0.0],[19.0,0.0],[19.0,12.0],[0.0,12.0]]}}]
        });
        let setup = common::scenario(
            &map.to_string(),
            serde_json::json!([]),
            serde_json::json!([]),
        );
        let battle = Battle::new(&setup, 1);
        let visibility = &battle.observe(Side::Blue).ground_visibility;
        assert_eq!([visibility.nx, visibility.ny], expected);
        assert_eq!(visibility.cell_m, cell);
        let foliage = battle.world().export_foliage();
        assert_eq!(
            &foliage[..3],
            &[expected[0] as f32, expected[1] as f32, cell as f32]
        );
        assert!(foliage[3..]
            .chunks_exact(4)
            .any(|row| row[2] > 0.0 && row[3] > 0.0));
    }
}

/// The grids' cell sizes are refused by name before the world is divided
/// into cells of them.
#[test]
#[should_panic(expected = "map.fog_cell_m must be finite and positive")]
fn a_zero_fog_cell_is_refused_by_name() {
    let mut setup = scenario();
    setup.map.fog_cell_m = 0.0;
    Battle::new(&setup, 1);
}

#[test]
#[should_panic(expected = "map.fog_cell_m must be finite and positive")]
fn an_infinite_fog_cell_is_refused_by_name() {
    let mut setup = scenario();
    setup.map.fog_cell_m = f64::INFINITY;
    Battle::new(&setup, 1);
}

#[test]
#[should_panic(expected = "ground.cell_m must be positive")]
fn a_zero_ground_cell_is_refused_by_name() {
    let mut setup = scenario();
    setup.rules.ground.cell_m = 0.0;
    Battle::new(&setup, 1);
}
