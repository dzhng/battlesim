//! JavaScript's arithmetic as the page's V8 does it, where Rust's differs:
//! the ground was resolved in TypeScript, and its records keep those bits.
//! `Math.hypot`, `min` and `max` and the `math` package's Mulberry32 are
//! reproduced exactly. The trigonometry is `libm`'s, one implementation for
//! every client: V8's `cos` and `sin` (glibc's) and `atan2` (fdlibm's)
//! differ from it in the last bit for a few inputs in a hundred, which moves
//! a plot's cut by under 1e-12 of its value and no record the GPU reads.
pub use libm::{atan2, cos, pow, sin};

/// `Math.hypot(x, y)`: V8 scales by the larger and sums with Kahan's
/// compensation, which is not `libm::hypot`.
pub fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    if x == f64::INFINITY || y == f64::INFINITY {
        return f64::INFINITY;
    }
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let top = if y > x { y } else { x };
    if top == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0, 0.0);
    for v in [x, y] {
        let n = v / top;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * top
}

/// `Math.min(a, b)`: NaN wins, and −0 is below +0.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`: NaN wins, and +0 is above −0.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

/// The `math` package's Mulberry32 (`math/random`), seeded as `create(seed)`.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(seed)
    }

    /// `sample`: the next number in [0, 1).
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(a | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (t ^ (t >> 14)) as f64 / 4294967296.0
    }

    /// `random.float(rng, lo, hi)`.
    pub fn float(&mut self, lo: f64, hi: f64) -> f64 {
        self.next() * (hi - lo) + lo
    }
}
