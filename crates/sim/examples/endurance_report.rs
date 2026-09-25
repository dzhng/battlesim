//! The endurance soak: the synthetic 100-a-side battle for N
//! simulated minutes, every five minutes a row of tick timings and the
//! battle's load, read from its owning stores. Accelerated: it steps as fast
//! as it can, which is not the same as a real-time rendered run.
//!
//!     cargo run -p sim --release --example endurance_report [minutes] [late]
use std::time::Instant;

use sim::battle::Battle;

fn main() {
    let minutes: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
        .max(5);
    let late = std::env::args().nth(2).as_deref() == Some("late");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/village.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let setup = sim::endurance::scenario(&fixture, 1, late).unwrap();
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
    println!("| minutes | tick p50 ms | p95 | p99 | max | ticks > 33 ms | living units | soldiers | corpses | wrecks | projectiles in flight (peak) | projectiles launched per sim-second (peak) | path searches | RSS MiB |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut all = Vec::new();
    let mut window = Vec::new();
    let (mut peak_active, mut peak_rate, mut second_start) = (0, 0, 0);
    for t in 1..=minutes * 60 * hz {
        let start = Instant::now();
        battle.step();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        window.push(ms);
        all.push(ms);
        let load = battle.load();
        peak_active = peak_active.max(load.active_projectiles);
        if t % hz == 0 {
            peak_rate = peak_rate.max(load.rounds_launched - second_start);
            second_start = load.rounds_launched;
        }
        if t % (5 * 60 * hz) == 0 {
            let q = quantiles(&mut window);
            println!(
                "| {} | {:.1} | {:.1} | {:.1} | {:.0} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                t / (60 * hz),
                q[0], q[1], q[2], q[3],
                window.iter().filter(|&&m| m > 1000.0 / hz as f64).count(),
                load.living_units, load.living_soldiers, load.corpses, load.wrecks,
                peak_active, peak_rate, load.path_searches, rss_mib(),
            );
            window.clear();
            peak_active = 0;
            peak_rate = 0;
        }
    }
    let q = quantiles(&mut all);
    println!(
        "\nWhole run: p50 {:.1} ms, p95 {:.1}, p99 {:.1}, max {:.0}; {} rounds launched.",
        q[0],
        q[1],
        q[2],
        q[3],
        battle.load().rounds_launched
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
