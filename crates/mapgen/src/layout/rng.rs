//! Named random streams. Each consumer draws from its own stream, so adding a
//! draw to one (dressing, say) cannot move what another already placed.
//! Integer state only: the same request draws the same numbers on every target.

use contract::random::{mix, Rng};

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
}
