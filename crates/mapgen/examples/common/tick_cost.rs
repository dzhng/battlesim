//! What a battle's ticks cost, in instructions retired (macOS only: `None`
//! elsewhere): in all, the dearest ticks, and by phase of the step, to
//! attribute a change in cost.
use std::collections::BTreeMap;

use serde::Serialize;
use sim::battle::{Battle, TickPhase};

use crate::instructions::instructions;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Cost {
    pub instructions: Option<u64>,
    pub mean_instructions: Option<u64>,
    /// All but one tick in a hundred cost no more than this.
    pub p99_instructions: Option<u64>,
    pub max_instructions: Option<u64>,
    /// The dearest tick, counted from one.
    pub max_instruction_tick: Option<u64>,
    /// What the dearest tick spent, by phase.
    pub max_tick_phase_instructions: Option<BTreeMap<String, u64>>,
    pub phase_instructions: Option<BTreeMap<String, u64>>,
}

type Phases = Vec<(TickPhase, u64)>;

fn add(phases: &mut Phases, phase: TickPhase, cost: u64) {
    match phases.iter_mut().find(|(p, _)| *p == phase) {
        Some((_, sum)) => *sum += cost,
        None => phases.push((phase, cost)),
    }
}

fn named(phases: &Phases) -> BTreeMap<String, u64> {
    (phases.iter())
        .map(|(phase, n)| (format!("{phase:?}"), *n))
        .collect()
}

/// Steps a battle, counting what each tick costs.
#[derive(Default)]
pub struct Meter {
    ticks: Vec<u64>,
    phases: Phases,
    /// The dearest tick so far, by phase.
    dearest: (u64, Phases),
}

impl Meter {
    /// Step `battle` one tick. The counter is read again after each phase's
    /// tally, so no phase pays for it.
    pub fn step(&mut self, battle: &mut Battle) {
        let before = instructions();
        let mut previous = before;
        let mut tick: Phases = Vec::new();
        battle.step_profiled(|phase| {
            if let Some(cost) = instructions().zip(previous) {
                add(&mut tick, phase, cost.0.saturating_sub(cost.1));
            }
            previous = instructions();
        });
        for &(phase, cost) in &tick {
            add(&mut self.phases, phase, cost);
        }
        if let Some(cost) = instructions().zip(before).map(|(a, b)| a.saturating_sub(b)) {
            self.ticks.push(cost);
            if cost > self.dearest.0 {
                self.dearest = (cost, tick);
            }
        }
    }

    pub fn finish(self) -> Cost {
        if self.ticks.is_empty() {
            return Cost::default();
        }
        let total: u64 = self.ticks.iter().sum();
        let (peak_tick, peak) = (self.ticks.iter().enumerate())
            .max_by_key(|&(tick, cost)| (*cost, std::cmp::Reverse(tick)))
            .expect("a tick was counted");
        let mut sorted = self.ticks.clone();
        sorted.sort_unstable();
        Cost {
            instructions: Some(total),
            mean_instructions: Some(total / self.ticks.len() as u64),
            p99_instructions: Some(sorted[(sorted.len() - 1) * 99 / 100]),
            max_instructions: Some(*peak),
            max_instruction_tick: Some(peak_tick as u64 + 1),
            max_tick_phase_instructions: Some(named(&self.dearest.1)),
            phase_instructions: Some(named(&self.phases)),
        }
    }
}
