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

use super::{
    chords, FlightConfig, Guidance, Launch, LaunchProfile, MissFall, Motor, Shooter, TIME_EPSILON_S,
};
use crate::math::{v3, V3};
use crate::world::{Collider, PropId, WorldGeometry};
use contract::random::Rng;
use std::cell::RefCell;
use std::collections::HashMap;

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
/// high arc first for indirect fire, falling back to the low arc. The arc
/// flies through `past`, the body a soldier fires from behind.
pub fn solve_launch_past(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
    past: Option<PropId>,
) -> Result<FiringSolution, NoSolution> {
    let (o, t, w) = (aim.origin, aim.target, aim.target_velocity);
    let motor = profile
        .motor
        .map_or([f64::NAN; 2], |m| [m.accel_mps2, m.top_speed_mps]);
    let flight = [
        o.x,
        o.y,
        o.z,
        t.x,
        t.y,
        t.z,
        w.x,
        w.y,
        w.z,
        profile.speed_mps,
        profile.gravity_scale,
        profile.lifetime_s,
        profile.turn_rad_s.unwrap_or(f64::NAN),
        motor[0],
        motor[1],
        profile.motor.map_or(0.0, |_| 1.0),
    ]
    .map(f64::to_bits);
    let world_at = (
        world as *const WorldGeometry as usize,
        world.obstacle_revision(),
    );
    let indirect = profile.trajectory == Trajectory::Indirect;
    let key = (flight, past, indirect, world_at.0, world_at.1);
    config
        .solved
        .launches
        .get_or(key, || launch_uncached(world, config, profile, aim, past))
}

/// [`solve_launch_past`], worked out.
fn launch_uncached(
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

/// What a ballistic intercept is solved from, to the bit: the aim's origin,
/// target and target velocity, the round's speed and lifetime, and the
/// gravity it falls under.
type InterceptKey = [u64; 14];

/// Answers already worked out, by their exact inputs. Weapons ask the same
/// aim again within a tick and the next (a mount weighing its targets, then
/// firing at one), and each answer kept is a pure function of its key, so a
/// kept one is the very answer. Two generations of at most [`KEPT`] each
/// bound it: the older is dropped when the newer fills.
#[derive(Clone, Debug)]
struct Kept<K, V> {
    generations: RefCell<[HashMap<K, V, BuildWords>; 2]>,
}

impl<K, V> Default for Kept<K, V> {
    fn default() -> Self {
        Self {
            generations: RefCell::new([HashMap::default(), HashMap::default()]),
        }
    }
}

/// A word-at-a-time hash for keys of float bits: no adversary picks them,
/// and the standard hasher costs more than the lookup saves.
#[derive(Clone, Copy, Debug, Default)]
struct BuildWords;

impl std::hash::BuildHasher for BuildWords {
    type Hasher = Words;
    fn build_hasher(&self) -> Words {
        Words(0)
    }
}

struct Words(u64);

impl std::hash::Hasher for Words {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
    }
    fn write_u64(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_usize(&mut self, word: usize) {
        self.write_u64(word as u64);
    }
    fn write_u32(&mut self, word: u32) {
        self.write_u64(u64::from(word));
    }
    fn write_u8(&mut self, word: u8) {
        self.write_u64(u64::from(word));
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// About a tick and a half of a 200-unit battle's aims.
const KEPT: usize = 4096;

impl<K: std::hash::Hash + Eq, V: Clone> Kept<K, V> {
    fn get_or(&self, key: K, answer: impl FnOnce() -> V) -> V {
        if let Some(found) = self.generations.borrow()[0].get(&key) {
            return found.clone();
        }
        let older = self.generations.borrow()[1].get(&key).cloned();
        let found = older.unwrap_or_else(answer);
        let mut generations = self.generations.borrow_mut();
        if generations[0].len() >= KEPT {
            generations[1] = std::mem::take(&mut generations[0]);
        }
        generations[0].insert(key, found.clone());
        found
    }
}

/// What a launch is solved from, to the bit: the aim (origin, target and
/// target velocity), the round's flight (speed, gravity scale, lifetime,
/// arc, turn limit, motor), the body it flies past, and the world it is
/// flown against: which one, at which obstacle revision.
type LaunchKey = ([u64; 16], Option<PropId>, bool, usize, u64);

/// The ballistic intercepts and the launches already solved: kept answers
/// ([`Kept`]). An intercept depends on nothing but its inputs; a launch also
/// on the world's static bodies, which change only with its obstacle
/// revision. One flight configuration serves one battle's world.
#[derive(Clone, Debug, Default)]
pub struct Solved {
    intercepts: Kept<InterceptKey, Vec<FiringSolution>>,
    launches: Kept<LaunchKey, Result<FiringSolution, NoSolution>>,
}

/// The low and (if distinct) high intercepts within the round's lifetime.
fn intercepts(config: &FlightConfig, profile: &LaunchProfile, aim: &Aim) -> Vec<FiringSolution> {
    if let Some(motor) = profile.motor {
        return motor_intercept(profile.speed_mps, motor, profile.lifetime_s, aim)
            .into_iter()
            .collect();
    }
    let g = profile.gravity(config);
    let (o, t, w) = (aim.origin, aim.target, aim.target_velocity);
    let key = [
        o.x,
        o.y,
        o.z,
        t.x,
        t.y,
        t.z,
        w.x,
        w.y,
        w.z,
        profile.speed_mps,
        profile.lifetime_s,
        g.x,
        g.y,
        g.z,
    ]
    .map(f64::to_bits);
    config
        .solved
        .intercepts
        .get_or(key, || ballistic_intercepts(g, profile, aim))
}

/// [`intercepts`] for a round with no motor, falling under `gravity`.
fn ballistic_intercepts(gravity: V3, profile: &LaunchProfile, aim: &Aim) -> Vec<FiringSolution> {
    let d = aim.target - aim.origin;
    let w = aim.target_velocity;
    let a = gravity * -0.5;
    let s = profile.speed_mps;
    let quartic = [
        d.dot(d),
        2.0 * w.dot(d),
        w.dot(w) + 2.0 * a.dot(d) - s * s,
        2.0 * a.dot(w),
        a.dot(a),
    ];
    let roots = real_roots(&quartic, 0.0, profile.lifetime_s);
    let mut roots = roots.values().iter().filter(|&&t| t > 0.0);
    let (first, last) = (roots.next(), roots.next_back());
    let solution = |t: f64, arc| FiringSolution {
        velocity: (d + w * t + a * (t * t)) * (1.0 / t),
        time_of_flight_s: t,
        intercept: aim.target + w * t,
        arc,
    };
    match (first, last.or(first)) {
        (Some(&low), Some(&high)) if high > low => {
            vec![solution(low, ArcKind::Low), solution(high, ArcKind::High)]
        }
        (Some(&low), _) => vec![solution(low, ArcKind::Low)],
        _ => Vec::new(),
    }
}

/// A motor round flies a straight line without gravity, its distance flown
/// by `t` being [`Motor::distance`]: quadratic in t through the burn, then
/// linear. It meets the target when that distance equals `|D + W·t|`; each
/// piece squared is a polynomial of degree four or less, and the earliest
/// root on either is the intercept (the one arc it flies).
fn motor_intercept(
    launch_mps: f64,
    motor: Motor,
    lifetime_s: f64,
    aim: &Aim,
) -> Option<FiringSolution> {
    let d = aim.target - aim.origin;
    let w = aim.target_velocity;
    let (v0, a, top) = (launch_mps, motor.accel_mps2, motor.top_speed_mps);
    let burn = ((top - v0) / a).clamp(0.0, lifetime_s);
    // |D + W·t|², subtracted from each piece's distance squared.
    let target = [d.dot(d), 2.0 * w.dot(d), w.dot(w)];
    let less_target = |mut c: [f64; 5]| {
        for (k, t) in target.iter().enumerate() {
            c[k] -= t;
        }
        c
    };
    // Burning: (v0·t + ½a·t²)².
    let burning = less_target([0.0, 0.0, v0 * v0, v0 * a, 0.25 * a * a]);
    // Holding top speed: (c0 + top·t)², c0 = distance(burn) − top·burn.
    let c0 = motor.distance(v0, burn) - top * burn;
    let holding = less_target([c0 * c0, 2.0 * c0 * top, top * top, 0.0, 0.0]);
    let t = real_roots(&burning, 0.0, burn)
        .values()
        .iter()
        .chain(real_roots(&holding, burn, lifetime_s).values())
        .copied()
        .find(|&t| t > 0.0)?;
    let to = d + w * t;
    Some(FiringSolution {
        velocity: to * (v0 / to.length()),
        time_of_flight_s: t,
        intercept: aim.target + w * t,
        arc: ArcKind::Low,
    })
}

/// A few values on the stack: the roots and knots of a polynomial of degree
/// four or less, which the launch solve finds for every firing solution.
#[derive(Clone, Copy)]
struct Few {
    v: [f64; 8],
    n: usize,
}

impl Few {
    fn new() -> Few {
        Few { v: [0.0; 8], n: 0 }
    }

    fn push(&mut self, x: f64) {
        self.v[self.n] = x;
        self.n += 1;
    }

    fn values(&self) -> &[f64] {
        &self.v[..self.n]
    }

    fn last(&self) -> Option<&f64> {
        self.values().last()
    }
}

/// Real roots of the polynomial `c[0] + c[1]·t + …` (degree four or less)
/// in [lo, hi], ascending.
fn real_roots(c: &[f64], lo: f64, hi: f64) -> Few {
    let Some(degree) = c.iter().rposition(|&x| x != 0.0) else {
        return Few::new();
    };
    if degree == 0 {
        return Few::new();
    }
    let c = &c[..=degree];
    let f = |t: f64| c.iter().rev().fold(0.0, |acc, &k| acc * t + k);
    let mut derivative = [0.0; 4];
    for i in 1..=degree {
        derivative[i - 1] = c[i] * i as f64;
    }
    let mut knots = Few::new();
    knots.push(lo);
    for &k in real_roots(&derivative[..degree], lo, hi).values() {
        knots.push(k);
    }
    knots.push(hi);
    let mut roots = Few::new();
    for w in knots.values().windows(2) {
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

/// The chord endpoints a round of `profile` launched now would fly over
/// `duration_s`, tick by tick exactly as [`super::advance_projectiles`]
/// flies them (unsteered: a guided round's first line).
pub fn predicted_path(
    config: &FlightConfig,
    profile: &LaunchProfile,
    origin: V3,
    velocity: V3,
    duration_s: f64,
) -> Vec<V3> {
    let mut points = vec![origin];
    let (mut p, mut v, mut elapsed) = (origin, velocity, 0.0);
    let mut tick = Vec::new();
    let gravity = profile.gravity(config);
    while duration_s - elapsed > TIME_EPSILON_S {
        let span = config.tick_s().min(duration_s - elapsed);
        let thrust = profile
            .motor
            .map_or(v3(0.0, 0.0, 0.0), |m| m.thrust(v, span));
        chords(
            p,
            v,
            gravity + thrust,
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
        profile,
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
    aim + right * (range * libm::tan(across)) + up * (range * libm::tan(vertical))
}

/// Launch a round on an arc already solved for `aim` (clear, or blocked by a
/// body the caller fires into): sample spread once, solve the scattered aim
/// on the same arc, and build the launch. `scatter_mrad` is the effective
/// one-axis σ after the caller's movement and cover multipliers.
#[allow(clippy::too_many_arguments)]
pub fn launch_along(
    world: &WorldGeometry,
    config: &FlightConfig,
    profile: &LaunchProfile,
    aim: &Aim,
    intended: &FiringSolution,
    scatter_mrad: f64,
    rng: &mut Rng,
    shooter: Option<Shooter>,
) -> Result<(Launch, FiringSolution), NoSolution> {
    let mut target = scatter_aim(aim.origin, aim.target, scatter_mrad, rng);
    let bounded = config.miss_fall_max_s > 0.0
        && profile.turn_rad_s.is_none()
        && profile.trajectory == Trajectory::Direct;
    if bounded {
        target.z = target.z.min(aim.target.z);
    }
    let scattered = Aim { target, ..*aim };
    let fired = intercepts(config, profile, &scattered)
        .into_iter()
        .find(|s| s.arc == intended.arc)
        .ok_or(NoSolution::OutOfReach)?;
    let fall = if bounded {
        let velocity = fired.velocity + profile.gravity(config) * fired.time_of_flight_s;
        let duration = config.miss_fall_max_s;
        let height = (fired.intercept.z - world.lowest_ground_height()).max(0.0);
        // Bound the descent even over lower terrain, without changing horizontal flight.
        let needed = 2.0 * (height + velocity.z * duration) / (duration * duration);
        let gravity = needed
            .max(config.gravity.length() * config.miss_gravity_multiplier)
            .max(-profile.gravity(config).z);
        Some(MissFall {
            gravity_scale: gravity / config.gravity.length(),
            after_s: fired.time_of_flight_s,
        })
    } else {
        None
    };
    Ok((
        Launch {
            origin: aim.origin,
            velocity: fired.velocity,
            gravity_scale: profile.gravity_scale,
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
            motor: profile.motor,
            fall,
        },
        fired,
    ))
}
