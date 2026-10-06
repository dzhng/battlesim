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
//! within the chord error". The earliest hit is judged by the caller's
//! [`ImpactResolver`]: it stops or detonates the round, or deflects it, and a
//! deflected round flies the rest of the tick from the hit, never meeting the
//! body it glanced off again that tick. Exact ties go to the lowest [`Struck`]
//! (terrain, then props, then bodies, each by id).
mod index;
mod solve;
mod sweep;

pub use solve::{
    launch_along, predicted_path, prepare_launch, scatter_aim, solve_launch, solve_launch_past,
    Aim, ArcKind, FiringSolution, NoSolution,
};

use contract::ballistics::{FlightRules, GuidedRules, Trajectory, WeaponBallistics};
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
    miss_fall_max_s: f64,
    ricochet_lifetime_s: f64,
    miss_gravity_multiplier: f64,
    tick_s: f64,
    subsegments: u32,
    chord_error_m: f64,
    max_unguided_lifetime_s: f64,
    min_spread_at_max_range_m: f64,
    /// Intercepts and launches already solved ([`solve::Solved`]).
    solved: solve::Solved,
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
    /// `accel_mps2` and `top_speed_mps` come together, on a guided row (a
    /// motor round flies without gravity), with a top speed above launch.
    Motor(&'static str),
    /// An unguided row more accurate at its range than
    /// `physics.min_spread_at_max_range_m` allows.
    TighterThanCeiling {
        spread_m: f64,
        min_m: f64,
    },
}

impl std::fmt::Display for FlightConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPositive(field) => write!(f, "{field} is outside its finite physical range"),
            Self::TooManySubsegments { required, max } => write!(
                f,
                "Trajectory needs {required} collision segments per tick; the limit is {max}. Reduce gravity strength"
            ),
            Self::LifetimeExceedsBound { lifetime_s, max_s } => write!(
                f,
                "Projectile lifetime must be at most {max_s} s; entered {lifetime_s} s"
            ),
            Self::Motor(reason) => write!(f, "Invalid missile motor: {reason}"),
            Self::TighterThanCeiling { spread_m, min_m } => write!(
                f,
                "Landing spread must be at least {min_m} m at maximum range; entered {spread_m} m"
            ),
        }
    }
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
            .and(positive(
                "miss_gravity_multiplier",
                rules.miss_gravity_multiplier,
            ))
            .and(
                (rules.ricochet_lifetime_s >= 0.0 && rules.ricochet_lifetime_s.is_finite())
                    .then_some(())
                    .ok_or("ricochet_lifetime_s"),
            )
            .and(
                (rules.miss_fall_max_s >= 0.0 && rules.miss_fall_max_s.is_finite())
                    .then_some(())
                    .ok_or("miss_fall_max_s"),
            )
            .and(
                (rules.min_spread_at_max_range_m >= 0.0
                    && rules.min_spread_at_max_range_m.is_finite())
                .then_some(())
                .ok_or("min_spread_at_max_range_m"),
            )
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
            solved: solve::Solved::default(),
            gravity: v3(0.0, 0.0, -rules.gravity_mps2),
            miss_fall_max_s: rules.miss_fall_max_s,
            ricochet_lifetime_s: rules.ricochet_lifetime_s,
            miss_gravity_multiplier: rules.miss_gravity_multiplier,
            tick_s,
            subsegments: required,
            chord_error_m: rules.curve_chord_error_m,
            max_unguided_lifetime_s: rules.max_unguided_lifetime_s,
            min_spread_at_max_range_m: rules.min_spread_at_max_range_m,
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

    /// A weapon row's launch profile; its lifetime is bounded, and an
    /// unguided row's spread at its range is no tighter than the rules'
    /// ceiling on accuracy.
    pub fn profile(&self, weapon: &WeaponBallistics) -> Result<LaunchProfile, FlightConfigError> {
        let lifetime_s = weapon.lifetime_s.unwrap_or(self.max_unguided_lifetime_s);
        if !(weapon.scatter_mrad >= 0.0 && weapon.scatter_mrad.is_finite()) {
            return Err(FlightConfigError::NotPositive("scatter_mrad"));
        }
        for (name, v) in [
            ("speed_mps", weapon.speed_mps),
            ("range_m", weapon.range_m),
            ("lifetime_s", lifetime_s),
            ("gravity_scale", weapon.gravity_scale),
        ] {
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
        let motor = match (weapon.accel_mps2, weapon.top_speed_mps) {
            (None, None) => None,
            (Some(accel_mps2), Some(top_speed_mps)) => {
                if weapon.turn_deg_s.is_none() {
                    return Err(FlightConfigError::Motor("a motor needs a guided row"));
                }
                if !(accel_mps2 > 0.0 && accel_mps2.is_finite()) {
                    return Err(FlightConfigError::NotPositive("accel_mps2"));
                }
                if !(top_speed_mps > weapon.speed_mps && top_speed_mps.is_finite()) {
                    return Err(FlightConfigError::Motor(
                        "top_speed_mps must exceed speed_mps",
                    ));
                }
                Some(Motor {
                    accel_mps2,
                    top_speed_mps,
                })
            }
            _ => {
                return Err(FlightConfigError::Motor(
                    "accel_mps2 and top_speed_mps come together",
                ))
            }
        };
        let spread_m = weapon.scatter_mrad * 1e-3 * weapon.range_m;
        if weapon.turn_deg_s.is_none() && spread_m < self.min_spread_at_max_range_m {
            return Err(FlightConfigError::TighterThanCeiling {
                spread_m,
                min_m: self.min_spread_at_max_range_m,
            });
        }
        // A heavier share of gravity bends each chord more: it must still
        // keep within the chord error at the declared chords per tick.
        let accel = self.gravity.length() * weapon.gravity_scale;
        let required = subsegments_for(accel, self.tick_s, self.chord_error_m);
        if required > self.subsegments {
            return Err(FlightConfigError::TooManySubsegments {
                required,
                max: self.subsegments,
            });
        }
        Ok(LaunchProfile {
            speed_mps: weapon.speed_mps,
            gravity_scale: weapon.gravity_scale,
            lifetime_s,
            trajectory: weapon.trajectory,
            scatter_mrad: weapon.scatter_mrad,
            suppression_radius_m: weapon.suppression_radius_m,
            turn_rad_s: weapon.turn_deg_s.map(f64::to_radians),
            motor,
        })
    }
}

/// What a weapon's launch needs from its row, validated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchProfile {
    pub speed_mps: f64,
    /// The share of the world's gravity an unguided round falls under.
    pub gravity_scale: f64,
    pub lifetime_s: f64,
    pub trajectory: Trajectory,
    /// One-axis angular standard deviation of launch spread, milliradians.
    pub scatter_mrad: f64,
    pub suppression_radius_m: f64,
    /// Guided rounds' turn limit; `None` flies ballistically.
    pub turn_rad_s: Option<f64>,
    /// A rocket motor speeding the round up along its heading.
    pub motor: Option<Motor>,
}

/// A rocket motor: from its launch speed the round speeds up at
/// `accel_mps2` along its heading, then holds `top_speed_mps`. The thrust
/// is along the velocity, so a tick's flight stays a straight line and
/// its chords are exact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motor {
    pub accel_mps2: f64,
    pub top_speed_mps: f64,
}

impl Motor {
    /// The thrust over the next `span` seconds for a round at `velocity`:
    /// the full rate, or what reaches top speed exactly at the span's end.
    pub fn thrust(&self, velocity: V3, span: f64) -> V3 {
        let speed = velocity.length();
        if speed <= 0.0 || span <= 0.0 {
            return v3(0.0, 0.0, 0.0);
        }
        let rate = self
            .accel_mps2
            .min((self.top_speed_mps - speed).max(0.0) / span);
        velocity * (rate / speed)
    }

    /// Distance flown `t` seconds after a launch at `launch_mps`, in a
    /// straight line.
    pub fn distance(&self, launch_mps: f64, t: f64) -> f64 {
        let burn = ((self.top_speed_mps - launch_mps) / self.accel_mps2).max(0.0);
        let tb = t.min(burn);
        launch_mps * tb + 0.5 * self.accel_mps2 * tb * tb + self.top_speed_mps * (t - tb)
    }
}

impl LaunchProfile {
    /// The acceleration the round flies under: its share of gravity, or
    /// none for a motor-sustained guided round.
    pub fn gravity(&self, config: &FlightConfig) -> V3 {
        match self.turn_rad_s {
            Some(_) => v3(0.0, 0.0, 0.0),
            None => config.gravity * self.gravity_scale,
        }
    }
}

/// A released missile must coast forward: at zero or less its point would lie
/// beneath or behind it, and it would circle within its turn limit.
pub fn validate_guided(rules: &GuidedRules) {
    assert!(
        rules.release_coast_s > 0.0 && rules.release_coast_s.is_finite(),
        "guided.release_coast_s must be positive"
    );
}

/// A guided round's steering: toward `point`, turning at most `turn_rad_s`.
/// While `supported`, its launcher renews the point from its side’s sighting;
/// once released, the point is fixed for good, a coast ahead of where the
/// release found it (P06: it never reacquires).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guidance {
    pub point: V3,
    pub turn_rad_s: f64,
    pub supported: bool,
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
    pub fn pose_at(&self, s: f64) -> Pose {
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
            Shape::Box { half } => libm::hypot(half.x, half.y),
        }
    }
}

/// The firing body never collides with its own round, and its unit is not
/// suppressed by its own outgoing fire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shooter {
    pub unit: UnitId,
    pub body: BodyId,
    /// The body he fires from behind: his round passes it untouched.
    pub cover: Option<Struck>,
}

/// A direct-fire miss falls to ground within the authored post-target time limit.
/// The stronger gravity is fixed at launch; position and velocity stay continuous.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissFall {
    pub gravity_scale: f64,
    pub after_s: f64,
}

/// Everything a round needs at launch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub origin: V3,
    pub velocity: V3,
    /// The share of the world's gravity it falls under while unguided.
    pub gravity_scale: f64,
    pub lifetime_s: f64,
    pub suppression_radius_m: f64,
    pub shooter: Option<Shooter>,
    pub guidance: Option<Guidance>,
    pub motor: Option<Motor>,
    pub fall: Option<MissFall>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectileId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Projectile {
    pub id: ProjectileId,
    pub position: V3,
    pub velocity: V3,
    /// The share of the world's gravity it falls under while unguided.
    pub gravity_scale: f64,
    pub age_s: f64,
    pub lifetime_s: f64,
    pub suppression_radius_m: f64,
    pub shooter: Option<Shooter>,
    pub guidance: Option<Guidance>,
    pub motor: Option<Motor>,
    pub fall: Option<MissFall>,
    /// Ricochets so far; the resolver bounds them.
    pub bounces: u8,
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
    /// The round's ricochets before this hit.
    pub bounces: u8,
    /// A struck body's pose at the moment of the hit, not the tick's end.
    pub pose: Option<Pose>,
    /// The resolver burst the round here (its blast applies).
    pub detonated: bool,
}

/// A round glancing off a body: it keeps its id and flies on from `point`
/// with `deflected` velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ricochet {
    pub projectile: ProjectileId,
    pub body: BodyId,
    pub point: V3,
    /// Outward surface normal at the hit, world frame.
    pub normal: V3,
    /// Velocity arriving at the hit.
    pub velocity: V3,
    pub deflected: V3,
    pub time: f64,
    /// The round's ricochets before this one.
    pub bounces: u8,
    /// The body's pose at the moment of the hit.
    pub pose: Pose,
}

/// A round meeting a collider, for the [`ImpactResolver`] to judge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactContext {
    pub projectile: ProjectileId,
    pub bounces: u8,
    pub struck: Struck,
    pub point: V3,
    /// Outward surface normal at the hit, world frame.
    pub normal: V3,
    pub velocity: V3,
    /// A struck body's pose at the moment of the hit.
    pub pose: Option<Pose>,
}

/// What becomes of a round that meets a collider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImpactDecision {
    /// Consumed where it hit.
    Stop,
    /// Consumed in a burst where it hit.
    Detonate,
    /// Deflected off a body: flies on from the hit with this velocity.
    /// Honoured only for bodies; anything else stops the round.
    Bounce { velocity: V3 },
}

/// Judges each hit during flight. Damage owns the battle's resolver
/// (penetration, faces, ricochet); flight only carries out the decision.
#[derive(Clone, Copy, Debug)]
pub struct Interception {
    pub projectile: ProjectileId,
    pub body: BodyId,
    pub unit: UnitId,
    pub point: V3,
    pub velocity: V3,
    /// Fraction of the advancing tick.
    pub time: f64,
    pub pose: Pose,
}

pub trait ImpactResolver {
    fn resolve(&mut self, impact: &ImpactContext) -> ImpactDecision;
    fn max_standoff(&self, _projectile: ProjectileId) -> f64 {
        0.0
    }
    fn standoff(&self, _projectile: ProjectileId, _body: &Body) -> Option<f64> {
        None
    }
    fn intercept(&mut self, _event: &Interception) -> bool {
        false
    }
}

impl<F: FnMut(&ImpactContext) -> ImpactDecision> ImpactResolver for F {
    fn resolve(&mut self, impact: &ImpactContext) -> ImpactDecision {
        self(impact)
    }
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

/// A round flying through a destroyable body that does not stop rounds (a
/// fence panel, a crate): it flies on, and the body takes the hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pass {
    pub projectile: ProjectileId,
    pub prop: PropId,
    /// Where the round entered it.
    pub point: V3,
    /// The round's unit direction of flight through it.
    pub along: V3,
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
    Ricochet(Ricochet),
    NearMiss(NearMiss),
    Pass(Pass),
    Expired(Expired),
}

impl FlightEvent {
    pub fn time(&self) -> f64 {
        match self {
            FlightEvent::Impact(e) => e.time,
            FlightEvent::Ricochet(e) => e.time,
            FlightEvent::NearMiss(e) => e.time,
            FlightEvent::Pass(e) => e.time,
            FlightEvent::Expired(e) => e.time,
        }
    }

    pub fn projectile(&self) -> ProjectileId {
        match self {
            FlightEvent::Impact(e) => e.projectile,
            FlightEvent::Ricochet(e) => e.projectile,
            FlightEvent::NearMiss(e) => e.projectile,
            FlightEvent::Pass(e) => e.projectile,
            FlightEvent::Expired(e) => e.projectile,
        }
    }

    /// Events order by tick time, then projectile, near misses before
    /// passes before a ricochet before the round's end, then unit or prop.
    fn order_key(&self) -> (f64, ProjectileId, u8, u32) {
        match self {
            FlightEvent::NearMiss(e) => (e.time, e.projectile, 0, e.unit.0),
            FlightEvent::Pass(e) => (e.time, e.projectile, 1, e.prop),
            FlightEvent::Ricochet(e) => (e.time, e.projectile, 2, 0),
            FlightEvent::Impact(e) => (e.time, e.projectile, 3, 0),
            FlightEvent::Expired(e) => (e.time, e.projectile, 3, 0),
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
            gravity_scale: launch.gravity_scale,
            age_s: 0.0,
            lifetime_s: launch.lifetime_s,
            suppression_radius_m: launch.suppression_radius_m,
            shooter: launch.shooter,
            guidance: launch.guidance,
            motor: launch.motor,
            fall: launch.fall,
            bounces: 0,
        });
        id
    }

    /// Renew a supported round's commanded point; a released one ignores it.
    pub fn steer(&mut self, id: ProjectileId, point: V3) {
        if let Some(g) = self.guidance_mut(id).filter(|g| g.supported) {
            g.point = point;
        }
    }

    /// End support for good (P06). The missile flies straight on:
    /// its point is fixed where it would be `coast_s` from now, dropped to the
    /// ground beneath, so it runs on its line and then goes to ground within
    /// its turn limit. Beyond the map the point keeps the missile's height.
    pub fn release(&mut self, id: ProjectileId, coast_s: f64, world: &WorldGeometry) {
        let Some(p) = self.active.iter_mut().find(|p| p.id == id) else {
            return;
        };
        let ahead = p.position + p.velocity * coast_s;
        if let Some(g) = p.guidance.as_mut() {
            g.supported = false;
            g.point = ahead
                .xy()
                .with_z(world.height_at(ahead.x, ahead.y).unwrap_or(ahead.z));
        }
    }

    fn guidance_mut(&mut self, id: ProjectileId) -> Option<&mut Guidance> {
        self.active
            .iter_mut()
            .find(|p| p.id == id)
            .and_then(|p| p.guidance.as_mut())
    }

    pub fn get(&self, id: ProjectileId) -> Option<&Projectile> {
        self.active.iter().find(|p| p.id == id)
    }

    pub fn active(&self) -> &[Projectile] {
        &self.active
    }

    /// Rounds launched since the battle began.
    pub fn launched(&self) -> u64 {
        self.next_id
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.next_id).u64(self.active.len() as u64);
        for p in &self.active {
            d.u64(p.id.0);
            for v in [p.position, p.velocity] {
                d.f64(v.x).f64(v.y).f64(v.z);
            }
            d.f64(p.age_s).f64(p.lifetime_s).f64(p.suppression_radius_m);
            d.f64(p.gravity_scale);
            d.u64(p.bounces as u64);
            d.u64(p.shooter.is_some() as u64);
            if let Some(s) = p.shooter {
                d.u64(s.unit.0 as u64).u64(s.body.0 as u64);
                match s.cover {
                    None => {
                        d.u64(0);
                    }
                    Some(Struck::Prop(id)) => {
                        d.u64(1).u64(id as u64);
                    }
                    Some(Struck::Body(id)) => {
                        d.u64(2).u64(id.0 as u64);
                    }
                    Some(Struck::Terrain) => {
                        d.u64(3);
                    }
                }
            }
            d.u64(p.guidance.is_some() as u64);
            if let Some(g) = p.guidance {
                d.f64(g.point.x)
                    .f64(g.point.y)
                    .f64(g.point.z)
                    .f64(g.turn_rad_s)
                    .u64(g.supported as u64);
            }
            if let Some(fall) = p.fall {
                d.u64(0x4c414e44).f64(fall.gravity_scale).f64(fall.after_s);
            }
            d.u64(p.motor.is_some() as u64);
            if let Some(m) = p.motor {
                d.f64(m.accel_mps2).f64(m.top_speed_mps);
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
/// same tick), each hit judged by `resolver` as it happens. Appends this
/// tick's events in time order; consumed and expired rounds leave the store.
pub fn advance_projectiles(
    store: &mut Projectiles,
    world: &WorldGeometry,
    bodies: &[Body],
    events: &mut Vec<FlightEvent>,
    resolver: &mut impl ImpactResolver,
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
    let flight = Flight {
        config,
        world,
        bodies,
        grid,
    };
    // Protection decisions consume shared unit resources, so evaluate imminent
    // swept hits first and resolve by physical event time, then projectile id.
    let mut candidates: Vec<_> = active
        .iter()
        .filter_map(|p| flight.interception(p, &mut scratch, resolver))
        .collect();
    candidates.sort_by(|a, b| {
        a.time
            .total_cmp(&b.time)
            .then(a.projectile.cmp(&b.projectile))
    });
    let intercepted: std::collections::BTreeMap<_, _> = candidates
        .into_iter()
        .filter(|event| resolver.intercept(event))
        .map(|event| (event.projectile, event))
        .collect();
    scratch.passes.clear();
    scratch.misses.clear();
    active.retain_mut(|p| {
        if let Some(event) = intercepted.get(&p.id) {
            // Trace only to the premature detonation, preserving ordinary
            // pass-through and near-miss evidence before that point.
            let mut traced = p.clone();
            let span = event.time * config.tick_s;
            let tick_span = config.tick_s.min(traced.lifetime_s - traced.age_s);
            let gravity = flight.acceleration(&mut traced, tick_span);
            scratch.misses.clear();
            let _ = flight.fly_leg(&traced, &mut scratch, gravity, span, 0.0, None);
            events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
            events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
            events.push(FlightEvent::Impact(Impact {
                projectile: p.id,
                struck: Struck::Terrain,
                point: event.point,
                normal: V3::default(),
                velocity: event.velocity,
                time: event.time,
                bounces: p.bounces,
                pose: None,
                detonated: true,
            }));
            false
        } else {
            flight.fly_tick(p, &mut scratch, events, resolver)
        }
    });
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
    path_times: Vec<f64>,
    accelerations: Vec<V3>,
    leg: Vec<(V3, V3)>,
    candidates: Vec<u32>,
    motions: Vec<(usize, sweep::Motion)>,
    misses: Vec<NearMiss>,
    passes: Vec<Pass>,
    crossed: Vec<(f64, PropId)>,
}

/// What every round flies against this tick.
struct Flight<'a> {
    config: &'a FlightConfig,
    world: &'a WorldGeometry,
    bodies: &'a [Body],
    grid: &'a BodyGrid,
}

/// The earliest hit along one leg of a round's flight.
struct Hit {
    struck: Struck,
    /// Index of the struck body in the tick's bodies.
    body: Option<usize>,
    point: V3,
    normal: V3,
    velocity: V3,
    time: f64,
}

impl Flight<'_> {
    fn acceleration(&self, p: &mut Projectile, span: f64) -> V3 {
        let gravity = match p.guidance {
            Some(g) => {
                p.velocity = steer(p.velocity, g.point - p.position, g.turn_rad_s * span);
                V3::default()
            }
            None => self.config.gravity * p.gravity_scale,
        };
        gravity
            + p.motor
                .map_or(V3::default(), |motor| motor.thrust(p.velocity, span))
    }

    fn interception(
        &self,
        original: &Projectile,
        scratch: &mut Scratch,
        resolver: &impl ImpactResolver,
    ) -> Option<Interception> {
        let max_reach = resolver.max_standoff(original.id);
        if max_reach <= 0.0 {
            return None;
        }
        let mut p = original.clone();
        let tick_span = self.config.tick_s.min(p.lifetime_s - p.age_s);
        let gravity = self.acceleration(&mut p, tick_span);
        // Look only one standoff travel ahead of this tick. Guidance and body
        // motion are re-evaluated on every ordinary advancement.
        let span =
            (tick_span + max_reach / p.velocity.length().max(1.0)).min(p.lifetime_s - p.age_s);
        scratch.passes.clear();
        scratch.misses.clear();
        let hit = self.fly_leg(&p, scratch, gravity, span, 0.0, None)?;
        let body = &self.bodies[hit.body?];
        let reach = resolver.standoff(p.id, body)?;
        for k in 0..scratch.accelerations.len() {
            let s0 = scratch.path_times[k] / self.config.tick_s;
            let s1 = scratch.path_times[k + 1] / self.config.tick_s;
            if s0 > hit.time || s0 > 1.0 {
                break;
            }
            let end = ((hit.time - s0) / (s1 - s0)).min(1.0);
            let motion = sweep::Motion::new(body, s0, s1);
            let ((a0, v0), (a1, _)) = (scratch.path[k], scratch.path[k + 1]);
            if let Some(u) = sweep::standoff_entry(&body.shape, &motion, a0, a1, end, reach) {
                let time = s0 + (s1 - s0) * u;
                if time > 1.0 {
                    return None;
                }
                return Some(Interception {
                    projectile: p.id,
                    body: body.id,
                    unit: body.unit,
                    point: a0 + (a1 - a0) * u,
                    velocity: v0 + scratch.accelerations[k] * (u * (s1 - s0) * self.config.tick_s),
                    time,
                    pose: body.pose_at(time),
                });
            }
        }
        None
    }
    /// Returns whether the round is still in flight.
    fn fly_tick(
        &self,
        p: &mut Projectile,
        scratch: &mut Scratch,
        events: &mut Vec<FlightEvent>,
        resolver: &mut impl ImpactResolver,
    ) -> bool {
        let config = self.config;
        let span = config.tick_s.min(p.lifetime_s - p.age_s);
        let gravity = self.acceleration(p, span);
        scratch.misses.clear();
        // Each leg flies from the round's state `flown` seconds into the tick;
        // a ricochet starts the next leg at its hit, clear of the body it
        // glanced off.
        let (mut flown, mut glanced) = (0.0, None);
        while let Some(hit) = self.fly_leg(p, scratch, gravity, span, flown, glanced) {
            events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
            let pose = hit.body.map(|i| self.bodies[i].pose_at(hit.time));
            let decision = resolver.resolve(&ImpactContext {
                projectile: p.id,
                bounces: p.bounces,
                struck: hit.struck,
                point: hit.point,
                normal: hit.normal,
                velocity: hit.velocity,
                pose,
            });
            if let (ImpactDecision::Bounce { velocity }, Struck::Body(body), Some(pose)) =
                (decision, hit.struck, pose)
            {
                events.push(FlightEvent::Ricochet(Ricochet {
                    projectile: p.id,
                    body,
                    point: hit.point,
                    normal: hit.normal,
                    velocity: hit.velocity,
                    deflected: velocity,
                    time: hit.time,
                    bounces: p.bounces,
                    pose,
                }));
                if p.bounces == 0 && config.ricochet_lifetime_s > 0.0 {
                    p.lifetime_s = p
                        .lifetime_s
                        .min(p.age_s + hit.time * config.tick_s + config.ricochet_lifetime_s);
                }
                p.fall = None;
                p.bounces += 1;
                p.position = hit.point;
                p.velocity = velocity;
                flown = hit.time * config.tick_s;
                glanced = Some(body);
                continue;
            }
            events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
            events.push(FlightEvent::Impact(Impact {
                projectile: p.id,
                struck: hit.struck,
                point: hit.point,
                normal: hit.normal,
                velocity: hit.velocity,
                time: hit.time,
                bounces: p.bounces,
                pose,
                detonated: decision == ImpactDecision::Detonate,
            }));
            return false;
        }
        events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
        events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
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
        let world = self.world;
        if leaving(end.x, v_end.x, world.width()) || leaving(end.y, v_end.y, world.depth()) {
            events.push(expire(Expiry::LeftMap));
            return false;
        }
        true
    }

    /// Fly the round from its state `flown` seconds into the tick to the
    /// tick's `span`, in chords, against the world and every body but
    /// `glanced`. Returns the earliest hit, leaving the flown path in
    /// `scratch.path` and folding each unit's closest pass into
    /// `scratch.misses`, so a round passes a unit at most once a tick however
    /// often it glances.
    fn fly_leg(
        &self,
        p: &Projectile,
        scratch: &mut Scratch,
        gravity: V3,
        span: f64,
        flown: f64,
        glanced: Option<BodyId>,
    ) -> Option<Hit> {
        let Flight {
            config,
            world,
            bodies,
            grid,
        } = *self;
        let rest = span - flown;
        let split = p
            .fall
            .map_or(rest, |l| (l.after_s - p.age_s - flown).clamp(0.0, rest));
        scratch.path.clear();
        scratch.path_times.clear();
        scratch.accelerations.clear();
        scratch.path.push((p.position, p.velocity));
        scratch.path_times.push(flown);
        for (start, duration, acceleration) in [
            (flown, split, gravity),
            (
                flown + split,
                rest - split,
                p.fall.map_or(gravity, |f| config.gravity * f.gravity_scale),
            ),
        ] {
            if duration <= TIME_EPSILON_S {
                continue;
            }
            let (position, velocity) = *scratch.path.last().unwrap();
            let n = subsegments_for(acceleration.length(), duration, config.chord_error_m);
            chords(
                position,
                velocity,
                acceleration,
                duration,
                n,
                &mut scratch.leg,
            );
            for (k, endpoint) in scratch.leg.iter().enumerate().skip(1) {
                scratch.path.push(*endpoint);
                scratch
                    .path_times
                    .push(start + duration * k as f64 / n as f64);
                scratch.accelerations.push(acceleration);
            }
        }
        // Bodies near this leg's path, out to the near-miss reach.
        let reach = p.suppression_radius_m;
        let (mut lo, mut hi) = (p.position.xy(), p.position.xy());
        for (q, _) in &scratch.path {
            lo = v2(lo.x.min(q.x), lo.y.min(q.y));
            hi = v2(hi.x.max(q.x), hi.y.max(q.y));
        }
        let pad = v2(reach, reach);
        grid.query(lo - pad, hi + pad, &mut scratch.candidates);
        // A unit's rounds never strike its own bodies (a squad keeps its own
        // fire lanes), and a ricochet never meets the body it glanced off.
        let shooter = p.shooter;
        let past = shooter.and_then(|s| match s.cover {
            Some(Struck::Prop(id)) => Some(id),
            _ => None,
        });
        scratch.candidates.retain(|&i| {
            let body = &bodies[i as usize];
            shooter.is_none_or(|s| s.unit != body.unit && s.cover != Some(Struck::Body(body.id)))
                && glanced != Some(body.id)
        });
        for k in 0..scratch.accelerations.len() {
            let chord_s = scratch.path_times[k + 1] - scratch.path_times[k];
            let acceleration = scratch.accelerations[k];
            let ((a0, v0), (a1, _)) = (scratch.path[k], scratch.path[k + 1]);
            let (s0, s1) = (
                scratch.path_times[k] / config.tick_s,
                scratch.path_times[k + 1] / config.tick_s,
            );
            let chord = a1 - a0;
            let len = chord.length();
            let mut best: Option<(f64, Struck, V3, Option<usize>)> = None;
            if len > 0.0 {
                if let Some(hit) = world.raycast_past(a0, chord * (1.0 / len), len, past) {
                    let struck = match hit.collider {
                        Collider::Terrain => Struck::Terrain,
                        Collider::Prop(id) => Struck::Prop(id),
                    };
                    best = Some((hit.t / len, struck, hit.normal, None));
                }
            }
            scratch.motions.clear();
            scratch.motions.extend(scratch.candidates.iter().map(|&i| {
                let body = &bodies[i as usize];
                (i as usize, sweep::Motion::new(body, s0, s1))
            }));
            for (i, motion) in &scratch.motions {
                let body = &bodies[*i];
                if let Some((u, normal)) = sweep::entry(&body.shape, motion, a0, a1) {
                    let struck = Struck::Body(body.id);
                    if best.is_none_or(|(bu, bs, ..)| u < bu || (u == bu && struck < bs)) {
                        best = Some((u, struck, normal, Some(*i)));
                    }
                }
            }
            let u_end = best.map_or(1.0, |b| b.0);
            // Bodies it flies through on the way (not past its hit).
            if len > 0.0 {
                scratch.crossed.clear();
                world.passes(
                    a0,
                    chord * (1.0 / len),
                    len * u_end,
                    past,
                    &mut scratch.crossed,
                );
                scratch
                    .passes
                    .extend(scratch.crossed.iter().map(|&(t, prop)| Pass {
                        projectile: p.id,
                        prop,
                        point: a0 + chord * (t / len),
                        along: chord * (1.0 / len),
                        time: s0 + (s1 - s0) * (t / len),
                    }));
            }
            for (i, motion) in &scratch.motions {
                let body = &bodies[*i];
                if best.is_some_and(|b| b.1 == Struck::Body(body.id)) {
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
                        if distance < m.distance
                            || (distance == m.distance && body.id < m.body) =>
                    {
                        *m = miss
                    }
                    Some(_) => {}
                    None => scratch.misses.push(miss),
                }
            }
            if let Some((u, struck, normal, body)) = best {
                return Some(Hit {
                    struck,
                    body,
                    point: a0 + chord * u,
                    normal,
                    velocity: v0 + acceleration * (u * chord_s),
                    time: s0 + (s1 - s0) * u,
                });
            }
        }
        None
    }
}

/// `velocity` turned toward `toward` by at most `max_angle`, keeping its speed.
pub fn steer(velocity: V3, toward: V3, max_angle: f64) -> V3 {
    let speed = velocity.length();
    if speed == 0.0 || toward.length() < 1e-9 {
        return velocity;
    }
    let (a, b) = (velocity * (1.0 / speed), toward.normalized());
    let angle = a.dot(b).clamp(-1.0, 1.0).acos();
    if angle <= max_angle {
        return b * speed;
    }
    // Rotate `a` toward `b` in their common plane by `max_angle`.
    let perp = b - a * a.dot(b);
    let perp = if perp.length() > 1e-12 {
        perp.normalized()
    } else {
        // Opposite directions: any perpendicular, preferring the horizontal.
        let side = a.cross(v3(0.0, 0.0, 1.0));
        if side.length() > 1e-9 {
            side.normalized()
        } else {
            v3(1.0, 0.0, 0.0)
        }
    };
    (a * libm::cos(max_angle) + perp * libm::sin(max_angle)) * speed
}
