//! A high-air aircraft's shadow on the ground beneath it. From a high-air
//! layer's height the sun throws a real shadow hundreds of metres off, where
//! nobody ties it to its aircraft; so a high-air unit casts none, and instead
//! a soft silhouette of it lies on the ground straight beneath it, turned
//! with its heading, fainter and softer the higher it flies. It is ground,
//! not light: the effect pass lays it on the surface it lies on and never on
//! a unit standing there. Which units take it is their altitude layer's
//! decision (`UnitType::high_air`), never what kind of aircraft they are; a
//! low-air aircraft keeps its sun-cast shadow.
use serde::Deserialize;

use crate::world::Vec3;

/// `presentation.effects.high_air_shadow`: how dark and soft the shadow
/// beneath a high-air aircraft lies at each height over the ground.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct HighAirShadowStyle {
    /// How much light it takes off the ground at no height, 0..1.
    pub opacity: f64,
    /// The height over the ground at which it has faded to half that, m.
    pub fade_half_m: f64,
    /// Its edge's blur at no height, and how much wider each metre of
    /// height makes it, m.
    pub blur_m: f64,
    pub blur_per_m: f64,
    /// How far above or below the ground under its middle the surface it
    /// lies on may be, m: the ground's own bumps take it, a unit standing
    /// there (taller than this) does not.
    pub ground_band_m: f64,
}

impl HighAirShadowStyle {
    pub(super) fn valid(&self) -> bool {
        (0.0..=1.0).contains(&self.opacity)
            && self.fade_half_m > 0.0
            && self.blur_m >= 0.0
            && self.blur_per_m >= 0.0
            && self.ground_band_m > 0.0
    }
}

/// One high-air aircraft's shadow as the effect pass lays it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundShadow {
    /// The ground straight beneath the middle of its plan.
    pub centre: Vec3,
    /// Up from the ground it lies on, fitted over its plan.
    pub normal: Vec3,
    /// Its nose's way, along that ground.
    pub along: Vec3,
    /// Half its plan's length (along) and span (across), m.
    pub half: [f64; 2],
    pub opacity: f64,
    /// Its edge's blur, m.
    pub blur_m: f64,
}

/// The shadow beneath a high-air aircraft at `at` heading `yaw`, whose
/// model's plan is `plan` (its box seen from above in its own frame: least
/// x, least y, greatest x, greatest y), over the ground `height` gives.
pub fn high_air_ground_shadow(
    at: Vec3,
    yaw: f64,
    plan: [f64; 4],
    height: impl Fn(f64, f64) -> f64,
    style: &HighAirShadowStyle,
) -> GroundShadow {
    let (s, c) = libm::sincos(yaw);
    let turn = |x: f64, y: f64| [x * c - y * s, x * s + y * c];
    let [x0, y0, x1, y1] = plan;
    let half = [(x1 - x0) / 2.0, (y1 - y0) / 2.0];
    let mid = turn((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (x, y) = (at[0] + mid[0], at[1] + mid[1]);
    // The ground's tilt over the plan, along and across.
    let slope = |d: [f64; 2], reach: f64| {
        let reach = reach.max(0.5);
        (height(x + d[0] * reach, y + d[1] * reach) - height(x - d[0] * reach, y - d[1] * reach))
            / (2.0 * reach)
    };
    let (ahead, side) = ([c, s], [-s, c]);
    let (rise, lean) = (slope(ahead, half[0]), slope(side, half[1]));
    let normal = unit([
        -rise * ahead[0] - lean * side[0],
        -rise * ahead[1] - lean * side[1],
        1.0,
    ]);
    let along = unit([ahead[0], ahead[1], rise]);
    let ground = height(x, y);
    let over = (at[2] - ground).max(0.0);
    GroundShadow {
        centre: [x, y, ground],
        normal,
        along,
        half,
        opacity: style.opacity * style.fade_half_m / (style.fade_half_m + over),
        blur_m: style.blur_m + style.blur_per_m * over,
    }
}

fn unit(v: Vec3) -> Vec3 {
    let l = crate::world::length(v);
    v.map(|x| x / l)
}
