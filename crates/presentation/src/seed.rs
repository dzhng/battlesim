//! The browser's seeded randomness, defined once for both clients: the
//! `math` package's Mulberry32, `hashString` (FNV-1a over UTF-16 code
//! units), and the text JavaScript writes for a number in a template string,
//! which the seeds' keys embed.

/// A Mulberry32 state (`mulberry32.create`).
#[derive(Clone, Copy, Debug)]
pub struct Mulberry32(u32);

impl Mulberry32 {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// The next number in [0, 1) (`mulberry32.sample`).
    pub fn sample(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// `hashString(key)`: FNV-1a of `key`'s UTF-16 code units.
pub fn hash_string(key: &str) -> u32 {
    key.encode_utf16().fold(2_166_136_261u32, |h, c| {
        (h ^ u32::from(c)).wrapping_mul(16_777_619)
    })
}

/// `String(x)` as JavaScript writes it: the shortest digits that read back
/// as `x`, in fixed notation from 1e-6 up to 1e21, `-0` as `0`. Where two
/// such digit strings are equally near `x`, JavaScript takes the even one
/// (`167.43655395507812`, not Rust's `…13`).
pub fn js_number(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    // The shortest length, then those digits rounded from `x`'s exact
    // value, halves to even: `d.ddde±n`.
    let shortest = format!("{x:e}");
    let digits = shortest
        .split_once('e')
        .expect("exponent form")
        .0
        .bytes()
        .filter(u8::is_ascii_digit)
        .count();
    let text = format!("{x:.*e}", digits - 1);
    let (mantissa, exponent) = text.split_once('e').expect("exponent form");
    let exponent: i32 = exponent.parse().expect("exponent");
    if !(1e-6..1e21).contains(&x.abs()) {
        return format!(
            "{mantissa}e{}{}",
            if exponent < 0 { "-" } else { "+" },
            exponent.abs()
        );
    }
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |m| ("-", m));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let point = exponent + 1;
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat(-point as usize))
    } else if point as usize >= digits.len() {
        format!(
            "{sign}{digits}{}",
            "0".repeat(point as usize - digits.len())
        )
    } else {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{sign}{whole}.{fraction}")
    }
}

/// `Math.hypot` as V8 computes it: each value over the largest, squared and
/// summed with Kahan's compensation, so lengths match the browser's bits.
pub fn js_hypot(values: &[f64]) -> f64 {
    let max = values.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if max == f64::INFINITY {
        return max;
    }
    if values.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0, 0.0);
    for v in values {
        let n = v.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// `Math.round`: the nearest whole number, halves up.
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// `x >>> 0` for a whole `x`: its value modulo 2³².
pub fn to_uint32(x: f64) -> u32 {
    x.trunc().rem_euclid(4_294_967_296.0) as u32
}

/// `sideKey(id, side, even)`: one number per (side, id), `even`'s ids even.
pub fn side_key(id: f64, side_is_even: bool) -> f64 {
    id * 2.0 + if side_is_even { 0.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_the_math_package() {
        // `mulberry32.sample(mulberry32.create(1))`, from node.
        let mut m = Mulberry32::new(1);
        assert_eq!(m.sample(), 0.6270739405881613);
        assert_eq!(m.sample(), 0.002735721180215478);
    }

    #[test]
    fn hash_string_is_fnv1a_over_code_units() {
        assert_eq!(hash_string(""), 2_166_136_261);
        assert_eq!(hash_string("a"), 0xe40c_292c);
    }

    #[test]
    fn numbers_read_as_javascript_writes_them() {
        for (x, text) in [
            (0.0, "0"),
            (-0.0, "0"),
            (5.0, "5"),
            (-2.5, "-2.5"),
            (f64::from(0.1f32), "0.10000000149011612"),
            (f64::from(2562.5f32), "2562.5"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (123456789012.0, "123456789012"),
            (f64::from(167.43655f32), "167.43655395507812"),
            (-0.00125, "-0.00125"),
            (1e20, "100000000000000000000"),
        ] {
            assert_eq!(js_number(x), text, "{x}");
        }
    }
}
