//! JavaScript's arithmetic as V8 does it, where Rust's differs, so the
//! ground's records keep the bits its recorded vectors hold.
//! `Math.hypot`, `min` and `max` are reproduced exactly; the seeds are
//! [`crate::seed`]'s. The trigonometry is `libm`'s, one implementation for
//! every client: V8's `cos` and `sin` (glibc's) and `atan2` (fdlibm's)
//! differ from it in the last bit for a few inputs in a hundred, which moves
//! a plot's cut by under 1e-12 of its value and no record the GPU reads.
pub use libm::{atan2, cos, pow, sin, tan};

use crate::seed::js_hypot;

/// `Math.hypot(x, y)`, which is not `libm::hypot` ([`js_hypot`]).
pub fn hypot(x: f64, y: f64) -> f64 {
    js_hypot(&[x, y])
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
