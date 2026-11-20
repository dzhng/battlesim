//! Ground movement each tick: ask for a route when one is needed and hold until
//! the planner has it ([`RoutePlanner`]; a vehicle may drive on meanwhile,
//! [`drive_on`]), follow the route toward surface speed, and learn obstacles
//! by running into them. A vehicle drives its route
//! as one hull, waits for every other vehicle whatever its side (Q14) until one
//! standing in its way has stalled it, and not at all on one parked there
//! ([`traffic`]), never for soldiers, and shoves bodies lighter than its push
//! class aside ([`push`]). A wheeled vehicle in a tight spot works out the legs
//! it shunts through ([`shunt`]). A squad plans one corridor on the coarse
//! grid; each soldier walks it as a body of his own ([`soldier`]), with his own
//! route on the exact bodies for the final stretch ([`final_leg`]). Craters
//! slow a driving vehicle here, in integration only: planning never reads them.

use contract::ids::Tick;
use contract::map::MoverClass;
use contract::observation::MoveState;

use crate::arrangement;
use crate::ground::GroundLayer;
use crate::math::{v2, Obb2, V2};
use crate::navigation::{NavGrid, Plan, RoadNet};
use crate::route_planner::{Request, RoutePlanner};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};
use traffic::hull_conflict;

pub(crate) mod air;
mod changes;
mod crowd;
pub(crate) mod drive;
mod drive_on;
mod final_leg;
mod line_up;
mod push;
mod shunt;
mod side;
mod soldier;
mod take_cover;
mod traffic;
mod vehicle;

pub use drive::{final_facing, final_yaw};
pub use final_leg::{final_leg, FinalLeg, FINE_CELL_M};
use line_up::line_up;
pub use push::Shove;
pub use shunt::Shunt;
use side::ROUTE_RECHECK_REACH_M;
pub use side::{Seen, SideGeometry};
pub use soldier::{clear_of, soldier_steer, Around, Corridor, Steer, Threat};
use vehicle::step_vehicle;

/// A pursuit goal this far from the planned one is replanned.
const GOAL_REPLAN_M: f64 = 5.0;
/// Seconds without closing on the next waypoint before a route is replanned.
const STALL_REPLAN_S: f64 = 2.0;
/// How long a unit may get nowhere before it stops near enough to its goal
/// or finds no way, and how long it holds blocked before it tries again
/// (unless its side learns something new of the ground first): longer than
/// a truck's shunt out of a tight spot takes. Waiting behind a rolling vehicle is
/// traffic, not a stall.
const GIVE_UP_S: f64 = 30.0;
/// A unit that has got no farther than this from where its stuck clock
/// started is not getting anywhere, however its routes change; a reversing
/// leg can take a vehicle most of this way and back.
const STUCK_REACH_M: f64 = 15.0;
/// A vehicle slower than this is standing, not rolling: waiting behind it
/// is not traffic (two vehicles waiting for each other would never move).
const ROLLING_MPS: f64 = 0.1;
/// A unit that has found no way to its goal this many times gives the
/// order up and stands, ready for another.
const MAX_TRIES: u8 = 3;
/// A vehicle sent on to a goal this near the one it is driving to keeps
/// the whole of its way while the new one is worked out.
const KEEP_ROUTE_M: f64 = 25.0;
/// A unit stuck this near its goal, past its own hull, is as near as the
/// ground lets it: it stops there and the move is done. A carrier this near
/// a squad it collects calls it over (`carriage`).
pub(crate) const NEAR_GOAL_M: f64 = 12.0;
/// Progress shorter than this does not reset the stall watch.
const PROGRESS_EPSILON_M: f64 = 0.5;
/// A vehicle planning its way out of a traffic knot plans round every
/// vehicle within this distance of it.
const KNOT_M: f64 = 40.0;
/// Extra room vehicles keep from other bodies.
const TRAFFIC_MARGIN_M: f64 = 0.4;
/// A unit learns of an obstacle within this distance of its footprint.
const ENCOUNTER_RANGE_M: f64 = 2.0;

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
    pub knowledge: [&'a crate::knowledge::SideKnowledge; 3],
    /// The weapons: how far a squad's reach when it seeks where to fight.
    pub arsenal: &'a crate::weapons::Arsenal,
}

/// Move every living unit one tick. Returns the bodies vehicles shoved,
/// for the caller to move in the world (movement reads it, never writes it).
pub(crate) fn advance(
    ctx: &MovementContext,
    units: &mut [Unit],
    sides: &mut [SideGeometry; 3],
    planner: &mut RoutePlanner,
    completed: &mut impl FnMut(crate::battle::TickPhase),
) -> Vec<Shove> {
    let mut footprints: Vec<Option<Obb2>> = units
        .iter()
        .map(|u| u.ground_footprint().filter(|_| u.on_field()))
        .collect();
    let field = take_cover::Field::gather(ctx, units);
    completed(crate::battle::TickPhase::Movement);
    // Who is rolling: waiting behind one of them is traffic.
    let rolling: Vec<bool> = units
        .iter()
        .map(|u| u.on_field() && u.drive_speed_mps.abs() > ROLLING_MPS)
        .collect();
    for unit in units.iter_mut() {
        // The destroyed stay put; a wreck is an obstacle prop, not traffic.
        if !unit.on_field() {
            planner.cancel(unit.id);
            continue;
        }
        // Aircraft plan their own routes as they fly (`air`).
        if unit.flies() {
            continue;
        }
        let s = unit.side.index();
        request_route(ctx, unit, &mut sides[s], &footprints, &rolling, planner);
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
        .filter(|u| u.on_field())
        .filter_map(|u| Threat::of(ctx, u))
        .collect();
    let mut crowd = soldier::Crowd::gather(units, ctx.rules.physics.soldier_radius_m);
    let mut shoves = Vec::new();
    for i in 0..units.len() {
        if !units[i].on_field() {
            continue;
        }
        if units[i].flies() {
            // The flight model picks the flight law, never the layer.
            if units[i].fixed_wing() {
                crate::air::step_fixed_wing(ctx, &mut units[i]);
            } else {
                air::step_aircraft(ctx, units, i, sides);
            }
            continue;
        }
        if units[i].is_vehicle() {
            let awaiting = (planner.pending(units[i].id)).is_some_and(|r| r.new_goal);
            step_vehicle(ctx, units, i, sides, &mut shoves, &footprints, awaiting);
            footprints[i] = units[i].ground_footprint().filter(|_| units[i].on_field());
            soldier::shove(ctx, units, i, &mut crowd);
        } else {
            let unit = &mut units[i];
            let advancing = may_advance(ctx, unit);
            let side = &mut sides[unit.side.index()];
            if !advancing && unit.hold.is_none() && (unit.route.is_none() || unit.halted()) {
                take_cover::hold(ctx, unit, side, &field);
            }
            soldier::step_squad(ctx, unit, i, side, &hulls, &threats, &mut crowd, advancing);
            crowd.refresh_radius(unit, ctx.rules.physics.soldier_radius_m);
        }
    }
    shoves
}

/// Ask the planner for a route when the unit needs one: a new goal, a route
/// its side now knows is blocked, or a stall. A vehicle with a way to the
/// same place drives on along it meanwhile, and one sent somewhere broadly
/// ahead along the stretch its brakes need ([`drive_on`]); otherwise the
/// unit holds until the route comes ([`plan_routes`]). A goal that moves on meanwhile asks
/// again. A unit that gets nowhere for a give-up spell stops if it is near
/// enough to its goal, and otherwise finds no way, tries again, and the
/// third time gives the order up: no unit is stuck for good.
fn request_route(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &mut SideGeometry,
    footprints: &[Option<Obb2>],
    rolling: &[bool],
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
    // A route on its way to (near) the same place is worth waiting for: an
    // order again does not throw the search away.
    let pending = planner
        .pending(unit.id)
        .filter(|r| !goal_moved || (r.goal - goal).within_radius(KEEP_ROUTE_M));
    if let Some(request) = pending {
        // Holding for its route somewhere new is planning, not being stuck;
        // working out a new way to where it is already stuck is not.
        if unit.route.is_none() && request.new_goal {
            unit.restart_stalls(ctx.tick);
        }
        return;
    }
    let changed = unit.planned_revision != side.revision;
    let stall_ticks = (STALL_REPLAN_S * ctx.rules.tick_hz as f64) as u64;
    let stalled = unit.route.is_some() && ctx.tick.saturating_sub(unit.progress.1) > stall_ticks;
    // The stuck clock: where it stood when the clock started, and when. A
    // new route does not wind it back, nor an order again to the same
    // place; getting well away from that spot does, a goal somewhere new
    // does, and waiting behind a vehicle that is rolling holds it (traffic).
    let same_place = unit
        .route
        .as_ref()
        .and_then(|r| r.last())
        .is_some_and(|&end| (end - goal).within_radius(KEEP_ROUTE_M));
    let traffic = unit.blocker.is_some_and(|b| rolling[b.0 as usize]);
    let here = unit.position.xy();
    if goal_moved && !same_place {
        unit.restart_stalls(ctx.tick);
        unit.tries = 0;
    } else if traffic {
        unit.restart_stalls(ctx.tick);
    } else if unit.route.is_some() {
        // A squad's men getting nearer their spots wind it back too
        // (`soldier::step_squad`).
        if (here - unit.stalls.0).outside_radius(STUCK_REACH_M) {
            unit.restart_stalls(ctx.tick);
        } else if ctx.tick.saturating_sub(unit.stalls.1)
            > (GIVE_UP_S * ctx.rules.tick_hz as f64) as u64
        {
            planner.cancel(unit.id);
            let reach = unit.hull.map_or(0.0, |h| h.x) + NEAR_GOAL_M;
            if (goal - here).within_radius(reach) {
                // As near as the ground (or what stands on the marker) lets it.
                arrive(ctx.rules, unit);
            } else {
                unit.route = None;
                blocked(unit, ctx.tick);
                unit.planned_revision = side.revision;
            }
            return;
        }
    }
    // A stalled vehicle goes round the whole knot, and round the bodies its
    // hull is pressed against: one it could shove alone may be jammed
    // against another (no chain shoves). A waiting rear vehicle must also
    // move aside when the one in front needs room to reverse.
    let detour: Vec<Obb2> = if stalled && unit.is_vehicle() {
        let here = unit.position.xy();
        footprints
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != unit.id.0 as usize)
            .filter_map(|(_, hull)| *hull)
            .filter(|hull| (hull.center - here).within_radius(KNOT_M))
            .chain(push::pressing(ctx.world, unit))
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
        // Blocked, it tries again once a give-up spell has passed since it
        // last tried: what its side has learned meanwhile may open a way.
        (_, MoveState::RouteBlocked) => {
            ctx.tick.saturating_sub(unit.stalls.1) > (GIVE_UP_S * ctx.rules.tick_hz as f64) as u64
        }
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
            // pushed through; only bodies new to the route are weighed again.
            let swerve = unit.hull.map_or(0.0, |h| h.x)
                + unit
                    .ground(ctx.rules)
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
                        let weighs = side.reweighs(
                            unit.planned_revision,
                            push::pressed(ctx.world, ctx.rules, unit),
                        );
                        let grid = side.grid(ctx.world, ctx.authored);
                        !grid.route_fits(from, route, &unit.ground(ctx.rules))
                            || grid.route_pushes_where(
                                from,
                                route,
                                &unit.ground(ctx.rules),
                                swerve,
                                &weighs,
                            )
                    })
        }
    };
    unit.planned_revision = side.revision;
    if !needs {
        return;
    }
    // Trying again after finding no way, it gets a whole spell to get
    // somewhere before it can be found stuck again.
    if unit.state == MoveState::RouteBlocked {
        unit.restart_stalls(ctx.tick);
    }
    side.searches += 1;
    unit.planned_goal = Some(goal);
    // A vehicle drives on along the way it has while it works out a new one
    // to the same place (a driver does not stop to rethink), and sent
    // somewhere new broadly ahead, along the stretch its brakes need.
    // Otherwise, or on foot, it holds until the route comes.
    if !(unit.is_vehicle() && same_place) {
        unit.route = (goal_moved.then(|| drive_on::stretch(ctx, unit, goal))).flatten();
    }
    if unit.route.is_none() {
        unit.state = MoveState::Planning;
    }
    let from = {
        let grid = side.grid(ctx.world, ctx.authored);
        route_start(unit, |p| grid.placement_fits(p, &unit.ground(ctx.rules)))
    };
    planner.submit(
        unit.id,
        Request {
            side: unit.side,
            from,
            goal,
            mobility: unit.ground(ctx.rules),
            policy,
            detour,
            new_goal: goal_moved,
        },
    );
}

/// Drop the waypoints of `route` (planned from `from`) that a vehicle now
/// at `here` has passed: it picks the route up at its stretch nearest it.
pub(super) fn rejoin(here: V2, from: V2, route: &mut Vec<V2>) {
    let mut a = from;
    let mut nearest = (f64::INFINITY, 0);
    for (k, &b) in route.iter().enumerate() {
        let ab = b - a;
        let t = ((here - a).dot(ab) / ab.dot(ab).max(1e-12)).clamp(0.0, 1.0);
        let d = (a + ab * t - here).length();
        if d < nearest.0 {
            nearest = (d, k);
        }
        a = b;
    }
    route.drain(..nearest.1);
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
    if unit.planned_goal.is_some() || unit.hold.is_some() {
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
    sides: &mut [SideGeometry; 3],
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
    let [blue, red, neutral] = &mut *sides;
    let ready = planner.advance(
        &ctx.rules.navigation,
        ctx.roads,
        [
            waiting[0].then(|| known(ctx, blue)),
            waiting[1].then(|| known(ctx, red)),
            waiting[2].then(|| known(ctx, neutral)),
        ],
    );
    for (id, request, plan, revision) in ready {
        let unit = &mut units[id.0 as usize];
        take_route(
            ctx,
            unit,
            &sides[unit.side.index()],
            field,
            request,
            plan,
            revision,
        );
        line_up(
            ctx.rules,
            unit,
            sides[unit.side.index()].grid(ctx.world, ctx.authored),
        );
    }
}

/// The planner's answer to `request` arrives: the unit sets off on the
/// route, or learns there is none. Unless it is a vehicle that drove on
/// meanwhile, it held while it waited, so the route starts where it stands;
/// a squad's soldiers, who may have shifted to cover meanwhile, join the
/// corridor from wherever they are.
fn take_route(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &take_cover::Field,
    request: Request,
    mut plan: Plan,
    revision: u64,
) {
    let from = request.from;
    // What its side learned while the route was worked out is checked
    // along it on the next tick, as every later change is.
    unit.planned_revision = revision;
    unit.progress = (f64::INFINITY, ctx.tick);
    // A vehicle that drove on meanwhile joins the new route where it is.
    if let (true, Plan::Route(route)) = (unit.is_vehicle(), &mut plan) {
        rejoin(unit.position.xy(), from, route);
    }
    // A shunt under way is finished first, as a driver gets clear of a
    // tight spot whatever his next order, then picks the new route up from
    // where it stands; one that found no way looks again along the new route.
    if matches!(plan, Plan::Route(_)) {
        let here = unit.position.xy();
        unit.shunt = unit
            .shunt
            .take()
            .filter(|s| !s.legs.is_empty())
            .map(|s| Shunt { from: here, ..s });
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
            Plan::Blocked(_) => unit.route.take(),
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
            blocked(unit, ctx.tick);
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
    unit.restart_stalls(ctx.tick);
    unit.state = held;
    false
}

/// No way to its goal: it says so and tries again later, but the third time
/// it gives the order up and stands, so no unit is blocked for good.
fn blocked(unit: &mut Unit, tick: u64) {
    unit.tries += 1;
    if unit.tries >= MAX_TRIES {
        unit.orders.pop_front();
        unit.planned_goal = None;
        unit.state = if unit.orders.is_empty() {
            MoveState::Idle
        } else {
            MoveState::Moving
        };
        return;
    }
    unit.state = MoveState::RouteBlocked;
    unit.restart_stalls(tick);
}

/// A route's end reached: a destination completes its move; a pursuit point
/// leaves the attack in place for its weapons.
fn arrive(rules: &contract::scenario::Rules, unit: &mut Unit) {
    if let Some(m) = unit.orders.front().and_then(|o| o.movement()) {
        // The right-drag's facing (Q9): a squad turns at once; tracks and
        // rotors pivot at rest; wheels drove in along it as far as they
        // could (`line_up`) and turn the rest of the way on the spot (a
        // deliberate game rule, listed in development.md).
        match m.facing {
            Some(f) if !unit.is_vehicle() => unit.yaw = f,
            Some(f) if unit.flies() || unit.ground(rules).drive.is_some() => unit.turn_to = Some(f),
            _ => {}
        }
        unit.orders.pop_front();
    }
    unit.route = None;
    unit.planned_goal = None;
    unit.shunt = None;
    unit.drive_speed_mps = 0.0;
    unit.state = if unit.orders.is_empty() {
        MoveState::Idle
    } else {
        MoveState::Moving
    };
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
    let mut draws = arrangement::rng(ctx.seed, unit.id.0, arrangement::arrival(end));
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
