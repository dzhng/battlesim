//! The endurance soak: the synthetic 100-a-side battle for N
//! simulated minutes, every five minutes a row of tick timings and the
//! battle's load, read from its owning stores. Blue's publication is packed
//! after every tick, as the worker does, outside the timed step: its bytes
//! per tick, and the ground patch's share. Accelerated: it steps as fast as
//! it can, which is not the same as a real-time rendered run.
//!
//! On macOS each row also counts the instructions the steps retired (the
//! publication packing excluded), in billions: unlike wall time it does not
//! move with machine load, so it is the number to compare between builds.
//!
//!     cargo run -p sim --release --example endurance_report [minutes] [late]
use std::time::Instant;

use contract::ids::Side;
use sim::battle::Battle;
use sim::publication::Publisher;

fn main() {
    let minutes: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
        .max(5);
    let late = std::env::args().nth(2).as_deref() == Some("late");
    let fixture = sim::fixtures::village();
    let setup = sim::endurance::scenario(
        &sim::maps::load("endurance").unwrap().definition,
        &fixture,
        1,
        late,
    )
    .unwrap();
    let hz = setup.rules.tick_hz as u64;
    let built = Instant::now();
    let mut battle = Battle::new(&setup, 1);
    println!(
        "{} units set up in {:.0} ms{}; host {} {}",
        setup.units.len(),
        built.elapsed().as_secs_f64() * 1000.0,
        if late { " (late state)" } else { "" },
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("| minutes | tick p50 ms | p95 | p99 | max | ticks > 33 ms | living units | soldiers | corpses | wrecks | projectiles in flight (peak) | projectiles launched per sim-second (peak) | path searches | ground KiB | learned ground KiB (both sides) | blue publication B/tick p50 / p95 / max | ground patch B/tick p50 / p95 / max | step instructions G | RSS MiB |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut all = Vec::new();
    let mut window = Vec::new();
    let mut publisher = Publisher::new();
    let (mut record_bytes, mut patch_bytes) = (Vec::new(), Vec::new());
    let (mut peak_active, mut peak_rate, mut second_start) = (0, 0, 0);
    let (mut window_instructions, mut run_instructions) = (0u64, 0u64);
    for t in 1..=minutes * 60 * hz {
        let before = instructions();
        let start = Instant::now();
        battle.step();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let stepped = instructions().zip(before).map_or(0, |(a, b)| a - b);
        window_instructions += stepped;
        run_instructions += stepped;
        window.push(ms);
        all.push(ms);
        let floats = publisher.publish(&battle, Side::Blue).unwrap().len();
        record_bytes.push(floats as f64 * 4.0);
        patch_bytes.push(publisher.ground_patch_bytes() as f64);
        let load = battle.load();
        peak_active = peak_active.max(load.active_projectiles);
        if t % hz == 0 {
            peak_rate = peak_rate.max(load.rounds_launched - second_start);
            second_start = load.rounds_launched;
        }
        if t % (5 * 60 * hz) == 0 {
            let q = quantiles(&mut window);
            let r = quantiles(&mut record_bytes);
            let g = quantiles(&mut patch_bytes);
            println!(
                "| {} | {:.1} | {:.1} | {:.1} | {:.0} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} / {:.0} / {:.0} | {:.0} / {:.0} / {:.0} | {} | {} |",
                t / (60 * hz),
                q[0], q[1], q[2], q[3],
                window.iter().filter(|&&m| m > 1000.0 / hz as f64).count(),
                load.living_units, load.living_soldiers, load.corpses, load.wrecks,
                peak_active, peak_rate, load.path_searches, load.ground_bytes / 1024,
                load.known_ground_bytes / 1024, r[0], r[1], r[3], g[0], g[1], g[3],
                billions(window_instructions), rss_mib(),
            );
            window_instructions = 0;
            window.clear();
            record_bytes.clear();
            patch_bytes.clear();
            peak_active = 0;
            peak_rate = 0;
        }
    }
    let q = quantiles(&mut all);
    println!(
        "\nWhole run: p50 {:.1} ms, p95 {:.1}, p99 {:.1}, max {:.0}; {} rounds launched; {} G step instructions; digest {:016x}.",
        q[0],
        q[1],
        q[2],
        q[3],
        battle.load().rounds_launched,
        billions(run_instructions),
        battle.digest()
    );
}

fn quantiles(v: &mut [f64]) -> [f64; 4] {
    v.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    [at(0.5), at(0.95), at(0.99), v[v.len() - 1]]
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

/// A count in billions, or a dash where the platform gives none.
fn billions(n: u64) -> String {
    match instructions() {
        Some(_) => format!("{:.1}", n as f64 / 1e9),
        None => "-".into(),
    }
}
