//! Seeded randomness, defined once for both clients: Mulberry32, seeded
//! from a [`Key`] of the thing it decides for, so the same battle draws and
//! sounds the same wherever it runs.

/// 2³² over the golden ratio: spreads consecutive keys across the seeds.
pub const GOLDEN: u32 = 0x9e37_79b1;

/// A Mulberry32 state.
#[derive(Clone, Copy, Debug)]
pub struct Mulberry32(u32);

impl Mulberry32 {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// The next number in [0, 1).
    pub fn sample(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    /// The next number in [lo, hi).
    pub fn float(&mut self, lo: f64, hi: f64) -> f64 {
        self.sample() * (hi - lo) + lo
    }
}

/// A seed hashed from the parts that name what it decides for (FNV-1a over
/// their bytes): numbers by their bits, names by their text.
#[derive(Clone, Copy, Debug)]
pub struct Key(u32);

impl Key {
    pub const fn new() -> Self {
        Self(2_166_136_261)
    }

    fn bytes(self, bytes: &[u8]) -> Self {
        Self(
            bytes
                .iter()
                .fold(self.0, |h, &b| (h ^ u32::from(b)).wrapping_mul(16_777_619)),
        )
    }

    /// Adds a number; −0 is 0.
    pub fn num(self, x: f64) -> Self {
        self.bytes(&(x + 0.0).to_bits().to_le_bytes())
    }

    /// Adds each number of `xs`: a point, say.
    pub fn nums(self, xs: &[f64]) -> Self {
        xs.iter().fold(self, |k, &x| k.num(x))
    }

    /// Adds a name, ended so that two names never run together.
    pub fn text(self, s: &str) -> Self {
        self.bytes(s.as_bytes()).bytes(&[0xff])
    }

    pub fn seed(self) -> u32 {
        self.0
    }

    /// A generator seeded by the key.
    pub fn rng(self) -> Mulberry32 {
        Mulberry32::new(self.0)
    }

    /// One number in [0, 1) for the key.
    pub fn unit(self) -> f64 {
        self.rng().sample()
    }
}

impl Default for Key {
    fn default() -> Self {
        Self::new()
    }
}

/// A seeded stream per purpose, so one population's count never shifts another's.
pub fn stream(seed: u32, salt: i64) -> Mulberry32 {
    Mulberry32::new(seed ^ ((salt + 1) as u32).wrapping_mul(GOLDEN))
}

/// One of `weights`' indices by weight, never one of no weight while any
/// has some (rounding that leaves the draw past every weight takes the last
/// weighted index).
pub fn weighted(rng: &mut Mulberry32, weights: impl Iterator<Item = f64> + Clone) -> usize {
    let mut r = rng.float(0.0, weights.clone().fold(0.0, |a, w| a + w));
    let mut last = 0;
    for (i, w) in weights.enumerate() {
        r -= w;
        if r < 0.0 {
            return i;
        }
        if w > 0.0 {
            last = i;
        }
    }
    last
}

/// `sideKey(id, side, even)`: one number per (side, id), `even`'s ids even.
pub fn side_key(id: f64, side_is_even: bool) -> f64 {
    id * 2.0 + if side_is_even { 0.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_is_the_published_generator() {
        // Mulberry32 seeded with 1, as its reference implementation gives it.
        let mut m = Mulberry32::new(1);
        assert_eq!(m.sample(), 0.6270739405881613);
        assert_eq!(m.sample(), 0.002735721180215478);
    }

    #[test]
    fn a_key_tells_its_parts_apart() {
        let a = Key::new().text("ab").text("c").seed();
        let b = Key::new().text("a").text("bc").seed();
        assert_ne!(a, b);
        assert_eq!(Key::new().num(-0.0).seed(), Key::new().num(0.0).seed());
        assert_ne!(Key::new().num(1.0).seed(), Key::new().num(2.0).seed());
    }
}
