//! Wakes: where a unit the side sees wading pushes on the water, decided
//! per tick from what is published, never from how often frames are drawn.
use super::*;
use crate::effects::tables::WakeStyle;

/// How long a wake impulse is kept after it is made, presentation seconds:
/// past it a renderer that has not taken it never will.
pub(super) const WAKE_KEEP_S: f64 = 1.0;

/// The wake wader `u` makes over the tick from `t0`, `dt` long, having come
/// `d` (x, y) since the last: at `vee_mps` or faster a V-wake push from
/// each of its waders for every `spacing_m` it covers, when and where it
/// passed (`carry`: how far it has come since its last); slower, rings
/// round each every 1/`ring_hz` s of the clock, each seeded by the unit,
/// the wader and its count. A unit first seen, or seen again after a gap,
/// starts clean.
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
    let radius = u.half.map_or(style.soldier_radius_m, |h| h[1]);
    let dist = libm::hypot(d[0], d[1]);
    if !gap && dist / dt >= style.vee_mps {
        let mut ahead = style.spacing_m - *carry;
        while ahead <= dist {
            let back = 1.0 - ahead / dist;
            out.extend(u.wading.iter().map(|w| WakeImpulse {
                xy: [w.at[0] - d[0] * back, w.at[1] - d[1] * back],
                radius,
                strength: style.vee_strength,
                kind: WakeKind::Vee,
                at: t0 + dt * ahead / dist,
            }));
            ahead += style.spacing_m;
        }
        *carry = dist - (ahead - style.spacing_m);
        return;
    }
    *carry = 0.0;
    let mut k = (t0 * style.ring_hz).ceil();
    while k / style.ring_hz < t0 + dt {
        for (i, w) in u.wading.iter().enumerate() {
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
