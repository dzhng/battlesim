//! Ground movement each tick: plan when needed, follow the route at surface
//! speed, and learn obstacles by running into them. A vehicle drives its route
//! as one hull and yields to friendly traffic. A squad plans one corridor from
//! its middle; each soldier walks it as a body of his own, offset toward his
//! spot in the arrangement where the move ends, sliding along whatever solid
//! he meets. Craters slow a driving vehicle here, in integration only:
//! planning never reads them.
use std::collections::BTreeSet;

use contract::ids::Tick;
use contract::map::{MoverClass, PropKind};
use contract::observation::MoveState;
use contract::scenario::InfantryMovementRules;

use crate::arrangement;
use crate::ground::GroundLayer;
use crate::math::{v2, wrap_angle, Obb2, V2, V3};
use crate::navigation::{NavGrid, Plan};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

/// A pursuit goal this far from the planned one is replanned.
const GOAL_REPLAN_M: f64 = 5.0;
/// Seconds without closing on the next waypoint before a route is replanned.
const STALL_REPLAN_S: f64 = 2.0;
/// Progress shorter than this does not reset the stall watch.
const PROGRESS_EPSILON_M: f64 = 0.5;
/// Extra room vehicles keep from other bodies.
const TRAFFIC_MARGIN_M: f64 = 0.4;
/// A soldier checks his lane this far ahead for solids his side knows.
const LANE_LOOKAHEAD_M: f64 = 8.0;
/// Times a soldier's step is pushed out of the solids it meets.
const SLIDE_PASSES: usize = 3;
/// A step that closes on its target by less than this share of its length
/// is a stop: sliding almost head-on along a face gets a soldier nowhere.
const MIN_HEADWAY: f64 = 0.25;
/// Headings a stopped soldier tries, in order, to step around what stops him.
const SIDE_STEPS_DEG: [f64; 6] = [30.0, -30.0, 60.0, -60.0, 90.0, -90.0];
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
    pub fn grid(&mut self, world: &WorldGeometry, authored: PropId) -> &mut NavGrid {
        if self.grid.as_ref().is_none_or(|(r, _)| *r != self.revision) {
            let known = world.props().filter(|p| self.knows(p, authored));
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
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        if units[i].is_vehicle() {
            step_vehicle(ctx, units, i, sides);
        } else {
            let unit = &mut units[i];
            step_squad(ctx, unit, &mut sides[unit.side.index()], &hulls);
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
            if !unit.is_vehicle() {
                spread_out(
                    ctx,
                    unit,
                    side,
                    *route.last().expect("a route ends somewhere"),
                );
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
            && other.alive()
            && !other.garrisoned()
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

/// Draw each living soldier's spot around the end of the squad's new route:
/// a fresh seeded arrangement on every plan (D1), clear of the solids the
/// side knows. Every soldier starts the route from its first waypoint.
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
    for s in &mut unit.members {
        s.leg = 0;
        s.spot = if s.alive() { spots.next() } else { None };
    }
}

/// Walk every soldier of a squad one tick along its route. Each heads for
/// his lane, the route's current waypoint shifted by his spot's offset from
/// the route's end (on the last leg, his spot itself), or for the waypoint
/// itself while his side knows a solid across that lane. The squad passes a
/// waypoint once every soldier has; it arrives once every soldier stands on
/// his spot. The squad's position follows its soldiers.
fn step_squad(ctx: &MovementContext, unit: &mut Unit, side: &mut SideGeometry, hulls: &[Obb2]) {
    let dt = 1.0 / ctx.tick_hz as f64;
    for s in &mut unit.members {
        s.velocity = V2::default();
    }
    if !may_advance(ctx, unit) {
        return;
    }
    let mut route = unit.route.take().expect("a route to walk");
    let last = route.len() - 1;
    let end = route[last];
    // Suppression slows infantry; it never turns them around (P14).
    let pace = (1.0 - ctx.suppression_move_penalty * unit.suppression).max(0.0);
    let (mut remaining, mut arrived) = (0.0, true);
    for k in 0..unit.members.len() {
        let s = &unit.members[k];
        if !s.alive() {
            continue;
        }
        let leg = s.leg.min(last);
        let spot = s.spot.unwrap_or(end);
        let here = s.position;
        let waypoint = route[leg];
        // His lane point, if he can go on from it to the next leg;
        // otherwise the corridor's own waypoint.
        let aim = if leg == last {
            spot
        } else {
            let lane = lane_point(ctx, side, &route, leg, spot);
            let onward = |to: V2| lane_open(ctx, side, lane, to, f64::INFINITY);
            if lane == waypoint
                || onward(lane_point(ctx, side, &route, leg + 1, spot))
                || onward(route[leg + 1])
            {
                lane
            } else {
                waypoint
            }
        };
        if leg == last && (spot - here.xy()).length() < 1e-9 {
            continue; // on his spot
        }
        // He takes his lane while it looks clear, else the corridor.
        let toward = if lane_open(ctx, side, here.xy(), aim, LANE_LOOKAHEAD_M) {
            aim
        } else {
            waypoint
        };
        let step = ctx.world.surface_at(here.x, here.y).map_or(0.0, |g| {
            let road = matches!(
                g.kind,
                crate::world::SurfaceKind::Road | crate::world::SurfaceKind::Bridge
            );
            unit.mobility.speed(road, g.forest, g.slope_deg)
        }) * pace
            * dt;
        let mut next = walk(ctx, side, hulls, here, toward, step);
        if next.is_none() && toward != waypoint {
            next = walk(ctx, side, hulls, here, waypoint, step);
        }
        let at = next.map_or(here.xy(), |p| p.xy());
        // He moves on to the next leg at his lane point or the waypoint.
        let passed = leg < last
            && ((aim - at).length() < PROGRESS_EPSILON_M
                || (waypoint - at).length() < PROGRESS_EPSILON_M);
        let s = &mut unit.members[k];
        if let Some(p) = next {
            s.velocity = (p.xy() - here.xy()) * (1.0 / dt);
            s.position = p;
        }
        s.leg = leg + passed as usize;
        let to_spot = (spot - at).length();
        remaining += if leg == last {
            to_spot
        } else {
            (toward - at).length()
        };
        arrived &= leg == last && to_spot < 1e-6;
    }
    let passed = unit
        .members
        .iter()
        .filter(|s| s.alive())
        .map(|s| s.leg)
        .min()
        .unwrap_or(0);
    if passed > 0 {
        route.drain(..passed);
        for s in &mut unit.members {
            s.leg = s.leg.saturating_sub(passed);
        }
        unit.progress = (f64::INFINITY, ctx.tick);
    } else if remaining < unit.progress.0 - PROGRESS_EPSILON_M {
        unit.progress = (remaining, ctx.tick);
    }
    unit.state = MoveState::Moving;
    unit.blocker = None;
    unit.settle();
    // The squad faces its next waypoint while it is still some way off.
    let ahead = route[0] - unit.position.xy();
    if ahead.length() > 1.0 {
        unit.yaw = ahead.y.atan2(ahead.x);
    }
    if arrived {
        arrive(unit);
        for s in &mut unit.members {
            s.spot = None;
            s.leg = 0;
        }
    } else {
        unit.route = Some(route);
    }
}

/// A soldier's lane point on leg `j` of `route`: the waypoint shifted by his
/// spot's offset from the route's end, unless his side knows a solid across
/// that shift (then the waypoint itself, the mock's lesson); on the last
/// leg, his spot.
fn lane_point(ctx: &MovementContext, side: &SideGeometry, route: &[V2], j: usize, spot: V2) -> V2 {
    let end = route[route.len() - 1];
    if j + 1 == route.len() {
        return spot;
    }
    let shifted = route[j] + (spot - end);
    if lane_open(ctx, side, route[j], shifted, f64::INFINITY) {
        shifted
    } else {
        route[j]
    }
}

/// Whether a soldier's disc can walk from `from` toward `lane` as far as
/// `ahead` metres without meeting a solid his side knows (one he already
/// stands in aside).
fn lane_open(ctx: &MovementContext, side: &SideGeometry, from: V2, lane: V2, ahead: f64) -> bool {
    let gap = lane - from;
    let length = gap.length();
    if length < 1e-9 {
        return true;
    }
    let to = from + gap * (length.min(ahead) / length);
    let r = ctx.soldier_radius_m;
    !ctx.world
        .props_near((from + to) * 0.5, (to - from).length() / 2.0 + r)
        .iter()
        .any(|p| {
            p.kind.blocks(MoverClass::Infantry)
                && side.knows(p, ctx.authored)
                && p.footprint().meets_segment(from, to, r)
                && !p.footprint().contains(from, r)
        })
}

/// One soldier's step of up to `step` metres from `here` toward `target`, as
/// a body (L4, L5): his disc slides along every prop that stops infantry
/// and every live hull, onto traversable ground, at the ground's height
/// under him. A solid met here becomes known to his side. Stopped (see
/// [`MIN_HEADWAY`]), he side-steps: the first turned heading, in
/// [`SIDE_STEPS_DEG`] order, that moves him at all. `None` when nothing does.
fn walk(
    ctx: &MovementContext,
    side: &mut SideGeometry,
    hulls: &[Obb2],
    here: V3,
    target: V2,
    step: f64,
) -> Option<V3> {
    let from = here.xy();
    let gap = target - from;
    let distance = gap.length();
    if distance < 1e-9 || step <= 0.0 {
        return None;
    }
    let r = ctx.soldier_radius_m;
    let reach = r + step + ENCOUNTER_RANGE_M;
    // A soldier may always step out of a solid he already stands in.
    let mut solids: Vec<Obb2> = hulls
        .iter()
        .filter(|h| (h.center - from).length() <= h.half.length() + reach)
        .filter(|h| !h.contains(from, r))
        .copied()
        .collect();
    for prop in ctx.world.props_near(from, reach) {
        if !prop.kind.blocks(MoverClass::Infantry) {
            continue;
        }
        let box_ = prop.footprint();
        if prop.id >= ctx.authored && box_.contains(from, r + ENCOUNTER_RANGE_M) {
            side.learn(prop.id);
        }
        if !box_.contains(from, r) {
            solids.push(box_);
        }
    }
    let heading = gap * (1.0 / distance);
    let slide = |mut next: V2| {
        for _ in 0..SLIDE_PASSES {
            let mut clear = true;
            for b in &solids {
                if b.contains(next, r) {
                    next = b.push_out(next, r);
                    clear = false;
                }
            }
            if clear {
                break;
            }
        }
        (!solids.iter().any(|b| b.contains(next, r))).then_some(next)
    };
    let onto_ground = |next: V2| {
        ctx.world
            .surface_at(next.x, next.y)
            .filter(|g| g.traversable)
            .map(|g| next.with_z(g.z))
    };
    let stride = step.min(distance);
    let straight = slide(from + heading * stride)
        .filter(|next| distance - (target - *next).length() >= MIN_HEADWAY * stride)
        .and_then(onto_ground);
    if straight.is_some() {
        return straight;
    }
    SIDE_STEPS_DEG.iter().find_map(|turn| {
        slide(from + heading.rotated(turn.to_radians()) * step)
            .filter(|next| (*next - from).length() > 1e-9)
            .and_then(onto_ground)
    })
}
