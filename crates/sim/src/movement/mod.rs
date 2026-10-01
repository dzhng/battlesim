//! Ground movement each tick: plan when needed, follow the route toward surface
//! speed, and learn obstacles by running into them. A vehicle drives its route
//! as one hull, waits for every other vehicle whatever its side (Q14), never
//! for soldiers, and shoves bodies lighter than its push class aside
//! ([`push`]). A squad
//! plans one corridor on the coarse grid; each soldier walks it as a body of
//! his own ([`soldier`]), with his own route on the exact bodies for the
//! final stretch ([`final_leg`]). Craters slow a driving vehicle here, in
//! integration only: planning never reads them.
use std::collections::{BTreeMap, VecDeque};

use contract::ids::Tick;
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::InfantryMovementRules;

use crate::arrangement;
use crate::ground::GroundLayer;
use crate::math::{v2, Obb2, V2};
use crate::navigation::{NavGrid, Plan};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

mod drive;
mod final_leg;
mod push;
mod soldier;
mod take_cover;

pub use drive::{final_yaw, Manoeuvre};
pub use final_leg::{final_leg, FINE_CELL_M};
pub use push::Shove;
pub use soldier::{clear_of, soldier_steer, Around, Corridor, Steer, Threat};

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
/// A side remembers where its latest this many planning changes were.
const CHANGE_LOG: usize = 1024;

/// Where a side last saw a body stand (L1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub center: V2,
    pub yaw: f64,
    pub base_z: f64,
}

/// One planning change (a body learned, moved or gone), as the circle round
/// every place the side planned it at before and after.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Change {
    revision: u64,
    at: V2,
    radius: f64,
}

/// The circle round `prop`'s footprint wherever `seen` (if anywhere) and
/// where it stands now.
fn span(prop: &Prop, seen: Option<V2>) -> (V2, f64) {
    let r = prop.footprint_radius();
    match seen {
        Some(was) => (
            (was + prop.center) * 0.5,
            (prop.center - was).length() * 0.5 + r,
        ),
        None => (prop.center, r),
    }
}

/// What one side may plan with: the authored map plus the dynamic obstacles
/// its units have encountered, each where the side last saw it. Hidden
/// changes never reach this.
#[derive(Default)]
pub struct SideGeometry {
    /// Every body this side places somewhere other than where it truly
    /// stands, or learned after setup: each learned body where it was last
    /// seen, and each authored body shoved out of sight where it stood
    /// before (L1). Any other authored body stands where it is.
    pub seen: BTreeMap<PropId, Seen>,
    /// Bodies gone out of this side's sight (trees knocked down, props
    /// destroyed): it plans around them, and draws them, until it sees the
    /// ground where they stood (Q16, Q17, L1).
    pub standing: BTreeMap<PropId, Prop>,
    pub revision: u64,
    /// Where each of the latest revisions changed the side's plan, oldest
    /// first, so a squad re-resolves only for changes within its reach.
    changes: VecDeque<Change>,
    grid: Option<(u64, NavGrid)>,
    /// Route searches run for this side (the no-per-frame-search contract).
    pub searches: u64,
}

impl SideGeometry {
    /// The side sees or touches `prop` where it stands now. A body new to it
    /// is learned; one it knows elsewhere is re-learned once it has moved
    /// more than `relearn_m` from where it was seen, or has come to rest
    /// (`resting`), so a body being shoved is not re-learned every tick
    /// (Q13). Each learning bumps the side's revision (L2).
    pub fn learn(&mut self, prop: &Prop, authored: PropId, relearn_m: f64, resting: bool) {
        let now = Seen {
            center: prop.center,
            yaw: prop.yaw,
            base_z: prop.base_z,
        };
        match self.seen.get(&prop.id) {
            None if prop.id < authored => return, // it stands where the map put it
            None => {}
            Some(was) if *was == now => return,
            Some(was) if !resting && (was.center - now.center).length() <= relearn_m => return,
            Some(_) => {}
        }
        let was = self.seen.insert(prop.id, now).map(|w| w.center);
        self.changed(span(prop, was));
    }

    /// Bump the revision for a change round `(at, radius)`.
    fn changed(&mut self, (at, radius): (V2, f64)) {
        self.revision += 1;
        self.changes.push_back(Change {
            revision: self.revision,
            at,
            radius,
        });
        if self.changes.len() > CHANGE_LOG {
            self.changes.pop_front();
        }
    }

    /// Whether any change after revision `since` came within `reach` of
    /// `at`: always, once the log no longer reaches back that far.
    pub fn changed_near(&self, since: u64, at: V2, reach: f64) -> bool {
        if since >= self.revision {
            return false;
        }
        if self.changes.front().is_none_or(|c| c.revision > since + 1) {
            return true;
        }
        self.changes
            .iter()
            .rev()
            .take_while(|c| c.revision > since)
            .any(|c| (c.at - at).length() <= reach + c.radius)
    }

    /// `prop` is about to be shoved: an authored body the side has not seen
    /// move stays, for it, where it stands now (L1).
    pub fn before_move(&mut self, prop: &Prop, authored: PropId) {
        if prop.id < authored {
            self.seen.entry(prop.id).or_insert(Seen {
                center: prop.center,
                yaw: prop.yaw,
                base_z: prop.base_z,
            });
        }
    }

    /// A body this side did not see go: it keeps planning around it.
    pub fn keep_standing(&mut self, prop: Prop) {
        self.standing.insert(prop.id, prop);
    }

    /// The side sees that a body it kept standing is gone.
    pub fn saw_fallen(&mut self, prop: PropId) {
        if let Some(gone) = self.standing.remove(&prop) {
            let was = self.seen.remove(&prop).map(|s| s.center);
            self.changed(span(&gone, was));
        }
    }

    /// A prop this side plans with is gone: plan again without it.
    pub fn forget(&mut self, prop: &Prop) {
        let was = self.seen.remove(&prop.id).map(|s| s.center);
        self.changed(span(prop, was));
    }

    /// Whether this side plans with `prop`: authored with the map, remains
    /// every side plans with ([`Prop::known_to_all`]), or learned.
    pub fn knows(&self, prop: &Prop, authored: PropId) -> bool {
        prop.id < authored || prop.known_to_all || self.seen.contains_key(&prop.id)
    }

    /// `prop` as this side believes it stands, if it knows it at all.
    pub fn belief(&self, prop: &Prop, authored: PropId) -> Option<Prop> {
        if !self.knows(prop, authored) {
            return None;
        }
        let mut p = prop.clone();
        if let Some(seen) = self.seen.get(&prop.id) {
            p.center = seen.center;
            p.yaw = seen.yaw;
            p.base_z = seen.base_z;
        }
        Some(p)
    }

    /// A body where this side knows it, including an unseen collapse.
    pub fn prop(&self, world: &WorldGeometry, authored: PropId, id: PropId) -> Option<Prop> {
        self.belief(world.prop(id).or_else(|| self.standing.get(&id))?, authored)
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
            let known = world
                .props()
                .chain(self.standing.values())
                .filter_map(|p| self.belief(p, authored));
            self.grid = Some((self.revision, NavGrid::build(world, known, soldier_radius)));
        }
        &mut self.grid.as_mut().unwrap().1
    }

    /// Fold the side's planning knowledge into a digest.
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.revision).u64(self.seen.len() as u64);
        for (id, s) in &self.seen {
            d.u64(*id as u64)
                .f64(s.center.x)
                .f64(s.center.y)
                .f64(s.yaw)
                .f64(s.base_z);
        }
        d.u64(self.standing.len() as u64);
        for id in self.standing.keys() {
            d.u64(*id as u64);
        }
        d.u64(self.changes.len() as u64);
        for c in &self.changes {
            d.u64(c.revision).f64(c.at.x).f64(c.at.y).f64(c.radius);
        }
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
    pub infantry: &'a InfantryMovementRules,
    pub soldier_radius_m: f64,
    /// The battle's seed: arrangements are drawn from it (D1).
    pub seed: u64,
    pub rules: &'a contract::scenario::Rules,
    /// Each side's knowledge: the enemies a squad takes cover from (Q7).
    pub knowledge: &'a [crate::knowledge::SideKnowledge; 2],
    /// The weapons: how far a squad's reach when it seeks where to fight.
    pub arsenal: &'a crate::weapons::Arsenal,
}

/// Move every living unit one tick. Returns the bodies vehicles shoved,
/// for the caller to move in the world (movement reads it, never writes it).
pub fn advance(
    ctx: &MovementContext,
    units: &mut [Unit],
    sides: &mut [SideGeometry; 2],
) -> Vec<Shove> {
    let footprints: Vec<Option<Obb2>> = units
        .iter()
        .map(|u| u.hull_box().filter(|_| u.alive()))
        .collect();
    let field = take_cover::Field::gather(ctx, units);
    // The destroyed stay put; a wreck is an obstacle prop, not traffic.
    for unit in units.iter_mut().filter(|u| u.alive()) {
        let s = unit.side.index();
        plan_if_needed(ctx, unit, &mut sides[s], &footprints, &field);
    }
    let hulls: Vec<Obb2> = footprints.iter().flatten().copied().collect();
    // Every live vehicle on the move, either side, as soldiers see it coming.
    let threats: Vec<Threat> = units
        .iter()
        .filter(|u| u.alive())
        .filter_map(|u| Threat::of(ctx, u))
        .collect();
    let mut crowd = soldier::Crowd::gather(units);
    let mut shoves = Vec::new();
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        if units[i].is_vehicle() {
            step_vehicle(ctx, units, i, sides, &mut shoves);
            soldier::shove(ctx, units, i, &mut crowd);
        } else {
            let unit = &mut units[i];
            let advancing = may_advance(ctx, unit);
            let side = &mut sides[unit.side.index()];
            if !advancing && unit.garrison.is_none() && (unit.route.is_none() || unit.halted()) {
                take_cover::hold(ctx, unit, side, &field);
            }
            soldier::step_squad(ctx, unit, i, side, &hulls, &threats, &mut crowd, advancing);
        }
    }
    shoves
}

fn plan_if_needed(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &mut SideGeometry,
    footprints: &[Option<Obb2>],
    field: &take_cover::Field,
) {
    let Some((goal, policy)) = unit.movement_goal() else {
        unit.route = None;
        unit.planned_goal = None;
        unit.state = MoveState::Idle;
        unit.blocker = None;
        for s in &mut unit.members {
            s.spot = None;
            // A soldier walking to his post keeps his way there.
            if s.post.is_none() {
                s.path.clear();
            }
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
            // A new body on the way replans a route it no longer fits, and
            // a pusher's route that now shoves one, so A* weighs the shove
            // against a detour (Q13).
            stalled
                || (changed && {
                    let grid = side.grid(ctx.world, ctx.authored, ctx.soldier_radius_m);
                    !grid.route_fits(from, route, &unit.mobility)
                        || grid.route_pushes(from, route, &unit.mobility)
                })
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
        set_off(unit)
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
                    spread_out(ctx, unit, side, field, from, end);
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
    // An attack-move halting on contact takes cover facing the enemy (Q9).
    if held == MoveState::Halted && unit.state != MoveState::Halted {
        unit.cover.due = true;
        take_cover::halt(ctx, unit);
    }
    unit.progress.1 = ctx.tick;
    unit.state = held;
    false
}

/// A route's end reached: a destination completes its move; a pursuit point
/// leaves the attack in place for its weapons.
fn arrive(unit: &mut Unit) {
    if let Some(m) = unit.orders.front().and_then(|o| o.movement()) {
        // The right-drag's facing (Q9): a squad turns at once; tracks pivot
        // at rest; wheels keep the way they came.
        match m.facing {
            Some(f) if !unit.is_vehicle() => unit.yaw = f,
            Some(f) if unit.mobility.drive.is_some_and(|d| d.tracked) => unit.turn_to = Some(f),
            _ => {}
        }
        unit.orders.pop_front();
    }
    unit.route = None;
    unit.planned_goal = None;
    unit.manoeuvre = None;
    unit.drive_speed_mps = 0.0;
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
    shoves: &mut Vec<Shove>,
) {
    let dt = 1.0 / ctx.tick_hz as f64;
    units[i].reversing = false;
    if !may_advance(ctx, &mut units[i]) {
        units[i].drive_speed_mps = 0.0;
        units[i].manoeuvre = None;
        if units[i].route.is_none() && units[i].turn_to.is_some() {
            let before = units[i].yaw;
            drive::pivot(ctx.world, &mut units[i], dt);
            let unit = &units[i];
            let here = unit.position.xy();
            if units.iter().enumerate().any(|(j, o)| {
                j != i && o.is_vehicle() && o.alive() && vehicle_conflict(unit, here, unit.yaw, o)
            }) {
                units[i].yaw = before;
                units[i].turn_to = None;
            }
        }
        return;
    }
    if drive::prune(&mut units[i]) {
        arrive(&mut units[i]);
        return;
    }
    let unit = &units[i];
    let target = unit.route.as_ref().unwrap()[0];
    let here = unit.position.xy();
    let surface = ctx.world.surface_at(here.x, here.y);
    let speed = surface.map_or(0.0, |s| {
        unit.mobility.speed(s.road_factor, s.forest, s.slope_deg)
    });
    // Craters under the hull slow it slightly; never to a stop (Q8).
    let speed = speed * ctx.ground.vehicle_speed(here.x, here.y, ctx.ground_rules);
    let before_manoeuvre = unit.manoeuvre;
    let mut motion = drive::steer(ctx.world, &mut units[i], target, speed, dt);
    let unit = &units[i];
    let mut next = here + motion.heading * motion.step;
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let current = Obb2 {
        center: here,
        yaw: unit.yaw,
        half,
    };
    let push = unit.mobility.push;
    // Box against box (L8): a body it cannot shove stops it; the bodies it
    // can shove slow it by the heaviest's class ratio and slide aside (Q2).
    let hull_at = |center: V2, yaw: f64| Obb2 { center, yaw, half };
    let mut met = push::meet(ctx.world, &hull_at(next, motion.yaw), &current, push);
    if met.solid.is_none() && !met.shoved.is_empty() {
        let heaviest = met
            .shoved
            .iter()
            .map(|p| p.body.weight_class.rank())
            .max()
            .unwrap_or(0);
        // Shoving limits the target speed, not the already ramped velocity:
        // multiplying carried speed every tick would compound the slowdown.
        units[i].manoeuvre = before_manoeuvre;
        motion = drive::steer(
            ctx.world,
            &mut units[i],
            target,
            speed * push.shove_speed(heaviest),
            dt,
        );
        next = here + motion.heading * motion.step;
        met = push::meet(ctx.world, &hull_at(next, motion.yaw), &current, push);
    }
    // Traffic, whatever its side (Q14): a vehicle waits for whatever it
    // would run into; live vehicles are never shoved (Q15).
    let unit = &units[i];
    let mut blocker = units.iter().enumerate().find_map(|(j, other)| {
        (j != i
            && other.is_vehicle()
            && other.alive()
            && vehicle_conflict(unit, next, motion.yaw, other))
        .then_some(other.id)
    });

    // A detouring tank must not pivot through its neighbour. Give the turn
    // room instead of deadlocking nose-to-nose.
    if blocker.is_some()
        && unit.mobility.drive.is_some_and(|d| d.tracked)
        && motion.yaw != unit.yaw
        && stationary_yaw(ctx.world, units, i, motion.yaw) == unit.yaw
    {
        let back = drive::give_space(unit, speed, dt);
        let at = here + back.heading * back.step;
        let behind = push::meet(ctx.world, &hull_at(at, back.yaw), &current, push);
        let traffic = units.iter().enumerate().any(|(j, other)| {
            j != i
                && other.is_vehicle()
                && other.alive()
                && vehicle_conflict(unit, at, back.yaw, other)
        });
        if behind.solid.is_none() && behind.shoved.is_empty() && !traffic {
            motion = back;
            next = at;
            met = behind;
            blocker = None;
        }
    }
    if let Some(b) = blocker {
        let held_yaw = stationary_yaw(ctx.world, units, i, motion.yaw);
        let unit = &mut units[i];
        unit.state = MoveState::Waiting;
        unit.blocker = Some(b);
        unit.yaw = held_yaw;
        unit.drive_speed_mps = 0.0;
        return;
    }
    let unit = &mut units[i];
    unit.state = MoveState::Moving;
    unit.blocker = None;

    // True geometry decides; an obstacle met here becomes known to the side.
    let radius = unit.footprint_radius(ctx.soldier_radius_m);
    let side = &mut sides[unit.side.index()];
    for prop in ctx.world.props_near(next, radius + ENCOUNTER_RANGE_M) {
        if prop.blocks(MoverClass::Vehicle)
            && prop.footprint().contains(next, radius + ENCOUNTER_RANGE_M)
        {
            let resting = ctx.world.resting(prop.id, ctx.tick);
            side.learn(prop, ctx.authored, ctx.rules.pushing.relearn_m, resting);
        }
    }
    let turn = ctx.rules.pushing.turn_deg_per_m.to_radians();
    let mut shoved = Vec::new();
    let mut solid = met.solid.is_some();
    for prop in &met.shoved {
        match push::shove(
            ctx.world,
            units,
            i,
            &hull_at(next, motion.yaw),
            motion.heading,
            prop,
            turn,
        ) {
            Some(s) => shoved.push(s),
            None => solid = true,
        }
    }
    let ground = ctx
        .world
        .surface_at(next.x, next.y)
        .filter(|s| s.traversable);
    if solid || ground.is_none() {
        let held_yaw = stationary_yaw(ctx.world, units, i, motion.yaw);
        let unit = &mut units[i];
        unit.yaw = held_yaw;
        unit.drive_speed_mps = 0.0;
        return;
    }
    let ground = ground.expect("traversable ground checked above");
    let unit = &mut units[i];
    shoves.extend(shoved);
    unit.yaw = motion.yaw;
    unit.position = next.with_z(ground.z);
    unit.reversing = motion.backwards && motion.step > 0.0;
    unit.drive_speed_mps = motion.step / dt * if motion.backwards { -1.0 } else { 1.0 };
    if let Some(m) = unit.manoeuvre.as_mut() {
        m.driven_m += motion.step;
    }
    // Turning or making room can move away from the waypoint without stalling.
    if motion.manoeuvring && motion.step > 0.0 {
        unit.progress.1 = ctx.tick;
    }

    let route = unit.route.as_mut().unwrap();
    let left = (route[0] - next).length();
    if left < PROGRESS_EPSILON_M.min(motion.step + 1e-9) || left < 1e-6 {
        route.remove(0);
        unit.progress = (f64::INFINITY, ctx.tick);
        if route.is_empty() {
            arrive(unit);
        }
    } else if left < unit.progress.0 - PROGRESS_EPSILON_M {
        unit.progress = (left, ctx.tick);
    }
}

/// A rejected step may still let tracks pivot, but only into a clear pose.
fn stationary_yaw(world: &WorldGeometry, units: &[Unit], i: usize, yaw: f64) -> f64 {
    let unit = &units[i];
    if yaw == unit.yaw || !unit.mobility.drive.is_some_and(|d| d.tracked) {
        return unit.yaw;
    }
    let here = unit.hull_box().expect("a vehicle has a hull");
    let turned = Obb2 { yaw, ..here };
    let met = push::meet(world, &turned, &here, unit.mobility.push);
    // A stationary pivot checks true hulls; translation's traffic buffer
    // would forbid using the clearance reserved by the stopped approach.
    let occupied = met.solid.is_some()
        || !met.shoved.is_empty()
        || units.iter().enumerate().any(|(j, other)| {
            j != i
                && other.is_vehicle()
                && other.alive()
                && turned.overlaps(&other.hull_box().expect("a vehicle has a hull"))
        });
    if occupied {
        unit.yaw
    } else {
        yaw
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
/// knows, where the best cover near each spot takes its place (D4), with
/// each soldier's seeded pace and start (the stagger).
fn spread_out(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &take_cover::Field,
    from: V2,
    end: V2,
) {
    let solid = |p: &Prop| p.blocks(MoverClass::Infantry) && side.knows(p, ctx.authored);
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
    let mut spots = spots;
    let tiers = take_cover::at_order(ctx, unit, side, field, from, end, &mut spots);
    let mut spots = spots.into_iter().zip(tiers);
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
        s.post = None;
        s.cover = None;
        s.lean = None;
        if s.alive() {
            let (spot, how) = spots.next().unzip();
            let (tier, lean) = how.unzip();
            s.spot = spot;
            s.cover = tier.flatten();
            s.lean = lean.flatten();
            s.pace = draws.unit();
            s.start = ctx.tick + (stagger * (delay - first)).round() as u64;
        }
    }
}

/// A replan toward the same goal keeps every soldier's spot (and pace and
/// start); a spot the side has since learned lies in a solid, or a soldier
/// who has none (a replacement), takes the nearest free one.
fn keep_spots(ctx: &MovementContext, unit: &mut Unit, side: &SideGeometry, end: V2) {
    let solid = |p: &Prop| p.blocks(MoverClass::Infantry) && side.knows(p, ctx.authored);
    let r = ctx.soldier_radius_m;
    let living = unit.members.iter().filter(|s| s.alive()).count();
    let reach = arrangement::spread(ctx.infantry, living).max(ctx.infantry.spacing_m) * 2.0;
    for s in &mut unit.members {
        s.post = None;
    }
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
        s.cover = None;
        s.lean = None;
        s.path.clear();
        s.post = None;
    }
}

/// Where a squad plans its corridor from: the living soldier nearest its
/// middle, who stands where soldiers can stand (the middle of a squad split
/// by a wall may lie inside it).
fn set_off(unit: &Unit) -> V2 {
    let middle = unit.position.xy();
    unit.member_positions()
        .map(|p| p.xy())
        .min_by(|a, b| (*a - middle).length().total_cmp(&(*b - middle).length()))
        .unwrap_or(middle)
}
