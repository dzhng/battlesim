//! Move admission runs the real movement owner on isolated, side-known state.
use contract::command::MovePreviewRequest;
use contract::observation::MoveState;

use super::{MovementContext, SideGeometry, STALL_REPLAN_S};
use crate::formation::Slot;
use crate::route_planner::RoutePlanner;
use crate::units::{MoveOrder, Unit, UnitOrder};

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
                unit.orders.clear();
                unit.route = None;
                unit.planned_goal = None;
                unit.pursuit = None;
                unit.turn_to = None;
                unit.manoeuvre = None;
                unit.drive_speed_mps = 0.0;
                unit.state = MoveState::Idle;
            }
        }
        let step_cost = units.iter().filter(|u| u.alive()).count().max(1) as u64;
        let mut approaches = vec![None; slots.len()];
        let mut unchanged = 0;
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
            let shoves = super::advance(&local, &mut units, &mut sides, &mut planner, &mut |_| {});
            remaining = remaining.saturating_sub(planner.spent());
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
                if result[i].is_some() {
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
            let moved = before.iter().zip(&units).any(|(p, u)| *p != progress(u));
            unchanged = if moved || planner.waiting() > 0 || !touched.is_empty() {
                0
            } else {
                unchanged + 1
            };
            if unchanged > (2.0 * STALL_REPLAN_S * ctx.tick_hz as f64) as u64 {
                break;
            }
            if remaining == 0 {
                break;
            }
            if touched.is_empty()
                && planner.waiting() == 0
                && active.iter().enumerate().all(|(i, a)| {
                    !*a || result[i].is_some()
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
    result
}
