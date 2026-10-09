//! Named random streams. Each consumer draws from its own stream, so adding a
//! draw to one (dressing, say) cannot move what another already placed.
//! Integer state only: the same request draws the same numbers on every target.

use contract::random::{mix, Rng};
use std::collections::BTreeMap;

pub struct Stream(Rng);

impl Stream {
    pub fn new(seed: u64, name: &str) -> Self {
        // FNV-1a spreads the name; the seed is mixed so that neighbouring
        // seeds do not give neighbouring streams.
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in name.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(Rng::new(mix(seed) ^ hash))
    }

    /// Uniform in `[0, 1)`, exact in an f64.
    pub fn unit(&mut self) -> f64 {
        self.0.unit()
    }

    pub fn range(&mut self, range: [f64; 2]) -> f64 {
        range[0] + (range[1] - range[0]) * self.unit()
    }

    /// Uniform in `0..count`; `count` must be positive.
    pub fn below(&mut self, count: u64) -> u64 {
        ((u128::from(self.0.next_u64()) * u128::from(count)) >> 64) as u64
    }

    /// Uniform over the inclusive range.
    pub fn count(&mut self, range: [u32; 2]) -> u32 {
        range[0] + self.below(u64::from(range[1] - range[0]) + 1) as u32
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }

    /// One of `items` drawn by its weight (`by_weight` of a fresh draw);
    /// `None` for no items.
    pub fn pick<T>(&mut self, items: impl IntoIterator<Item = (T, f64)> + Clone) -> Option<T> {
        by_weight(self.unit(), items)
    }

    /// Every key of `table` in an order drawn by weight: each in turn the
    /// heavier the likelier to come first (Efraimidis–Spirakis), so where
    /// the first will not do the next is tried.
    pub fn drawn<'k, K>(&mut self, table: &'k BTreeMap<K, f64>) -> Vec<&'k K> {
        in_drawn_order(
            table
                .iter()
                .map(|(key, weight)| (-libm::log(1.0 - self.unit()) / weight, key))
                .collect(),
        )
    }

    /// `items` in an order this stream draws.
    pub fn shuffled<T>(&mut self, items: Vec<T>) -> Vec<T> {
        in_drawn_order(items.into_iter().map(|item| (self.unit(), item)).collect())
    }
}

/// The item a uniform draw `drawn` in `[0, 1)` lands on when `items` share
/// the sum of their weights in their order, each its weight's span; the last
/// where rounding leaves the draw past every span. `None` for no items.
pub fn by_weight<T>(drawn: f64, items: impl IntoIterator<Item = (T, f64)> + Clone) -> Option<T> {
    let mut pick = drawn
        * items
            .clone()
            .into_iter()
            .map(|(_, weight)| weight)
            .sum::<f64>();
    let mut last = None;
    for (item, weight) in items {
        pick -= weight;
        if pick < 0.0 {
            return Some(item);
        }
        last = Some(item);
    }
    last
}

/// The items of `keyed` by their drawn keys, least first; items whose keys
/// tie keep their order.
pub fn in_drawn_order<T>(mut keyed: Vec<(f64, T)>) -> Vec<T> {
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|(_, item)| item).collect()
}
