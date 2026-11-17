//! Native report attribution over the production tick. Counter reads live in
//! the report, so ordinary simulation ticks have no clock or OS dependency.
use super::instructions::instructions;
use sim::battle::TickPhase;

pub const PHASES: [TickPhase; 10] = [
    TickPhase::Orders,
    TickPhase::Navigation,
    TickPhase::Movement,
    TickPhase::Flight,
    TickPhase::Sight,
    TickPhase::Fog,
    TickPhase::Learning,
    TickPhase::Weapons,
    TickPhase::Observation,
    TickPhase::Other,
];

/// Instructions by phase over a span of ticks.
#[derive(Default)]
pub struct Profile {
    totals: [u64; 10],
    maxima: [u64; 10],
    /// The most one whole tick retired.
    dearest: u64,
    ticks: u64,
}

impl Profile {
    /// Step the battle, count the tick, and return its instructions by
    /// phase in [`PHASES`] order.
    pub fn step(&mut self, battle: &mut sim::battle::Battle) -> [u64; 10] {
        let mut previous = instructions();
        let mut tick = [0; 10];
        battle.step_profiled(|phase| {
            let now = instructions();
            let cost = now.zip(previous).map_or(0, |(a, b)| a.saturating_sub(b));
            tick[PHASES.iter().position(|p| *p == phase).unwrap()] += cost;
            previous = now;
        });
        self.record(tick);
        tick
    }

    /// Count a tick another profile stepped.
    pub fn record(&mut self, tick: [u64; 10]) {
        self.ticks += 1;
        self.dearest = self.dearest.max(tick.iter().sum());
        for (i, cost) in tick.into_iter().enumerate() {
            self.totals[i] += cost;
            self.maxima[i] = self.maxima[i].max(cost);
        }
    }

    pub fn print(&self, label: &str) {
        if self.ticks == 0 {
            return;
        }
        println!(
            "\n{label}: {} ticks; instructions include counter-read overhead",
            self.ticks
        );
        println!("| system | mean M instructions/tick | max M/tick | total G |");
        println!("|---|---:|---:|---:|");
        let phases = (PHASES.iter().zip(self.totals).zip(self.maxima))
            .map(|((phase, total), max)| (format!("{phase:?}"), total, max));
        let all = ("all".to_owned(), self.totals.iter().sum(), self.dearest);
        for (name, total, max) in phases.chain([all]) {
            if instructions().is_some() {
                println!(
                    "| {name} | {:.3} | {:.3} | {:.3} |",
                    total as f64 / self.ticks as f64 / 1e6,
                    max as f64 / 1e6,
                    total as f64 / 1e9
                );
            } else {
                println!("| {name} | unavailable | unavailable | unavailable |");
            }
        }
    }
}
