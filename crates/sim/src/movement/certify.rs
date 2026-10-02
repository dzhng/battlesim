//! Move admission runs the real movement owner on isolated, side-known state.
//! It drives the two ends of each leg, where bodies jam, squeeze and settle,
//! and the approach to any vehicle standing on the way; the stretch between
//! is taken on the route the planner found ([`carry`]), so what a move costs
//! to admit does not grow with how far it goes.
use contract::command::MovePreviewRequest;
use contract::map::MoverClass;
use contract::observation::MoveState;

use super::{MovementContext, SideGeometry, KNOT_M, STALL_REPLAN_S, TRAFFIC_MARGIN_M};
use crate::formation::Slot;
use crate::math::{v2, Obb2, V2};
use crate::route_planner::RoutePlanner;
use crate::units::{MoveOrder, Unit, UnitOrder};
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
fn way(unit: &Unit) -> Option<Vec<V2>> {
    let route = unit.route.as_ref().filter(|r| !r.is_empty())?;
    let here = std::iter::once(unit.position.xy());
    Some(here.chain(route.iter().copied()).collect())
}

/// Metres of the way left from each of its points.
fn left(way: &[V2]) -> Vec<f64> {
    let mut left = vec![0.0; way.len()];
    for k in (0..way.len() - 1).rev() {
        left[k] = left[k + 1] + (way[k + 1] - way[k]).length();
    }
    left
}

/// Carry `units[index]` forward along its own route, over the stretch the
/// route alone proves: to `move_rehearsal_m` short of the route's end, or of
/// the first vehicle standing on it (the planner does not plan round
/// vehicles). A hull is set down on the route heading the way it was
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
    let Some(way) = way(unit) else {
        return false;
    };
    let left = left(&way);
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
            ((p - c).length() < near).then(|| left[k + 1] + (w[1] - p).length())
        });
        if let Some(met) = met {
            stop = stop.max(met + reach);
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
                let free = soldiers.iter().all(|q| (*q - p.xy()).length() >= apart)
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

/// An approach heading is returned only for destinations reached by their group.
/// Failed members are held on the next pass, so successes cannot rely on them vacating.
pub(crate) fn certify(
    ctx: &MovementContext,
    source: &[Unit],
    known: &SideGeometry,
    slots: &[Slot],
    request: Option<&MovePreviewRequest>,
) -> Vec<Option<f64>> {
    let queued = request.is_none_or(|r| r.queued);
    let finite = |u: &Unit| {
        u.orders
            .iter()
            .all(|o| !matches!(o, UnitOrder::Attack { .. }))
    };
    let mut active: Vec<bool> = slots
        .iter()
        .map(|s| s.point.is_some() && (!queued || finite(&source[s.id.0 as usize])))
        .collect();
    let mut remaining = ctx.rules.navigation.move_validation_work as u64;
    let mut result = vec![None; slots.len()];
    let mut probe_ticks = 0u64; // PROBE
    if !active.iter().any(|a| *a) {
        return result;
    }
    let initial_world = ctx.world.planning_snapshot(
        |p| known.belief(p, ctx.authored),
        known.standing().values().cloned(),
    );
    let own = source[slots[0].id.0 as usize].side;
    // Opponents are observed bodies, not observers whose private spotting
    // can choose new cover posts during this side's movement proof.
    let opposing =
        crate::knowledge::SideKnowledge::new(ctx.seed, ctx.knowledge[own.index()].ground().layer());
    let mut knowledge = [&opposing; 2];
    knowledge[own.index()] = ctx.knowledge[own.index()];
    while active.iter().any(|v| *v) && remaining > 0 {
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
            if let Some(request) = request {
                unit.enqueue(
                    UnitOrder::Move(MoveOrder {
                        destination: slot.point.unwrap(),
                        policy: request.route,
                        gesture: 0,
                        direction: request.direction,
                        facing: request.facing,
                    }),
                    queued,
                );
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
        // Each member's leg under way (its orders left), and how much route
        // that leg had when it set off.
        let mut legs: Vec<Option<(usize, f64)>> = vec![None; slots.len()];
        // How long each member has stood with nothing near it on the move,
        // and the members stopped for it.
        let mut still = vec![0u64; slots.len()];
        let mut stopped = vec![false; slots.len()];
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
            if remaining < step_cost {
                break;
            }
            remaining -= step_cost;
            probe_ticks += 1; // PROBE
            offset += 1;
            let tick = ctx.tick + offset;
            let before: Vec<_> = units.iter().map(progress).collect();
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
            // A leg's first route is the planner's own bounded search, the
            // one the real move runs: only planning again is counted.
            let proving = slots.iter().enumerate().any(|(i, s)| {
                let unit = &units[s.id.0 as usize];
                active[i]
                    && result[i].is_none()
                    && planner.pending(unit.id).map_or(
                        unit.planned_goal.is_none() && unit.movement_goal().is_some(),
                        |r| r.new_goal,
                    )
            });
            let shoves = super::advance(&local, &mut units, &mut sides, &mut planner, &mut |_| {});
            if !proving {
                remaining = remaining.saturating_sub(planner.spent());
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
            // carried to the last, or to the next vehicle standing on it.
            let reach = ctx.rules.navigation.move_rehearsal_m;
            for (i, slot) in slots.iter().enumerate() {
                let index = slot.id.0 as usize;
                let unit = &units[index];
                let Some(left) = way(unit).filter(|_| active[i]).map(|w| left(&w)[0]) else {
                    continue;
                };
                let orders = unit.orders.len();
                let whole = legs[i]
                    .filter(|leg| leg.0 == orders)
                    .map_or(left, |leg| leg.1);
                legs[i] = Some((orders, whole));
                if whole - left >= reach
                    && left > reach
                    && carry(ctx, &world, &mut units, index, tick)
                {
                    // The next carry, past a vehicle standing on the way, is
                    // earned by driving up to it.
                    legs[i] = None;
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
                if std::env::var_os("MOVE_PROBE2").is_some() && offset < 400 && offset % 10 == 0 { let u = &units[index]; println!("  t{} unit {} pos ({:.3},{:.3}) yaw {:.4} state {:?} progress {:?} moved {} pending {} still {}", offset, u.id.0, u.position.x, u.position.y, u.yaw, u.state, u.progress, moved[index], planner.pending(u.id).is_some(), still[i]); } // PROBE
                if moved[index] {
                    still[i] = 0;
                } else if (0..units.len())
                    .all(|k| !busy[k] || (units[k].position.xy() - here).length() > KNOT_M)
                {
                    still[i] += 1;
                }
                if still[i] > (2.0 * STALL_REPLAN_S * ctx.tick_hz as f64) as u64 {
                    stopped[i] = true;
                    stand(&mut units[index]);
                }
            }
            if remaining == 0 {
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
        if std::env::var_os("MOVE_PROBE").is_some() {
            // PROBE
            for (i, sl) in slots.iter().enumerate() {
                // PROBE
                let u = &units[sl.id.0 as usize]; // PROBE
                if active[i] && result[i].is_none() {
                    // PROBE
                    println!("  FAIL unit {} {:?} at ({:.0},{:.0}) {:.1} m from its place, orders {}, blocker {:?}, route {:?} left", u.id.0, u.state, u.position.x, u.position.y, (u.position.xy() - sl.point.unwrap()).length(), u.orders.len(), u.blocker, way(u).map(|w| left(&w)[0])); // PROBE
                    for m in &u.members {
                        println!("    soldier {} alive {} at ({:.1},{:.1}) spot {:?} leg {} path {:?} start {} post {:?}", m.id, m.alive(), m.position.x, m.position.y, m.spot, m.leg, m.path, m.start, m.post);
                    } // PROBE
                    println!(
                        "    route_from {:?} route {:?} progress {:?} tick {}",
                        u.route_from,
                        u.route,
                        u.progress,
                        ctx.tick + offset
                    ); // PROBE
                } // PROBE
            } // PROBE
        } // PROBE
        if !failed || request.is_none() {
            break;
        }
        // Out of allowance: what arrived was demonstrated, and stands. A
        // second pass could not run, and clearing it would refuse a whole
        // group because one member's journey outlasted the allowance.
        if remaining < step_cost {
            break;
        }
        for (i, a) in active.iter_mut().enumerate() {
            *a &= result[i].is_some();
        }
        result.fill(None);
    }
    if std::env::var_os("MOVE_PROBE").is_some() {
        // PROBE
        println!(
            "  work {} ticks {}",
            ctx.rules.navigation.move_validation_work as u64 - remaining,
            probe_ticks
        ); // PROBE
    } // PROBE
    result
}
