//! Full-extent scale report: what a compiled map costs the simulation.
//!
//!     cargo run -p sim --release --example city_report <map.json> [sim-seconds] [reach] [units-per-side]
//!
//! `reach` is the share of the map's width each unit is sent across (default
//! 0.92: edge to edge) on a fast move; `units-per-side` is at most six.
//! With `centre` for `reach`, one jeep is given an ordinary move from the
//! middle of the west edge to the middle of the map.
//!
//! Stages: parse, world build (terrain, surfaces, forests' trunks), battle
//! build, then a small force crossing the whole map by road and across
//! country. Each stage prints wall time, instructions retired (the
//! load-independent number, macOS only) and resident memory. The crossing
//! prints tick timings and the planning work behind them, the slowest
//! ticks, how long each unit held for its route, when it set off, when it
//! came near its goal and when it stopped, and the battle digest.
use contract::ids::{Side, UnitId};
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
    let reach = args.next().unwrap_or("0.92".into());
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

    // Each unit's side, kind, place, goal and route policy.
    type Row = (&'static str, &'static str, [f64; 2], [f64; 2], &'static str);
    let [w, d] = map.size;
    let mut rows: Vec<Row> = Vec::new();
    if reach == "centre" {
        rows.push((
            "blue",
            "jeep",
            [10.0, d / 2.0],
            [w / 2.0, d / 2.0],
            "shortest",
        ));
    } else {
        // Six units a side near the west and east edges, sent across.
        let reach: f64 = reach
            .parse()
            .expect("reach: a share of the width, or `centre`");
        let (west, east) = (w * 0.04, w * 0.96);
        let kinds = &["jeep", "tank", "rifle", "jeep", "tank", "rifle"][..per_side];
        for (side, x, goal_x) in [
            ("blue", west, west + w * reach),
            ("red", east, east - w * reach),
        ] {
            for (i, kind) in kinds.iter().enumerate() {
                let y = d * (0.3 + 0.08 * i as f64);
                rows.push((side, kind, [x, y], [goal_x, y], "fastest"));
            }
        }
    }
    let mut units = Vec::new();
    let mut scripts = Vec::new();
    for (index, (side, kind, at, goal, route)) in rows.iter().enumerate() {
        let yaw = if goal[0] < at[0] {
            std::f64::consts::PI
        } else {
            0.0
        };
        units.push(
            json!({ "side": side, "kind": kind, "position": at, "yaw": yaw,
            "engagement": "return_fire_only" }),
        );
        scripts.push(json!({ "tick": 1, "side": side, "order": { "kind": "move",
            "units": [index], "gesture": index + 1, "goal": goal, "route": route } }));
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
    let mut arrived: Vec<Option<f64>> = vec![None; rows.len()];
    // Ticks each unit spent holding for a route, and when it first set off.
    let mut held = vec![0u64; rows.len()];
    let mut set_off: Vec<Option<f64>> = vec![None; rows.len()];
    // When each unit first came within 10 m of its goal.
    let mut reached: Vec<Option<f64>> = vec![None; rows.len()];
    let (start, before) = (Instant::now(), instructions());
    let mut worst = (0.0, 0);
    // Planning work: the run's total, the busiest tick's, and the ticks
    // over 33 ms that did any.
    let (mut work, mut busiest, mut slow_planning) = (0u64, 0u64, 0usize);
    // The ticks over 33 ms on which a side rebuilt its planning grid: its
    // knowledge changed the tick before (a tank felled or shoved a tree).
    let revisions = |b: &Battle| Side::ALL.map(|s| b.navigation_revision(s));
    let (mut known, mut slow_rebuilds) = (revisions(&battle), 0usize);
    // The slowest ticks: (ms, tick, instructions, planning work).
    let mut slowest: Vec<(f64, u64, u64, u64)> = Vec::new();
    for t in 1..=seconds * hz {
        let (tick, counted) = (Instant::now(), instructions());
        let before = known;
        battle.step();
        let ms = tick.elapsed().as_secs_f64() * 1000.0;
        let cost = instructions().zip(counted).map_or(0, |(a, b)| a - b);
        slowest.push((ms, t, cost, battle.load().planning_work));
        slowest.sort_by(|a, b| b.0.total_cmp(&a.0));
        slowest.truncate(5);
        let rebuilt = before != std::mem::replace(&mut known, revisions(&battle));
        slow_rebuilds += usize::from(ms > 33.0 && rebuilt);
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
            let near = battle.unit(UnitId(i as u32)).is_some_and(|u| {
                let goal = rows[i].3;
                (u.position.x - goal[0]).hypot(u.position.y - goal[1]) < 10.0
            });
            if near && reached[i].is_none() {
                reached[i] = Some(now);
            }
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
    println!(
        "knowledge: a side's planning picture changed {} times; {slow_rebuilds} ticks over 33 ms followed a change",
        known.iter().sum::<u64>(),
    );
    for (ms, t, cost, planned) in slowest {
        println!(
            "slow tick {t}: {ms:.1} ms, {:.1} M instructions, {planned} planning work",
            cost as f64 / 1e6
        );
    }
    for (i, when) in arrived.iter().enumerate() {
        let (side, kind, ..) = rows[i];
        let unit = battle.unit(UnitId(i as u32));
        let at = unit.map(|u| u.position.xy());
        println!(
            "{side} {kind}: held {:.2} s for routes, set off at {}, within 10 m of its goal at {}, {} at {:?}",
            held[i] as f64 / hz as f64,
            set_off[i].map_or("never".into(), |s| format!("{s:.2} s")),
            reached[i].map_or("never".into(), |s| format!("{s:.1} s")),
            when.map_or_else(
                || format!("{:?}", unit.map(|u| u.state)),
                |s| format!("stopped after {s:.1} s")
            ),
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
