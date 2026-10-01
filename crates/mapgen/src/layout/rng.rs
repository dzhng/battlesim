//! Named random streams. Each consumer draws from its own stream, so adding a
//! draw to one (dressing, say) cannot move what another already placed.
//! Integer state only: the same request draws the same numbers on every target.

pub struct Stream(u64);

impl Stream {
    pub fn new(seed: u64, name: &str) -> Self {
        // FNV-1a spreads the name; the seed is mixed so that neighbouring
        // seeds do not give neighbouring streams.
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in name.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(mix(seed) ^ hash)
    }

    /// SplitMix64.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.0)
    }

    /// Uniform in `[0, 1)`, exact in an f64.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    pub fn range(&mut self, range: [f64; 2]) -> f64 {
        range[0] + (range[1] - range[0]) * self.unit()
    }

    /// Uniform in `0..count`; `count` must be positive.
    pub fn below(&mut self, count: u64) -> u64 {
        ((u128::from(self.next()) * u128::from(count)) >> 64) as u64
    }

    /// Uniform over the inclusive range.
    pub fn count(&mut self, range: [u32; 2]) -> u32 {
        range[0] + self.below(u64::from(range[1] - range[0]) + 1) as u32
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
