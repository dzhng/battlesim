//! Slice-00 native report: current side boundary and deterministic baseline.
//!
//! Run with `cargo run -p sim --example side_topology_probe`. The output is a
//! small, fixture-backed report rather than a second runtime authority.

use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

fn main() {
    let map = serde_json::to_value(sim::maps::load("geometry").unwrap().definition).unwrap();
    let rules = sim::fixtures::test_rules();
    let two_side = json!({
        "map": map,
        "rules": rules,
        "units": [
        { "side": "blue", "kind": "test_rifle", "position": [40.0, 80.0] },
        { "side": "red", "kind": "test_rifle", "position": [280.0, 80.0] }
        ],
        "events": [],
        "scripts": []
    });
    let setup: ScenarioDefinition = serde_json::from_value(two_side).unwrap();
    let mut battle = Battle::new(&setup, 17);
    for _ in 0..6 {
        battle.step();
    }
    let replay = battle.replay();
    let mut restored = Battle::from_replay(&setup, &replay).unwrap();
    for _ in 0..6 {
        restored.step();
    }

    let candidate = json!([
        { "side": "blue", "kind": "test_rifle", "position": [40.0, 80.0] },
        { "side": "red", "kind": "test_rifle", "position": [280.0, 80.0] },
        { "side": "green", "kind": "test_rifle", "position": [40.0, 160.0] },
        { "side": "yellow", "kind": "test_rifle", "position": [280.0, 160.0] }
    ]);
    let candidate_fixture = json!({
        "map": serde_json::to_value(sim::maps::load("geometry").unwrap().definition).unwrap(),
        "rules": sim::fixtures::test_rules(),
        "units": candidate,
        "events": [],
        "scripts": []
    });
    let refusal = serde_json::from_value::<ScenarioDefinition>(candidate_fixture)
        .expect_err("the current boundary refuses a four-side candidate")
        .to_string();

    println!(
        "{}",
        serde_json::json!({
            "fixed_side_count": Side::ALL.len(),
            "candidate_side_count": 4,
            "candidate_admission": "refused",
            "candidate_refusal": refusal,
            "observation": {
                "blue_own": battle.observe(Side::Blue).own.len(),
                "red_own": battle.observe(Side::Red).own.len()
            },
            "baseline_digest": battle.digest(),
            "replay_digest": restored.digest(),
            "replay_equal": restored.digest() == battle.digest(),
            "accepted_commands": replay.accepted.len()
        })
    );
}
