//! How a smoke screen draws (`specs/smoke-screen`): the clouds the side
//! sees, each a billow of puffs seeded by its id that swirl round its
//! centre inside the circle `contract::smoke` grows in steps, and stay below
//! its top. Its optical depth follows its stepped density, as sight's does:
//! a puff passes `(1 − opacity)^density` of the light behind it, so the
//! layered billow thins as the cloud does. Nothing blows it: the
//! simulation's cloud stands still (D8), so the drawn one never leaves it.
//! Its look is the `smoke_looks` row its capability names, and every screen
//! together keeps to `smoke_screen_budget`, apart from the wrecks' smoke.
use contract::smoke::{state, SmokeCloud};

use super::*;
use crate::effects::tables::SmokeLook;

/// Frames into the dust flipbook a puff may stand at: a full, round puff,
/// neither the burst it starts as nor the wisps it ends as.
const FRAMES: [f64; 2] = [10.0, 40.0];
/// How far a puff wanders in and out round its orbit, as a share of the room.
const BREATHE: f64 = 0.12;
/// A puff's lowest centre, as a share of its radius above the ground.
const SIT: f64 = 0.4;
/// How much smaller a puff at the rim is than one at the middle, so the
/// billow's edge tapers rather than ending in a ring of equal lobes.
const TAPER: f64 = 0.25;

/// The smoke clouds the side sees, as of the last publication taken.
#[derive(Default)]
pub(super) struct Screens {
    clouds: Vec<SmokeCloud>,
}

impl Screens {
    /// The side's clouds now: one not published any more stops drawing.
    pub(super) fn note(&mut self, clouds: &[SmokeCloud]) {
        self.clouds.clear();
        self.clouds.extend_from_slice(clouds);
    }

    pub(super) fn clear(&mut self) {
        self.clouds.clear();
    }

    /// Every cloud's puffs at presentation time `clock`, ticks `dt` apart;
    /// past the budget each cloud draws the same share of its puffs.
    pub(super) fn draw(
        &self,
        p: &EffectPresentation,
        clock: f64,
        dt: f64,
        batch: &mut EffectBatch,
    ) {
        let looks: Vec<_> = (self.clouds.iter())
            .map(|c| p.smoke_looks.get(&c.spec.look))
            .collect();
        let demand: f64 = looks.iter().flatten().map(|l| f64::from(l.puffs)).sum();
        let keep = (p.smoke_screen_budget / demand.max(1.0)).min(1.0);
        let hz = libm::round(1.0 / dt) as u32;
        let tick = libm::floor(clock / dt + 1e-6) as u64;
        for (c, look) in self.clouds.iter().zip(looks) {
            if let Some(look) = look {
                draw_cloud(c, look, clock - c.popped as f64 * dt, tick, hz, keep, batch);
            }
        }
    }
}

/// Cloud `c`, `age` seconds after its pop, at tick `tick`: the first `keep`
/// share of its puffs. The drawn cloud grows smoothly inside the stepped
/// circle (never past it), and each puff with it.
fn draw_cloud(
    c: &SmokeCloud,
    look: &SmokeLook,
    age: f64,
    tick: u64,
    hz: u32,
    keep: f64,
    batch: &mut EffectBatch,
) {
    let Some(now) = state(c, tick, hz) else {
        return;
    };
    let s = &c.spec;
    let grown = (now.radius_m / s.radius_m).min(age.max(0.0) / s.bloom_s);
    let density = now.depth_per_m / s.depth_per_m;
    let opacity = 1.0 - libm::pow(1.0 - look.opacity, density);
    let reach = s.radius_m * grown;
    let n = libm::floor(f64::from(look.puffs) * keep) as u32;
    for i in 0..n {
        let mut rng = Key::new()
            .text("smoke-screen")
            .num(f64::from(c.id))
            .num(f64::from(i))
            .rng();
        let size = look.size_m[0] + (look.size_m[1] - look.size_m[0]) * rng.sample();
        let orbit = libm::sqrt(rng.sample());
        let r = size.min(s.radius_m).min(s.height_m) * grown * (1.0 - TAPER * orbit);
        let phase = rng.sample() * 2.0 * PI;
        let rate = look.swirl_rad_s * (0.6 + 0.8 * rng.sample());
        let breathe = BREATHE * libm::sin(rng.sample() * 2.0 * PI + age * rate * 1.7);
        let out = (orbit + breathe).clamp(0.0, 1.0) * (reach - r);
        let (sin, cos) = libm::sincos(phase + age * rate);
        let share = look.height[0] + (look.height[1] - look.height[0]) * rng.sample();
        let top = s.height_m - r;
        let up = (share * s.height_m).max(SIT * r).min(top);
        let turn = rng.sample() * 2.0 * PI + age * rate * 0.5;
        let frame = FRAMES[0] + (FRAMES[1] - FRAMES[0]) * rng.sample();
        batch.screen_puff(
            [
                c.centre[0] + out * cos,
                c.centre[1] + out * sin,
                c.centre[2] + up,
            ],
            r,
            turn,
            frame,
            look.albedo,
            opacity,
            look.soft_m,
        );
    }
}
