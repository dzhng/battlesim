//! A downed aircraft's fall (D3, D31). It keeps the momentum it died with,
//! spins, and falls under the world's gravity. A body it cannot break (an
//! immovable one: a building, a wall) turns it away: it glances off a side,
//! and slides off a roof it lands on. Where it meets the ground it bursts as
//! the `helicopter_crash` row, felling the trees and hurting what lies under
//! it, and leaves its wreck on the nearest ground no live hull stands on. The
//! falling airframe is no body: rounds pass through it.

use crate::damage::LethalSource;
use crate::digest::Digest;
use crate::math::{v2, v3, Obb2, V2, V3};
use crate::world::WorldGeometry;
use contract::ids::{Side, UnitId};
use contract::scenario::WeightClass;

/// The weapon row a crash bursts as.
pub const CRASH_WEAPON: &str = "helicopter_crash";
/// How fast a downed airframe spins, radians a second.
const SPIN_RAD_S: f64 = std::f64::consts::PI;
/// The share of its speed into a wall an airframe keeps, glancing off it.
const RESTITUTION: f64 = 0.3;
/// How fast it slides off a roof it fell on.
const SLIDE_MPS: f64 = 4.0;
/// The wreck's search for clear ground: rings this far apart, this far out.
const CLEAR_STEP_M: f64 = 1.0;
const CLEAR_REACH_M: f64 = 30.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Crash {
    pub unit: UnitId,
    pub position: V3,
    pub velocity: V3,
    pub yaw: f64,
    pub spin_rad_s: f64,
    /// Who brought it down: the kill, and every casualty of its crash.
    pub source: Option<LethalSource>,
    /// The sides that saw it go down, and so learn where its wreck lies.
    pub knowing: Vec<Side>,
}

impl Crash {
    /// The fall of an airframe that died at `position` flying `velocity`.
    pub fn new(
        unit: UnitId,
        position: V3,
        velocity: V2,
        yaw: f64,
        source: Option<LethalSource>,
        knowing: Vec<Side>,
    ) -> Self {
        // Which way it spins is fixed by the unit, so a replay matches.
        let spin = if unit.0.is_multiple_of(2) {
            SPIN_RAD_S
        } else {
            -SPIN_RAD_S
        };
        Crash {
            unit,
            position,
            velocity: velocity.with_z(0.0),
            yaw,
            spin_rad_s: spin,
            source,
            knowing,
        }
    }

    /// Fall one tick of `dt` under `gravity`. Returns whether it met the
    /// ground; its position is then where it struck.
    pub fn fall(&mut self, world: &WorldGeometry, gravity: V3, half: V3, dt: f64) -> bool {
        let was = self.position;
        self.velocity = self.velocity + gravity * dt;
        self.position = self.position + self.velocity * dt;
        self.yaw = crate::math::wrap_angle(self.yaw + self.spin_rad_s * dt);
        let reach = half.y;
        for prop in world.props_near(self.position.xy(), reach) {
            let body = prop.footprint();
            let here = self.position.xy();
            if prop.body.weight_class != WeightClass::Immovable
                || self.position.z >= prop.top_z()
                || body.distance(here) > reach
            {
                continue;
            }
            let out = body.push_out(here, reach);
            let normal = (out - here).normalized();
            if was.z >= prop.top_z() {
                // On a roof: it rests on it, and slides off the nearer edge.
                self.position.z = prop.top_z();
                let away = if normal.length() > 0.0 {
                    normal
                } else {
                    (here - prop.center).normalized()
                };
                self.velocity = (away * SLIDE_MPS).with_z(0.0);
            } else {
                // Into a side: it glances off.
                let flat = self.velocity.xy();
                let into = flat.dot(normal).min(0.0);
                let glance = (flat - normal * (2.0 * into)) * RESTITUTION;
                self.velocity = glance.with_z(self.velocity.z);
                self.position = out.with_z(self.position.z);
            }
        }
        let ground = world
            .height_at(self.position.x, self.position.y)
            .unwrap_or(0.0);
        if self.position.z <= ground {
            self.position.z = ground;
            return true;
        }
        false
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.unit.0 as u64)
            .f64(self.position.x)
            .f64(self.position.y)
            .f64(self.position.z)
            .f64(self.velocity.x)
            .f64(self.velocity.y)
            .f64(self.velocity.z)
            .f64(self.yaw);
    }
}

/// Where a wreck of footprint `half` at `yaw` comes to rest near `at`: the
/// nearest point, ring by ring, where it lies clear of `hulls` and of every
/// immovable body; `at` itself when there is none in reach.
pub fn resting_place(world: &WorldGeometry, at: V2, yaw: f64, half: V2, hulls: &[Obb2]) -> V2 {
    let clear = |center: V2| {
        let wreck = Obb2 { center, yaw, half };
        !hulls.iter().any(|h| wreck.overlaps(h))
            && world.props_near(center, half.length()).iter().all(|p| {
                p.body.weight_class != WeightClass::Immovable || !wreck.overlaps(&p.footprint())
            })
    };
    let rings = (CLEAR_REACH_M / CLEAR_STEP_M) as i32;
    for ring in 0..=rings {
        let r = ring as f64 * CLEAR_STEP_M;
        let points = if ring == 0 { 1 } else { 8 * ring };
        for k in 0..points {
            let angle = std::f64::consts::TAU * k as f64 / points as f64;
            let p = at + v2(libm::cos(angle), libm::sin(angle)) * r;
            if clear(p) {
                return p;
            }
        }
    }
    at
}

/// The point a crash bursts at: just off the ground it struck.
pub fn burst_point(position: V3) -> V3 {
    position + v3(0.0, 0.0, 0.5)
}
