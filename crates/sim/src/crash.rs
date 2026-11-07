//! A downed aircraft's fall (D3, D31). It keeps the momentum it died with,
//! spins, and falls under the world's gravity. A body it cannot break (an
//! immovable one: a building, a wall) stops it: an airframe crumples on
//! masonry rather than springing off it, so striking a side it keeps almost
//! none of its speed into the face, and its skin scraping the face takes
//! speed along it too (an impulse with Coulomb friction); on a roof it lands
//! and slides off. A hard strike is published where it struck, once; a
//! wreck resting or grinding against a face strikes nothing. A strike on the
//! tick it meets the ground is not published: the crash ends that tick, and
//! its burst on the ground is its only fireball either way (a strike throws
//! sparks and dust, never fire). Every downed aircraft falls this way, a
//! fixed-wing jet as a helicopter does; a live aircraft and a movable prop
//! never strike. Where it meets
//! the ground it bursts as the `helicopter_crash` row, felling the trees and
//! hurting what lies under it, and leaves its wreck on the nearest ground no
//! live hull stands on. The falling airframe is no body: rounds pass
//! through it.

use crate::damage::LethalSource;
use crate::digest::Digest;
use crate::math::{v2, v3, Obb2, V2, V3};
use crate::world::WorldGeometry;
use contract::catalog::TypeIndex;
use contract::ids::{Side, UnitId};
use contract::observation::{AirframeStrike, FallingAirframe};
use contract::scenario::WeightClass;

/// The weapon row a crash bursts as.
pub const CRASH_WEAPON: &str = "helicopter_crash";
/// How fast a downed airframe spins, radians a second.
const SPIN_RAD_S: f64 = std::f64::consts::PI;
/// The share of its speed into a face an airframe gives back: a crumpling
/// fuselage on masonry, nearly a dead stop.
const RESTITUTION: f64 = 0.1;
/// Its skin's friction on the face it strikes: the speed along the face it
/// loses per m/s of the strike's impulse into it.
const FRICTION: f64 = 0.5;
/// Closing on a face slower than this, m/s, is contact, not a strike.
const STRIKE_MPS: f64 = 4.0;
/// After a strike it strikes again no sooner than this, s: the parts of one
/// building it falls against make one strike, not one each.
const STRIKE_QUIET_S: f64 = 0.25;
/// Its attitude in the fall: the nose drop and the lean into the spin it
/// reaches by this sink rate (D3's war-film tumble, not an aerodynamic one).
const TIP_SINK_MPS: f64 = 15.0;
const MAX_PITCH_RAD: f64 = 0.35;
const MAX_ROLL_RAD: f64 = 0.25;
/// How far clear of a wall it is put, glancing off it.
const CLEAR_M: f64 = 1e-6;
/// An edge whose ends lie within this of each other along the contact's
/// normal, m, meets the other body flat, not by one corner.
const FLAT_M: f64 = 0.25;
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
    /// The sides that saw it go down, and so see it fall and learn where its
    /// wreck lies, each with the id it knew the airframe by.
    pub knowing: Vec<(Side, u32)>,
    /// The structure it struck this tick, if any. Presentation only: it
    /// changes nothing it does, so it adds nothing to the digest.
    pub strike: Option<Strike>,
    /// How long until it may strike again, s; presentation only.
    quiet_s: f64,
}

/// Where a falling airframe struck a face, which way the face looks, and how
/// fast it closed on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub point: V3,
    pub normal: V3,
    pub speed_mps: f64,
}

/// What one tick of a crash's fall came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fall {
    Falling,
    /// It met the ground and bursts there.
    Landed,
    /// It left the map; nothing is tracked past the edge (D31).
    LeftMap,
}

impl Crash {
    /// The fall of an airframe that died at `position` flying `velocity`.
    pub fn new(
        unit: UnitId,
        position: V3,
        velocity: V2,
        yaw: f64,
        source: Option<LethalSource>,
        knowing: Vec<(Side, u32)>,
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
            strike: None,
            quiet_s: 0.0,
        }
    }

    /// Fall one tick of `dt` under `gravity`. When it lands, its position is
    /// where it struck; when it leaves the map it just ends (D31).
    pub fn fall(&mut self, world: &WorldGeometry, gravity: V3, half: V3, dt: f64) -> Fall {
        let was = self.position;
        self.velocity = self.velocity + gravity * dt;
        self.position = self.position + self.velocity * dt;
        self.yaw = crate::math::wrap_angle(self.yaw + self.spin_rad_s * dt);
        self.strike = None;
        self.quiet_s = (self.quiet_s - dt).max(0.0);
        // The hardest face it closes on this tick, struck hard enough.
        let mut hardest: Option<Strike> = None;
        // Its hull's footprint at its yaw: nose and tail strike as its sides do.
        let reach = half.xy().length();
        for prop in world.props_near(self.position.xy(), reach) {
            let body = prop.footprint();
            let here = self.position.xy();
            let hull = Obb2 {
                center: here,
                yaw: self.yaw,
                half: half.xy(),
            };
            // Bridge decks are the support surface, rather than a roof that
            // catches a falling airframe at the slab's upper edge.
            let is_bridge_deck = world.surface_at(here.x, here.y).is_some_and(|surface| {
                surface.kind == crate::world::SurfaceKind::Bridge
                    && prop.base_z < surface.z
                    && (prop.top_z() - surface.z).abs() < 1e-9
            });
            if prop.body.weight_class != WeightClass::Immovable
                || is_bridge_deck
                || self.position.z >= prop.top_z()
            {
                continue;
            }
            let Some(out) = body.separation(&hull) else {
                continue;
            };
            let normal = out.normalized();
            let (closing, point, face) = if was.z >= prop.top_z() {
                // On a roof: it lands on it, and slides off the nearer edge.
                let closing = -self.velocity.z;
                self.position.z = prop.top_z();
                self.velocity = (normal * SLIDE_MPS).with_z(0.0);
                let point = body.closest(here).with_z(prop.top_z());
                (closing, point, v3(0.0, 0.0, 1.0))
            } else {
                // Into a side: it crumples on the face and scrapes along it.
                let face = normal.with_z(0.0);
                let closing = -self.velocity.dot(face);
                if closing > 0.0 {
                    self.velocity = struck(self.velocity, face, closing);
                }
                let center = here + out + normal * CLEAR_M;
                self.position = center.with_z(self.position.z);
                let hull = Obb2 {
                    center,
                    yaw: self.yaw,
                    half: half.xy(),
                };
                // Halfway up its hull, on the face.
                let z = (self.position.z + half.z).clamp(prop.base_z, prop.top_z());
                (closing, contact(&hull, &body, normal).with_z(z), face)
            };
            if closing >= STRIKE_MPS && hardest.is_none_or(|h| closing > h.speed_mps) {
                hardest = Some(Strike {
                    point,
                    normal: face,
                    speed_mps: closing,
                });
            }
        }
        if self.quiet_s == 0.0 && hardest.is_some() {
            self.strike = hardest;
            self.quiet_s = STRIKE_QUIET_S;
        }
        if self.position.x < 0.0
            || self.position.x > world.width()
            || self.position.y < 0.0
            || self.position.y > world.depth()
        {
            return Fall::LeftMap;
        }
        // A deck catches the airframe only when it was below the airframe's
        // previous position; otherwise use the ground beneath an underflight.
        let ground = match world.surface_at(self.position.x, self.position.y) {
            Some(surface)
                if surface.kind != crate::world::SurfaceKind::Bridge || surface.z <= was.z =>
            {
                surface.z
            }
            _ => world
                .height_at(self.position.x, self.position.y)
                .unwrap_or(0.0),
        };
        if self.position.z <= ground {
            self.position.z = ground;
            return Fall::Landed;
        }
        Fall::Falling
    }

    /// How it hangs in the air, (pitch, roll): level the moment it dies, its
    /// nose dropping and its body leaning into its spin as its sink builds.
    /// Derived from its fall, so it adds nothing to the digest.
    pub fn attitude(&self) -> (f64, f64) {
        let tip = (-self.velocity.z / TIP_SINK_MPS).clamp(0.0, 1.0);
        (
            -MAX_PITCH_RAD * tip,
            MAX_ROLL_RAD * tip * self.spin_rad_s.signum(),
        )
    }

    /// How `side` sees it fall this tick, if that side saw it go down; it
    /// flew for `own` and is a `kind`.
    pub fn seen_by(&self, side: Side, own: Side, kind: TypeIndex) -> Option<FallingAirframe> {
        let &(_, id) = self.knowing.iter().find(|(s, _)| *s == side)?;
        let (pitch, roll) = self.attitude();
        let xyz = |v: V3| [v.x, v.y, v.z];
        Some(FallingAirframe {
            id,
            own: own == side,
            kind,
            position: xyz(self.position),
            yaw: self.yaw,
            pitch,
            roll,
            strike: self.strike.map(|s| AirframeStrike {
                point: xyz(s.point),
                normal: xyz(s.normal),
                speed_mps: s.speed_mps,
            }),
        })
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
        match self.source {
            Some(s) => d.u64(1).u64(s.unit.0 as u64).u64(s.side.index() as u64),
            None => d.u64(0),
        };
        d.u64(self.knowing.len() as u64);
        for (side, id) in &self.knowing {
            d.u64(side.index() as u64).u64(*id as u64);
        }
    }
}

/// Where `hull`, just pushed clear of `body` along `normal`, meets it: the
/// body's corner where that pokes into the hull's side; otherwise the hull's
/// leading corner, or the middle of its leading edge where it lies flat
/// against the face, kept on the body's outline, so a long hull overhanging
/// a short wall meets it on the wall, not out past its end.
fn contact(hull: &Obb2, body: &Obb2, normal: V2) -> V2 {
    let hull_lead = leading(hull, -normal);
    if let ([corner], [_, _]) = (leading(body, normal).as_slice(), hull_lead.as_slice()) {
        return *corner;
    }
    let sum = (hull_lead.iter()).fold(v2(0.0, 0.0), |sum, &c| sum + body.closest(c));
    sum * (1.0 / hull_lead.len() as f64)
}

/// The corners of `r` furthest along `dir`: one, or both ends of an edge
/// that lies square to it.
fn leading(r: &Obb2, dir: V2) -> Vec<V2> {
    let corners = r.corners();
    let lead = (corners.iter().map(|c| c.dot(dir))).fold(f64::NEG_INFINITY, f64::max);
    (corners.into_iter())
        .filter(|c| c.dot(dir) >= lead - FLAT_M)
        .collect()
}

/// `velocity` after striking a face of outward unit normal `face`, closing on
/// it at `closing` m/s: the impulse into the face stops all but `RESTITUTION`
/// of that speed, and friction takes `FRICTION` of the impulse off its speed
/// along the face (up and down it too), never reversing it.
fn struck(velocity: V3, face: V3, closing: f64) -> V3 {
    let along = velocity + face * closing;
    let speed = along.length();
    let scrub = (FRICTION * (1.0 + RESTITUTION) * closing).min(speed);
    let kept = if speed > 0.0 {
        1.0 - scrub / speed
    } else {
        0.0
    };
    along * kept + face * (RESTITUTION * closing)
}

/// Where a wreck of footprint `half` at `yaw` comes to rest near `at`: the
/// nearest point, ring by ring, where it lies clear of `hulls` and of every
/// immovable body; `at` itself when there is none in reach.
pub fn resting_place(world: &WorldGeometry, at: V2, yaw: f64, half: V2, hulls: &[Obb2]) -> V2 {
    let clear = |center: V2| {
        let wreck = Obb2 { center, yaw, half };
        let deck_z = world
            .surface_at(center.x, center.y)
            .filter(|surface| surface.kind == crate::world::SurfaceKind::Bridge)
            .map(|surface| surface.z);
        !hulls.iter().any(|h| wreck.overlaps(h))
            && world.props_near(center, half.length()).iter().all(|p| {
                p.body.weight_class != WeightClass::Immovable
                    || !wreck.overlaps(&p.footprint())
                    || deck_z.is_some_and(|z| p.base_z < z && p.top_z() <= z + 1e-9)
            })
    };
    let reach = half.length();
    let on_map = |p: V2| {
        p.x >= reach && p.y >= reach && p.x <= world.width() - reach && p.y <= world.depth() - reach
    };
    let rings = (CLEAR_REACH_M / CLEAR_STEP_M) as i32;
    for ring in 0..=rings {
        let r = ring as f64 * CLEAR_STEP_M;
        let points = if ring == 0 { 1 } else { 8 * ring };
        for k in 0..points {
            let angle = std::f64::consts::TAU * k as f64 / points as f64;
            let p = at + v2(libm::cos(angle), libm::sin(angle)) * r;
            if on_map(p) && clear(p) {
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
