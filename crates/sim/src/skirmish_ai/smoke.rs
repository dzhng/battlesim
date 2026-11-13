//! A unit of the AI's that is losing a fight pops its smoke and backs out of
//! the cloud (D4, D22), as a crew in a war film would. Player units never do
//! (D3): this runs only on the AI player's own units.
use super::{live_goal, transport, Tuning};
use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::ids::{Tick, UnitId};
use contract::observation::OwnUnit;
use contract::scenario::Rules;
use std::collections::BTreeMap;

/// Each own unit's strength at the decisions inside the smoke window,
/// oldest first: the losses the smoke rule reads. It reads only how much
/// was lost, so friendly fire counts like any other loss (D22).
#[derive(Clone, Debug, Default)]
pub(super) struct HpWatch {
    samples: BTreeMap<UnitId, Vec<(Tick, f64)>>,
}

/// A unit's strength: its hull's health, or the sum of its living soldiers'
/// (a squad's own `hp` is 0).
fn strength(unit: &OwnUnit) -> f64 {
    unit.hp + unit.member_hp.iter().sum::<f64>()
}

/// A unit's strength at full health: its hull's, or every slot's soldier's.
fn full_strength(rules: &Rules, unit: &OwnUnit) -> f64 {
    let kind = rules.catalog.get(unit.kind);
    let hull = kind.hull().map_or(0.0, |hull| hull.hp);
    let soldiers = kind.slots().unwrap_or_default().iter();
    hull + soldiers
        .map(|slot| rules.catalog.soldier(slot).hp)
        .sum::<f64>()
}

/// Whether the unit is backing out: ordered back by `smoke_orders` at this
/// decision, or still reversing toward a goal its route reaches. The
/// doctrine gives it no other order until the reverse ends, so a reverse is
/// never overwritten by a new attack. The AI orders no other reverse.
pub(super) fn backing_out(unit: &OwnUnit, smoke: &[Order]) -> bool {
    let ordered = smoke
        .iter()
        .any(|order| matches!(order, Order::Move { units, .. } if units.contains(&unit.id)));
    ordered || unit.direction == Some(MoveDirection::Reverse) && live_goal(unit).is_some()
}

/// Samples every own unit's strength, then pops smoke for each unit with a
/// charge left that has lost `smoke_loss_share` of its full strength within
/// `smoke_window_s`, and sends it `smoke_reverse_m` straight back from the
/// way it faces, out of its cloud, when a route leads there. The pops come
/// first, as one order, so the order rate limit may drop a reverse but
/// never a pop (D21). A unit backing out is left to it, and a passenger
/// takes no orders. No random draw is made.
pub(super) fn smoke_orders(
    own: &[OwnUnit],
    watch: &mut HpWatch,
    tuning: &Tuning,
    rules: &Rules,
    tick: Tick,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> Vec<Order> {
    let window = Tuning::ticks(tuning.smoke_window_s, rules.tick_hz);
    watch
        .samples
        .retain(|id, _| own.iter().any(|unit| unit.id == *id));
    let mut popped = Vec::new();
    let mut reverses = Vec::new();
    for unit in own {
        let now = strength(unit);
        let samples = watch.samples.entry(unit.id).or_default();
        samples.retain(|&(at, _)| tick.saturating_sub(at) <= window);
        samples.push((tick, now));
        let peak = samples.iter().map(|&(_, hp)| hp).fold(now, f64::max);
        let full = full_strength(rules, unit);
        let losing = full > 0.0 && peak - now >= tuning.smoke_loss_share * full;
        if !losing
            || unit.smoke.is_none_or(|charges| charges == 0)
            || transport::carrier_of(unit).is_some()
            || backing_out(unit, &[])
        {
            continue;
        }
        // The loss that popped this charge pops no other.
        *samples = vec![(tick, now)];
        popped.push(unit.id);
        let back = [
            unit.position[0] - libm::cos(unit.yaw) * tuning.smoke_reverse_m,
            unit.position[1] - libm::sin(unit.yaw) * tuning.smoke_reverse_m,
        ];
        if route_time(unit.id, back).is_some_and(|t| t.is_finite() && t >= 0.0) {
            reverses.push(Order::Move {
                units: vec![unit.id],
                gesture: tick,
                goal: back,
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Reverse,
                facing: None,
            });
        }
    }
    if popped.is_empty() {
        return reverses;
    }
    std::iter::once(Order::PopSmoke { units: popped })
        .chain(reverses)
        .collect()
}
