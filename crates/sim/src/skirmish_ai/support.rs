//! The standing supplier's post: it waits between the road-edge base and the
//! observed combat group, and sets up once there.
use contract::command::Order;
use contract::ids::UnitId;
use contract::observation::OwnUnit;
use contract::scenario::Rules;

use super::purchase::drives;
use super::{distance, live_goal, Tuning};

/// Orders that keep each supplier at its post: move there, or set up once
/// it stands there. An impossible support route falls back to `entry`.
pub(super) fn support_orders(
    own: &[&OwnUnit],
    rules: &Rules,
    entry: [f64; 2],
    tuning: &Tuning,
    tick: u64,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    orders: &mut Vec<Order>,
) {
    let combat: Vec<_> = own
        .iter()
        .filter(|u| !rules.catalog.get(u.kind).has_role("logistics"))
        .collect();
    let support = if combat.is_empty() {
        entry
    } else {
        let mut center = [0.0; 2];
        for u in &combat {
            center[0] += u.position[0];
            center[1] += u.position[1];
        }
        let share = tuning.supply_share;
        [
            entry[0] + share * (center[0] / combat.len() as f64 - entry[0]),
            entry[1] + share * (center[1] / combat.len() as f64 - entry[1]),
        ]
    };
    // Only a supplier that drives stands at the post.
    for truck in own
        .iter()
        .filter(|u| rules.catalog.get(u.kind).has_role("logistics") && drives(rules, u.kind))
    {
        let here = [truck.position[0], truck.position[1]];
        let slack = tuning.truck_slack_m;
        if distance(here, support) > slack {
            if live_goal(truck).is_some_and(|old| distance(old, support) <= slack) {
                continue;
            }
            let goal = if route_time(truck.id, support).is_some_and(|t| t.is_finite() && t >= 0.0) {
                support
            } else if route_time(truck.id, entry).is_some_and(|t| t.is_finite() && t >= 0.0) {
                entry
            } else {
                continue;
            };
            if distance(here, goal) > slack {
                if truck.goal.is_none_or(|old| distance(old, goal) > slack) {
                    orders.push(Order::Move {
                        units: vec![truck.id],
                        gesture: tick,
                        goal,
                        route: contract::command::RoutePolicy::Fastest,
                        direction: contract::command::MoveDirection::Forward,
                        facing: None,
                    });
                }
                continue;
            }
        }
        if truck.state != contract::observation::MoveState::RouteBlocked
            && truck.goal.is_none()
            && truck
                .deployment
                .is_some_and(|d| d.target == contract::observation::Posture::Packed)
        {
            orders.push(Order::SetDeployment {
                units: vec![truck.id],
                deployed: true,
            });
        }
    }
}
