//! Probe the native simulation seam used by a future desktop client.
//!
//!     cargo run -p sim --release --example native_binding_probe -- <scenario.json> [seed] [ticks] [side]
//!
//! The probe deliberately measures the host-facing operations separately:
//! fixed-tick stepping, side-filtered bulk publication, copying the published
//! record, and replay/digest verification. It has no renderer or engine
//! dependency, so its output is also the offline-capture control for the
//! Godot binding spike.

use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;
use sim::publication::Publisher;
use std::time::Instant;

fn parse_side(value: &str) -> Side {
    match value {
        "blue" => Side::Blue,
        "red" => Side::Red,
        other => panic!("side must be blue or red, got {other}"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: native_binding_probe <scenario.json> [seed] [ticks] [side]");
    let seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let ticks = args.next().and_then(|v| v.parse().ok()).unwrap_or(120);
    let side = parse_side(&args.next().unwrap_or_else(|| "blue".into()));
    let scenario: ScenarioDefinition = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}")),
    )
    .unwrap_or_else(|e| panic!("parse {path}: {e}"));

    let start = Instant::now();
    let mut battle = Battle::new(&scenario, seed);
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut publisher = Publisher::new();
    let mut publish_bytes = 0usize;
    let mut publish_ms = 0.0;
    let mut step_ms = 0.0;

    for _ in 0..ticks {
        let started = Instant::now();
        battle.step();
        step_ms += started.elapsed().as_secs_f64() * 1000.0;

        let started = Instant::now();
        let copied = publisher
            .publish(&battle, side)
            .unwrap_or_else(|e| panic!("publish at tick {}: {e}", battle.tick()))
            .to_vec();
        publish_ms += started.elapsed().as_secs_f64() * 1000.0;
        publish_bytes += copied.len() * std::mem::size_of::<f32>();
    }

    let digest = battle.digest();
    let replay = battle.replay();
    let replay_json = serde_json::to_string(&replay).expect("replay serializes");
    let replay_battle = Battle::from_replay(&scenario, &replay).expect("replay loads");
    assert_eq!(replay_battle.digest(), digest, "replay digest diverged");

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "candidate": "offline-native-simulation",
            "scenario": path,
            "seed": seed,
            "ticks": ticks,
            "side": format!("{side:?}").to_lowercase(),
            "build_ms": build_ms,
            "step_ms": step_ms,
            "publish_ms": publish_ms,
            "published_bytes": publish_bytes,
            "digest": format!("{digest:016x}"),
            "replay_bytes": replay_json.len(),
            "replay_digest_match": true,
            "teardown": "drop",
        }))
        .expect("report serializes")
    );
}
