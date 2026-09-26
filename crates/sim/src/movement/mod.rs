//! Ground movement each tick: plan when needed, follow the route at surface
//! speed, and learn obstacles by running into them. A vehicle drives its route
//! as one hull, waits for other vehicles and never for soldiers. A squad
//! plans one corridor on the coarse grid; each soldier walks it as a body of
//! his own ([`soldier`]), with his own route on the exact bodies for the
//! final stretch ([`final_leg`]). Craters slow a driving vehicle here, in
//! integration only: planning never reads them.
use std::collections::BTreeSet;

use contract::ids::Tick;
use contract::map::{MoverClass, PropKind};
use contract::observation::MoveState;
use contract::scenario::InfantryMovementRules;

use crate::arrangement;
use crate::ground::GroundLayer;
use crate::math::{v2, wrap_angle, Obb2, V2};
use crate::navigation::{NavGrid, Plan};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

mod final_leg;
mod soldier;

pub use final_leg::{final_leg, FINE_CELL_M};
pub use soldier::{soldier_steer, Around, Corridor, Steer, Threat};

/// A pursuit goal this far from the planned one is replanned.
const GOAL_REPLAN_M: f64 = 5.0;
/// Seconds without closing on the next waypoint before a route is replanned.
const STALL_REPLAN_S: f64 = 2.0;
/// Progress shorter than this does not reset the stall watch.
const PROGRESS_EPSILON_M: f64 = 0.5;
/// Extra room vehicles keep from other bodies.
const TRAFFIC_MARGIN_M: f64 = 0.4;
/// A unit learns of an obstacle within this distance of its footprint.
const ENCOUNTER_RANGE_M: f64 = 2.0;
/// Vehicles turn in place beyond this heading error.
const TURN_IN_PLACE_DEG: f64 = 60.0;

/// What one side may plan with: the authored map plus the dynamic obstacles
/// its units have encountered. Hidden changes never reach this.
#[derive(Default)]
pub struct SideGeometry {
    pub known_dynamic: BTreeSet<PropId>,
    pub revision: u64,
    grid: Option<(u64, NavGrid)>,
    /// Route searches run for this side (the no-per-frame-search contract).
    pub searches: u64,
}

impl SideGeometry {
    pub fn learn(&mut self, prop: PropId) {
        if self.known_dynamic.insert(prop) {
            self.revision += 1;
        }
    }

    /// Whether this side plans with `prop`: authored with the map, learned,
    /// or a ruin. A ruin covers exactly the authored building it replaced,
    /// so every side plans with it whether or not it saw the collapse: a fall
    /// it never saw cannot open a route through the footprint.
    pub fn knows(&self, prop: &Prop, authored: PropId) -> bool {
        prop.id < authored || prop.kind == PropKind::Ruin || self.known_dynamic.contains(&prop.id)
    }

    /// The side's planning grid, rebuilt only when its knowledge changed.
    /// `soldier_radius` sizes infantry's gaps.
    pub fn grid(
        &mut self,
        world: &WorldGeometry,
        authored: PropId,
        soldier_radius: f64,
    ) -> &mut NavGrid {
        if self.grid.as_ref().is_none_or(|(r, _)| *r != self.revision) {
            let known = world.props().filter(|p| self.knows(p, authored));
            self.grid = Some((self.revision, NavGrid::build(world, known, soldier_radius)));
        }
        &mut self.grid.as_mut().unwrap().1
    }
}

pub struct MovementContext<'a> {
    pub world: &'a WorldGeometry,
    pub ground: &'a GroundLayer,
    pub ground_rules: &'a contract::scenario::GroundRules,
    /// Props with ids below this were authored with the map and are known to all.
    pub authored: PropId,
    pub tick: Tick,
    pub tick_hz: u32,
    pub vehicle_turn_deg_s: f64,
    /// Infantry speed lost at full suppression (P14).
    pub suppression_move_penalty: f64,
    pub infantry: &'a InfantryMovementRules,
    pub soldier_radius_m: f64,
    /// The battle's seed: arrangements are drawn from it (D1).
    pub seed: u64,
}

pub fn advance(ctx: &MovementContext, units: &mut [Unit], sides: &mut [SideGeometry; 2]) {
    let footprints: Vec<Option<Obb2>> = units
        .iter()
        .map(|u| u.hull_box().filter(|_| u.alive()))
        .collect();
    // The destroyed stay put; a wreck is an obstacle prop, not traffic.
    for unit in units.iter_mut().filter(|u| u.alive()) {
        plan_if_needed(ctx, unit, &mut sides[unit.side.index()], &footprints);
    }
    let hulls: Vec<Obb2> = footprints.iter().flatten().copied().collect();
    // Every live vehicle on the move, either side, as soldiers see it coming.
    let threats: Vec<Threat> = units
        .iter()
        .filter(|u| u.alive())
        .filter_map(|u| Threat::of(ctx, u))
        .collect();
    let mut crowd = soldier::Crowd::gather(units);
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        if units[i].is_vehicle() {
            step_vehicle(ctx, units, i, sides);
            soldier::shove(ctx, units, i, &mut crowd);
        } else {
            let unit = &mut units[i];
            let advancing = may_advance(ctx, unit);
            let side = &mut sides[unit.side.index()];
            soldier::step_squad(ctx, unit, i, side, &hulls, &threats, &mut crowd, advancing);
        }
    }
}

fn plan_if_needed(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &mut SideGeometry,
    footprints: &[Option<Obb2>],
) {
    let Some((goal, policy)) = unit.movement_goal() else {
        unit.route = None;
        unit.planned_goal = None;
        unit.state = MoveState::Idle;
        unit.blocker = None;
        for s in &mut unit.members {
            s.spot = None;
            s.path.clear();
        }
        return;
    };
    // A pursuit point that has moved on needs a new route.
    let goal_moved = unit
        .planned_goal
        .is_none_or(|g| (g - goal).length() > GOAL_REPLAN_M);
    let changed = unit.planned_revision != side.revision;
    let stall_ticks = (STALL_REPLAN_S * ctx.tick_hz as f64) as u64;
    let stalled = unit.route.is_some() && ctx.tick.saturating_sub(unit.progress.1) > stall_ticks;
    // Deadlocked vehicles: the lower-priority one (higher id) plans around
    // the one it waits for; the other keeps its route and proceeds when clear.
    let detour = match (unit.state, unit.blocker) {
        (MoveState::Waiting, Some(b)) if stalled && unit.is_vehicle() && unit.id > b => {
            footprints[b.0 as usize]
        }
        _ => None,
    };
    let needs = match (&unit.route, unit.state) {
        _ if goal_moved => true,
        (_, MoveState::RouteBlocked) => changed,
        (None, _) => true,
        (Some(route), _) => {
            let from = if unit.is_vehicle() {
                unit.position.xy()
            } else {
                unit.route_from
            };
            stalled
                || (changed
                    && !side
                        .grid(ctx.world, ctx.authored, ctx.soldier_radius_m)
                        .route_fits(from, route, &unit.mobility))
        }
    };
    unit.planned_revision = side.revision;
    if !needs {
        return;
    }
    side.searches += 1;
    let grid = side.grid(ctx.world, ctx.authored, ctx.soldier_radius_m);
    let from = if unit.is_vehicle() {
        unit.position.xy()
    } else {
        anchor(unit)
    };
    // A new goal draws a new arrangement; a replan toward the same one
    // keeps every soldier's spot.
    let new_goal = goal_moved;
    unit.planned_goal = Some(goal);
    unit.progress = (f64::INFINITY, ctx.tick);
    if let Some(blocker) = detour {
        // A failed detour keeps waiting and may try again after another stall.
        if let Plan::Route(route) =
            grid.plan_avoiding(from, goal, &unit.mobility, policy, &[blocker])
        {
            unit.route = Some(route);
        }
        return;
    }
    match grid.plan(from, goal, &unit.mobility, policy) {
        Plan::Route(route) => {
            if !unit.is_vehicle() {
                let end = *route.last().expect("a route ends somewhere");
                if new_goal {
                    spread_out(ctx, unit, side, end);
                } else {
                    keep_spots(ctx, unit, side, end);
                }
                unit.route_from = from;
                soldier::join(unit, from, &route);
            }
            unit.route = Some(route);
            unit.state = MoveState::Moving;
        }
        Plan::Blocked(_) => {
            unit.route = None;
            unit.state = MoveState::RouteBlocked;
        }
    }
}

/// Whether a unit with a route may translate this tick: an attack-move
/// holding to engage halts, and a deploying unit packs first (L01); neither
/// counts as a stall.
fn may_advance(ctx: &MovementContext, unit: &mut Unit) -> bool {
    if unit.state == MoveState::Halted && !unit.halted() {
        unit.state = MoveState::Moving;
    }
    if unit.route.as_ref().is_none_or(|r| r.is_empty()) {
        return false;
    }
    let held = if unit.halted() {
        MoveState::Halted
    } else if !unit.may_translate() {
        MoveState::Packing
    } else {
        return true;
    };
    unit.progress.1 = ctx.tick;
    unit.state = held;
    false
}

/// A route's end reached: a destination completes its move; a pursuit point
/// leaves the attack in place for its weapons.
fn arrive(unit: &mut Unit) {
    if unit.orders.front().is_some_and(|o| o.movement().is_some()) {
        unit.orders.pop_front();
    }
    unit.route = None;
    unit.planned_goal = None;
    unit.state = if unit.orders.is_empty() {
        MoveState::Idle
    } else {
        MoveState::Moving
    };
}

fn step_vehicle(
    ctx: &MovementContext,
    units: &mut [Unit],
    i: usize,
    sides: &mut [SideGeometry; 2],
) {
    let dt = 1.0 / ctx.tick_hz as f64;
    if !may_advance(ctx, &mut units[i]) {
        return;
    }
    let unit = &units[i];
    let target = unit.route.as_ref().unwrap()[0];
    let here = unit.position.xy();
    let to_target = target - here;
    let distance = to_target.length();
    let surface = ctx.world.surface_at(here.x, here.y);
    let speed = surface.map_or(0.0, |s| {
        unit.mobility.speed(
            s.kind == crate::world::SurfaceKind::Road
                || s.kind == crate::world::SurfaceKind::Bridge,
            s.forest,
            s.slope_deg,
        )
    });
    let desired_yaw = to_target.y.atan2(to_target.x);
    // Craters under the hull slow it slightly; never to a stop (Q8).
    let speed = speed * ctx.ground.vehicle_speed(here.x, here.y, ctx.ground_rules);
    let error = wrap_angle(desired_yaw - unit.yaw);
    let max_turn = ctx.vehicle_turn_deg_s.to_radians() * dt;
    let yaw = unit.yaw + error.clamp(-max_turn, max_turn);
    let remaining = wrap_angle(desired_yaw - yaw).abs();
    let factor = if remaining > TURN_IN_PLACE_DEG.to_radians() {
        0.0
    } else {
        remaining.cos()
    };
    let speed = speed * factor;
    let step = (speed * dt).min(distance);
    let heading = if distance > 0.0 {
        to_target * (1.0 / distance)
    } else {
        v2(0.0, 0.0)
    };

    // Friendly traffic: a vehicle waits for whatever it would run into.
    let next = here + heading * step;
    let blocker = units.iter().enumerate().find_map(|(j, other)| {
        (j != i
            && other.is_vehicle()
            && other.alive()
            && other.side == unit.side
            && vehicle_conflict(unit, next, yaw, other))
        .then_some(other.id)
    });

    let unit = &mut units[i];
    if let Some(b) = blocker {
        unit.state = MoveState::Waiting;
        unit.blocker = Some(b);
        unit.yaw = yaw;
        return;
    }
    unit.state = MoveState::Moving;
    unit.blocker = None;

    // True geometry decides; an obstacle met here becomes known to the side.
    let radius = unit.footprint_radius();
    let side = &mut sides[unit.side.index()];
    let mut solid = false;
    for prop in ctx.world.props_near(next, radius + ENCOUNTER_RANGE_M) {
        if !prop.kind.blocks(unit.mobility.class) {
            continue;
        }
        if prop.id >= ctx.authored && prop.footprint().contains(next, radius + ENCOUNTER_RANGE_M) {
            side.learn(prop.id);
        }
        // A unit may always step out of a solid it already overlaps.
        solid |= prop.footprint().contains(next, unit.mobility.half_width_m)
            && !prop.footprint().contains(here, unit.mobility.half_width_m);
    }
    let Some(ground) = ctx
        .world
        .surface_at(next.x, next.y)
        .filter(|s| s.traversable)
    else {
        unit.yaw = yaw;
        return;
    };
    if solid {
        unit.yaw = yaw;
        return;
    }
    unit.yaw = yaw;
    unit.position = next.with_z(ground.z);

    let route = unit.route.as_mut().unwrap();
    let left = (route[0] - next).length();
    if left < PROGRESS_EPSILON_M.min(step + 1e-9) || left < 1e-6 {
        route.remove(0);
        unit.progress = (f64::INFINITY, ctx.tick);
        if route.is_empty() {
            arrive(unit);
        }
    } else if left < unit.progress.0 - PROGRESS_EPSILON_M {
        unit.progress = (left, ctx.tick);
    }
}

/// A vehicle's footprint at `center`/`yaw`, grown by `margin`.
fn rect_of(unit: &Unit, center: V2, yaw: f64, margin: f64) -> Option<Obb2> {
    unit.hull.map(|h| Obb2 {
        center,
        yaw,
        half: v2(h.x + margin, h.y + margin),
    })
}

/// Would this vehicle, moved to `next`, run into the vehicle `other`? Only a move that
/// makes an existing overlap no worse is allowed, so touching units can part.
fn vehicle_conflict(unit: &Unit, next: V2, yaw: f64, other: &Unit) -> bool {
    let me = rect_of(unit, next, yaw, TRAFFIC_MARGIN_M).unwrap();
    let hits = |at: V2| {
        let probe = Obb2 { center: at, ..me };
        rect_of(other, other.position.xy(), other.yaw, 0.0).is_some_and(|r| probe.overlaps(&r))
    };
    hits(next)
        && !(hits(unit.position.xy())
            && (other.position.xy() - next).length()
                > (other.position.xy() - unit.position.xy()).length())
}

/// Draw each living soldier's spot around the end of the squad's route for
/// a new goal: a fresh seeded arrangement (D1), clear of the solids the side
/// knows, with each soldier's seeded pace and start (the stagger).
fn spread_out(ctx: &MovementContext, unit: &mut Unit, side: &SideGeometry, end: V2) {
    let solid = |p: &Prop| p.kind.blocks(MoverClass::Infantry) && side.knows(p, ctx.authored);
    let living = unit.members.iter().filter(|s| s.alive()).count();
    let mut draws = arrangement::rng(ctx.seed, unit.id.0, ctx.tick);
    let spots = arrangement::squad_spots(
        ctx.world,
        end,
        living,
        ctx.infantry,
        ctx.soldier_radius_m,
        &solid,
        &mut draws,
    );
    let mut spots = spots.into_iter();
    let rules = ctx.infantry;
    let stagger = rules.stagger_s * ctx.tick_hz as f64;
    // The first man sets off at once, so the squad answers on the order's
    // tick; the rest follow within the stagger.
    let delays: Vec<f64> = unit.members.iter().map(|_| draws.unit()).collect();
    let first = unit
        .members
        .iter()
        .zip(&delays)
        .filter(|(s, _)| s.alive())
        .map(|(_, d)| *d)
        .fold(f64::INFINITY, f64::min);
    for (s, delay) in unit.members.iter_mut().zip(delays) {
        s.path.clear();
        s.spot = None;
        if s.alive() {
            s.spot = spots.next();
            s.pace = draws.unit();
            s.start = ctx.tick + (stagger * (delay - first)).round() as u64;
        }
    }
}

/// A replan toward the same goal keeps every soldier's spot (and pace and
/// start); a spot the side has since learned lies in a solid, or a soldier
/// who has none (a replacement), takes the nearest free one.
fn keep_spots(ctx: &MovementContext, unit: &mut Unit, side: &SideGeometry, end: V2) {
    let solid = |p: &Prop| p.kind.blocks(MoverClass::Infantry) && side.knows(p, ctx.authored);
    let r = ctx.soldier_radius_m;
    let living = unit.members.iter().filter(|s| s.alive()).count();
    let reach = arrangement::spread(ctx.infantry, living).max(ctx.infantry.spacing_m) * 2.0;
    for k in 0..unit.members.len() {
        let s = &unit.members[k];
        if !s.alive() {
            continue;
        }
        let wanted = s.spot.unwrap_or(end);
        if s.spot.is_some() && arrangement::standing_room(ctx.world, wanted, r, &solid) {
            continue;
        }
        let taken: Vec<V2> = unit
            .members
            .iter()
            .enumerate()
            .filter(|(j, o)| *j != k && o.alive())
            .filter_map(|(_, o)| o.spot)
            .collect();
        let spacing = ctx.infantry.spacing_m / 2.0;
        let spot = arrangement::nearest_free(wanted, reach, |p| {
            taken.iter().all(|t| (*t - p).length() >= spacing)
                && arrangement::standing_room(ctx.world, p, r, &solid)
                && arrangement::reachable(ctx.world, end, p, r, &solid)
        })
        .unwrap_or(end);
        let s = &mut unit.members[k];
        s.spot = Some(spot);
        s.path.clear();
    }
}

/// Where a squad plans its corridor from: the living soldier nearest its
/// middle, who stands where soldiers can stand (the middle of a squad split
/// by a wall may lie inside it).
fn anchor(unit: &Unit) -> V2 {
    let middle = unit.position.xy();
    unit.member_positions()
        .map(|p| p.xy())
        .min_by(|a, b| (*a - middle).length().total_cmp(&(*b - middle).length()))
        .unwrap_or(middle)
}
