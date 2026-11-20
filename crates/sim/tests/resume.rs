//! A recorded battle resumed at a tick: replayed there exactly, then live.

use crate::common;
use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::players::ControllerKind;
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::{Battle, ResumeError};

/// Two riflemen out of each other's sight, each owned by its side's player.
fn standoff() -> ScenarioDefinition {
    common::scenario(
        common::saved_map("geometry"),
        json!([
            { "side": "blue", "owner": "blue-1", "kind": "test_rifle", "position": [40.0, 40.0] },
            { "side": "red", "owner": "red-1", "kind": "test_rifle", "position": [360.0, 360.0] }
        ]),
        json!([]),
    )
}

fn walk(battle: &mut Battle, goal: [f64; 2], gesture: u64) {
    let order = Order::Move {
        units: vec![UnitId(0)],
        gesture,
        goal,
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    };
    let ack = battle.accept_order(Side::Blue, Some("blue-1".into()), order, false);
    assert_eq!(ack.error, None, "a live battle takes the player's order");
}

fn run(battle: &mut Battle, to: u64) {
    while battle.tick() < to {
        battle.step();
    }
}

/// A battle with the human's order before `at` and none after, played past it.
fn played(controller: ControllerKind, at: u64) -> (ScenarioDefinition, Battle, u64) {
    let setup = standoff();
    let mut battle = Battle::new(&setup, 61);
    battle.set_controllers(vec![
        ("blue-1".into(), ControllerKind::Human),
        ("red-1".into(), controller),
    ]);
    run(&mut battle, at / 2);
    walk(&mut battle, [120.0, 60.0], 1);
    run(&mut battle, at);
    let digest = battle.digest();
    run(&mut battle, at * 2);
    (setup, battle, digest)
}

#[test]
fn a_resumed_battle_is_the_recorded_one_at_its_tick_and_live_after() {
    let at = 4 * 20;
    let (setup, original, digest_at) = played(ControllerKind::DirectedAi, at);
    let replay = original.replay();
    let mut resumed = Battle::resume(&setup, &replay, at).unwrap();
    assert_eq!(resumed.tick(), at);
    assert_eq!(resumed.digest(), digest_at, "replayed to the tick exactly");

    // Left alone it goes on as the recording did.
    let mut alone = Battle::resume(&setup, &replay, at).unwrap();
    run(&mut alone, at * 2);
    assert_eq!(alone.digest(), original.digest());

    // Given an order it diverges, and its own replay is that battle.
    walk(&mut resumed, [60.0, 140.0], 2);
    run(&mut resumed, at * 2);
    assert_ne!(
        resumed.digest(),
        original.digest(),
        "the new order changes it"
    );
    let mut again = Battle::from_replay(&setup, &resumed.replay()).unwrap();
    run(&mut again, at * 2);
    assert_eq!(
        again.digest(),
        resumed.digest(),
        "its record goes on from the prefix"
    );
}

#[test]
fn a_battle_an_active_ai_played_does_not_resume() {
    let at = 4 * 20;
    let (setup, original, _) = played(ControllerKind::ActiveAi, at);
    assert_eq!(
        Battle::resume(&setup, &original.replay(), at).err(),
        Some(ResumeError::AiMemory {
            player: "red-1".into()
        })
    );
}

#[test]
fn a_battle_does_not_resume_past_its_end() {
    let mut setup = standoff();
    setup.victory = Some("if tick() == 1 { victory(true); }".into());
    let mut battle = Battle::new(&setup, 62);
    while !battle.finished() {
        assert!(battle.tick() < 20, "the victory script ends the battle");
        battle.step();
    }
    assert_eq!(
        Battle::resume(&setup, &battle.replay(), 5).err(),
        Some(ResumeError::Beyond { tick: 5 })
    );
}
