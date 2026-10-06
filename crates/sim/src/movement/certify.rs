//! Garrison entry is proven by running the real movement owner on isolated,
//! side-known state (move orders are admitted by reach alone). It drives the two ends of each leg, vehicle bends, the approach to
//! stationary hulls and props, and meetings with movers coming the other way;
//! straight stretches between checkpoints are taken on the planned route
//! ([`carry`]). Rehearsal cost follows physical interactions rather than
//! travel distance. Planning still pays for the route it must find.
use contract::ground::segment_distance;
use contract::map::MoverClass;
use contract::observation::MoveState;

use super::{MovementContext, SideGeometry, KNOT_M, STALL_REPLAN_S, TRAFFIC_MARGIN_M};
use crate::formation::Slot;
use crate::math::{v2, Obb2, V2};
use crate::route_planner::RoutePlanner;
use crate::units::{Unit, UnitOrder};
use crate::world::WorldGeometry;

/// The unit stands where it is, with nothing left to do.
fn stand(unit: &mut Unit) {
    unit.orders.clear();
    unit.route = None;
    unit.planned_goal = None;
    unit.pursuit = None;
    unit.turn_to = None;
    unit.manoeuvre = None;
    unit.drive_speed_mps = 0.0;
    unit.state = MoveState::Idle;
}

/// What is left of a unit's route: where it stands, then its waypoints.
fn remaining_route(unit: &Unit) -> Option<Vec<V2>> {
    let route = unit.route.as_ref().filter(|r| !r.is_empty())?;
    let here = std::iter::once(unit.position.xy());
    Some(here.chain(route.iter().copied()).collect())
}

/// The least distance between segments `a0 a1` and `b0 b1`.
fn segment_gap(a0: V2, a1: V2, b0: V2, b1: V2) -> f64 {
    let (da, db) = (a1 - a0, b1 - b0);
    let side = |d: V2, o: V2, p: V2| d.cross(p - o);
    let crosses =
        side(da, a0, b0) * side(da, a0, b1) < 0.0 && side(db, b0, a0) * side(db, b0, a1) < 0.0;
    if crosses {
        return 0.0;
    }
    let to = |p: V2, s0: V2, s1: V2| segment_distance([s0.x, s0.y], [s1.x, s1.y], [p.x, p.y]);
    to(a0, b0, b1)
        .min(to(a1, b0, b1))
        .min(to(b0, a0, a1))
        .min(to(b1, a0, a1))
}

/// Metres of the way left from each of its points.
fn distances_to_end(way: &[V2]) -> Vec<f64> {
    let mut left = vec![0.0; way.len()];
    for k in (0..way.len() - 1).rev() {
        left[k] = left[k + 1] + (way[k + 1] - way[k]).length();
    }
    left
}

/// Carry `units[index]` along its planned route between physical checkpoints:
/// to `move_rehearsal_m` short of the route's end, or of the next vehicle
/// bend, stationary vehicle or prop its hull meets, or of where it would meet
/// a body coming the other way along its route, which the two must pass.
/// Navigation may admit pushable props, but only movement can demonstrate their shoves.
/// A hull is set down on the route heading the way it was
/// travelling, a squad in file along it, and where other bodies stand there
/// it queues behind them. `false`, and nothing changes, if there is no
/// clear place for it.
fn carry(
    ctx: &MovementContext,
    world: &WorldGeometry,
    units: &mut [Unit],
    index: usize,
    tick: u64,
) -> bool {
    let reach = ctx.rules.navigation.move_rehearsal_m;
    let unit = &units[index];
    let Some(way) = remaining_route(unit) else {
        return false;
    };
    let left = distances_to_end(&way);
    let others = || units.iter().filter(|o| o.id != unit.id && o.alive());
    let radius = unit.footprint_radius(ctx.soldier_radius_m);
    let mut stop = reach;
    for other in others().filter(|o| o.is_vehicle() && o.orders.is_empty()) {
        let c = other.position.xy();
        let near = radius + other.footprint_radius(ctx.soldier_radius_m) + TRAFFIC_MARGIN_M;
        // Where the way first comes by it.
        let met = way.windows(2).enumerate().find_map(|(k, w)| {
            let ab = w[1] - w[0];
            let l2 = ab.dot(ab);
            let t = if l2 > 0.0 {
                ((c - w[0]).dot(ab) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let p = w[0] + ab * t;
            ((p - c).inside_radius(near)).then(|| left[k + 1] + (w[1] - p).length())
        });
        if let Some(met) = met {
            stop = stop.max(met + reach);
        }
    }
    // Where the way first runs against another mover's way, within reach of
    // it: their meeting there is driven, not jumped (where each stands at
    // that moment is only known by driving).
    for other in others().filter(|o| !o.orders.is_empty()) {
        let theirs = remaining_route(other).or_else(|| {
            other
                .movement_goal()
                .map(|(goal, _)| vec![other.position.xy(), goal])
        });
        let Some(theirs) = theirs else {
            continue;
        };
        let near = radius + other.footprint_radius(ctx.soldier_radius_m) + TRAFFIC_MARGIN_M;
        let met = way.windows(2).enumerate().find_map(|(k, w)| {
            let ours = w[1] - w[0];
            theirs.windows(2).find_map(|v| {
                let against = ours.dot(v[1] - v[0]) < 0.0;
                (against && segment_gap(w[0], w[1], v[0], v[1]) < near)
                    .then(|| left[k + 1] + ours.length())
            })
        });
        if let Some(met) = met {
            stop = stop.max(met + reach);
        }
    }
    if let Some(hull) = unit.hull_box() {
        // Navigation proves standing clearance, not steering. Rehearse each
        // bend even when terrain, rather than a prop, constrains its turn.
        for (k, w) in way.windows(3).enumerate() {
            let incoming = (w[1] - w[0]).normalized();
            let outgoing = (w[2] - w[1]).normalized();
            if incoming.dot(outgoing) < 1.0 - 1e-9 {
                stop = stop.max(left[k + 1] + reach);
            }
        }
        for (k, w) in way.windows(2).enumerate() {
            let ab = w[1] - w[0];
            let length = ab.length();
            if length == 0.0 {
                continue;
            }
            // The straight hull sweep, not a radius: a car legally beside
            // the lane does not require a shove rehearsal.
            let sweep = Obb2 {
                center: (w[0] + w[1]) * 0.5,
                yaw: libm::atan2(ab.y, ab.x),
                half: hull.half + v2(length * 0.5, 0.0),
            };
            for prop in world.props_near(sweep.center, sweep.half.length()) {
                if prop.blocks(MoverClass::Vehicle) && sweep.overlaps(&prop.footprint()) {
                    // A long rotated body's near end may cross the lane far
                    // before its centre. The circumscribed hull margin gives
                    // an entry no later than physical contact; the exact sweep
                    // above keeps legal roadside bodies out of this test.
                    if let Some((entry, _)) =
                        prop.footprint()
                            .clip_segment(w[0], w[1], hull.half.length())
                    {
                        stop = stop.max(left[k + 1] + length * (1.0 - entry) + reach);
                    }
                }
            }
        }
    }
    // The place `e` metres short of the end, the travel there, and the
    // first waypoint after it.
    let at = |e: f64| {
        let k = (0..way.len() - 1)
            .rev()
            .find(|&k| left[k] >= e && left[k] > left[k + 1])?;
        let along = (way[k + 1] - way[k]).normalized();
        let p = way[k + 1] - along * (e - left[k + 1]);
        let ground = world.surface_at(p.x, p.y).filter(|s| s.traversable)?;
        Some((p.with_z(ground.z), along, k + 1))
    };
    // Each place tried is a metre further back, as far as the stretch driven.
    let mut tries = (0..=reach as usize)
        .map(|n| stop + n as f64)
        .take_while(|e| e + 1.0 < left[0]);
    let hulls: Vec<Obb2> = others().filter_map(|o| o.hull_box()).collect();
    // Each body's place, its heading, and the first waypoint after it.
    let mut places = Vec::new();
    if let Some(hull) = unit.hull_box() {
        let travel = (way[1] - way[0]).normalized();
        let backwards = v2(libm::cos(unit.yaw), libm::sin(unit.yaw)).dot(travel) < 0.0;
        places.extend(tries.find_map(|e| {
            let (p, along, next) = at(e)?;
            let along = if backwards { along * -1.0 } else { along };
            let yaw = libm::atan2(along.y, along.x);
            let set = Obb2 {
                center: p.xy(),
                yaw,
                ..hull
            };
            let padded = Obb2 {
                half: set.half + v2(TRAFFIC_MARGIN_M, TRAFFIC_MARGIN_M),
                ..set
            };
            let clear = hulls.iter().all(|o| !padded.overlaps(o))
                && world
                    .props_near(set.center, set.half.length())
                    .iter()
                    .all(|b| !b.blocks(MoverClass::Vehicle) || !set.overlaps(&b.footprint()));
            clear.then_some((p, yaw, next))
        }));
    } else {
        let apart = 2.0 * ctx.soldier_radius_m;
        let soldiers: Vec<V2> = others()
            .filter(|o| !o.is_vehicle())
            .flat_map(|o| o.member_positions())
            .map(|p| p.xy())
            .collect();
        let mut tries = tries.step_by(ctx.infantry.spacing_m.ceil().max(1.0) as usize);
        for _ in unit.members.iter().filter(|s| s.alive()) {
            places.extend(tries.find_map(|e| {
                let (p, _, next) = at(e)?;
                let free = soldiers
                    .iter()
                    .all(|q| (*q - p.xy()).at_least_radius(apart))
                    && hulls
                        .iter()
                        .all(|h| !h.contains(p.xy(), ctx.soldier_radius_m));
                free.then_some((p, unit.yaw, next))
            }));
        }
    }
    let bodies = unit.members.iter().filter(|s| s.alive()).count().max(1);
    let Some(&(rear, yaw, next)) = places.last().filter(|_| places.len() == bodies) else {
        return false;
    };
    let unit = &mut units[index];
    unit.route = Some(way[next..].to_vec());
    unit.progress = (f64::INFINITY, tick);
    if unit.is_vehicle() {
        unit.position = rear;
        unit.yaw = yaw;
        unit.drive_speed_mps = 0.0;
        unit.manoeuvre = None;
        unit.blocker = None;
        return true;
    }
    // The corridor now starts where the file's rear man stands; each man
    // walks on from the stretch of it he was set down on.
    unit.route_from = rear.xy();
    for (s, (p, _, after)) in unit.members.iter_mut().filter(|s| s.alive()).zip(places) {
        s.position = p;
        s.path.clear();
        s.leg = after - next;
        s.lateral = 0.0;
    }
    unit.settle();
    true
}

pub(crate) struct Certification {
    pub facings: Vec<Option<f64>>,
    /// The actual reached pose, including garrison exits, for queued origins.
    pub arrivals: Vec<Option<Unit>>,
    /// Failures have been held and all retained arrivals proved again.
    pub stable: bool,
    pub unproven: bool,
}

pub(crate) struct ProofRequest<'a> {
    pub slots: &'a [Slot],
    pub orders: Option<&'a [Vec<UnitOrder>]>,
    pub queued: bool,
    pub reserve_repair: bool,
}

/// The same physical rehearsal for prepared per-unit orders or finite queue prefixes.
pub(crate) fn certify_orders(
    ctx: &MovementContext,
    source: &[Unit],
    known: &SideGeometry,
    request: ProofRequest<'_>,
    allowance: &mut u64,
) -> Certification {
    let ProofRequest {
        slots,
        orders,
        queued,
        reserve_repair,
    } = request;
    let finite = |u: &Unit| {
        u.orders
            .iter()
            .all(|o| !matches!(o, UnitOrder::Attack { .. }))
    };
    let mut active: Vec<bool> = slots
        .iter()
        .map(|s| s.point.is_some() && (!queued || finite(&source[s.id.0 as usize])))
        .collect();
    let mut remaining = *allowance;
    let mut result = vec![None; slots.len()];
    let mut arrivals = vec![None; slots.len()];
    let mut stable = true;
    let mut unproven = slots
        .iter()
        .any(|s| s.point.is_some() && queued && !finite(&source[s.id.0 as usize]));
    if !active.iter().any(|a| *a) {
        return Certification {
            facings: result,
            arrivals,
            stable,
            unproven,
        };
    }
    let initial_world = known.planning_world(ctx.world, ctx.authored);
    let own = source[slots[0].id.0 as usize].side;
    // Opponents are observed bodies, not observers whose private spotting
    // can choose new cover posts during this side's movement proof.
    let opposing =
        crate::knowledge::SideKnowledge::new(ctx.seed, ctx.knowledge[own.index()].ground().layer());
    let mut knowledge = [&opposing; 2];
    knowledge[own.index()] = ctx.knowledge[own.index()];
    // Only overlapping departure and arrival rehearsals require the whole
    // journey. Road choice is independent of how much travel proves a move.
    let full_rehearsal_m = 2.0 * ctx.rules.navigation.move_rehearsal_m;
    while active.iter().any(|v| *v) && remaining > 0 {
        // New combined plans need work left to restore failures and reprove
        // survivors. Each pass removes at least one failure or finishes.
        let floor = if reserve_repair {
            remaining - remaining.div_ceil(active.iter().filter(|a| **a).count() as u64)
        } else {
            0
        };
        let mut world = initial_world.clone();
        let mut units = source.to_vec();
        let mut sides = [known.planning_snapshot(), known.planning_snapshot()];
        let ground = ctx.knowledge[own.index()].ground().layer();
        let mut planner = RoutePlanner::default();
        for (i, slot) in slots.iter().enumerate() {
            if !active[i] {
                continue;
            }
            let unit = &mut units[slot.id.0 as usize];
            if queued {
                for order in &mut unit.orders {
                    if let UnitOrder::AttackMove(m) = order {
                        *order = UnitOrder::Move(m.clone());
                    }
                }
            }
            if let Some(orders) = orders {
                for (k, order) in orders[i].iter().enumerate() {
                    unit.enqueue(order.clone(), queued || k > 0);
                }
            }
        }
        for unit in &mut units {
            let member = slots.iter().position(|s| s.id == unit.id);
            let keep_prefix = queued && member.is_some() || member.is_none() && unit.side == own;
            if keep_prefix && !unit.orders.is_empty() && finite(unit) {
                continue;
            }
            if !slots
                .iter()
                .enumerate()
                .any(|(i, s)| active[i] && s.id == unit.id)
            {
                stand(unit);
            }
        }
        let step_cost = units.iter().filter(|u| u.alive()).count().max(1) as u64;
        let mut approaches = vec![None; slots.len()];
        // Orders left identify the queued leg; its long-distance decision
        // freezes before planning, and only actual movement earns a carry.
        let mut legs: Vec<Option<(usize, bool, V2)>> = vec![None; slots.len()];
        // How long each member has stood with nothing near it on the move,
        // and the members stopped for it.
        let mut still = vec![0u64; slots.len()];
        let mut stopped = vec![false; slots.len()];
        // Keep one share for the movement rehearsal itself. A mover may use
        // at most one other share on repeated route searches in this pass.
        let moving = units.iter().filter(|u| !u.orders.is_empty()).count();
        let mut replan_left = vec![(remaining - floor) / (moving as u64 + 1); units.len()];
        let mut offset = 0;
        // Opposing bodies are known obstacles; their private setup timers
        // cannot keep this side's admission running or spend its retry budget.
        let progress = |u: &Unit| {
            let owned = u.side == own;
            (
                u.alive().then_some((u.position, u.yaw)),
                u.deployment.as_ref().filter(|_| owned).map(|d| d.current),
                u.garrison.as_ref().filter(|_| owned).map(|g| g.phase),
            )
        };
        loop {
            if remaining < step_cost || remaining <= floor {
                unproven = true;
                break;
            }
            remaining -= step_cost;
            offset += 1;
            let tick = ctx.tick + offset;
            let before: Vec<_> = units.iter().map(progress).collect();
            for (i, slot) in slots.iter().enumerate() {
                let unit = &units[slot.id.0 as usize];
                let orders = unit.orders.len();
                if legs[i].is_none_or(|leg| leg.0 != orders) {
                    legs[i] = unit.movement_goal().map(|(goal, _)| {
                        (
                            orders,
                            (goal - unit.position.xy()).outside_radius(full_rehearsal_m),
                            unit.position.xy(),
                        )
                    });
                }
            }
            crate::garrison::advance(
                &world,
                &sides,
                ctx.authored,
                &mut units,
                ctx.rules,
                ctx.seed,
                tick,
            );
            crate::deployment::advance_all(&mut units);
            let local = MovementContext {
                world: &world,
                ground,
                knowledge,
                tick,
                ..*ctx
            };
            let shoves = super::advance(&local, &mut units, &mut sides, &mut planner, &mut |_| {});
            // Long initial routes use the live planner's bound. Repeated
            // planning has a share per mover, so one blocked member cannot
            // spend the other members' arrival rehearsal.
            for index in 0..planner.charges().len() {
                let charge = planner.charges()[index];
                if charge.new_goal && !reserve_repair {
                    if charge.distance_m <= full_rehearsal_m {
                        remaining = remaining.saturating_sub(charge.work);
                    }
                    continue;
                }
                let id = charge.unit;
                let index = id.0 as usize;
                let charged = charge.work.min(replan_left[index]);
                replan_left[index] -= charged;
                remaining =
                    remaining.saturating_sub(if reserve_repair { charge.work } else { charged });
                if replan_left[index] == 0 {
                    unproven |= reserve_repair;
                    planner.cancel(id);
                    stand(&mut units[index]);
                    if let Some(i) = slots.iter().position(|s| s.id == id) {
                        stopped[i] = true;
                    }
                }
            }
            for shove in shoves {
                let Some(prop) = world.prop(shove.prop).cloned() else {
                    continue;
                };
                if prop.body.topples {
                    world.knock_down(prop.id);
                    for side in &mut sides {
                        side.forget(&prop);
                    }
                } else {
                    for side in &mut sides {
                        side.before_move(&prop, ctx.authored);
                    }
                    world.move_prop(prop.id, shove.center, shove.yaw, tick);
                    if let Some(p) = world.prop(prop.id) {
                        super::clear_of(&world, &mut units, &p.footprint(), ctx.soldier_radius_m);
                    }
                }
            }
            let touched = world.take_touched();
            for side in &mut sides {
                side.touch(&touched);
            }
            for (i, slot) in slots.iter().enumerate() {
                if !active[i] {
                    continue;
                }
                if result[i].is_some() || stopped[i] {
                    continue;
                }
                let unit = &units[slot.id.0 as usize];
                if approaches[i].is_none() && unit.orders.len() == 1 && unit.route.is_some() {
                    approaches[i] = Some(super::final_facing(unit));
                }
                if !unit.orders.is_empty() || unit.turn_to.is_some() {
                    continue;
                }
                let fits = unit.hull_box().is_none_or(|h| {
                    h.contains(slot.point.unwrap(), 0.0)
                        && units.iter().all(|other| {
                            other.id == unit.id
                                || !other.alive()
                                || other.hull_box().is_none_or(|o| !h.overlaps(&o))
                        })
                        && world.props_near(h.center, h.half.length()).iter().all(|p| {
                            !p.blocks(contract::map::MoverClass::Vehicle)
                                || !h.overlaps(&p.footprint())
                        })
                });
                if fits {
                    result[i] = Some(approaches[i].unwrap_or(unit.yaw));
                    arrivals[i] = Some(unit.clone());
                }
            }
            if active
                .iter()
                .enumerate()
                .all(|(i, a)| !*a || result[i].is_some())
            {
                break;
            }
            // A member that has driven the first stretch of its leg is
            // carried to the last, or to the next physical interaction on it.
            let reach = ctx.rules.navigation.move_rehearsal_m;
            for (i, slot) in slots.iter().enumerate() {
                let index = slot.id.0 as usize;
                let unit = &units[index];
                let Some(left) = remaining_route(unit)
                    .filter(|_| active[i])
                    .map(|w| distances_to_end(&w)[0])
                else {
                    continue;
                };
                let Some((orders, long, from)) = legs[i].as_mut() else {
                    continue;
                };
                if *orders != unit.orders.len() {
                    continue;
                }
                if *long
                    && (unit.position.xy() - *from).at_least_radius(reach)
                    && left > reach
                    && carry(ctx, &world, &mut units, index, tick)
                {
                    // The synthetic jump earns no travel. A new carry must
                    // follow real movement through the next interaction.
                    *from = units[index].position.xy();
                }
            }
            // A member that has stood through two stalls, with nothing within
            // a knot of it moving or planning, stays stuck however long the
            // rehearsal runs: it stops where it is, and the others go on.
            let moved: Vec<bool> = before
                .iter()
                .zip(&units)
                .map(|(p, u)| *p != progress(u))
                .collect();
            let busy: Vec<bool> = units
                .iter()
                .zip(&moved)
                .map(|(u, moved)| *moved || planner.pending(u.id).is_some())
                .collect();
            for (i, slot) in slots.iter().enumerate() {
                let index = slot.id.0 as usize;
                if !active[i] || result[i].is_some() || stopped[i] {
                    continue;
                }
                let here = units[index].position.xy();
                if moved[index] {
                    still[i] = 0;
                } else if (0..units.len())
                    .all(|k| !busy[k] || (units[k].position.xy() - here).outside_radius(KNOT_M))
                {
                    still[i] += 1;
                }
                if still[i] > (2.0 * STALL_REPLAN_S * ctx.tick_hz as f64) as u64 {
                    stopped[i] = true;
                    stand(&mut units[index]);
                }
            }
            if remaining == 0 {
                unproven = true;
                break;
            }
            if touched.is_empty()
                && planner.waiting() == 0
                && active.iter().enumerate().all(|(i, a)| {
                    !*a || result[i].is_some()
                        || stopped[i]
                        || units[slots[i].id.0 as usize].state == MoveState::RouteBlocked
                })
            {
                break;
            }
        }
        let failed = active
            .iter()
            .enumerate()
            .any(|(i, a)| *a && result[i].is_none());
        if !failed || orders.is_none() {
            break;
        }
        // Out of allowance: what arrived was demonstrated, and stands. A
        // second pass could not run, and clearing it would refuse a whole
        // group because one member's journey outlasted the allowance.
        if remaining < step_cost {
            stable = false;
            break;
        }
        for (i, a) in active.iter_mut().enumerate() {
            *a &= result[i].is_some();
        }
        result.fill(None);
        arrivals.fill(None);
    }
    *allowance = remaining;
    Certification {
        facings: result,
        arrivals,
        stable,
        unproven,
    }
}
