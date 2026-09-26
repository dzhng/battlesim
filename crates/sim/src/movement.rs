//! Ground movement each tick: plan when needed, follow the route at surface
//! speed, yield to friendly traffic, and learn obstacles by running into them.
//! Craters slow a driving vehicle here, in integration only: planning never
//! reads them.
use std::collections::BTreeSet;

use contract::ids::Tick;
use contract::map::PropKind;
use contract::observation::MoveState;

use crate::ground::GroundLayer;
use crate::math::{v2, wrap_angle, Obb2, V2};
use crate::navigation::{NavGrid, Plan};
use crate::units::Unit;
use crate::world::{PropId, WorldGeometry};

/// A pursuit goal this far from the planned one is replanned.
const GOAL_REPLAN_M: f64 = 5.0;
/// Seconds without closing on the next waypoint before a route is replanned.
const STALL_REPLAN_S: f64 = 2.0;
/// Progress shorter than this does not reset the stall watch.
const PROGRESS_EPSILON_M: f64 = 0.5;
/// Extra room vehicles keep from other bodies.
const TRAFFIC_MARGIN_M: f64 = 0.4;
/// Headings a squad tries, in order, to step around a vehicle.
const DEFLECTIONS_DEG: [f64; 6] = [30.0, -30.0, 60.0, -60.0, 90.0, -90.0];
/// Squads closer than their combined radii push apart this fast (m/s).
const SQUAD_SEPARATION_MPS: f64 = 0.6;
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

    /// The side's planning grid, rebuilt only when its knowledge changed.
    pub fn grid(&mut self, world: &WorldGeometry, authored: PropId) -> &mut NavGrid {
        if self.grid.as_ref().is_none_or(|(r, _)| *r != self.revision) {
            // A ruin covers exactly the authored building it replaced, so
            // every side plans with it whether or not it saw the collapse:
            // a fall it never saw cannot open a route through the footprint.
            let known = world.props().filter(|p| {
                p.id < authored || p.kind == PropKind::Ruin || self.known_dynamic.contains(&p.id)
            });
            self.grid = Some((self.revision, NavGrid::build(world, known)));
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
    for i in 0..units.len() {
        if units[i].alive() {
            step_unit(ctx, units, i, sides);
        }
    }
    let dt = 1.0 / ctx.tick_hz as f64;
    for unit in units.iter_mut().filter(|u| u.alive() && !u.garrisoned()) {
        reform(unit, dt);
    }
}

/// Scattered soldiers (collapse survivors) walk back to their places in the
/// squad at infantry pace.
fn reform(unit: &mut Unit, dt: f64) {
    let step = unit.mobility.off_road_mps * dt;
    for s in unit.members.iter_mut().filter(|s| s.alive()) {
        let gap = s.formation - s.offset;
        let len = gap.length();
        if len > 0.0 {
            s.offset = if len <= step {
                s.formation
            } else {
                s.offset + gap * (step / len)
            };
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
            stalled
                || (changed
                    && !side.grid(ctx.world, ctx.authored).route_fits(
                        unit.position.xy(),
                        route,
                        &unit.mobility,
                    ))
        }
    };
    unit.planned_revision = side.revision;
    if !needs {
        return;
    }
    side.searches += 1;
    let grid = side.grid(ctx.world, ctx.authored);
    let from = unit.position.xy();
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
            unit.route = Some(route);
            unit.state = MoveState::Moving;
        }
        Plan::Blocked(_) => {
            unit.route = None;
            unit.state = MoveState::RouteBlocked;
        }
    }
}

fn step_unit(ctx: &MovementContext, units: &mut [Unit], i: usize, sides: &mut [SideGeometry; 2]) {
    let dt = 1.0 / ctx.tick_hz as f64;
    if units[i].state == MoveState::Halted && !units[i].halted() {
        units[i].state = MoveState::Moving;
    }
    let unit = &units[i];
    let Some(target) = unit.route.as_ref().and_then(|r| r.first().copied()) else {
        return;
    };
    if unit.halted() {
        // An attack-move holding to engage is not stalled.
        units[i].progress.1 = ctx.tick;
        units[i].state = MoveState::Halted;
        return;
    }
    if !unit.may_translate() {
        // Packing first (L01): no translation, and no stall, until packed.
        units[i].progress.1 = ctx.tick;
        units[i].state = MoveState::Packing;
        return;
    }
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
    let (yaw, speed) = if unit.is_vehicle() {
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
        (yaw, speed * factor)
    } else {
        // Suppression slows infantry; it never turns them around (P14).
        (
            desired_yaw,
            speed * (1.0 - ctx.suppression_move_penalty * unit.suppression).max(0.0),
        )
    };
    let step = (speed * dt).min(distance);
    let mut heading = if distance > 0.0 {
        to_target * (1.0 / distance)
    } else {
        v2(0.0, 0.0)
    };

    // Friendly traffic: vehicles wait; squads step around vehicles and spread.
    let mut blocker = None;
    if unit.is_vehicle() {
        let next = here + heading * step;
        blocker = units.iter().enumerate().find_map(|(j, other)| {
            (j != i
                && other.alive()
                && !other.garrisoned()
                && other.side == unit.side
                && vehicle_conflict(unit, next, yaw, other))
            .then_some(other.id)
        });
    } else {
        let blocked_by = |dir: V2| {
            let next = here + dir * step;
            units.iter().enumerate().find_map(|(j, other)| {
                (j != i
                    && other.is_vehicle()
                    && other.alive()
                    && squad_meets_vehicle(unit, here, next, other))
                .then_some(other.id)
            })
        };
        if let Some(first) = blocked_by(heading) {
            match DEFLECTIONS_DEG
                .iter()
                .map(|d| heading.rotated(d.to_radians()))
                .find(|&d| blocked_by(d).is_none())
            {
                Some(dir) => heading = dir,
                None => blocker = Some(first),
            }
        }
    }

    let unit = &mut units[i];
    if let Some(b) = blocker {
        unit.state = MoveState::Waiting;
        unit.blocker = Some(b);
        unit.yaw = yaw;
        return;
    }
    unit.state = MoveState::Moving;
    unit.blocker = None;
    let mut next = here + heading * step;
    if !unit.is_vehicle() {
        next = next + separation(units, i) * dt;
    }
    let unit = &mut units[i];

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
            // Reaching a destination completes a move; reaching a pursuit point
            // leaves the attack in place for its weapons.
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

/// Would this vehicle, moved to `next`, run into `other`? Only a move that
/// makes an existing overlap no worse is allowed, so touching units can part.
fn vehicle_conflict(unit: &Unit, next: V2, yaw: f64, other: &Unit) -> bool {
    let me = rect_of(unit, next, yaw, TRAFFIC_MARGIN_M).unwrap();
    let hits = |at: V2| {
        let probe = Obb2 { center: at, ..me };
        match rect_of(other, other.position.xy(), other.yaw, 0.0) {
            Some(r) => probe.overlaps(&r),
            None => other
                .member_positions()
                .any(|p| probe.contains(p.xy(), 0.3)),
        }
    };
    hits(next)
        && !(hits(unit.position.xy())
            && (other.position.xy() - next).length()
                > (other.position.xy() - unit.position.xy()).length())
}

/// Would this squad, moving from `here` to `next`, walk into a vehicle?
fn squad_meets_vehicle(unit: &Unit, here: V2, next: V2, vehicle: &Unit) -> bool {
    let r = rect_of(vehicle, vehicle.position.xy(), vehicle.yaw, 0.0).unwrap();
    let margin = unit.mobility.half_width_m + TRAFFIC_MARGIN_M;
    let inside = |c: V2| r.contains(c, margin);
    inside(next) && (!inside(here) || (next - r.center).length() < (here - r.center).length())
}

/// Gentle push away from overlapping friendly squads: flexible spacing, not collision.
fn separation(units: &[Unit], i: usize) -> V2 {
    let me = &units[i];
    let radius = me.footprint_radius();
    let mut push = v2(0.0, 0.0);
    for (j, other) in units.iter().enumerate() {
        if j == i
            || other.is_vehicle()
            || other.side != me.side
            || !other.alive()
            || other.garrisoned()
        {
            continue;
        }
        let d = me.position.xy() - other.position.xy();
        let gap = radius + other.footprint_radius() - d.length();
        if gap > 0.0 && d.length() > 1e-6 {
            push =
                push + d.normalized() * (SQUAD_SEPARATION_MPS * (gap / (radius + 1e-6)).min(1.0));
        }
    }
    push
}
