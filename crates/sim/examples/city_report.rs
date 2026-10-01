//! Full-extent scale report: what a compiled map costs the simulation.
//!
//!     cargo run -p sim --release --example city_report <map.json> [sim-seconds] [reach] [units-per-side] [probe-trees]
//!
//! `reach` is the share of the map's width each unit is sent across (default
//! 0.92: edge to edge) on a fast move. Larger forces repeat the six-unit
//! mix on parallel crossing lanes.
//! With `centre` for `reach`, one jeep is given an ordinary move from the
//! middle of the west edge to the middle of the map.
//!
//! `probe-trees` (default 0: no probe) changes what the sides know mid-run:
//! after 1.5 s a heavy wreck appears 30 m ahead of every vehicle, and after
//! 3 s that many trees are shelled at once, the ones nearest the unit that
//! starts closest to a wood. The report prints what the ticks that took
//! those changes into a side's planning picture cost.
//!
//! Stages: parse, world build (terrain, surfaces, forests' trunks), the two
//! parts of a battle's build that grow with the map (the planning grid both
//! sides share, and the road graph), each built here once more to be
//! measured alone, the battle build itself, then a small force crossing
//! the whole map by road and across country. Each stage prints wall time,
//! instructions retired (the load-independent number, macOS only) and
//! resident memory. The crossing prints tick timings and the planning work
//! behind them, the slowest ticks, how long each unit held for its route,
//! when it set off, when it came near its goal and when it stopped, and
//! the battle digest.
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::observation::MoveState;
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::json;
use sim::battle::Battle;
use sim::navigation::{NavBase, NavGrid, RoadNet};
use sim::world::WorldGeometry;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: city_report <map.json> [sim-seconds]");
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
    let reach = args.next().unwrap_or("0.92".into());
    let per_side: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(6).max(1);
    let probe_trees: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
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
    let world = stage("world build", || {
        let world = WorldGeometry::new(&map, &rules);
        let note = format!("{} props (trees included)", world.props().count());
        (world, note)
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
        let kinds = ["jeep", "tank", "rifle"];
        for (side, x, goal_x) in [
            ("blue", west, west + w * reach),
            ("red", east, east - w * reach),
        ] {
            for i in 0..per_side {
                let kind = kinds[i % kinds.len()];
                let y = if per_side <= 6 {
                    d * (0.3 + 0.08 * i as f64)
                } else {
                    d * (0.1 + 0.8 * i as f64 / (per_side - 1) as f64)
                };
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
    let hz = rules.tick_hz as u64;
    let mut events = Vec::new();
    if probe_trees > 0 {
        for (_, kind, at, goal, _) in rows.iter().filter(|row| row.1 != "rifle") {
            let ahead = (goal[0] - at[0]).signum() * 30.0;
            let hull = rules.catalog.by_id(kind).hull().expect("a vehicle");
            events.push(json!({ "tick": hz * 3 / 2, "add_prop": {
                "kind": "heavy_wreck", "center": [at[0] + ahead, at[1]], "yaw": 0.0,
                "half_extents": hull.half_extents_m } }));
        }
        // The trees nearest each unit, nearest first; the unit whose
        // furthest of them is nearest is the one beside a wood.
        let nearest = |at: [f64; 2]| {
            let mut trees: Vec<(f64, [f64; 2])> = world
                .props()
                .filter(|p| p.forest_tree)
                .map(|p| {
                    let away = (p.center.x - at[0]).hypot(p.center.y - at[1]);
                    (away, [p.center.x, p.center.y])
                })
                .collect();
            trees.sort_by(|a, b| a.0.total_cmp(&b.0));
            trees.truncate(probe_trees);
            trees
        };
        let furthest = |trees: &[(f64, [f64; 2])]| trees.last().map_or(f64::INFINITY, |t| t.0);
        let trees = rows
            .iter()
            .map(|row| nearest(row.2))
            .min_by(|a, b| furthest(a).total_cmp(&furthest(b)))
            .unwrap_or_default();
        println!(
            "probe: {} wrecks at 1.5 s; {} trees shelled at 3 s, {:.0} to {:.0} m from the nearest unit",
            events.len(),
            trees.len(),
            trees.first().map_or(0.0, |t| t.0),
            furthest(&trees),
        );
        for (_, at) in trees {
            events.push(json!({ "tick": hz * 3, "burst": {
                "point": at, "weapon": "tank_he" } }));
        }
    }
    stage("map grid (alone)", || {
        let base = NavBase::build(&world, world.props(), rules.physics.soldier_radius_m);
        let storage = NavGrid::new(std::sync::Arc::new(base)).storage();
        (
            (),
            format!(
                "{} pages of 256 cells hold a body, a road, a wood or a slope",
                storage.cell_pages
            ),
        )
    });
    stage("road graph (alone)", || {
        RoadNet::build(&world);
        ((), String::new())
    });
    drop(world);
    let scenario = json!({ "map": map, "rules": fixture, "units": units, "events": events,
        "scripts": scripts });
    let setup: ScenarioDefinition = stage("scenario", || {
        (
            serde_json::from_value(scenario).expect("a scenario"),
            String::new(),
        )
    });
    let mut battle = stage("battle build", || (Battle::new(&setup, 1), String::new()));

    let mut ticks = Vec::with_capacity((seconds * hz) as usize);
    let mut arrived: Vec<Option<f64>> = vec![None; rows.len()];
    // Ticks each unit spent holding for a route, and when it first set off.
    let mut held = vec![0u64; rows.len()];
    let mut set_off: Vec<Option<f64>> = vec![None; rows.len()];
    // When each unit first came within 10 m of its goal.
    let mut reached: Vec<Option<f64>> = vec![None; rows.len()];
    let (start, before) = (Instant::now(), instructions());
    let mut worst = (0.0, 0);
    let mut worst_cpu = (0.0f64, 0);
    // The tick that retired the most instructions: (instructions, tick).
    let mut dearest = (0u64, 0u64);
    // Planning work: the run's total, the busiest tick's, and the ticks
    // over 33 ms that did any.
    let (mut work, mut busiest, mut slow_planning) = (0u64, 0u64, 0usize);
    // The ticks that take a change of a side's knowledge (a wreck seen, a
    // tree felled or shoved) into its planning picture, each the tick after
    // the change: how many, how many of them ran over 33 ms, and the
    // costliest (instructions, tick).
    let revisions = |b: &Battle| Side::ALL.map(|s| b.navigation_revision(s));
    let (mut known, mut changed) = (revisions(&battle), false);
    let (mut applied, mut slow_applied, mut costliest) = (0usize, 0usize, (0u64, 0u64));
    // Grid cells worked out again to take the changes in: the run's total
    // and the most in one tick.
    let (mut relaid, mut most_relaid) = (0u64, 0u64);
    // The slowest ticks: (ms, tick, instructions, planning work).
    let mut slowest: Vec<(f64, u64, u64, u64, f64)> = Vec::new();
    for t in 1..=seconds * hz {
        let (tick, counted) = (Instant::now(), resources());
        battle.step();
        let ms = tick.elapsed().as_secs_f64() * 1000.0;
        let after = resources();
        let cost = after.zip(counted).map_or(0, |(a, b)| a.0 - b.0);
        let cpu_ms = after
            .zip(counted)
            .map_or(0.0, |(a, b)| (a.1 - b.1) as f64 / 1e6);
        if cpu_ms > worst_cpu.0 {
            worst_cpu = (cpu_ms, t);
        }
        slowest.push((ms, t, cost, battle.load().planning_work, cpu_ms));
        slowest.sort_by(|a, b| b.0.total_cmp(&a.0));
        slowest.truncate(5);
        if changed {
            applied += 1;
            slow_applied += usize::from(ms > 33.0);
            costliest = costliest.max((cost, t));
        }
        changed = std::mem::replace(&mut known, revisions(&battle)) != known;
        if ms > worst.0 {
            worst = (ms, t);
        }
        dearest = dearest.max((cost, t));
        ticks.push(ms);
        let load = battle.load();
        most_relaid = most_relaid.max(load.grid_cells_relaid - relaid);
        relaid = load.grid_cells_relaid;
        let planned = load.planning_work;
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
        "| crossing {seconds} s | {:.0} | {:.1} | {} | tick p50 {:.2} ms, p95 {:.2}, p99 {:.2}, max {:.0} (tick {}); {} ticks over 33 ms; the costliest tick retired {:.1} M instructions (tick {}) |",
        start.elapsed().as_secs_f64() * 1000.0,
        spent as f64 / 1e9,
        rss_mib(),
        at(0.5),
        at(0.95),
        at(0.99),
        worst.0,
        worst.1,
        ticks.iter().filter(|ms| **ms > 33.0).count(),
        dearest.0 as f64 / 1e6,
        dearest.1,
    );
    if resources().is_some() {
        println!(
            "process CPU: maximum tick {:.2} ms (tick {}); wall time includes scheduling delays",
            worst_cpu.0, worst_cpu.1
        );
    } else {
        println!("process CPU: unavailable on this host");
    }
    println!(
        "planning: {work} work in all, {busiest} in the busiest tick (allowance {}); {slow_planning} ticks over 33 ms did any",
        setup.rules.navigation.work_per_tick,
    );
    println!(
        "knowledge: a side's planning picture changed {} times, taken in over {applied} ticks; {slow_applied} of those ran over 33 ms; the costliest retired {:.1} M instructions (tick {}); {relaid} grid cells worked out again, {most_relaid} in the busiest tick",
        known.iter().sum::<u64>(),
        costliest.0 as f64 / 1e6,
        costliest.1,
    );
    for (ms, t, cost, planned, cpu_ms) in slowest {
        println!(
            "slow tick {t}: {ms:.1} ms wall, {cpu_ms:.2} ms CPU, {:.1} M instructions, {planned} planning work",
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
use instructions::{instructions, resources};
