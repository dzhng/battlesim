//! Full-extent scale report: what a compiled map costs the simulation.
//!
//!     cargo run -p sim --release --example city_report <map.json> [sim-seconds] [reach] [units-per-side]
//!
//! `reach` is the share of the map's width each unit is sent across (default
//! 0.92: edge to edge); `units-per-side` is at most six.
//!
//! Stages: parse, world build (terrain, surfaces, forests' trunks), battle
//! build, then a small force crossing the whole map by road and across
//! country. Each stage prints wall time, instructions retired (the
//! load-independent number, macOS only) and resident memory. The crossing
//! prints tick timings and the planning work behind them, how long each
//! unit held for its route and when it arrived, and the battle digest.
use contract::ids::UnitId;
use contract::map::MapDefinition;
use contract::observation::MoveState;
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::json;
use sim::battle::Battle;
use sim::world::WorldGeometry;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: city_report <map.json> [sim-seconds]");
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
    let reach: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.92);
    let per_side: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(6).min(6);
    let fixture = sim::fixtures::village();

    println!("| stage | wall ms | instructions G | RSS MiB | note |");
    println!("|---|---|---|---|---|");
    let raw = std::fs::read_to_string(&path).expect("read the map");
    let map: MapDefinition = stage("parse map", || {
        let map: MapDefinition = serde_json::from_str(&raw).expect("a compiled map");
        let note = format!(
            "{} KiB; {:.0} × {:.0} m; {} surfaces, {} forests, {} buildings",
            raw.len() / 1024,
            map.size[0],
            map.size[1],
            map.surfaces.len(),
            map.forests.len(),
            map.buildings.len()
        );
        (map, note)
    });
    let rules: Rules = serde_json::from_value(fixture.clone()).expect("the village rules");
    stage("world build", || {
        let world = WorldGeometry::new(&map, &rules);
        let note = format!("{} props (trees included)", world.props().count());
        ((), note)
    });

    // Six units a side on the outer north–south roads, sent to the far side.
    let [w, d] = map.size;
    let (west, east) = (w * 0.04, w * 0.96);
    let kinds = &["jeep", "tank", "rifle", "jeep", "tank", "rifle"][..per_side];
    let mut units = Vec::new();
    let mut scripts = Vec::new();
    for (side, x, goal_x, yaw) in [
        ("blue", west, west + w * reach, 0.0),
        ("red", east, east - w * reach, std::f64::consts::PI),
    ] {
        for (i, kind) in kinds.iter().enumerate() {
            let y = d * (0.3 + 0.08 * i as f64);
            let index = units.len();
            units.push(
                json!({ "side": side, "kind": kind, "position": [x, y], "yaw": yaw,
                "engagement": "return_fire_only" }),
            );
            scripts.push(json!({ "tick": 1, "side": side, "order": { "kind": "move",
                "units": [index], "gesture": index + 1, "goal": [goal_x, y], "route": "fastest" } }));
        }
    }
    let scenario = json!({ "map": map, "rules": fixture, "units": units, "events": [],
        "scripts": scripts });
    let setup: ScenarioDefinition = stage("scenario", || {
        (
            serde_json::from_value(scenario).expect("a scenario"),
            String::new(),
        )
    });
    let mut battle = stage("battle build", || (Battle::new(&setup, 1), String::new()));

    let hz = setup.rules.tick_hz as u64;
    let mut ticks = Vec::with_capacity((seconds * hz) as usize);
    let mut arrived: Vec<Option<f64>> = vec![None; kinds.len() * 2];
    // Ticks each unit spent holding for a route, and when it first set off.
    let mut held = vec![0u64; kinds.len() * 2];
    let mut set_off: Vec<Option<f64>> = vec![None; kinds.len() * 2];
    let (start, before) = (Instant::now(), instructions());
    let mut worst = (0.0, 0);
    // Planning work: the run's total, the busiest tick's, and the ticks
    // over 33 ms that did any.
    let (mut work, mut busiest, mut slow_planning) = (0u64, 0u64, 0usize);
    for t in 1..=seconds * hz {
        let tick = Instant::now();
        battle.step();
        let ms = tick.elapsed().as_secs_f64() * 1000.0;
        if ms > worst.0 {
            worst = (ms, t);
        }
        ticks.push(ms);
        let planned = battle.load().planning_work;
        work += planned;
        busiest = busiest.max(planned);
        slow_planning += usize::from(ms > 33.0 && planned > 0);
        for i in 0..arrived.len() {
            let state = battle.unit(UnitId(i as u32)).map(|u| u.state);
            let now = t as f64 / hz as f64;
            match state {
                Some(MoveState::Planning) => held[i] += 1,
                Some(MoveState::Moving) if set_off[i].is_none() => set_off[i] = Some(now),
                Some(MoveState::Idle) if t > 2 && arrived[i].is_none() => arrived[i] = Some(now),
                _ => {}
            }
        }
    }
    let spent = instructions().zip(before).map_or(0, |(a, b)| a - b);
    ticks.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| ticks[((ticks.len() - 1) as f64 * q).round() as usize];
    println!(
        "| crossing {seconds} s | {:.0} | {:.1} | {} | tick p50 {:.2} ms, p95 {:.2}, p99 {:.2}, max {:.0} (tick {}); {} ticks over 33 ms |",
        start.elapsed().as_secs_f64() * 1000.0,
        spent as f64 / 1e9,
        rss_mib(),
        at(0.5),
        at(0.95),
        at(0.99),
        worst.0,
        worst.1,
        ticks.iter().filter(|ms| **ms > 33.0).count(),
    );
    println!(
        "planning: {work} work in all, {busiest} in the busiest tick (allowance {}); {slow_planning} ticks over 33 ms did any",
        setup.rules.navigation.work_per_tick,
    );
    for (i, when) in arrived.iter().enumerate() {
        let side = if i < kinds.len() { "blue" } else { "red" };
        let unit = battle.unit(UnitId(i as u32));
        let at = unit.map(|u| u.position.xy());
        println!(
            "{side} {}: held {:.2} s for routes, set off at {}, {} at {:?}",
            kinds[i % kinds.len()],
            held[i] as f64 / hz as f64,
            set_off[i].map_or("never".into(), |s| format!("{s:.2} s")),
            when.map_or("still moving".into(), |s| format!("stopped after {s:.1} s")),
            at.map(|p| (p.x.round(), p.y.round())),
        );
    }
    println!("digest {:016x}", battle.digest());
}

/// Run `work`, then print its row; `work` returns its value and a note.
fn stage<T>(name: &str, work: impl FnOnce() -> (T, String)) -> T {
    let (start, before) = (Instant::now(), instructions());
    let (value, note) = work();
    let spent = instructions().zip(before).map_or(0, |(a, b)| a - b);
    println!(
        "| {name} | {:.0} | {:.2} | {} | {note} |",
        start.elapsed().as_secs_f64() * 1000.0,
        spent as f64 / 1e9,
        rss_mib(),
    );
    value
}

/// This process's resident memory, from `ps` (native only).
fn rss_mib() -> u64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(0, |kib| kib / 1024)
}

#[path = "common/instructions.rs"]
mod instructions;
use instructions::instructions;
