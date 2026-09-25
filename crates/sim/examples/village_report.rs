//! The village tactical comparison report (encounter.md): every script over
//! the fixture's repeatability seeds, run in parallel, as a Markdown table.
//!
//!     cargo run -p sim --release --example village_report [max_seconds] [script]
use std::thread;

use sim::village::scripts::Plan;
use sim::village::{trial, Trial};

fn main() {
    let max_s: f64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(900.0);
    let only = std::env::args().nth(2);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/village.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let seeds: Vec<u64> = serde_json::from_value(fixture["repeatability_seeds"].clone()).unwrap();
    let runs: [(&str, &str, Plan); 5] = [
        ("unsupported-road-push", "ordinary", Plan::UnsupportedPush),
        ("scout-suppress-flank", "ordinary", Plan::ScoutSuppressFlank),
        (
            "ordinary-ambush-retreat (0.75 s)",
            "ordinary",
            Plan::AmbushRetreat { delay_s: 0.75 },
        ),
        (
            "ordinary-ambush-retreat (3 s)",
            "ordinary",
            Plan::AmbushRetreat { delay_s: 3.0 },
        ),
        (
            "prepared-crossfire (0.75 s)",
            "prepared_crossfire",
            Plan::AmbushRetreat { delay_s: 0.75 },
        ),
    ];
    println!("| script | seed | result | captured s | blue cost lost | tanks lost | rejoined |");
    println!("|---|---|---|---|---|---|---|");
    for (name, variant, plan) in runs {
        if only.as_ref().is_some_and(|o| !name.starts_with(o.as_str())) {
            continue;
        }
        let results: Vec<(u64, Trial)> = thread::scope(|s| {
            let handles: Vec<_> = seeds
                .iter()
                .map(|&seed| {
                    let fixture = &fixture;
                    s.spawn(move || (seed, trial(fixture, variant, plan, seed, max_s)))
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let (mut caps, mut cost, mut tanks, mut survivors, mut rejoined) = (0, 0.0, 0, 0, 0);
        for (seed, t) in &results {
            println!(
                "| {name} | {seed} | {:?} | {} | {:.0} | {} | {} |",
                t.result,
                t.captured_s.map_or("—".into(), |c| format!("{c:.0}")),
                t.blue_cost_lost,
                t.tanks_lost,
                t.rejoined
            );
            caps += t.captured_s.is_some() as u32;
            cost += t.blue_cost_lost;
            tanks += t.tanks_lost;
            survivors += (t.tanks_lost < t.tanks) as u32;
            assert_eq!(
                t.rejected, 0,
                "{name} seed {seed}: a script order was refused"
            );
            rejoined += t.rejoined;
        }
        println!(
                        "| **{name}** | all | **{caps}/{} captured** | | **{cost:.0}** | **{tanks}** (a tank survived in {survivors}) | **{rejoined}** |",
            results.len()
        );
    }
}
