//! How smoke draws: a puff rising, slowing as it spreads and drifting
//! downwind as it climbs, and a trail's ribbon along the smoke a round left.
use super::*;

/// A puff fades in over its first quarter second.
const PUFF_FADE_IN: f64 = 4.0;
/// A trail's ribbon's width over its puffs' diameter.
const RIBBON_WIDTH: f64 = 0.7;

/// Where smoke born at `p`, moving at `n` (its own rise and spread), is
/// `age` into a life of `life`: rising, slowing as it spreads, and drifting
/// on its own and more downwind as it climbs.
fn smoke_at(wind: &[f64; 2], p: Vec3, n: Vec3, life: f64, age: f64) -> Vec3 {
    let x = age / life;
    let climb = life * 0.5 * (1.0 - (1.0 - x) * (1.0 - x));
    let drift = age * (0.3 + 0.7 * x);
    [
        p[0] + n[0] * climb + wind[0] * drift,
        p[1] + n[1] * climb + wind[1] * drift,
        p[2] + n[2] * climb,
    ]
}

/// A puff's radius `x` of the way through its life.
fn puff_radius(s: &PuffStyle, x: f64) -> f64 {
    s.size_m[0] + (s.size_m[1] - s.size_m[0]) * x.sqrt()
}

/// A trail's ribbon along the part of its stretch the round has flown: each
/// end where its smoke has drifted since the round passed it, as wide and
/// faded as its middle's age, so stretches meet end to end unbroken.
pub(super) fn draw_trail(
    p: &EffectPresentation,
    wind: &[f64; 2],
    e: &Effect,
    s: &TrailStyle,
    age: f64,
    dt: f64,
    batch: &mut EffectBatch,
) {
    let length = e.length;
    if length <= 1e-6 {
        return;
    }
    let flown = length.min(length * age / dt);
    let puff = &s.puff;
    let rise = [0.0, 0.0, puff.rise_mps];
    for i in 0..e.cum.len().saturating_sub(1) {
        let (lo, hi) = (e.cum[i], e.cum[i + 1].min(flown));
        if hi <= lo {
            continue;
        }
        // Each point's age: since the round passed it.
        let per_m = dt / length;
        let age_lo = age - lo * per_m;
        let age_hi = (age - hi * per_m).max(0.0);
        let a = smoke_at(wind, point_along(e, lo), rise, puff.life_s, age_lo);
        let b = smoke_at(wind, point_along(e, hi), rise, puff.life_s, age_hi);
        let x = ((age_lo + age_hi) / 2.0 / puff.life_s).min(1.0);
        // Thinning before its puffs fade, so an old trail loosens into them.
        let opacity = s.ribbon * ((1.0 - x) * (1.0 - x));
        let width = 2.0 * RIBBON_WIDTH * puff_radius(puff, x);
        batch.ribbon(a, b, width, p.min_px, puff.albedo, opacity);
    }
}

/// A puff: rising, slowing as it spreads, drifting downwind as it climbs;
/// fading in quickly and out slowly.
pub(super) fn draw_puff(
    wind: &[f64; 2],
    e: &Effect,
    s: &PuffStyle,
    age: f64,
    batch: &mut EffectBatch,
) {
    let x = age / e.span;
    let a = smoke_at(wind, e.p, e.n, s.life_s, age);
    let radius = e.scale * puff_radius(s, x);
    let fade = (age * PUFF_FADE_IN).min(1.0) * libm::pow(1.0 - x, 1.2);
    let turn = e.rotation + e.size * x * 1.5;
    let frame = e.frame + x * PUFF_TURN_FRAMES;
    let opacity = s.opacity * e.alpha * fade;
    batch.flipbook(
        a,
        radius,
        turn,
        DUST,
        frame,
        s.albedo,
        opacity,
        0.0,
        radius * 0.8,
        true,
    );
}
