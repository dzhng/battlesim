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
mod advance;
mod config;
mod index;
mod release;
mod solve;
mod sweep;

pub use advance::advance_projectiles;
pub(crate) use advance::chords;
pub use config::{FlightConfig, FlightConfigError, LaunchProfile, Loft, Motor};
pub use release::{release, Release};
// Every public item of the solver is the flight module's.
pub use solve::*;

use contract::ballistics::{Delivery, FlightRules, Trajectory, WeaponBallistics};
use contract::ids::UnitId;

use crate::digest::Digest;
use crate::math::{v2, v3, V3};
use crate::world::{Collider, PropId, WorldGeometry};
use index::BodyGrid;

/// Summed tick lengths drift by ulps; a remainder this small is no more flight.
const TIME_EPSILON_S: f64 = 1e-9;

/// A guided round's steering: toward `point`, turning at most `turn_rad_s`.
/// While `supported`, its launcher renews the point from its side’s sighting;
/// once released, the point is fixed for good, a coast ahead of where the
/// release found it (P06: it never reacquires).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guidance {
    pub point: V3,
    pub turn_rad_s: f64,
    pub supported: bool,
    /// A top-attack round's path above its point.
    pub loft: Option<Loft>,
}

impl Guidance {
    /// Where a round at `position` steers. A supported top-attack round
    /// shoots up toward its loft height, as steeply as it will dive, holds
    /// that height until its commanded point lies inside its dive cone, then
    /// dives onto it; any other, and a released one, flies at its point.
    ///
    /// The climb aims at the loft height one climb's run (`height_m /
    /// slope`) ahead, never past the point: the missile rises at the dive
    /// angle and levels off as it nears the loft. That run is also where the
    /// cone opens at the loft height, so the hold ends exactly as the dive
    /// begins, and a shot shorter than the run climbs straight at the lofted
    /// point above its target.
    pub fn aim(&self, position: V3) -> V3 {
        match self.loft {
            Some(loft) if self.supported => {
                let toward = self.point.xy() - position.xy();
                let across = toward.length();
                if position.z - self.point.z >= loft.slope * across {
                    return self.point;
                }
                let run = (loft.height_m / loft.slope).min(across);
                let ahead = if across > 0.0 {
                    position.xy() + toward * (run / across)
                } else {
                    self.point.xy()
                };
                v3(ahead.x, ahead.y, self.point.z + loft.height_m)
            }
            _ => self.point,
        }
    }
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
    /// Shooter motion carried into the round at launch. Added to `velocity`
    /// exactly once when the projectile enters the store.
    pub carrier: V3,
    /// Initial velocity relative to the shooter. The store adds `carrier`.
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
    /// The seeker was diverted to a flare; its point is a physical burst aim
    /// point and no longer follows the aircraft target.
    pub decoy: Option<Decoy>,
}

/// Physical flare tracked by a diverted seeker, independent of the aircraft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decoy {
    pub position: V3,
    pub velocity: V3,
    /// The seeker bursts on the flare once this close (`guided.flare_meet_m`).
    pub meet_m: f64,
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
    /// Reach of an authored anti-missile round, in metres. The default keeps
    /// the projectile store unaware of weapon catalogs and preserves the
    /// existing active-protection-only behaviour.
    fn projectile_interceptor_reach(&self, _projectile: ProjectileId) -> f64 {
        0.0
    }
    /// A fused round may burst at a swept standoff from the hull of the unit
    /// it was fired at.
    fn target_fuse(&self, _projectile: ProjectileId) -> Option<(UnitId, f64)> {
        None
    }
    /// Resolve a projectile-to-projectile interception. Returning true
    /// consumes both rounds; ordering is stable by projectile id.
    fn intercept_projectile(&mut self, _interceptor: ProjectileId, _target: ProjectileId) -> bool {
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

    /// The world's gravity, before any round's own share of it.
    pub fn gravity(&self) -> V3 {
        self.config.gravity
    }

    /// Rounds launched now fly from the start of the next advance.
    pub fn launch(&mut self, launch: Launch) -> ProjectileId {
        let id = ProjectileId(self.next_id);
        self.next_id += 1;
        self.active.push(Projectile {
            id,
            position: launch.origin,
            velocity: launch.velocity + launch.carrier,
            gravity_scale: launch.gravity_scale,
            age_s: 0.0,
            lifetime_s: launch.lifetime_s,
            suppression_radius_m: launch.suppression_radius_m,
            shooter: launch.shooter,
            guidance: launch.guidance,
            motor: launch.motor,
            fall: launch.fall,
            bounces: 0,
            decoy: None,
        });
        id
    }

    /// Renew a supported round's commanded point; a released one ignores it.
    pub fn steer(&mut self, id: ProjectileId, point: V3) {
        if let Some(g) = self.guidance_mut(id).filter(|g| g.supported) {
            g.point = point;
        }
    }

    /// Renew a seeker command after launch. Unlike launcher support, this
    /// remains valid while the firing aircraft moves or loses optical LOS.
    pub fn steer_seeker(&mut self, id: ProjectileId, point: V3) {
        if let Some(g) = self.guidance_mut(id) {
            g.point = point;
        }
    }

    /// Divert this seeker to one physical flare. The flare keeps its release
    /// velocity and falls under gravity; no unit pose is followed afterwards.
    pub fn decoy_seeker(&mut self, id: ProjectileId, decoy: Decoy) {
        if let Some(p) = self.active.iter_mut().find(|p| p.id == id) {
            p.decoy = Some(decoy);
            if let Some(g) = p.guidance.as_mut() {
                g.point = decoy.position;
                g.supported = false;
            }
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
                // Hashed only when present: a battle firing no top-attack
                // round keeps its digest.
                if let Some(loft) = g.loft {
                    d.u64(0x4c4f4654).f64(loft.height_m).f64(loft.slope);
                }
            }
            if let Some(decoy) = p.decoy {
                d.u64(0x464c415245)
                    .f64(decoy.position.x)
                    .f64(decoy.position.y)
                    .f64(decoy.position.z)
                    .f64(decoy.velocity.x)
                    .f64(decoy.velocity.y)
                    .f64(decoy.velocity.z);
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

/// Where a seeker at `from`, flying at `from_velocity`, aims to meet a
/// target at `at` flying at `velocity`: ahead of the target by the time the
/// range takes to close at its present rate. A target not closing is
/// aimed at where it is.
pub fn lead(from: V3, from_velocity: V3, at: V3, velocity: V3) -> V3 {
    let range = at - from;
    let distance = range.length();
    let closing = -range.dot(velocity - from_velocity) / distance.max(1e-9);
    if distance > 0.0 && closing > 0.0 {
        at + velocity * (distance / closing)
    } else {
        at
    }
}
