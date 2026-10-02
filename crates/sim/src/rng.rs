//! Deterministic combat randomness (SplitMix64). Integer draws and pinned
//! normal-sampling math agree across native and Wasm. The state is one u64
//! so it enters digests and replays directly.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1) with 53 bits of precision.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Standard normal (Box–Muller, one value per pair of uniforms).
    pub fn normal(&mut self) -> f64 {
        // 1 - unit() lies in (0, 1], so the logarithm is finite.
        let r = (-2.0 * libm::log(1.0 - self.unit())).sqrt();
        r * libm::cos(std::f64::consts::TAU * self.unit())
    }

    /// Standard normal truncated to ±`limit` standard deviations by rejection.
    pub fn truncated_normal(&mut self, limit: f64) -> f64 {
        loop {
            let z = self.normal();
            if z.abs() <= limit {
                return z;
            }
        }
    }
}
