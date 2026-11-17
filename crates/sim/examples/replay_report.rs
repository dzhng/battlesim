//! A saved battle's tick cost: replay the file a player exported and report,
//! per game minute, how long each tick took against the real-time budget and
//! which phase of the step the work went to. Both sides' publications are
//! packed after every tick, as the worker does, and count toward the budget.
//! Accelerated: it steps as fast as it can.
//!
//! The replay is admitted to this build whatever build recorded it, so a
//! change to the simulation can be measured on the same battle. The closing
//! digest says whether that change kept the battle's outcome.
//!
//! On macOS the phase table counts instructions retired, which do not move
//! with machine load; compare those between builds, not the wall times.
//!
//! A third argument names a CSV that receives every tick's wall time and
//! instructions by phase, for attributing the slow ticks.
//!
//!     cargo run -p sim --release --example replay_report -- <battle.json> [ticks] [ticks.csv]
use std::io::Write;
use std::time::Instant;

use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use sim::battle::{Battle, Replay, ENGINE_BUILD_ID};
use sim::publication::Publisher;

fn main() {
    let path = std::env::args().nth(1).expect("a saved battle file");
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read the battle file"))
            .expect("battle file JSON");
    let setup: ScenarioDefinition =
        serde_json::from_str(file["battle"]["scenario"].as_str().expect("scenario"))
            .expect("scenario JSON");
    let mut replay: Replay =
        serde_json::from_str(file["replay"].as_str().expect("replay")).expect("replay JSON");
    let recorded = replay.accepted.last().map_or(0, |(tick, _)| *tick);
    let ticks: u64 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(recorded);
    let mut csv = std::env::args().nth(3).map(|path| {
        let mut file = std::io::BufWriter::new(std::fs::File::create(path).expect("create CSV"));
        let names: Vec<String> = PHASES.iter().map(|p| format!("{p:?}")).collect();
        writeln!(file, "tick,wall_ms,{}", names.join(",")).unwrap();
        file
    });
    if replay.engine_build != ENGINE_BUILD_ID {
        println!("recorded by another simulation build: the digest says whether the outcome held");
        replay.engine_build = ENGINE_BUILD_ID.to_owned();
    }
    let mut battle = Battle::from_replay(&setup, &replay).expect("admit the replay");
    let hz = setup.rules.tick_hz as u64;
    let budget_ms = 1000.0 / hz as f64;
    println!(
        "{ticks} ticks at {hz} Hz; budget {budget_ms:.1} ms a tick; host {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("| minute | tick p50 ms | p95 | max | ticks over budget | over by CPU time | living units | soldiers | corpses | wrecks | projectiles | path searches | step instructions G |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let (mut blue, mut red) = (Publisher::new(), Publisher::new());
    let mut window = Window::default();
    let mut whole = Profile::default();
    let minute = 60 * hz;
    for t in 1..=ticks {
        let before = resources();
        let start = Instant::now();
        let tick = whole.step(&mut battle);
        window.phases.record(tick);
        window.instructions += tick.iter().sum::<u64>();
        blue.publish(&battle, Side::Blue).unwrap();
        red.publish(&battle, Side::Red).unwrap();
        let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
        let cpu_ms = resources()
            .zip(before)
            .map_or(0.0, |(a, b)| (a.1 - b.1) as f64 / 1e6);
        if let Some(file) = &mut csv {
            let costs: Vec<String> = tick.iter().map(u64::to_string).collect();
            writeln!(file, "{t},{wall_ms:.3},{}", costs.join(",")).unwrap();
        }
        window.wall.push(wall_ms);
        window.over += usize::from(wall_ms > budget_ms);
        window.over_cpu += usize::from(cpu_ms > budget_ms);
        if t % minute == 0 || t == ticks {
            let load = battle.load();
            let n = window.wall.len();
            let q = quantiles(&mut window.wall);
            println!(
                "| {:.1} | {:.1} | {:.1} | {:.0} | {} of {n} ({:.0}%) | {} ({:.0}%) | {} | {} | {} | {} | {} | {} | {} |",
                t as f64 / minute as f64,
                q[0], q[1], q[2],
                window.over, 100.0 * window.over as f64 / n as f64,
                window.over_cpu, 100.0 * window.over_cpu as f64 / n as f64,
                load.living_units, load.living_soldiers, load.corpses, load.wrecks,
                load.active_projectiles, load.path_searches,
                billions(window.instructions),
            );
            if t % minute == 0 && t + minute > ticks {
                window
                    .phases
                    .print(&format!("Minute to {:.1}", t as f64 / minute as f64));
            }
            window = Window::default();
        }
    }
    whole.print("Whole battle");
    println!("\ndigest {:016x}", battle.digest());
}

/// One reported span of ticks.
#[derive(Default)]
struct Window {
    wall: Vec<f64>,
    over: usize,
    over_cpu: usize,
    instructions: u64,
    phases: Profile,
}

fn quantiles(v: &mut [f64]) -> [f64; 3] {
    v.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    [at(0.5), at(0.95), v[v.len() - 1]]
}

/// A count in billions, or a dash where the platform gives none.
fn billions(n: u64) -> String {
    match instructions() {
        Some(_) => format!("{:.1}", n as f64 / 1e9),
        None => "-".into(),
    }
}

#[path = "common/instructions.rs"]
mod instructions;
use instructions::{instructions, resources};

#[path = "common/profile.rs"]
mod profile;
use profile::{Profile, PHASES};
