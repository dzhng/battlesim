//! Tick-batched kill rewards; only the credit wallet is published.
use crate::damage::LethalSource;
use crate::skirmish::CREDIT_SCALE;
use contract::ids::{Side, UnitId};
use std::collections::BTreeSet;

/// A full-unit death enriched from its immutable entry receipt.
pub struct EconomicDeath {
    pub victim: UnitId,
    pub side: Side,
    pub price: u32,
    pub source: Option<LethalSource>,
    pub supply: bool,
}

#[derive(Default)]
pub struct Settlement {
    pools: [u64; 2],
    // Sub-microcredit bonus numerators, over the fixed starting budget.
    bonus_remainder: [u64; 2],
    processed: BTreeSet<UnitId>,
}

impl Settlement {
    /// Both sides use the same pre-batch pools. Since every hostile victim
    /// side has one opposing wallet, proportionally capped bonuses can be
    /// accumulated exactly before rounding, without per-death rounding bias.
    pub fn settle(
        &mut self,
        deaths: &[EconomicDeath],
        starting_budget: u64,
        wallets: &mut [u64; 2],
    ) {
        let snapshot = self.pools;
        let mut rewards = [0u64; 2];
        let mut losses = [0u64; 2];
        let mut growth = [0u64; 2];
        let mut bonus_numerator = [0u128; 2];
        for death in deaths {
            if !self.processed.insert(death.victim) {
                continue;
            }
            let Some(source) = death.source.filter(|s| s.side != death.side) else {
                continue;
            };
            let victim = death.side.index();
            let killer = source.side.index();
            let cost = u64::from(death.price) * CREDIT_SCALE;
            rewards[killer] += cost / 4 * if death.supply { 2 } else { 1 };
            losses[victim] += cost / 2;
            growth[killer] += cost / 2;
            if starting_budget > 0 {
                let denominator = u128::from(starting_budget);
                bonus_numerator[victim] += (u128::from(snapshot[victim]) * u128::from(death.price))
                    .min(u128::from(snapshot[victim]) * denominator)
                    .min(u128::from(cost / 2) * denominator);
            }
        }
        for victim in 0..2 {
            let bonus = if starting_budget == 0 {
                0
            } else {
                let denominator = u128::from(starting_budget);
                let available = u128::from(snapshot[victim]) * denominator;
                let numerator = bonus_numerator[victim].min(available)
                    + u128::from(self.bonus_remainder[victim]);
                let paid = (numerator / denominator).min(u128::from(snapshot[victim]));
                self.bonus_remainder[victim] = (numerator - paid * denominator) as u64;
                paid as u64
            };
            rewards[1 - victim] += bonus;
            self.pools[victim] = snapshot[victim]
                .saturating_sub(bonus)
                .saturating_sub(losses[victim]);
        }
        for side in 0..2 {
            self.pools[side] += growth[side];
            wallets[side] += rewards[side];
        }
    }

    pub fn pools(&self) -> [u64; 2] {
        self.pools
    }

    pub fn digest(&self, digest: &mut crate::digest::Digest) {
        for side in 0..2 {
            digest.u64(self.pools[side]).u64(self.bonus_remainder[side]);
        }
        digest.u64(self.processed.len() as u64);
        for id in &self.processed {
            digest.u64(u64::from(id.0));
        }
    }
}
