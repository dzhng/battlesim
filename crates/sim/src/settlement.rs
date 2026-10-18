//! Tick-batched kill rewards; only the credit wallet is published.
use crate::damage::LethalSource;
use crate::skirmish::CREDIT_SCALE;
use contract::ids::{Side, UnitId};
use std::collections::BTreeMap;
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
    pub fn settle_players(
        &mut self,
        deaths: &[EconomicDeath],
        starting_budget: u64,
        killer_players: &BTreeMap<UnitId, usize>,
        players: &mut [crate::skirmish::PlayerState],
    ) {
        let snapshot = self.pools;
        let mut rewards = vec![0u64; players.len()];
        let mut losses = [0u64; 2];
        let mut growth = vec![0u64; players.len()];
        let mut bonus_numerator: [BTreeMap<usize, u128>; 2] =
            std::array::from_fn(|_| BTreeMap::new());
        for death in deaths {
            if !self.processed.insert(death.victim) {
                continue;
            }
            let Some(source) = death.source.filter(|s| s.side != death.side) else {
                continue;
            };
            let victim = death.side.index();
            let Some(&killer) = killer_players.get(&source.unit) else {
                continue;
            };
            let cost = u64::from(death.price) * CREDIT_SCALE;
            rewards[killer] += cost / 4 * if death.supply { 2 } else { 1 };
            growth[killer] += cost / 2;
            losses[victim] += cost / 2;
            if starting_budget > 0 {
                let n = (u128::from(snapshot[victim]) * u128::from(death.price))
                    .min(u128::from(snapshot[victim]) * u128::from(starting_budget))
                    .min(u128::from(cost / 2) * u128::from(starting_budget));
                *bonus_numerator[victim].entry(killer).or_default() += n;
            }
        }
        for victim in 0..2 {
            if bonus_numerator[victim].is_empty() {
                continue;
            }
            let denominator = u128::from(starting_budget.max(1));
            let available = u128::from(snapshot[victim]) * denominator;
            let total: u128 = bonus_numerator[victim].values().copied().sum();
            if total == 0 {
                continue;
            }
            // Cap what is owed at the pool before carrying the remainder, so
            // the carry stays below one credit's denominator.
            let numerator = total.min(available) + u128::from(self.bonus_remainder[victim]);
            let paid = (numerator / denominator).min(u128::from(snapshot[victim]));
            self.bonus_remainder[victim] = (numerator - paid * denominator) as u64;
            let mut remainder = paid;
            let mut fractional = Vec::with_capacity(bonus_numerator[victim].len());
            for (&killer, &n) in &bonus_numerator[victim] {
                let numerator = paid * n;
                let share = numerator / total;
                rewards[killer] += share as u64;
                remainder -= share;
                fractional.push((numerator % total, killer));
            }
            fractional.sort_by(
                |(left_fraction, left_player), (right_fraction, right_player)| {
                    right_fraction
                        .cmp(left_fraction)
                        .then_with(|| left_player.cmp(right_player))
                },
            );
            for (_, killer) in fractional.into_iter().take(remainder as usize) {
                rewards[killer] += 1;
            }
            self.pools[victim] = snapshot[victim]
                .saturating_sub(paid as u64)
                .saturating_sub(losses[victim]);
        }
        for (i, player) in players.iter_mut().enumerate() {
            player.wallet += rewards[i];
        }
        for side in 0..2 {
            self.pools[side] += growth
                .iter()
                .enumerate()
                .filter(|(i, _)| players[*i].side.index() == side)
                .map(|(_, v)| *v)
                .sum::<u64>();
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
