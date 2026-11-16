//! A wheeled vehicle working its way out of a tight spot (Q29). When the
//! forward turn onto its way is blocked, or its next waypoint lies inside
//! its turning circle, a driver looks at the room around him and shunts:
//! forward and back, at full lock or straight, as the bodies and vehicles
//! about him allow. So the vehicle searches short arcs both ways from
//! where it stands, against the bodies it cannot shove and the vehicles
//! where they stand, for the quickest legs that leave it on its route
//! facing along it, past the waypoint it could not turn to (or at the
//! route's end). Reverse is slower and every change of gear costs a stop,
//! so it backs only where backing is quicker: out of a pocket the way it
//! came, or a few metres to come round. The search is bounded; finding no
//! way, the vehicle holds and looks again a little later.
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use contract::map::MoverClass;

use crate::math::{v2, wrap_angle, Obb2, V2};
use crate::units::Unit;
use crate::world::WorldGeometry;

use super::drive::{self, Motion};

/// Each arc the search tries is this long.
const ARC_M: f64 = 1.0;
/// A hull is checked for bodies this often along an arc: a 10 m hull at
/// full lock swings its corners well under a trunk's width between checks.
const SAMPLE_M: f64 = 0.25;
/// A change of gear costs a stop and a start, as this many metres forward.
const GEAR_CHANGE_M: f64 = 4.0;
/// A shunt ends this close to its route, sideways ...
const ON_ROUTE_M: f64 = 1.5;
/// ... and facing along it this nearly.
const ALONG_RAD: f64 = 0.45;
/// It looks this many turning circles plus hull lengths along its route
/// for where to rejoin it.
const REACH_TURNS: f64 = 2.0;
/// Poses tried before a search gives up.
const MAX_POSES: usize = 4000;
/// Poses are told apart this finely.
const CELL_M: f64 = 1.0;
const CELL_RAD: f64 = std::f64::consts::TAU / 48.0;
/// The search leans this hard toward its goal: it takes a way a little
/// dearer than the quickest to find one at all in good time.
const GREED: f64 = 2.0;
/// Finding no way, it looks again after this long.
const RETRY_S: f64 = 5.0;

/// One leg of a shunt: driven in `gear` (+1 forward, -1 back), turning
/// `turn` radians per metre, for `length` metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leg {
    pub gear: f64,
    pub turn: f64,
    pub length: f64,
}

/// A vehicle's worked-out way out of a tight spot.
#[derive(Clone, Debug, PartialEq)]
pub struct Shunt {
    /// The legs still to drive; none when no way was found.
    pub legs: Vec<Leg>,
    /// Metres of the first leg driven.
    pub driven_m: f64,
    /// Where it stood when it worked the shunt out: its route runs on from there.
    pub from: V2,
    /// When it worked the shunt out.
    pub tick: u64,
}

impl Shunt {
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.legs.len() as u64);
        for leg in &self.legs {
            d.f64(leg.gear).f64(leg.turn).f64(leg.length);
        }
        d.f64(self.driven_m)
            .f64(self.from.x)
            .f64(self.from.y)
            .u64(self.tick);
    }
}

/// The route from `from` as one line, and where on it a shunt may end.
struct Way {
    points: Vec<V2>,
    /// A shunt ends at least this far along ...
    need: f64,
    /// ... and no further than this is looked at.
    upto: f64,
    /// The route ends within reach: a shunt may end anywhere near its end.
    end: Option<V2>,
}

impl Way {
    fn new(from: V2, route: &[V2], reach: f64) -> Self {
        let points: Vec<V2> = std::iter::once(from).chain(route.iter().copied()).collect();
        let total: f64 = points.windows(2).map(|w| (w[1] - w[0]).length()).sum();
        // Past the waypoint it could not turn to; one far off it may
        // come round to anywhere on its way to.
        let first = (route[0] - from).length();
        let need = if first + ON_ROUTE_M <= reach {
            first + ON_ROUTE_M
        } else {
            0.0
        };
        let end = (need >= total - ON_ROUTE_M).then(|| points[points.len() - 1]);
        Way {
            points,
            need,
            upto: need + reach,
            end,
        }
    }

    /// The nearest point on the line to `p` up to `upto`: how far along,
    /// how far off, and which way the line runs there.
    fn locate(&self, p: V2) -> (f64, f64, f64) {
        let mut best = (0.0, f64::INFINITY, 0.0);
        let mut s = 0.0;
        for w in self.points.windows(2) {
            let ab = w[1] - w[0];
            let len = ab.length();
            if len < 1e-9 {
                continue;
            }
            let t = ((p - w[0]).dot(ab) / (len * len)).clamp(0.0, 1.0);
            let off = (w[0] + ab * t - p).length();
            if off < best.1 {
                best = (s + t * len, off, libm::atan2(ab.y, ab.x));
            }
            s += len;
            if s > self.upto {
                break;
            }
        }
        best
    }
}

/// What stops a hull in the search: the bodies it cannot shove and the
/// vehicles where they stand (none it already touches: it may always draw
/// away from those), and ground it cannot cross.
struct Room<'a> {
    world: &'a WorldGeometry,
    solids: Vec<(Obb2, f64)>,
    half: V2,
    corners: bool,
}

impl Room<'_> {
    fn free(&self, at: V2, yaw: f64) -> bool {
        let hull = Obb2 {
            center: at,
            yaw,
            half: self.half,
        };
        let reach = self.half.length();
        !self
            .solids
            .iter()
            .any(|(body, r)| (body.center - at).length() < reach + r && hull.overlaps(body))
    }

    /// Whether the hull stands on ground it can cross: its corners too,
    /// never one over the water beside a bridge (unless it already stands
    /// so, when its centre is what the follower holds it to).
    fn on_ground(&self, at: V2, yaw: f64) -> bool {
        let hull = Obb2 {
            center: at,
            yaw,
            half: self.half,
        };
        let crossable = |p: V2| {
            self.world
                .surface_at(p.x, p.y)
                .is_some_and(|s| s.traversable)
        };
        crossable(at) && (!self.corners || hull.corners().into_iter().all(crossable))
    }

    /// The poses of an arc from `at`/`yaw` in `gear` turning `turn`, if
    /// every one is free.
    fn arc(&self, at: V2, yaw: f64, gear: f64, turn: f64) -> Option<(V2, f64)> {
        let pieces = (ARC_M / SAMPLE_M).ceil() as usize;
        let piece = ARC_M / pieces as f64;
        let (mut at, mut yaw) = (at, yaw);
        for _ in 0..pieces {
            let dyaw = turn * piece;
            at = at + drive::dir(drive::travel(yaw, gear) + dyaw / 2.0) * piece;
            yaw += dyaw;
            if !self.free(at, yaw) {
                return None;
            }
        }
        // Ground it cannot cross (water, a cliff) is far coarser than a body.
        self.on_ground(at, yaw).then_some((at, yaw))
    }
}

struct Pose {
    at: V2,
    yaw: f64,
    gear: f64,
    turn: f64,
    cost: f64,
    parent: usize,
}

/// Work out `unit`'s shunt onto its route from where it stands, with the
/// other vehicles in `traffic` and the bodies in `held` (a body it could
/// shove but whose shove jammed) as solid.
pub fn search(
    world: &WorldGeometry,
    unit: &Unit,
    traffic: &[Option<Obb2>],
    held: &[Obb2],
    tick: u64,
) -> Shunt {
    let none = |from| Shunt {
        legs: Vec::new(),
        driven_m: 0.0,
        from,
        tick,
    };
    let here = unit.position.xy();
    let (Some(drive), Some(route), Some(hull)) = (
        unit.ground()
            .drive
            .filter(|d| !d.tracked && d.radius_m > 0.0),
        unit.route.as_deref().filter(|r| !r.is_empty()),
        unit.ground_footprint(),
    ) else {
        return none(here);
    };
    let order = drive::gear(unit);
    let radius = drive.radius_m;
    let reach = REACH_TURNS * (radius + hull.half.x);
    let way = Way::new(here, route, reach);
    let push = unit.ground().push;
    let around = reach + way.need + hull.half.length();
    let touching = |body: &Obb2| hull.overlaps(body);
    let solids: Vec<(Obb2, f64)> = world
        .props_near(here, around)
        .into_iter()
        .filter(|p| p.blocks(MoverClass::Vehicle) && !push.pushes(p.body.weight_class))
        .map(|p| p.footprint())
        .chain(held.iter().copied())
        .chain(
            // Kept as far from as traffic keeps.
            traffic
                .iter()
                .enumerate()
                .filter(|&(id, _)| id != unit.id.0 as usize)
                .filter_map(|(_, h)| *h)
                .filter(|h| (h.center - here).length() < around + h.half.length())
                .map(|h| Obb2 {
                    half: h.half + v2(super::TRAFFIC_MARGIN_M, super::TRAFFIC_MARGIN_M),
                    ..h
                }),
        )
        .filter(|b| !touching(b))
        .map(|b| (b, b.half.length()))
        .collect();
    let mut room = Room {
        world,
        solids,
        half: hull.half,
        corners: true,
    };
    room.corners = room.on_ground(here, unit.yaw);
    let backing = 1.0 / drive.reverse_speed_factor.max(0.05);
    // Where the follower carries on from: the route's end just ahead, or
    // on the route facing along it with a hull's length clear ahead.
    let ahead = (2.0 * hull.half.x / ARC_M).ceil() as usize;
    let clear_ahead = |at: V2, yaw: f64| {
        let mut pose = Some((at, yaw));
        for _ in 0..ahead {
            pose = pose.and_then(|(at, yaw)| room.arc(at, yaw, order, 0.0));
        }
        pose.is_some()
    };
    let goal = |at: V2, yaw: f64| -> bool {
        if let Some(end) = way.end {
            let to = end - at;
            let off = wrap_angle(libm::atan2(to.y, to.x) - drive::travel(yaw, order)).abs();
            if to.length() <= drive.feel.abeam_m && off <= ALONG_RAD {
                return true;
            }
        }
        let (s, off, along) = way.locate(at);
        s >= way.need
            && off <= ON_ROUTE_M
            && wrap_angle(drive::travel(yaw, order) - along).abs() <= ALONG_RAD
            && clear_ahead(at, yaw)
    };
    // How far it has yet to go at least: onto the route and along it to
    // where it may end, and round to face along it (no tighter than its
    // turning circle).
    let estimate = |at: V2, yaw: f64| -> f64 {
        if let Some(end) = way.end {
            return ((end - at).length() - drive.feel.abeam_m).max(0.0);
        }
        let (s, off, along) = way.locate(at);
        let go = (off - ON_ROUTE_M).max(0.0) + (way.need - s).max(0.0);
        let turn = (wrap_angle(drive::travel(yaw, order) - along).abs() - ALONG_RAD).max(0.0);
        go.max(radius * turn)
    };
    let key = |at: V2, yaw: f64, gear: f64| {
        (
            libm::floor(at.x / CELL_M) as i64,
            libm::floor(at.y / CELL_M) as i64,
            libm::floor(yaw.rem_euclid(std::f64::consts::TAU) / CELL_RAD) as i64,
            gear as i64,
        )
    };
    let moving = unit.drive_speed_mps;
    let start_gear = if moving.abs() > 0.1 {
        moving.signum()
    } else {
        0.0
    };
    let mut poses = vec![Pose {
        at: here,
        yaw: unit.yaw,
        gear: start_gear,
        turn: 0.0,
        cost: 0.0,
        parent: usize::MAX,
    }];
    let mut open = BinaryHeap::new();
    open.push(Reverse((ordered(GREED * estimate(here, unit.yaw)), 0usize)));
    let mut seen = BTreeSet::new();
    let mut found = None;
    while let Some(Reverse((_, i))) = open.pop() {
        let (at, yaw, gear, cost) = (poses[i].at, poses[i].yaw, poses[i].gear, poses[i].cost);
        if !seen.insert(key(at, yaw, gear)) {
            continue;
        }
        if i > 0 && goal(at, yaw) {
            found = Some(i);
            break;
        }
        if seen.len() >= MAX_POSES {
            break;
        }
        for next_gear in [1.0, -1.0] {
            for turn in [1.0 / radius, 0.0, -1.0 / radius] {
                let Some((to, to_yaw)) = room.arc(at, yaw, next_gear, turn) else {
                    continue;
                };
                if (to - here).length() > around || seen.contains(&key(to, to_yaw, next_gear)) {
                    continue;
                }
                let change = if gear != 0.0 && gear != next_gear {
                    GEAR_CHANGE_M
                } else {
                    0.0
                };
                let step = if next_gear < 0.0 { backing } else { 1.0 } * ARC_M;
                let to_cost = cost + step + change;
                poses.push(Pose {
                    at: to,
                    yaw: to_yaw,
                    gear: next_gear,
                    turn,
                    cost: to_cost,
                    parent: i,
                });
                let f = to_cost + GREED * estimate(to, to_yaw);
                open.push(Reverse((ordered(f), poses.len() - 1)));
            }
        }
    }
    let Some(mut i) = found else {
        return none(here);
    };
    let mut arcs = Vec::new();
    while i != 0 {
        arcs.push((poses[i].gear, poses[i].turn));
        i = poses[i].parent;
    }
    let mut legs: Vec<Leg> = Vec::new();
    for (gear, turn) in arcs.into_iter().rev() {
        match legs.last_mut() {
            Some(leg) if leg.gear == gear && leg.turn == turn => leg.length += ARC_M,
            _ => legs.push(Leg {
                gear,
                turn,
                length: ARC_M,
            }),
        }
    }
    Shunt {
        legs,
        driven_m: 0.0,
        from: here,
        tick,
    }
}

/// An f64 cost the heap can order (costs are never negative).
fn ordered(f: f64) -> u64 {
    f.max(0.0).to_bits()
}

/// This tick's motion along `unit`'s shunt, at surface speed `speed`: a
/// standstill on the tick it finishes (its route picked up where it
/// stands) and while no way was found; `None` with no shunt, or when it is
/// time to look again.
pub(super) fn drive_on(
    world: &WorldGeometry,
    unit: &mut Unit,
    speed: f64,
    dt: f64,
    tick: u64,
) -> Option<Motion> {
    let shunt = unit.shunt.as_mut()?;
    if shunt.legs.is_empty() {
        if (tick.saturating_sub(shunt.tick) as f64) < RETRY_S * (1.0 / dt) {
            return Some(hold(unit));
        }
        unit.shunt = None;
        return None;
    }
    while shunt
        .legs
        .first()
        .is_some_and(|leg| leg.length - shunt.driven_m <= 1e-6)
    {
        shunt.legs.remove(0);
        shunt.driven_m = 0.0;
    }
    let Some(leg) = shunt.legs.first().copied() else {
        // Driven: it picks its route up where it stands, and steers along
        // it from the next tick.
        let from = shunt.from;
        unit.shunt = None;
        if let Some(route) = unit.route.as_mut() {
            super::rejoin(unit.position.xy(), from, route);
        }
        return Some(hold(unit));
    };
    let remaining = leg.length - shunt.driven_m;
    // It stops where the gear changes, or the shunt ends.
    let to_stop = remaining
        + shunt.legs[1..]
            .iter()
            .take_while(|next| next.gear == leg.gear)
            .map(|next| next.length)
            .sum::<f64>();
    let d = unit.ground().drive.expect("a vehicle has a drive");
    let braking = unit.ground().road_mps / d.feel.braking_s;
    let desired = drive::speed_in(&d, leg.gear, speed)
        .min(d.radius_m * d.turn_rad_s)
        .min((2.0 * braking * to_stop).sqrt());
    let v = drive::accelerate(unit, desired, leg.gear, dt);
    let step = (v * dt).min(remaining);
    let dyaw = leg.turn * step;
    let motion = Motion {
        yaw: unit.yaw + dyaw,
        heading: drive::dir(drive::travel(unit.yaw, leg.gear) + dyaw / 2.0),
        step,
        backwards: leg.gear < 0.0,
        making_space: true,
        blocker: None,
    };
    if drive::lands(world, unit, &motion, leg.gear).is_some() {
        // Something stands where the shunt meant to go: work it out again.
        unit.shunt = None;
        return Some(hold(unit));
    }
    Some(motion)
}

fn hold(unit: &Unit) -> Motion {
    Motion {
        yaw: unit.yaw,
        heading: drive::travel_heading(unit),
        step: 0.0,
        backwards: false,
        making_space: false,
        blocker: None,
    }
}
