//! Slice-00 evidence for the scenario-builder topology decision.
//!
//! These tests deliberately exercise the existing public scenario → Battle →
//! observation/replay path. They record the current binary boundary and keep a
//! small deterministic baseline beside the later topology implementation.

use crate::common;
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;
use sim::publication::{self, Publisher};

fn scenario(units: serde_json::Value) -> ScenarioDefinition {
    common::scenario(common::saved_map("geometry"), units, json!([]))
}

#[test]
fn current_scenario_boundary_refuses_a_four_side_candidate() {
    let mut fixture = sim::fixtures::test_game();
    fixture["map"] = serde_json::to_value(sim::maps::load("geometry").unwrap().definition).unwrap();
    fixture["units"] = json!([
        { "side": "blue", "kind": "test_rifle", "position": [40.0, 80.0] },
        { "side": "red", "kind": "test_rifle", "position": [280.0, 80.0] },
        { "side": "green", "kind": "test_rifle", "position": [40.0, 160.0] },
        { "side": "yellow", "kind": "test_rifle", "position": [280.0, 160.0] }
    ]);
    fixture["events"] = json!([]);
    fixture["scripts"] = json!([]);
    let error = serde_json::from_value::<ScenarioDefinition>(fixture)
        .expect_err("the current binary side contract must expose its refusal");
    let text = error.to_string();
    assert!(
        text.contains("unknown variant") || text.contains("green"),
        "{text}"
    );
}

#[test]
fn existing_binary_path_proves_observation_and_replay_baseline() {
    let setup = scenario(json!([
        { "side": "blue", "kind": "test_rifle", "position": [40.0, 80.0] },
        { "side": "red", "kind": "test_rifle", "position": [280.0, 80.0] }
    ]));
    let mut live = Battle::new(&setup, 17);
    for _ in 0..6 {
        live.step();
    }
    assert_eq!(live.observe(Side::Blue).own.len(), 1);
    assert_eq!(live.observe(Side::Red).own.len(), 1);
    assert_eq!(live.observe(Side::Blue).own[0].id, UnitId(0));
    assert_eq!(live.observe(Side::Red).own[0].id, UnitId(1));

    let replay = live.replay();
    let mut restored = Battle::from_replay(&setup, &replay).expect("baseline replay loads");
    for _ in 0..6 {
        restored.step();
    }
    assert_eq!(restored.digest(), live.digest());
    assert_eq!(restored.replay().accepted, replay.accepted);
}

#[test]
fn neutral_is_visible_as_a_third_slot_and_only_returns_fire() {
    let setup = scenario(serde_json::json!([
        { "side": "blue", "kind": "test_rifle", "position": [80.0, 100.0] },
        { "side": "neutral", "kind": "test_rifle", "position": [220.0, 100.0] },
        { "side": "red", "kind": "test_rifle", "position": [360.0, 100.0] }
    ]));
    let mut battle = Battle::new(&setup, 19);
    assert_eq!(battle.observe(Side::Blue).own.len(), 1);
    assert_eq!(battle.observe(Side::Red).own.len(), 1);
    assert_eq!(battle.observe(Side::Neutral).own.len(), 1);

    let mut neutral_shots = 0;
    for _ in 0..120 {
        battle.step();
        neutral_shots += battle
            .rounds()
            .filter(|(_, round)| round.unit == UnitId(1))
            .count();
    }
    assert!(
        neutral_shots > 0,
        "neutral returns fire after being threatened"
    );
}

#[test]
fn publication_layout_and_stream_include_the_neutral_slot() {
    let setup = scenario(serde_json::json!([
        { "side": "blue", "kind": "test_rifle", "position": [80.0, 100.0] },
        { "side": "neutral", "kind": "test_rifle", "position": [220.0, 100.0] },
        { "side": "red", "kind": "test_rifle", "position": [360.0, 100.0] }
    ]));
    let battle = Battle::new(&setup, 23);
    let layout: serde_json::Value =
        serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    assert_eq!(
        layout["ground"]["sides"],
        serde_json::json!(["blue", "red", "neutral"])
    );
    let mut publisher = Publisher::new();
    for side in Side::ALL {
        assert!(!publisher.publish(&battle, side).unwrap().is_empty());
    }
}

#[test]
fn neutral_rejects_commands_but_still_returns_fire() {
    let setup = scenario(serde_json::json!([
        { "side": "blue", "kind": "test_rifle", "position": [80.0, 100.0] },
        { "side": "neutral", "kind": "test_rifle", "position": [220.0, 100.0] },
        { "side": "red", "kind": "test_rifle", "position": [360.0, 100.0] }
    ]));
    let mut battle = Battle::new(&setup, 29);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Neutral,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [260.0, 100.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, Some(OrderError::PassiveSide));
}
