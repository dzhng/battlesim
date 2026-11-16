//! A bomb leaving a jet: one bomb, or a cluster canister's bomblets.
use super::assessment::Ready;
use super::Weapon;
use crate::flight::{Aim, Launch, Shooter};
use crate::math::{v2, V2};
use contract::random::Rng;
use contract::weapons::Submunitions;

/// Chance spots tried for each bomblet: the one farthest from those already
/// placed is kept.
const CANDIDATES: usize = 3;

/// The rounds one bomb released now becomes. It leaves the jet with the
/// jet's velocity and a small push off the rack that takes it from where the
/// drop alone would carry it to its launch spread round the target, so a
/// release a step late, or from a rack ahead of the jet's middle, still comes
/// down on what it was aimed at. A canister falls the same way and opens:
/// its bomblets come down spread evenly across its disc in no pattern the
/// eye picks out, and land one after another over its ripple, each falling
/// at its own rate, as a cluster crackles across the ground.
pub(super) fn bomb_release(
    weapon: &Weapon,
    aim: &Aim,
    ready: Ready,
    scatter: f64,
    shooter: Option<Shooter>,
    rng: &mut Rng,
) -> Vec<Launch> {
    let falls = crate::flight::scatter_aim(aim.origin, aim.target, scatter, rng).xy();
    let fall_s = ready.time_of_flight_s.max(f64::MIN_POSITIVE);
    let landing: Vec<(V2, f64)> = match weapon.def.submunitions {
        None => vec![(falls, fall_s)],
        Some(canister) => spots(canister, rng)
            .into_iter()
            .map(|spot| {
                let ripple = (rng.unit() - 0.5) * canister.ripple_s;
                (falls + spot, (fall_s + ripple).max(fall_s * 0.5))
            })
            .collect(),
    };
    // What a round must drop in a fall of `t`: the height to the ground it
    // was aimed at, less what the jet's climb carries it up. The row's own
    // gravity drops it that far in the assessment's fall; a longer or
    // shorter fall onto the same ground takes gravity in proportion.
    let drop = |t: f64| aim.origin.z - ready.intercept.z + aim.carrier.z * t;
    landing
        .into_iter()
        .map(|(at, t)| {
            let carried = aim.origin.xy() + aim.carrier.xy() * t;
            Launch {
                origin: aim.origin,
                carrier: aim.carrier,
                velocity: ((at - carried) * (1.0 / t)).with_z(0.0),
                gravity_scale: weapon.profile.gravity_scale * drop(t) / drop(fall_s)
                    * (fall_s / t).powi(2),
                lifetime_s: weapon.profile.lifetime_s,
                suppression_radius_m: weapon.profile.suppression_radius_m,
                shooter,
                guidance: None,
                motor: None,
                fall: None,
            }
        })
        .collect()
}

/// Where each of a canister's bomblets comes down, round where it opens, by
/// best-candidate sampling: each spot is the farthest of a few chance spots
/// in the disc from those placed before it, so together they cover the disc
/// evenly, with neither clumps nor a lattice nor spiral arms.
fn spots(canister: Submunitions, rng: &mut Rng) -> Vec<V2> {
    let mut placed: Vec<V2> = Vec::with_capacity(canister.count as usize);
    for _ in 0..canister.count {
        let mut best = (v2(0.0, 0.0), f64::NEG_INFINITY);
        for _ in 0..CANDIDATES {
            let r = canister.spread_m * libm::sqrt(rng.unit());
            let a = rng.unit() * std::f64::consts::TAU;
            let p = v2(libm::cos(a), libm::sin(a)) * r;
            let gap = (placed.iter())
                .map(|q| (p - *q).length())
                .fold(f64::INFINITY, f64::min);
            if gap > best.1 {
                best = (p, gap);
            }
        }
        placed.push(best.0);
    }
    placed
}
