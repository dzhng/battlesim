//! Physical projectile flight: the one store of rounds in the air, launch
//! solving against gravity and observed motion, and a swept collision pass each
//! tick against the static world and moving bodies.
//!
//! Each tick a round flies `n` straight chords of its exact parabola, `n` being
//! the fewest that keep every chord within the authored chord error of the
//! curve. Each chord is swept against terrain triangles and props (through
//! [`WorldGeometry::raycast`]) and against bodies moving over the same
//! interval, in the body's frame. Hits are exact against the chord (a turning
//! box is at most 1 mm conservative), so the reproduced oracle is "the true arc
//! within the chord error". The earliest hit consumes the round; exact ties go
//! to the lowest [`Struck`] (terrain, then props, then bodies, each by id).
mod index;
mod solve;
mod sweep;

pub use solve::{
    predicted_path, prepare_launch, scatter_aim, solve_launch, Aim, ArcKind, FiringSolution,
    NoSolution,
};

use contract::ballistics::{FlightRules, Trajectory, WeaponBallistics};
use contract::ids::UnitId;

use crate::digest::Digest;
use crate::math::{v2, v3, V3};
use crate::world::{Collider, PropId, WorldGeometry};
use index::BodyGrid;

/// Summed tick lengths drift by ulps; a remainder this small is no more flight.
const TIME_EPSILON_S: f64 = 1e-9;

/// Validated flight bounds at one tick rate.
#[derive(Clone, Debug)]
pub struct FlightConfig {
    gravity: V3,
    tick_s: f64,
    subsegments: u32,
    max_unguided_lifetime_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FlightConfigError {
    NotPositive(&'static str),
    /// Keeping chords within the chord error would need more chords per tick
    /// than declared; contacts would otherwise be skipped silently.
    TooManySubsegments {
        required: u32,
        max: u32,
    },
    LifetimeExceedsBound {
        lifetime_s: f64,
        max_s: f64,
    },
}

/// Chords per tick keeping each within `chord_error` of an arc under constant
/// acceleration `accel`: a chord of duration h strays at most accel·h²/8.
fn subsegments_for(accel: f64, tick_s: f64, chord_error: f64) -> u32 {
    ((accel * tick_s * tick_s / (8.0 * chord_error))
        .sqrt()
        .ceil() as u32)
        .max(1)
}

impl FlightConfig {
    pub fn new(rules: &FlightRules, tick_hz: u32) -> Result<Self, FlightConfigError> {
        let positive = |name, v: f64| (v > 0.0 && v.is_finite()).then_some(()).ok_or(name);
        positive("tick_hz", tick_hz as f64)
            .and(positive("curve_chord_error_m", rules.curve_chord_error_m))
            .and(positive(
                "max_subsegments_per_tick",
                rules.max_subsegments_per_tick as f64,
            ))
            .and(positive(
                "max_unguided_lifetime_s",
                rules.max_unguided_lifetime_s,
            ))
            .and(positive("gravity_mps2", rules.gravity_mps2))
            .map_err(FlightConfigError::NotPositive)?;
        let tick_s = 1.0 / tick_hz as f64;
        let required = subsegments_for(rules.gravity_mps2, tick_s, rules.curve_chord_error_m);
        if required > rules.max_subsegments_per_tick {
            return Err(FlightConfigError::TooManySubsegments {
                required,
                max: rules.max_subsegments_per_tick,
            });
        }
        Ok(FlightConfig {
            gravity: v3(0.0, 0.0, -rules.gravity_mps2),
            tick_s,
            subsegments: required,
            max_unguided_lifetime_s: rules.max_unguided_lifetime_s,
        })
    }

    pub fn gravity(&self) -> V3 {
        self.gravity
    }

    pub fn tick_s(&self) -> f64 {
        self.tick_s
    }

    pub fn subsegments_per_tick(&self) -> u32 {
        self.subsegments
    }

    /// A weapon row's launch profile; its lifetime is bounded.
    pub fn profile(&self, weapon: &WeaponBallistics) -> Result<LaunchProfile, FlightConfigError> {
        let lifetime_s = weapon.lifetime_s.unwrap_or(self.max_unguided_lifetime_s);
        for (name, v) in [("speed_mps", weapon.speed_mps), ("lifetime_s", lifetime_s)] {
            if !(v > 0.0 && v.is_finite()) {
                return Err(FlightConfigError::NotPositive(name));
            }
        }
        if lifetime_s > self.max_unguided_lifetime_s {
            return Err(FlightConfigError::LifetimeExceedsBound {
                lifetime_s,
                max_s: self.max_unguided_lifetime_s,
            });
        }
        Ok(LaunchProfile {
            speed_mps: weapon.speed_mps,
            lifetime_s,
            trajectory: weapon.trajectory,
            scatter_mrad: weapon.scatter_mrad,
            suppression_radius_m: weapon.suppression_radius_m,
        })
    }
}

/// What a weapon's launch needs from its row, validated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchProfile {
    pub speed_mps: f64,
    pub lifetime_s: f64,
    pub trajectory: Trajectory,
    pub scatter_mrad: f64,
    pub suppression_radius_m: f64,
}

/// Stable collider id of a moving body (a soldier or a vehicle hull).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// Upright soldier: radius, total height from the base.
    Capsule { radius: f64, height: f64 },
    /// Vehicle hull: half extents along heading, across heading, vertical;
    /// standing on the base.
    Box { half: V3 },
}

/// Ground contact centre and heading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub base: V3,
    pub yaw: f64,
}

/// A moving collider over one tick: posed at `from` at the tick's start and at
/// `to` at its end, moving linearly (and turning at a constant rate) between.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub id: BodyId,
    /// The unit (squad or vehicle) this body belongs to; near misses group by it.
    pub unit: UnitId,
    pub shape: Shape,
    pub from: Pose,
    pub to: Pose,
}

impl Body {
    /// Pose a tick fraction `s` into the tick; turns take the shorter way.
    fn pose_at(&self, s: f64) -> Pose {
        Pose {
            base: self.from.base + (self.to.base - self.from.base) * s,
            yaw: self.from.yaw + self.turn() * s,
        }
    }

    fn turn(&self) -> f64 {
        let d = (self.to.yaw - self.from.yaw).rem_euclid(std::f64::consts::TAU);
        if d > std::f64::consts::PI {
            d - std::f64::consts::TAU
        } else {
            d
        }
    }

    fn footprint_radius(&self) -> f64 {
        match self.shape {
            Shape::Capsule { radius, .. } => radius,
            Shape::Box { half } => half.x.hypot(half.y),
        }
    }
}

/// The firing body never collides with its own round, and its unit is not
/// suppressed by its own outgoing fire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shooter {
    pub unit: UnitId,
    pub body: BodyId,
}

/// Everything a round needs at launch; weapons and the lab emitter build it
/// through [`prepare_launch`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub origin: V3,
    pub velocity: V3,
    pub lifetime_s: f64,
    pub suppression_radius_m: f64,
    pub shooter: Option<Shooter>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectileId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Projectile {
    pub id: ProjectileId,
    pub position: V3,
    pub velocity: V3,
    pub age_s: f64,
    pub lifetime_s: f64,
    pub suppression_radius_m: f64,
    pub shooter: Option<Shooter>,
}

/// What a round struck. The derived order is the tie-break order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Struck {
    Terrain,
    Prop(PropId),
    Body(BodyId),
}

/// `time` fields are fractions of the tick in [0, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impact {
    pub projectile: ProjectileId,
    pub struck: Struck,
    pub point: V3,
    /// Outward surface normal at the hit, world frame.
    pub normal: V3,
    pub velocity: V3,
    pub time: f64,
}

/// A round's closest pass to a unit this tick: at most one per projectile per
/// unit per tick, measured from its flown path to the nearest body surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearMiss {
    pub projectile: ProjectileId,
    pub unit: UnitId,
    pub body: BodyId,
    pub distance: f64,
    /// The path point of closest approach.
    pub point: V3,
    pub time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    Lifetime,
    /// Outside the closed map bounds and moving away: nothing remains to hit.
    LeftMap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expired {
    pub projectile: ProjectileId,
    pub cause: Expiry,
    pub point: V3,
    pub time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlightEvent {
    Impact(Impact),
    NearMiss(NearMiss),
    Expired(Expired),
}

impl FlightEvent {
    pub fn time(&self) -> f64 {
        match self {
            FlightEvent::Impact(e) => e.time,
            FlightEvent::NearMiss(e) => e.time,
            FlightEvent::Expired(e) => e.time,
        }
    }

    pub fn projectile(&self) -> ProjectileId {
        match self {
            FlightEvent::Impact(e) => e.projectile,
            FlightEvent::NearMiss(e) => e.projectile,
            FlightEvent::Expired(e) => e.projectile,
        }
    }

    /// Events order by tick time, then projectile, near misses before the
    /// round's end, then unit.
    fn order_key(&self) -> (f64, ProjectileId, u8, u32) {
        match self {
            FlightEvent::NearMiss(e) => (e.time, e.projectile, 0, e.unit.0),
            FlightEvent::Impact(e) => (e.time, e.projectile, 1, 0),
            FlightEvent::Expired(e) => (e.time, e.projectile, 1, 0),
        }
    }
}

/// The one store of rounds in flight, in launch order.
pub struct Projectiles {
    config: FlightConfig,
    active: Vec<Projectile>,
    next_id: u64,
    grid: BodyGrid,
}

impl Projectiles {
    pub fn new(config: FlightConfig) -> Self {
        Projectiles {
            config,
            active: Vec::new(),
            next_id: 0,
            grid: BodyGrid::default(),
        }
    }

    pub fn config(&self) -> &FlightConfig {
        &self.config
    }

    /// Rounds launched now fly from the start of the next advance.
    pub fn launch(&mut self, launch: Launch) -> ProjectileId {
        let id = ProjectileId(self.next_id);
        self.next_id += 1;
        self.active.push(Projectile {
            id,
            position: launch.origin,
            velocity: launch.velocity,
            age_s: 0.0,
            lifetime_s: launch.lifetime_s,
            suppression_radius_m: launch.suppression_radius_m,
            shooter: launch.shooter,
        });
        id
    }

    pub fn active(&self) -> &[Projectile] {
        &self.active
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.next_id).u64(self.active.len() as u64);
        for p in &self.active {
            d.u64(p.id.0);
            for v in [p.position, p.velocity] {
                d.f64(v.x).f64(v.y).f64(v.z);
            }
            d.f64(p.age_s).f64(p.lifetime_s).f64(p.suppression_radius_m);
            if let Some(s) = p.shooter {
                d.u64(s.unit.0 as u64).u64(s.body.0 as u64);
            }
        }
    }
}

/// Positions and velocities at the ends of `n` equal chords over `span`
/// seconds: `p + v·h + ½a·h²`, then `v + a·h`, exact under constant gravity.
pub(crate) fn chords(p: V3, v: V3, a: V3, span: f64, n: u32, out: &mut Vec<(V3, V3)>) {
    let h = span / n as f64;
    out.clear();
    out.push((p, v));
    let (mut p, mut v) = (p, v);
    for _ in 0..n {
        p = p + v * h + a * (0.5 * h * h);
        v = v + a * h;
        out.push((p, v));
    }
}

/// Fly every round one tick against the world and `bodies` (posed over this
/// same tick). Appends this tick's events in time order; consumed and expired
/// rounds leave the store.
pub fn advance_projectiles(
    store: &mut Projectiles,
    world: &WorldGeometry,
    bodies: &[Body],
    events: &mut Vec<FlightEvent>,
) {
    let first = events.len();
    let Projectiles {
        config,
        active,
        grid,
        ..
    } = store;
    grid.rebuild(world.width(), world.depth(), bodies);
    let mut scratch = Scratch::default();
    active.retain_mut(|p| fly_tick(p, config, world, bodies, grid, &mut scratch, events));
    events[first..].sort_by(|a, b| {
        let (ka, kb) = (a.order_key(), b.order_key());
        ka.0.total_cmp(&kb.0)
            .then(ka.1.cmp(&kb.1))
            .then(ka.2.cmp(&kb.2))
            .then(ka.3.cmp(&kb.3))
    });
}

#[derive(Default)]
struct Scratch {
    path: Vec<(V3, V3)>,
    candidates: Vec<u32>,
    motions: Vec<(usize, sweep::Motion)>,
    misses: Vec<NearMiss>,
}

/// Returns whether the round is still in flight.
fn fly_tick(
    p: &mut Projectile,
    config: &FlightConfig,
    world: &WorldGeometry,
    bodies: &[Body],
    grid: &BodyGrid,
    scratch: &mut Scratch,
    events: &mut Vec<FlightEvent>,
) -> bool {
    let span = config.tick_s.min(p.lifetime_s - p.age_s);
    let n = config.subsegments;
    chords(
        p.position,
        p.velocity,
        config.gravity,
        span,
        n,
        &mut scratch.path,
    );
    // Bodies near this tick's path, out to the near-miss reach.
    let reach = p.suppression_radius_m;
    let (mut lo, mut hi) = (p.position.xy(), p.position.xy());
    for (q, _) in &scratch.path {
        lo = v2(lo.x.min(q.x), lo.y.min(q.y));
        hi = v2(hi.x.max(q.x), hi.y.max(q.y));
    }
    let pad = v2(reach, reach);
    grid.query(lo - pad, hi + pad, &mut scratch.candidates);
    scratch.misses.clear();
    let shooter = p.shooter;
    let chord_s = span / n as f64;
    let mut impact = None;
    for k in 0..n as usize {
        let ((a0, v0), (a1, _)) = (scratch.path[k], scratch.path[k + 1]);
        let (s0, s1) = (
            k as f64 * chord_s / config.tick_s,
            (k + 1) as f64 * chord_s / config.tick_s,
        );
        let chord = a1 - a0;
        let len = chord.length();
        let mut best: Option<(f64, Struck, V3)> = None;
        if len > 0.0 {
            if let Some(hit) = world.raycast(a0, chord * (1.0 / len), len) {
                let struck = match hit.collider {
                    Collider::Terrain => Struck::Terrain,
                    Collider::Prop(id) => Struck::Prop(id),
                };
                best = Some((hit.t / len, struck, hit.normal));
            }
        }
        scratch.motions.clear();
        scratch.motions.extend(scratch.candidates.iter().map(|&i| {
            let body = &bodies[i as usize];
            (i as usize, sweep::Motion::new(body, s0, s1))
        }));
        for (i, motion) in &scratch.motions {
            let body = &bodies[*i];
            if shooter.is_some_and(|s| s.body == body.id) {
                continue;
            }
            if let Some((u, normal)) = sweep::entry(&body.shape, motion, a0, a1) {
                let struck = Struck::Body(body.id);
                if best.is_none_or(|(bu, bs, _)| u < bu || (u == bu && struck < bs)) {
                    best = Some((u, struck, normal));
                }
            }
        }
        let u_end = best.map_or(1.0, |b| b.0);
        for (i, motion) in &scratch.motions {
            let body = &bodies[*i];
            if shooter.is_some_and(|s| s.unit == body.unit)
                || best.is_some_and(|b| b.1 == Struck::Body(body.id))
            {
                continue;
            }
            let Some((distance, u)) = sweep::closest_approach(
                &body.shape,
                motion,
                body.footprint_radius(),
                a0,
                a1,
                u_end,
                reach,
            ) else {
                continue;
            };
            let miss = NearMiss {
                projectile: p.id,
                unit: body.unit,
                body: body.id,
                distance,
                point: a0 + chord * u,
                time: s0 + (s1 - s0) * u,
            };
            match scratch.misses.iter_mut().find(|m| m.unit == body.unit) {
                Some(m)
                    if distance < m.distance || (distance == m.distance && body.id < m.body) =>
                {
                    *m = miss
                }
                Some(_) => {}
                None => scratch.misses.push(miss),
            }
        }
        if let Some((u, struck, normal)) = best {
            impact = Some(Impact {
                projectile: p.id,
                struck,
                point: a0 + chord * u,
                normal,
                velocity: v0 + config.gravity * (u * chord_s),
                time: s0 + (s1 - s0) * u,
            });
            break;
        }
    }
    events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
    if let Some(hit) = impact {
        events.push(FlightEvent::Impact(hit));
        return false;
    }
    let (end, v_end) = *scratch.path.last().expect("path has its start point");
    p.position = end;
    p.velocity = v_end;
    p.age_s += span;
    let expire = |cause| {
        FlightEvent::Expired(Expired {
            projectile: p.id,
            cause,
            point: end,
            time: span / config.tick_s,
        })
    };
    if p.age_s >= p.lifetime_s - TIME_EPSILON_S {
        events.push(expire(Expiry::Lifetime));
        return false;
    }
    let leaving = |x: f64, vx: f64, hi: f64| (x < 0.0 && vx <= 0.0) || (x > hi && vx >= 0.0);
    if leaving(end.x, v_end.x, world.width()) || leaving(end.y, v_end.y, world.depth()) {
        events.push(expire(Expiry::LeftMap));
        return false;
    }
    true
}
