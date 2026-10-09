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
    let path = args
        .next()
        .expect("usage: native_binding_probe <scenario.json> [seed] [ticks] [side]");
    if path == "--ipc" {
        use std::io::{BufRead, Write};
        let scenario: ScenarioDefinition =
            serde_json::from_slice(&std::fs::read(args.next().expect("scenario path")).unwrap())
                .unwrap();
        let mut battle = Battle::new(&scenario, 11);
        let mut publisher = Publisher::new();
        for line in std::io::stdin().lock().lines() {
            let request: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            if let Some(command) = request.get("command") {
                assert!(battle
                    .accept(serde_json::from_value(command.clone()).unwrap())
                    .error
                    .is_none());
            }
            let started = Instant::now();
            battle.step();
            let step_ms = started.elapsed().as_secs_f64() * 1000.0;
            let started = Instant::now();
            let record = publisher
                .publish(&battle, parse_side(request["side"].as_str().unwrap()))
                .unwrap();
            let publish_ms = started.elapsed().as_secs_f64() * 1000.0;
            // Integer bit patterns preserve packed f32 words, including NaN payloads.
            println!(
                "{}",
                json!({"tick":battle.tick(), "digest":format!("{:016x}",battle.digest()), "words":record.iter().map(|v|v.to_bits()).collect::<Vec<_>>(), "step_ms":step_ms, "publish_ms":publish_ms})
            );
            std::io::stdout().flush().unwrap();
        }
        return;
    }
    if path == "--write-fixture" {
        let output = args.next().expect("fixture output path");
        let fixture = json!({
            "map": {"size": [1024, 1024], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
            "rules": sim::fixtures::test_game(),
            "units": [
                {"side": "blue", "kind": "test_rifle", "position": [40, 40]},
                {"side": "red", "kind": "test_rifle", "position": [900, 900]}
            ]
        });
        let scenario: ScenarioDefinition = serde_json::from_value(fixture).expect("fixture admits");
        std::fs::write(output, serde_json::to_vec(&scenario).unwrap()).unwrap();
        return;
    }
    let seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let ticks = args.next().and_then(|v| v.parse().ok()).unwrap_or(120);
    let side = parse_side(&args.next().unwrap_or_else(|| "blue".into()));
    let evidence = args.next().map(std::path::PathBuf::from);
    let scenario: ScenarioDefinition = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}")),
    )
    .unwrap_or_else(|e| panic!("parse {path}: {e}"));

    let start = Instant::now();
    let mut battle = Battle::new(&scenario, seed);
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut publisher = Publisher::new();
    let mut frames = Vec::new();
    let command = json!({"side":"blue", "seq":1, "order":{"kind":"stop", "units":[0]}});
    if evidence.is_some() {
        assert!(battle
            .accept(serde_json::from_value(command.clone()).unwrap())
            .error
            .is_none());
    }
    let mut publish_bytes = 0usize;
    let mut publish_ms = 0.0;
    let mut step_ms = 0.0;
    let mut copy_total_ms = 0.0;

    for _ in 0..ticks {
        let started = Instant::now();
        battle.step();
        step_ms += started.elapsed().as_secs_f64() * 1000.0;

        let started = Instant::now();
        let record = publisher
            .publish(&battle, side)
            .unwrap_or_else(|e| panic!("publish at tick {}: {e}", battle.tick()));
        publish_ms += started.elapsed().as_secs_f64() * 1000.0;
        let copied_started = Instant::now();
        let copied = record.to_vec();
        let copy_ms = copied_started.elapsed().as_secs_f64() * 1000.0;
        publish_bytes += copied.len() * std::mem::size_of::<f32>();
        copy_total_ms += copy_ms;
        if let Some(dir) = &evidence {
            std::fs::create_dir_all(dir).unwrap();
            let name = format!(
                "{}-{}.bin",
                battle.tick(),
                format!("{side:?}").to_lowercase()
            );
            let bytes: Vec<u8> = copied.iter().flat_map(|v| v.to_le_bytes()).collect();
            std::fs::write(dir.join(&name), bytes).unwrap();
            frames.push(json!({"tick": battle.tick(), "digest":format!("{:016x}", battle.digest()), "file":name}));
        }
    }

    let digest = battle.digest();
    let replay = battle.replay();
    let replay_json = serde_json::to_string(&replay).expect("replay serializes");
    let mut replay_battle = Battle::from_replay(&scenario, &replay).expect("replay loads");
    for _ in 0..ticks {
        replay_battle.step();
    }
    assert_eq!(replay_battle.digest(), digest, "replay digest diverged");

    if let Some(dir) = evidence {
        std::fs::write(dir.join("expected.json"), serde_json::to_vec(&json!({"seed":seed, "side":format!("{side:?}").to_lowercase(), "command":command, "frames":frames})).unwrap()).unwrap();
    }
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
            "copy_ms": copy_total_ms,
            "published_bytes": publish_bytes,
            "digest": format!("{digest:016x}"),
            "replay_bytes": replay_json.len(),
            "replay_digest_match": true,
            "teardown": "drop",
        }))
        .expect("report serializes")
    );
}
