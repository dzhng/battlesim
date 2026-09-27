//! Launch solving (P02, P03). A round launched from `O` at speed `s` meets a
//! target at `T + W·t` (its observed position and constant observed velocity)
//! after time t when the launch velocity is `v = (D + W·t + A·t²) / t`, with
//! `D = T − O` and `A = −g/2`. Requiring |v| = s gives the quartic
//! `|D + W·t + A·t²|² − s²·t² = 0`, whose real roots on (0, lifetime] are
//! isolated exactly: critical points split the interval into monotone pieces,
//! recursively, and each sign change is bisected. The earliest root is the low
//! arc and the latest the high arc. The chosen arc is then flown through the
//! same chords as real flight against the static world; a solution whose path
//! meets geometry before arriving is blocked, never swapped for an arc the
//! weapon may not use.
use contract::ballistics::Trajectory;

use super::{chords, FlightConfig, Guidance, Launch, LaunchProfile, Shooter, TIME_EPSILON_S};
use crate::math::{v3, V3};
use crate::rng::Rng;
use crate::world::{Collider, PropId, WorldGeometry};

/// A static hit this close to the intended intercept counts as arrival: an aim
/// point on the ground, or a target standing on a surface.
const ARRIVAL_TOLERANCE_M: f64 = 0.5;
/// Scatter is truncated at this many standard deviations per axis.
const SCATTER_LIMIT_SIGMA: f64 = 3.0;

/// Where to send a round: from the muzzle to a target point moving at its
/// observed constant velocity (zero for ground and stationary targets).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    pub origin: V3,
    pub target: V3,
    pub target_velocity: V3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcKind {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiringSolution {
    pub velocity: V3,
    pub time_of_flight_s: f64,
    /// Where the target is predicted to be on arrival.
    pub intercept: V3,
    pub arc: ArcKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoSolution {
    /// No launch at this speed meets the target within the round's lifetime.
    OutOfReach,
    /// Every arc the weapon may use meets static geometry first: the preferred
    /// arc, its first obstruction and what that is.
    Blocked {
        arc: FiringSolution,
        point: V3,
        by: Collider,
    },
}

/// Solve the weapon's preferred usable arc: low only for direct fire; the
/// high arc first for indirect fire, falling back to the low arc.
pub fn solve_launch(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
) -> Result<FiringSolution, NoSolution> {
    solve_launch_past(world, config, profile, aim, None)
}

/// [`solve_launch`] flying through `past`, the body a soldier fires from
/// behind (27c).
pub fn solve_launch_past(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
    past: Option<PropId>,
) -> Result<FiringSolution, NoSolution> {
    let arcs = intercepts(config, profile, aim);
    let preferred: Vec<FiringSolution> = match profile.trajectory {
        Trajectory::Direct => arcs.into_iter().filter(|s| s.arc == ArcKind::Low).collect(),
        Trajectory::Indirect => arcs.into_iter().rev().collect(),
    };
    let mut blocked = None;
    for solution in preferred {
        match first_obstruction(world, config, profile, aim.origin, &solution, past) {
            None => return Ok(solution),
            Some((point, by)) => {
                blocked.get_or_insert(NoSolution::Blocked {
                    arc: solution,
                    point,
                    by,
                });
            }
        }
    }
    Err(blocked.unwrap_or(NoSolution::OutOfReach))
}

/// The low and (if distinct) high intercepts within the round's lifetime.
fn intercepts(config: &FlightConfig, profile: &LaunchProfile, aim: &Aim) -> Vec<FiringSolution> {
    let d = aim.target - aim.origin;
    let w = aim.target_velocity;
    let a = profile.gravity(config) * -0.5;
    let s = profile.speed_mps;
    let quartic = [
        d.dot(d),
        2.0 * w.dot(d),
        w.dot(w) + 2.0 * a.dot(d) - s * s,
        2.0 * a.dot(w),
        a.dot(a),
    ];
    let roots: Vec<f64> = real_roots(&quartic, 0.0, profile.lifetime_s)
        .into_iter()
        .filter(|&t| t > 0.0)
        .collect();
    let solution = |t: f64, arc| FiringSolution {
        velocity: (d + w * t + a * (t * t)) * (1.0 / t),
        time_of_flight_s: t,
        intercept: aim.target + w * t,
        arc,
    };
    match (roots.first(), roots.last()) {
        (Some(&low), Some(&high)) if high > low => {
            vec![solution(low, ArcKind::Low), solution(high, ArcKind::High)]
        }
        (Some(&low), _) => vec![solution(low, ArcKind::Low)],
        _ => Vec::new(),
    }
}

/// Real roots of the polynomial `c[0] + c[1]·t + …` in [lo, hi], ascending.
fn real_roots(c: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    let Some(degree) = c.iter().rposition(|&x| x != 0.0) else {
        return Vec::new();
    };
    if degree == 0 {
        return Vec::new();
    }
    let c = &c[..=degree];
    let f = |t: f64| c.iter().rev().fold(0.0, |acc, &k| acc * t + k);
    let derivative: Vec<f64> = (1..=degree).map(|i| c[i] * i as f64).collect();
    let mut knots = vec![lo];
    knots.extend(real_roots(&derivative, lo, hi));
    knots.push(hi);
    let mut roots: Vec<f64> = Vec::new();
    for w in knots.windows(2) {
        let (mut a, mut b) = (w[0], w[1]);
        let (fa, fb) = (f(a), f(b));
        if fa == 0.0 {
            if roots.last() != Some(&a) {
                roots.push(a);
            }
            continue;
        }
        if fb == 0.0 || fa.signum() == fb.signum() {
            continue;
        }
        // Monotone on [a, b] with a sign change: bisect to adjacent floats.
        loop {
            let m = 0.5 * (a + b);
            if m <= a || m >= b {
                break;
            }
            if f(m).signum() == fa.signum() {
                a = m;
            } else {
                b = m;
            }
        }
        roots.push(0.5 * (a + b));
    }
    if f(hi) == 0.0 && roots.last() != Some(&hi) {
        roots.push(hi);
    }
    roots
}

/// The chord endpoints a round launched now would fly over `duration_s`,
/// tick by tick exactly as [`super::advance_projectiles`] flies them.
pub fn predicted_path(
    config: &FlightConfig,
    gravity: V3,
    origin: V3,
    velocity: V3,
    duration_s: f64,
) -> Vec<V3> {
    let mut points = vec![origin];
    let (mut p, mut v, mut elapsed) = (origin, velocity, 0.0);
    let mut tick = Vec::new();
    while duration_s - elapsed > TIME_EPSILON_S {
        let span = config.tick_s().min(duration_s - elapsed);
        chords(
            p,
            v,
            gravity,
            span,
            config.subsegments_per_tick(),
            &mut tick,
        );
        points.extend(tick.iter().skip(1).map(|(q, _)| *q));
        (p, v) = *tick.last().expect("chords include the start");
        elapsed += span;
    }
    points
}

/// First static obstruction on the flown path short of arrival, if any, and
/// what it is.
fn first_obstruction(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    origin: V3,
    solution: &FiringSolution,
    past: Option<PropId>,
) -> Option<(V3, Collider)> {
    let path = predicted_path(
        config,
        profile.gravity(config),
        origin,
        solution.velocity,
        solution.time_of_flight_s,
    );
    for w in path.windows(2) {
        let chord = w[1] - w[0];
        let len = chord.length();
        if len == 0.0 {
            continue;
        }
        if let Some(hit) = world.raycast_past(w[0], chord * (1.0 / len), len, past) {
            return ((hit.point - solution.intercept).length() > ARRIVAL_TOLERANCE_M)
                .then_some((hit.point, hit.collider));
        }
    }
    None
}

/// The aim point displaced by launch spread: independent truncated Gaussian
/// angles (±3σ, σ = `scatter_mrad`) about the two axes perpendicular to the
/// line of fire, converted with a tangent to a displacement in the aim plane.
pub fn scatter_aim(origin: V3, aim: V3, scatter_mrad: f64, rng: &mut Rng) -> V3 {
    let sigma = scatter_mrad * 1e-3;
    let across = rng.truncated_normal(SCATTER_LIMIT_SIGMA) * sigma;
    let vertical = rng.truncated_normal(SCATTER_LIMIT_SIGMA) * sigma;
    let line = aim - origin;
    let range = line.length();
    if range == 0.0 {
        return aim;
    }
    let forward = line * (1.0 / range);
    let side = forward.cross(v3(0.0, 0.0, 1.0));
    let right = if side.length() > 1e-9 {
        side.normalized()
    } else {
        v3(1.0, 0.0, 0.0)
    };
    let up = right.cross(forward);
    aim + right * (range * across.tan()) + up * (range * vertical.tan())
}

/// The one way a round is launched: solve the intended aim (a weapon that
/// cannot reach or is blocked does not fire), then [`launch_along`] it.
pub fn prepare_launch(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
    scatter_mrad: f64,
    rng: &mut Rng,
    shooter: Option<Shooter>,
) -> Result<(Launch, FiringSolution), NoSolution> {
    let intended = solve_launch_past(world, config, profile, aim, shooter.and_then(|s| s.cover))?;
    launch_along(config, profile, aim, &intended, scatter_mrad, rng, shooter)
}

/// Launch a round on an arc already solved for `aim` (clear, or blocked by a
/// body the caller fires into): sample spread once, solve the scattered aim
/// on the same arc, and build the launch. `scatter_mrad` is the effective
/// one-axis σ after the caller's movement and cover multipliers.
pub fn launch_along(
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
    intended: &FiringSolution,
    scatter_mrad: f64,
    rng: &mut Rng,
    shooter: Option<Shooter>,
) -> Result<(Launch, FiringSolution), NoSolution> {
    let scattered = Aim {
        target: scatter_aim(aim.origin, aim.target, scatter_mrad, rng),
        ..*aim
    };
    let fired = intercepts(config, profile, &scattered)
        .into_iter()
        .find(|s| s.arc == intended.arc)
        .ok_or(NoSolution::OutOfReach)?;
    Ok((
        Launch {
            origin: aim.origin,
            velocity: fired.velocity,
            lifetime_s: profile.lifetime_s,
            suppression_radius_m: profile.suppression_radius_m,
            shooter,
            // A guided round starts steering where its launch was solved to meet the
            // target, so its first tick flies the checked line.
            guidance: profile.turn_rad_s.map(|turn_rad_s| Guidance {
                point: intended.intercept,
                turn_rad_s,
                supported: true,
            }),
        },
        fired,
    ))
}
