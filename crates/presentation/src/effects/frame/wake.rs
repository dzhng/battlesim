//! Wakes: where a unit the side sees wading pushes on the water, decided
//! per tick from what is published, never from how often frames are drawn.
use super::*;
use crate::effects::publication::Wading;
use crate::effects::tables::WakeStyle;
use crate::records::WaveSplat;
use crate::view::Camera;
use crate::wave::{WaveStep, WAVE};

/// How long a wake impulse is kept after it is made, presentation seconds:
/// past it a renderer that has not taken it never will.
pub(super) const WAKE_KEEP_S: f64 = 1.0;

/// The wake wader `u` makes over the tick from `t0`, `dt` long, having come
/// `d` (x, y) since the last: at `vee_mps` or faster a V-wake push from
/// each of its waders in water `vee_depth_m` deep or more, for every
/// `spacing_m` it covers, when and where it passed (`carry`: how far it has
/// come since its last); slower, and from waders in shallower water (a
/// puddle splashes, it drags no V), rings round each every 1/`ring_hz` s of
/// the clock, each seeded by the unit, the wader and its count. A unit
/// first seen, or seen again after a gap, starts clean.
#[allow(clippy::too_many_arguments)]
pub(super) fn wake(
    out: &mut Vec<WakeImpulse>,
    style: &WakeStyle,
    u: &Shooter,
    d: [f64; 2],
    carry: &mut f64,
    t0: f64,
    dt: f64,
    gap: bool,
) {
    let radius = u.half.map_or(style.soldier_push_radius_m, |h| h[1]);
    let dist = libm::hypot(d[0], d[1]);
    let moving = !gap && dist / dt >= style.vee_mps;
    let drags = |w: &Wading| moving && w.depth_m >= style.vee_depth_m;
    if moving {
        let mut ahead = style.spacing_m - *carry;
        while ahead <= dist {
            let back = 1.0 - ahead / dist;
            out.extend(u.wading.iter().filter(|w| drags(w)).map(|w| WakeImpulse {
                xy: [w.at[0] - d[0] * back, w.at[1] - d[1] * back],
                radius,
                strength: style.vee_strength,
                kind: WakeKind::Vee {
                    heading: [d[0] / dist, d[1] / dist],
                },
                at: t0 + dt * ahead / dist,
            }));
            ahead += style.spacing_m;
        }
        *carry = dist - (ahead - style.spacing_m);
    } else {
        *carry = 0.0;
    }
    let mut k = (t0 * style.ring_hz).ceil();
    while k / style.ring_hz < t0 + dt {
        for (i, w) in u.wading.iter().enumerate().filter(|(_, w)| !drags(w)) {
            let mut rng = Key::new()
                .text("wake")
                .num(u.key)
                .num(i as f64)
                .num(k)
                .rng();
            out.push(WakeImpulse {
                xy: [w.at[0], w.at[1]],
                radius,
                strength: style.ring_strength * (0.8 + 0.4 * rng.sample()),
                kind: WakeKind::Rings,
                at: k / style.ring_hz,
            });
        }
        k += 1.0;
    }
}

/// A frame of the wave field: its steps, and the wake impulses each splats.
#[derive(Clone, Debug, Default)]
pub struct WaveFrame {
    pub step: WaveStep,
    pub splats: Vec<WaveSplat>,
}

impl EffectFrame {
    /// Start the wave field over, in a new epoch (a side switch), with none
    /// of the wakes the old side saw made still waiting to splat.
    pub fn restart_waves(&mut self) {
        self.waves.restart();
        self.wakes.clear();
    }

    /// The wave field's frame at presentation time `clock` for `camera`
    /// (`WaveClock::advance`), with every wake impulse made in the steps it
    /// runs, each at the step that covers it: none is splatted twice, and
    /// any split into frames splats the same.
    pub fn waves(&mut self, clock: f64, camera: &Camera) -> WaveFrame {
        let step = self.waves.advance(clock, camera);
        let to = self.waves.done().unwrap_or_default();
        let first = to - i64::from(step.steps);
        let [ox, oy] = step.origin_texel.map(f64::from);
        let splats = self
            .wakes
            .iter()
            .filter_map(|w| {
                let k = (w.at * WAVE.hz).floor() as i64;
                (first..to).contains(&k).then(|| {
                    let (kind, heading) = match w.kind {
                        WakeKind::Rings => (0.0, [0.0; 2]),
                        WakeKind::Vee { heading } => (1.0, heading.map(|v| v as f32)),
                    };
                    WaveSplat {
                        splat: [
                            (w.xy[0] / WAVE.texel_m - ox) as f32,
                            (w.xy[1] / WAVE.texel_m - oy) as f32,
                            (w.radius / WAVE.texel_m) as f32,
                            w.strength as f32,
                        ],
                        step: (k - first) as f32,
                        kind,
                        heading,
                    }
                })
            })
            .collect();
        WaveFrame { step, splats }
    }
}
