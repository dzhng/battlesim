//! Ground movement each tick: ask for a route when one is needed and hold
//! until the planner has it ([`RoutePlanner`]), follow the route toward surface
//! speed, and learn obstacles by running into them. A vehicle drives its route
//! as one hull, waits for every other vehicle whatever its side (Q14), never
//! for soldiers, and shoves bodies lighter than its push class aside
//! ([`push`]). A squad
//! plans one corridor on the coarse grid; each soldier walks it as a body of
//! his own ([`soldier`]), with his own route on the exact bodies for the
//! final stretch ([`final_leg`]). Craters slow a driving vehicle here, in
//! integration only: planning never reads them.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use contract::command::MoveDirection;
use contract::ids::Tick;
use contract::map::MoverClass;
use contract::observation::MoveState;

use crate::arrangement;
use crate::ground::GroundLayer;
use crate::math::{v2, Obb2, V2};
use crate::navigation::{NavBase, NavGrid, Plan, RoadNet};
use crate::route_planner::{Request, RoutePlanner};
use crate::units::Unit;
use crate::world::{Prop, PropId, PropIndex, WorldGeometry, PROP_BUCKET_M};

pub(crate) mod air;
mod certify;
pub(crate) mod drive;
mod final_leg;
mod push;
mod soldier;
mod take_cover;

pub(crate) use certify::{certify_orders, ProofRequest};
pub use drive::{final_facing, final_yaw, Manoeuvre};
pub use final_leg::{final_leg, FinalLeg, FINE_CELL_M};
pub use push::Shove;
pub use soldier::{clear_of, soldier_steer, Around, Corridor, Steer, Threat};

/// A pursuit goal this far from the planned one is replanned.
const GOAL_REPLAN_M: f64 = 5.0;
/// Seconds without closing on the next waypoint before a route is replanned.
const STALL_REPLAN_S: f64 = 2.0;
/// How long a stalled unit's way may stay no shorter before it gives the
/// move up and holds until its side learns something new of the ground:
/// longer than a truck's three-point turn takes. Waiting for a vehicle in
/// the way is traffic, not a stall.
const GIVE_UP_S: f64 = 30.0;
/// How much shorter its way must have become to count as progress.
const STALL_GAIN_M: f64 = 2.0;
/// Progress shorter than this does not reset the stall watch.
const PROGRESS_EPSILON_M: f64 = 0.5;
/// A vehicle planning its way out of a traffic knot plans round every
/// vehicle within this distance of it.
const KNOT_M: f64 = 40.0;
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
#[derive(Clone)]
pub struct SideGeometry {
    /// Every body this side places somewhere other than where it truly
    /// stands, or learned after setup: each learned body where it was last
    /// seen, and each authored body shoved out of sight where it stood
    /// before (L1). Any other authored body stands where it is.
    pub seen: BTreeMap<PropId, Seen>,
    /// Bodies gone out of this side's sight (trees knocked down, props
    /// destroyed): it plans around them, and draws them, until it sees the
    /// ground where they stood (Q16, Q17, L1).
    standing: BTreeMap<PropId, Prop>,
    /// The same remembered snapshots, indexed for sight queries.
    standing_index: PropIndex,
    pub revision: u64,
    /// Where each of the latest revisions changed the side's plan, oldest
    /// first, so a squad re-resolves only for changes within its reach.
    changes: VecDeque<Change>,
    /// The side's planning grid as of revision `grid_revision`: the map's
    /// grid, with what the side had learned by then.
    grid: NavGrid,
    grid_revision: u64,
    /// The bodies the side may place otherwise than its grid has them.
    stale: BTreeSet<PropId>,
    /// How many of the world's cleared ground cells the grid has taken in.
    cleared_taken: usize,
    /// Route searches run for this side (the no-per-frame-search contract).
    pub searches: u64,
    /// Where this side's aircraft may fly, and the side and world revisions
    /// it was built at; built when an aircraft first asks.
    air: Option<(u64, u64, Arc<crate::navigation::air::AirGrid>)>,
    /// Cells the air route searches expanded (the bounded-search contract).
    pub air_cells_searched: u64,
}

/// A change farther than this from a unit's route cannot put a body in
/// its way: the widest clearance a footprint is judged by, and a cell more.
const ROUTE_RECHECK_REACH_M: f64 = 24.0;

impl SideGeometry {
    /// The scratch world owns a fresh clearance journal; its cursor starts there.
    pub(crate) fn planning_snapshot(&self) -> Self {
        let mut snapshot = self.clone();
        snapshot.cleared_taken = 0;
        snapshot
    }

    /// A side that knows the map as `base` has it, and nothing else.
    pub fn new(base: Arc<NavBase>) -> Self {
        let [width, depth] = base.extent();
        SideGeometry {
            seen: BTreeMap::new(),
            standing: BTreeMap::new(),
            standing_index: PropIndex::new(width, depth, PROP_BUCKET_M),
            revision: 0,
            changes: VecDeque::new(),
            grid: NavGrid::new(base),
            grid_revision: 0,
            stale: BTreeSet::new(),
            cleared_taken: 0,
            searches: 0,
            air: None,
            air_cells_searched: 0,
        }
    }

    /// The side's air grid: walls are the bodies it believes stand taller
    /// than the obstacle height, where it believes they stand (D32).
    pub fn air_grid(
        &mut self,
        world: &WorldGeometry,
        rules: &contract::scenario::Rules,
        authored: PropId,
    ) -> Arc<crate::navigation::air::AirGrid> {
        use crate::navigation::air::{walls_aircraft, AirGrid};
        let stamp = (self.revision, world.obstacle_revision());
        if let Some((side, at, grid)) = &self.air {
            if (*side, *at) == stamp {
                return grid.clone();
            }
        }
        let walls: Vec<Prop> = world
            .props()
            .chain(self.standing.values())
            .filter(|p| walls_aircraft(world, rules, p))
            .filter_map(|p| self.belief(p, authored))
            .filter(|p| walls_aircraft(world, rules, p))
            .collect();
        let grid = Arc::new(AirGrid::build(world, rules, walls.iter()));
        self.air = Some((stamp.0, stamp.1, grid.clone()));
        grid
    }

    /// Cells the side's grid has worked out again as the side learned.
    pub fn cells_relaid(&self) -> u64 {
        self.grid.relaid()
    }

    /// The world changed these props (added, moved, removed or made known
    /// to all): what the side believes of each may have changed with it.
    pub fn touch(&mut self, props: &[PropId]) {
        self.stale.extend(props);
    }

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
            Some(was) if !resting && (was.center - now.center).within_radius(relearn_m) => return,
            Some(_) => {}
        }
        let was = self.seen.insert(prop.id, now).map(|w| w.center);
        self.stale.insert(prop.id);
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
            .any(|c| (c.at - at).within_radius(reach + c.radius))
    }

    /// Whether any change after revision `since` came within `reach` of
    /// the way from `from` along `route`: always, once the log no longer
    /// reaches back that far.
    pub fn changed_along(&self, since: u64, from: V2, route: &[V2], reach: f64) -> bool {
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
            .any(|c| {
                let mut a = from;
                route.iter().any(|&b| {
                    let ab = b - a;
                    let len2 = ab.dot(ab);
                    let t = if len2 > 0.0 {
                        ((c.at - a).dot(ab) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let near = (a + ab * t - c.at).within_radius(reach + c.radius);
                    a = b;
                    near
                })
            })
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
            self.stale.insert(prop.id);
        }
    }

    /// A body this side did not see go: it keeps planning around it.
    pub fn keep_standing(&mut self, prop: Prop) {
        self.stale.insert(prop.id);
        if let Some(was) = self.standing.remove(&prop.id) {
            self.standing_index.remove(&was);
        }
        self.standing_index.insert(&prop);
        self.standing.insert(prop.id, prop);
    }

    /// Read-only remembered snapshots; mutations keep the sight index in sync.
    pub(crate) fn standing(&self) -> &BTreeMap<PropId, Prop> {
        &self.standing
    }

    /// Ascending unique remembered candidates across all visibility views.
    pub(crate) fn standing_ids_near_many(&self, views: &[(V2, f64)], out: &mut Vec<PropId>) {
        self.standing_index.near_many(views, out);
    }

    /// The side sees that a body it kept standing is gone.
    pub fn saw_fallen(&mut self, prop: PropId) {
        if let Some(gone) = self.standing.remove(&prop) {
            self.standing_index.remove(&gone);
            let was = self.seen.remove(&prop).map(|s| s.center);
            self.stale.insert(prop);
            self.changed(span(&gone, was));
        }
    }

    /// A prop this side plans with is gone: plan again without it.
    pub fn forget(&mut self, prop: &Prop) {
        let was = self.seen.remove(&prop.id).map(|s| s.center);
        self.stale.insert(prop.id);
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

    /// Where this side plans otherwise than `world` stands: each body added
    /// since setup it does not know (none), each it places elsewhere, and
    /// each gone that it keeps standing. Every other body of `world` it plans
    /// with as it stands, so this costs what the side has learned, not the
    /// map's size.
    pub(crate) fn beliefs<'a>(
        &'a self,
        world: &'a WorldGeometry,
        authored: PropId,
    ) -> impl Iterator<Item = (PropId, Option<Prop>)> + 'a {
        let unknown = world
            .props_from(authored)
            .filter(move |p| !self.knows(p, authored))
            .map(|p| (p.id, None));
        let placed = self
            .seen
            .keys()
            .filter_map(move |&id| Some((id, self.belief(world.prop(id)?, authored))));
        let standing = self.standing.values().map(|p| (p.id, Some(p.clone())));
        unknown.chain(placed).chain(standing)
    }

    /// The world as this side plans in it ([`Self::beliefs`]).
    pub(crate) fn planning_world(&self, world: &WorldGeometry, authored: PropId) -> WorldGeometry {
        world.planning_snapshot(self.beliefs(world, authored))
    }

    /// A body where this side knows it, including an unseen collapse.
    pub fn prop(&self, world: &WorldGeometry, authored: PropId, id: PropId) -> Option<Prop> {
        self.belief(world.prop(id).or_else(|| self.standing.get(&id))?, authored)
    }

    /// The side's planning grid. It refreshes changed body beliefs and
    /// newly cleared ground, which advance independently.
    pub fn grid(&mut self, world: &WorldGeometry, authored: PropId) -> &NavGrid {
        // Body beliefs and cleared ground advance independently.
        if self.grid_revision != self.revision
            || self.cleared_taken != world.cleared_cells() as usize
        {
            self.stale.extend(world.touched());
            let beliefs: Vec<(PropId, Option<Prop>)> = std::mem::take(&mut self.stale)
                .into_iter()
                .map(|id| (id, self.prop(world, authored, id)))
                .collect();
            let cleared = world.cleared_since(self.cleared_taken);
            self.cleared_taken = world.cleared_cells() as usize;
            self.grid.update(world, beliefs.into_iter(), cleared);
            self.grid_revision = self.revision;
        }
        &self.grid
    }

    /// The side's grid as it stands once it has taken in everything the
    /// side believes now, beside a grid built whole from those beliefs.
    #[cfg(test)]
    pub(crate) fn grid_beside_whole_build(
        &mut self,
        world: &WorldGeometry,
        authored: PropId,
        soldier_radius: f64,
    ) -> (&NavGrid, NavGrid) {
        let believed: Vec<Prop> = world
            .props()
            .chain(self.standing.values())
            .filter_map(|p| self.belief(p, authored))
            .collect();
        let whole = NavBase::build(world, believed.iter(), soldier_radius);
        (self.grid(world, authored), NavGrid::new(Arc::new(whole)))
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
    /// The map's road graph: the same for both sides.
    pub roads: &'a RoadNet,
    pub ground: &'a GroundLayer,
    /// Props with ids below this were authored with the map and are known to all.
    pub authored: PropId,
    pub tick: Tick,
    /// The battle's seed: arrangements are drawn from it (D1).
    pub seed: u64,
    pub rules: &'a contract::scenario::Rules,
    /// Each side's knowledge: the enemies a squad takes cover from (Q7).
    pub knowledge: [&'a crate::knowledge::SideKnowledge; 2],
    /// The weapons: how far a squad's reach when it seeks where to fight.
    pub arsenal: &'a crate::weapons::Arsenal,
}

/// Move every living unit one tick. Returns the bodies vehicles shoved,
/// for the caller to move in the world (movement reads it, never writes it).
pub(crate) fn advance(
    ctx: &MovementContext,
    units: &mut [Unit],
    sides: &mut [SideGeometry; 2],
    planner: &mut RoutePlanner,
    completed: &mut impl FnMut(crate::battle::TickPhase),
) -> Vec<Shove> {
    let mut footprints: Vec<Option<Obb2>> = units
        .iter()
        .map(|u| u.ground_footprint().filter(|_| u.alive()))
        .collect();
    let field = take_cover::Field::gather(ctx, units);
    completed(crate::battle::TickPhase::Movement);
    for unit in units.iter_mut() {
        // The destroyed stay put; a wreck is an obstacle prop, not traffic.
        if !unit.alive() {
            planner.cancel(unit.id);
            continue;
        }
        // Aircraft plan their own routes as they fly (`air`).
        if unit.airborne() {
            continue;
        }
        let s = unit.side.index();
        request_route(ctx, unit, &mut sides[s], &footprints, planner);
    }
    plan_routes(ctx, units, sides, &field, planner);
    completed(crate::battle::TickPhase::Navigation);
    // The infantry snapshot shares each hull's unchanged circle bound this tick.
    let hulls: Vec<(Obb2, f64)> = footprints
        .iter()
        .flatten()
        .map(|&hull| (hull, hull.half.length()))
        .collect();
    // Every live vehicle on the move, either side, as soldiers see it coming.
    let threats: Vec<Threat> = units
        .iter()
        .filter(|u| u.alive())
        .filter_map(|u| Threat::of(ctx, u))
        .collect();
    let mut crowd = soldier::Crowd::gather(units, ctx.rules.physics.soldier_radius_m);
    let mut shoves = Vec::new();
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        if units[i].airborne() {
            air::step_aircraft(ctx, units, i, sides);
            continue;
        }
        if units[i].is_vehicle() {
            step_vehicle(ctx, units, i, sides, &mut shoves, &footprints);
            footprints[i] = units[i].ground_footprint().filter(|_| units[i].alive());
            soldier::shove(ctx, units, i, &mut crowd);
        } else {
            let unit = &mut units[i];
            let advancing = may_advance(ctx, unit);
            let side = &mut sides[unit.side.index()];
            if !advancing && unit.garrison.is_none() && (unit.route.is_none() || unit.halted()) {
                take_cover::hold(ctx, unit, side, &field);
            }
            soldier::step_squad(ctx, unit, i, side, &hulls, &threats, &mut crowd, advancing);
            crowd.refresh_radius(unit, ctx.rules.physics.soldier_radius_m);
        }
    }
    shoves
}

/// Ask the planner for a route when the unit needs one: a new goal, a route
/// its side now knows is blocked, or a stall. It holds until the route comes
/// ([`plan_routes`]); a goal that moves on meanwhile asks again.
fn request_route(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &mut SideGeometry,
    footprints: &[Option<Obb2>],
    planner: &mut RoutePlanner,
) {
    prefer_roads(ctx, unit);
    let Some((goal, policy)) = unit.movement_goal() else {
        planner.cancel(unit.id);
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
        .is_none_or(|g| (g - goal).outside_radius(GOAL_REPLAN_M));
    if planner.pending(unit.id).is_some() && !goal_moved {
        return;
    }
    let changed = unit.planned_revision != side.revision;
    let stall_ticks = (STALL_REPLAN_S * ctx.rules.tick_hz as f64) as u64;
    let stalled = unit.route.is_some() && ctx.tick.saturating_sub(unit.progress.1) > stall_ticks;
    if goal_moved {
        unit.stalls = (f64::INFINITY, ctx.tick);
    } else if stalled && unit.blocker.is_none() {
        let route = unit.route.as_deref().unwrap_or(&[]);
        let left = way_left(unit.position.xy(), route);
        if left < unit.stalls.0 - STALL_GAIN_M {
            unit.stalls = (left, ctx.tick);
        } else if ctx.tick.saturating_sub(unit.stalls.1)
            > (GIVE_UP_S * ctx.rules.tick_hz as f64) as u64
        {
            planner.cancel(unit.id);
            unit.route = None;
            unit.state = MoveState::RouteBlocked;
            unit.stalls = (f64::INFINITY, ctx.tick);
            unit.planned_revision = side.revision;
            return;
        }
    }
    // A stalled vehicle goes round the whole knot. A waiting rear vehicle
    // must also move aside when the one in front needs room to reverse.
    let detour: Vec<Obb2> = if stalled && unit.is_vehicle() {
        let here = unit.position.xy();
        footprints
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != unit.id.0 as usize)
            .filter_map(|(_, hull)| *hull)
            .filter(|hull| (hull.center - here).within_radius(KNOT_M))
            .map(|hull| Obb2 {
                half: hull.half + v2(TRAFFIC_MARGIN_M, TRAFFIC_MARGIN_M),
                ..hull
            })
            .collect()
    } else {
        Vec::new()
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
            // against a detour (Q13): one it can still steer round, beyond
            // its nose and, on wheels, its turning radius. Nearer, it is
            // pushed through.
            let swerve = unit.hull.map_or(0.0, |h| h.x)
                + unit
                    .ground()
                    .drive
                    .filter(|d| !d.tracked)
                    .map_or(0.0, |d| d.radius_m);
            stalled
                || (changed
                    && side.changed_along(
                        unit.planned_revision,
                        from,
                        route,
                        ROUTE_RECHECK_REACH_M,
                    )
                    && {
                        let grid = side.grid(ctx.world, ctx.authored);
                        !grid.route_fits(from, route, unit.ground())
                            || grid.route_pushes(from, route, unit.ground(), swerve)
                    })
        }
    };
    unit.planned_revision = side.revision;
    if !needs {
        return;
    }
    side.searches += 1;
    unit.planned_goal = Some(goal);
    // It holds without a route while it waits; a detour keeps the one it has.
    let kept = unit.route.take().filter(|_| !detour.is_empty());
    unit.state = MoveState::Planning;
    let from = {
        let grid = side.grid(ctx.world, ctx.authored);
        route_start(unit, |p| grid.placement_fits(p, unit.ground()))
    };
    planner.submit(
        unit.id,
        Request {
            side: unit.side,
            from,
            goal,
            mobility: *unit.ground(),
            policy,
            detour,
            kept,
            new_goal: goal_moved,
        },
    );
}

/// Metres from `here` along `route` to its end.
fn way_left(here: V2, route: &[V2]) -> f64 {
    let mut at = here;
    route.iter().fold(0.0, |sum, p| {
        let leg = (*p - at).length();
        at = *p;
        sum + leg
    })
}

/// A long ordered leg goes by road: a move or attack-move leg strictly over
/// `navigation.road_leg_m` from where the unit stands when the leg starts
/// takes the fastest policy, and keeps it however short the leg has become
/// when it plans again. Pursuit and a building's approach are never legs.
fn prefer_roads(ctx: &MovementContext, unit: &mut Unit) {
    if unit.planned_goal.is_some() || unit.garrison.is_some() {
        return;
    }
    let from = route_start(unit, |_| true);
    if let Some(leg) = unit.orders.front_mut().and_then(|o| o.movement_mut()) {
        if (leg.destination - from).outside_radius(ctx.rules.navigation.road_leg_m) {
            leg.policy = contract::command::RoutePolicy::Fastest;
        }
    }
}

/// Spend this tick's planning work, and give every unit whose route is
/// whole its route.
fn plan_routes(
    ctx: &MovementContext,
    units: &mut [Unit],
    sides: &mut [SideGeometry; 2],
    field: &take_cover::Field,
    planner: &mut RoutePlanner,
) {
    let waiting = planner.sides();
    // Each side's grid as it knows the map now, built only if it is needed.
    fn known<'a>(ctx: &MovementContext, side: &'a mut SideGeometry) -> (&'a NavGrid, u64) {
        let revision = side.revision;
        let grid = side.grid(ctx.world, ctx.authored);
        (grid, revision)
    }
    let [blue, red] = &mut *sides;
    let ready = planner.advance(
        &ctx.rules.navigation,
        ctx.roads,
        [
            waiting[0].then(|| known(ctx, blue)),
            waiting[1].then(|| known(ctx, red)),
        ],
    );
    for (id, request, plan) in ready {
        let unit = &mut units[id.0 as usize];
        take_route(ctx, unit, &sides[unit.side.index()], field, request, plan);
        line_up(unit, sides[unit.side.index()].grid(ctx.world, ctx.authored));
    }
}

/// A wheeled vehicle sent forward to a right-drag's facing drives in along
/// it: its route ends on a straight lead-in on the heading it can line up on
/// ([`drive::line_up_heading`]), long enough that the turn onto it is a
/// forward one ([`drive::lead_in`]). Without room for it, the vehicle keeps
/// the route it has.
fn line_up(unit: &mut Unit, grid: &NavGrid) {
    let Some(drive) = unit.ground().drive.filter(|d| !d.tracked) else {
        return;
    };
    let Some(order) = unit.orders.front().and_then(|o| o.movement()) else {
        return;
    };
    let (Some(facing), MoveDirection::Forward) = (order.facing, order.direction) else {
        return;
    };
    let Some(route) = unit
        .route
        .as_ref()
        .filter(|r| r.last() == Some(&order.destination))
    else {
        return;
    };
    let end = order.destination;
    let here = unit.position.xy();
    let before = route.len().checked_sub(2).map_or(here, |i| route[i]);
    let Some(heading) = drive::line_up_heading(before, end, facing) else {
        return;
    };
    let lead = drive::lead_in(end, heading, &drive, unit.hull.map_or(0.0, |h| h.x));
    let mut lined = route[..route.len() - 1].to_vec();
    lined.extend([lead, end]);
    if grid.placement_fits(lead, unit.ground()) && grid.route_fits(here, &lined, unit.ground()) {
        unit.route = Some(lined);
    }
}

/// The planner's answer to `request` arrives: the unit sets off on the
/// route, or learns there is none. It held while it waited, so the route
/// starts where it stands; a squad's soldiers, who may have shifted to
/// cover meanwhile, join the corridor from wherever they are.
fn take_route(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &take_cover::Field,
    request: Request,
    plan: Plan,
) {
    let from = request.from;
    unit.planned_revision = side.revision;
    unit.progress = (f64::INFINITY, ctx.tick);
    if matches!(plan, Plan::Route(_)) {
        unit.manoeuvre = None;
    }
    if !request.detour.is_empty() {
        // No way round keeps it waiting on the route it had; it may try
        // again after another stall.
        unit.route = match plan {
            Plan::Route(mut route) => {
                // Traffic can temporarily cover the destination and snap the
                // detour's end aside. Finish the original order after passing it.
                if route.last() != Some(&request.goal) {
                    route.push(request.goal);
                }
                Some(route)
            }
            Plan::Blocked(_) => request.kept,
        };
        unit.state = MoveState::Waiting;
        return;
    }
    let Some(plan) = weigh_stop(unit, &request, plan) else {
        return;
    };
    match plan {
        Plan::Route(route) => {
            if !unit.is_vehicle() {
                let end = *route.last().expect("a route ends somewhere");
                // A new goal draws a new arrangement; a replan toward the
                // same one keeps every soldier's spot.
                if request.new_goal {
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

/// A unit sent to the room nearest ground it could not stand on stops at
/// the room on its way instead ([`crate::units::StopShort`]) when the way to
/// the nearest runs longer than the way to it by more than the ground it
/// gains is worth. The route to the nearest is weighed first against the
/// straight line to the room on the way, which no route undercuts; only if
/// that cannot settle it is the way to the room on the way planned, and the
/// two routes weighed. `None`: the unit plans again.
fn weigh_stop(unit: &mut Unit, request: &Request, plan: Plan) -> Option<Plan> {
    let order = unit
        .orders
        .front_mut()
        .and_then(|o| o.movement_mut())
        .filter(|o| o.destination == request.goal);
    let Some(order) = order else {
        return Some(plan);
    };
    let Some(short) = order.short.take() else {
        return Some(plan);
    };
    let from = request.from;
    match (short.nearest, plan) {
        // The way to the nearest room: plainly worth it, or none.
        (None, Plan::Route(route))
            if way_left(from, &route) <= (short.at - from).length() + short.detour_m =>
        {
            Some(Plan::Route(route))
        }
        (None, Plan::Route(route)) => {
            let nearest = order.destination;
            order.destination = short.at;
            order.short = Some(crate::units::StopShort {
                nearest: Some((nearest, route)),
                ..short
            });
            unit.route = None;
            None
        }
        (None, blocked) => Some(blocked),
        // The way to the room on the way: the nearest if it gains enough.
        (Some((nearest, kept)), plan) => {
            let worth = match &plan {
                Plan::Route(route) => {
                    way_left(from, &kept) - way_left(from, route) <= short.detour_m
                }
                Plan::Blocked(_) => true,
            };
            if !worth {
                return Some(plan);
            }
            order.destination = nearest;
            unit.planned_goal = Some(nearest);
            Some(Plan::Route(kept))
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
        // at rest; wheels drove in along it as far as they could (`line_up`).
        match m.facing {
            Some(f) if !unit.is_vehicle() => unit.yaw = f,
            Some(f) if unit.airborne() || unit.ground().drive.is_some_and(|d| d.tracked) => {
                unit.turn_to = Some(f)
            }
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
    traffic: &[Option<Obb2>],
) {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
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
                j != i
                    && o.ground_footprint().is_some()
                    && o.alive()
                    && vehicle_conflict(unit, here, unit.yaw, o)
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
        unit.ground().speed(s.road_factor, s.forest, s.slope_deg)
    });
    // Craters under the hull slow it slightly; never to a stop (Q8).
    let speed = speed * ctx.ground.vehicle_speed(here.x, here.y, &ctx.rules.ground);
    let before_manoeuvre = unit.manoeuvre;
    let mut motion = drive::steer(ctx.world, &mut units[i], target, speed, dt, traffic);
    let unit = &units[i];
    let mut next = here + motion.heading * motion.step;
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let current = Obb2 {
        center: here,
        yaw: unit.yaw,
        half,
    };
    let push = unit.ground().push;
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
            traffic,
        );
        next = here + motion.heading * motion.step;
        met = push::meet(ctx.world, &hull_at(next, motion.yaw), &current, push);
    }
    // Traffic, whatever its side (Q14): a vehicle waits for whatever it
    // would run into; live vehicles are never shoved (Q15).
    let unit = &units[i];
    let mut blocker = motion.blocker.or_else(|| {
        units.iter().enumerate().find_map(|(j, other)| {
            (j != i
                && other.ground_footprint().is_some()
                && other.alive()
                && vehicle_conflict(unit, next, motion.yaw, other))
            .then_some(other.id)
        })
    });

    // A detouring tank must not pivot through its neighbour, nor a tank
    // nosed up to a body through that body. Give the turn room instead of
    // deadlocking nose-to-nose: against a body, only when backing off by
    // what its corners swing out past its nose would free the turn.
    let back = drive::give_space(unit, speed, dt);
    if unit.ground().drive.is_some_and(|d| d.tracked)
        && motion.yaw != unit.yaw
        && (blocker.is_some()
            || (motion.step == 0.0
                && met.solid.is_some()
                && backing_frees_pivot(ctx.world, unit, back.heading, motion.yaw)))
        && stationary_yaw(ctx.world, units, i, motion.yaw) == unit.yaw
    {
        let at = here + back.heading * back.step;
        let behind = push::meet(ctx.world, &hull_at(at, back.yaw), &current, push);
        let traffic = units.iter().enumerate().any(|(j, other)| {
            j != i
                && other.ground_footprint().is_some()
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
    let radius = unit.footprint_radius(ctx.rules.physics.soldier_radius_m);
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
    // Tracks backing to clear a pivot may move away from the waypoint.
    // Wheeled shuffling must still close on it, or traffic recovery never runs.
    if motion.making_space && motion.step > 0.0 {
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
    if yaw == unit.yaw || !unit.ground().drive.is_some_and(|d| d.tracked) {
        return unit.yaw;
    }
    let here = unit.ground_footprint().expect("a vehicle has a hull");
    let turned = Obb2 { yaw, ..here };
    let met = push::meet(world, &turned, &here, unit.ground().push);
    // A stationary pivot checks true hulls; translation's traffic buffer
    // would forbid using the clearance reserved by the stopped approach.
    let occupied = met.solid.is_some()
        || !met.shoved.is_empty()
        || units.iter().enumerate().any(|(j, other)| {
            j != i
                && other.ground_footprint().is_some()
                && other.alive()
                && turned.overlaps(&other.ground_footprint().expect("a vehicle has a hull"))
        });
    if occupied {
        unit.yaw
    } else {
        yaw
    }
}

/// Whether a hull whose pivot to `yaw` a body blocks would turn clear of the
/// bodies once backed off along `away` by as far as its corners swing out
/// past its nose (half its diagonal less half its length). Squeezed side
/// to side, backing frees nothing, and it does not back away.
fn backing_frees_pivot(world: &WorldGeometry, unit: &Unit, away: V2, yaw: f64) -> bool {
    let here = unit.ground_footprint().expect("a vehicle has a hull");
    let backed = Obb2 {
        center: here.center + away * (here.half.length() - here.half.x),
        ..here
    };
    let turned = Obb2 { yaw, ..backed };
    push::meet(world, &turned, &backed, unit.ground().push)
        .solid
        .is_none()
}

/// Would this vehicle, moved to `next`, run into the vehicle `other`? Only a move that
/// makes an existing overlap no worse is allowed, so touching units can part.
fn vehicle_conflict(unit: &Unit, next: V2, yaw: f64, other: &Unit) -> bool {
    let before = unit.ground_footprint().expect("a vehicle has a hull");
    let after = Obb2 {
        center: next,
        yaw,
        ..before
    };
    let other = other.ground_footprint().expect("a vehicle has a hull");
    hull_conflict(before, after, other)
}

/// Look-ahead and actual steps keep the same clearance, including when parting.
fn hull_conflict(before: Obb2, after: Obb2, other: Obb2) -> bool {
    let padded = |hull: Obb2| Obb2 {
        half: hull.half + v2(TRAFFIC_MARGIN_M, TRAFFIC_MARGIN_M),
        ..hull
    };
    let depth = |hull: Obb2| hull.separation(&other).map_or(0.0, |v| v.length());
    depth(padded(after)) > depth(padded(before)) + 1e-9 || depth(after) > depth(before) + 1e-9
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
        &ctx.rules.infantry_movement,
        ctx.rules.physics.soldier_radius_m,
        &solid,
        &mut draws,
    );
    let mut spots = spots;
    let tiers = take_cover::at_order(ctx, unit, side, field, from, end, &mut spots);
    let mut spots = spots.into_iter().zip(tiers);
    let rules = &ctx.rules.infantry_movement;
    let stagger = rules.stagger_s * ctx.rules.tick_hz as f64;
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
    let r = ctx.rules.physics.soldier_radius_m;
    let living = unit.members.iter().filter(|s| s.alive()).count();
    let reach = arrangement::spread(&ctx.rules.infantry_movement, living)
        .max(ctx.rules.infantry_movement.spacing_m)
        * 2.0;
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
        let spacing = ctx.rules.infantry_movement.spacing_m / 2.0;
        let spot = arrangement::nearest_free(wanted, reach, |p| {
            taken.iter().all(|t| (*t - p).at_least_radius(spacing))
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

/// Where a unit's route starts: a vehicle's hull, or for a squad the living
/// soldier nearest its middle among the caller's standing places (the middle
/// of a squad split by a wall may lie inside it). Planning judges standing by
/// its known grid; measuring a leg's distance needs only the physical member.
fn route_start(unit: &Unit, stands: impl Fn(V2) -> bool) -> V2 {
    let middle = unit.position.xy();
    if unit.is_vehicle() {
        return middle;
    }
    unit.member_positions()
        .map(|p| p.xy())
        .map(|p| (!stands(p), (p - middle).length(), p))
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
        .map_or(middle, |(_, _, p)| p)
}

#[cfg(test)]
mod remembered_sight_tests {
    use super::*;

    #[test]
    fn remembered_body_queries_follow_replacements_and_observed_removals() {
        let rules = serde_json::from_value(crate::fixtures::test_game()).unwrap();
        let map = serde_json::from_value(serde_json::json!({
            "size":[256,256],"fog_cell_m":8,"height_grid_m":4,
            "slope_cutoff_deg":35,"props":[{
                "kind":"crate","center":[20,20],"yaw":0,
                "half_extents":[0.8,0.8,0.6]
            }]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let base = NavBase::build(&world, world.props(), 0.3);
        let mut side = SideGeometry::new(Arc::new(base));
        let mut prop = world.prop(0).unwrap().clone();
        let at = v2(20.0, 20.0);
        side.keep_standing(prop.clone());
        let mut nearby = Vec::new();
        side.standing_ids_near_many(&[(at, 2.0)], &mut nearby);
        assert_eq!(nearby, vec![0]);
        prop.center = v2(200.0, 200.0);
        side.keep_standing(prop);
        nearby.clear();
        side.standing_ids_near_many(&[(at, 2.0)], &mut nearby);
        assert!(nearby.is_empty(), "replaced memory has no old footprint");
        side.standing_ids_near_many(&[(v2(200.0, 200.0), 2.0)], &mut nearby);
        assert_eq!(nearby, vec![0]);
        side.saw_fallen(0);
        nearby.clear();
        side.standing_ids_near_many(&[(v2(200.0, 200.0), 2.0)], &mut nearby);
        assert!(
            nearby.is_empty(),
            "observed removals leave no remembered body"
        );
    }
}
