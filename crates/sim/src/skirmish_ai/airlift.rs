//! Emergency supply-helicopter planning for the skirmish doctrine.
//!
//! The battle still owns landing, service and movement.  This module only
//! decides when a combat group needs help, where its far side is, and which
//! ordinary orders keep one flying supplier on that mission.

use contract::catalog::TypeIndex;
use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::ids::UnitId;
use contract::observation::{ObservationFrame, OwnUnit};
use contract::scenario::Rules;
use contract::skirmish::PlayerView;
use contract::weapons::AmmoCapacity;

use super::purchase::{flying_supplier, standing_supplier, Buy};
use super::team;
use super::{distance, support, Assignment, Tuning};
use std::collections::BTreeSet;

/// A deterministic combat grouping used by the emergency planner.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Group {
    pub units: Vec<UnitId>,
    pub centroid: [f64; 2],
}

/// The group and first unit that caused an emergency.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Need {
    pub group: Group,
    pub unit: UnitId,
}

/// The team planner's detached parties plus the remaining main combat group.
pub(super) fn groups(
    frame: &ObservationFrame,
    rules: &Rules,
    assignment: Option<&Assignment>,
) -> Vec<Group> {
    let combat = |id: UnitId| {
        frame
            .own
            .iter()
            .find(|unit| unit.id == id)
            .is_some_and(|unit| team::is_group_unit(rules, unit))
    };
    let all: BTreeSet<_> = frame
        .own
        .iter()
        .filter(|unit| team::is_group_unit(rules, unit))
        .map(|unit| unit.id)
        .collect();
    let mut groups = Vec::new();
    let mut detached = BTreeSet::new();
    if let Some(assignment) = assignment {
        for party in &assignment.parties {
            let mut units: Vec<_> = party
                .units
                .iter()
                .copied()
                .filter(|&id| combat(id))
                .collect();
            units.sort_unstable();
            units.dedup();
            detached.extend(units.iter().copied());
            if !units.is_empty() {
                groups.push(Group {
                    centroid: group_centroid(frame, &units),
                    units,
                });
            }
        }
    }
    let main: Vec<_> = all.difference(&detached).copied().collect();
    if !main.is_empty() {
        groups.push(Group {
            centroid: group_centroid(frame, &main),
            units: main,
        });
    }
    groups
}

fn group_centroid(frame: &ObservationFrame, ids: &[UnitId]) -> [f64; 2] {
    let points: Vec<_> = ids
        .iter()
        .filter_map(|id| frame.own.iter().find(|u| u.id == *id))
        .map(|u| [u.position[0], u.position[1]])
        .collect();
    centroid(&points).expect("a combat group is nonempty")
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Mission {
    #[default]
    Idle,
    Inbound {
        heli: Option<UnitId>,
        need: Need,
    },
    Serving {
        heli: UnitId,
        group: Group,
    },
    Returning {
        heli: UnitId,
        post: [f64; 2],
    },
}

impl Mission {
    pub(super) fn is_busy(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// Whether any unit in `group` is below an emergency threshold.
pub(super) fn group_needs(
    frame: &ObservationFrame,
    rules: &Rules,
    group: &Group,
    tuning: &Tuning,
) -> Option<UnitId> {
    group
        .units
        .iter()
        .copied()
        .filter_map(|id| frame.own.iter().find(|u| u.id == id))
        .find(|unit| unit_needs(unit, rules, tuning))
        .map(|unit| unit.id)
}

fn unit_needs(unit: &OwnUnit, rules: &Rules, tuning: &Tuning) -> bool {
    let ammo_low = rules
        .catalog
        .mounts(unit.kind)
        .iter()
        .enumerate()
        .flat_map(|(mount, carried)| {
            carried
                .def
                .weapons
                .iter()
                .enumerate()
                .map(move |(kind, id)| (mount, kind, id))
        })
        .any(|(mount, kind, id)| {
            let Some(weapon) = rules.weapons.get(id) else {
                return false;
            };
            let AmmoCapacity::Rounds(cap) = weapon.ammo else {
                return false;
            };
            unit.mounts
                .get(mount)
                .and_then(|m| m.ammo.get(kind).copied())
                .flatten()
                .is_some_and(|left| {
                    (left as f64) / f64::from(cap.max(1)) < tuning.emergency_ammo_share
                })
        });
    if ammo_low {
        return true;
    }
    let catalog = rules.catalog.get(unit.kind);
    if let Some(hull) = catalog.hull() {
        return hull.hp > 0.0 && unit.hp / hull.hp < tuning.emergency_health_share;
    }
    let size = catalog.squad_size();
    size > 0 && (unit.members.len() as f64) / (size as f64) < tuning.emergency_health_share
}

/// Find the first needy unit for which every own supplier is too slow.
pub(super) fn emergency(
    frame: &ObservationFrame,
    rules: &Rules,
    tuning: &Tuning,
    groups: &[Group],
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> Option<Need> {
    let suppliers: Vec<UnitId> = frame
        .own
        .iter()
        .filter(|u| standing_supplier(rules, u.kind))
        .map(|u| u.id)
        .collect();
    groups.iter().find_map(|group| {
        group.units.iter().copied().find_map(|unit| {
            let target = frame.own.iter().find(|u| u.id == unit)?;
            if !unit_needs(target, rules, tuning) {
                return None;
            }
            let point = [target.position[0], target.position[1]];
            let reachable = suppliers.iter().any(|&supplier| {
                route_time(supplier, point)
                    .is_some_and(|seconds| seconds < tuning.emergency_reach_s)
            });
            (!reachable).then(|| Need {
                group: group.clone(),
                unit,
            })
        })
    })
}

/// A point half a supplier's radius from the group, on the side away from
/// known enemies. With no enemy tracks, the side's entry gives the bearing.
pub(super) fn far_side(
    group: &Group,
    known_enemies: &[[f64; 2]],
    entry: [f64; 2],
    radius: f64,
) -> [f64; 2] {
    let centre = group.centroid;
    let away = if let Some(enemy) = centroid(known_enemies) {
        [centre[0] - enemy[0], centre[1] - enemy[1]]
    } else {
        [entry[0] - centre[0], entry[1] - centre[1]]
    };
    let length = libm::hypot(away[0], away[1]);
    let direction = if length > 1e-6 {
        [away[0] / length, away[1] / length]
    } else {
        let toward_entry = [entry[0] - centre[0], entry[1] - centre[1]];
        let entry_length = libm::hypot(toward_entry[0], toward_entry[1]);
        if entry_length > 1e-6 {
            [
                toward_entry[0] / entry_length,
                toward_entry[1] / entry_length,
            ]
        } else {
            [1.0, 0.0]
        }
    };
    [
        centre[0] + direction[0] * radius * 0.5,
        centre[1] + direction[1] * radius * 0.5,
    ]
}

fn centroid(points: &[[f64; 2]]) -> Option<[f64; 2]> {
    (!points.is_empty()).then(|| {
        let sum = points.iter().fold([0.0; 2], |mut sum, point| {
            sum[0] += point[0];
            sum[1] += point[1];
            sum
        });
        [sum[0] / points.len() as f64, sum[1] / points.len() as f64]
    })
}

/// Answer `need`: send the flying supplier the side keeps in reserve, wait
/// for one already bought, or buy the cheapest it can afford, and start the
/// mission toward the group's far side. The purchase, if one is made.
pub(super) fn respond(
    mission: &mut Mission,
    need: Need,
    frame: &ObservationFrame,
    rules: &Rules,
    me: &PlayerView,
    eligible: &[TypeIndex],
) -> Option<Buy> {
    let inbound = |heli| Mission::Inbound {
        heli,
        need: need.clone(),
    };
    if let Some(heli) = frame.own.iter().find(|u| flying_supplier(rules, u.kind)) {
        *mission = inbound(Some(heli.id));
        return None;
    }
    let pending = me
        .pending
        .iter()
        .flat_map(|purchase| purchase.kinds())
        .any(|kind| flying_supplier(rules, kind));
    if pending {
        *mission = inbound(None);
        return None;
    }
    let buy = eligible
        .iter()
        .copied()
        .filter(|&kind| flying_supplier(rules, kind))
        .min_by(|&a, &b| {
            let cost = |k| rules.catalog.get(k).cost;
            cost(a)
                .cmp(&cost(b))
                .then(rules.catalog.id(a).cmp(rules.catalog.id(b)))
        })
        .filter(|&kind| {
            me.unit_count < me.cap && f64::from(rules.catalog.get(kind).cost) <= me.credits
        })
        .map(Buy::alone);
    if buy.is_some() {
        *mission = inbound(None);
    }
    buy
}

/// Arrival tolerance beyond the clear-spot reach, allowing for flight settling.
const ARRIVAL_SLACK_M: f64 = 8.0;

/// How near its goal the flyer counts as there: it lands on the nearest clear
/// spot within the landing reach, so anywhere in that reach has arrived.
fn arrived_within(rules: &Rules) -> f64 {
    rules.air.rotor.landing_reach_m + ARRIVAL_SLACK_M
}

/// Advance the mission and append its ordinary movement/deployment orders.
pub(super) fn orders(
    mission: &mut Mission,
    frame: &ObservationFrame,
    rules: &Rules,
    entry: [f64; 2],
    tuning: &Tuning,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    orders: &mut Vec<Order>,
) {
    loop {
        match mission.clone() {
            Mission::Idle => {}
            Mission::Inbound { heli, mut need } => {
                let heli = heli.or_else(|| {
                    frame
                        .own
                        .iter()
                        .find(|u| flying_supplier(rules, u.kind))
                        .map(|u| u.id)
                });
                let Some(heli_id) = heli else {
                    let pending = frame
                        .skirmish
                        .as_ref()
                        .and_then(super::purchase::own_player)
                        .is_some_and(|me| {
                            me.pending
                                .iter()
                                .flat_map(|p| p.kinds())
                                .any(|kind| flying_supplier(rules, kind))
                        });
                    if !pending {
                        *mission = Mission::Idle;
                    }
                    return;
                };
                let Some(unit) = frame.own.iter().find(|u| u.id == heli_id) else {
                    *mission = Mission::Idle;
                    return;
                };
                if !frame.own.iter().any(|u| u.id == need.unit)
                    || emergency(
                        frame,
                        rules,
                        tuning,
                        std::slice::from_ref(&need.group),
                        route_time,
                    )
                    .is_none()
                {
                    let post = support::support_point(
                        &frame.own.iter().collect::<Vec<_>>(),
                        rules,
                        entry,
                        tuning,
                    );
                    *mission = Mission::Returning {
                        heli: heli_id,
                        post,
                    };
                    continue;
                }
                need.group.centroid = group_centroid(frame, &need.group.units);
                let enemies: Vec<_> = frame
                    .identified
                    .iter()
                    .map(|u| [u.position[0], u.position[1]])
                    .collect();
                let goal = far_side(&need.group, &enemies, entry, rules.service.radius_m);
                *mission = Mission::Inbound {
                    heli: Some(heli_id),
                    need: need.clone(),
                };
                let stale_goal = unit
                    .goal
                    .is_some_and(|old| distance(old, goal) > ARRIVAL_SLACK_M);
                let recipient = frame.own.iter().find(|u| u.id == need.unit).unwrap();
                let out_of_service = distance(
                    [unit.position[0], unit.position[1]],
                    [recipient.position[0], recipient.position[1]],
                ) > rules.service.radius_m;
                if stale_goal
                    || (unit.goal.is_none()
                        && (out_of_service
                            || distance([unit.position[0], unit.position[1]], goal)
                                > arrived_within(rules)))
                {
                    if route_time(unit.id, goal).is_some() {
                        orders.push(Order::Move {
                            units: vec![unit.id],
                            gesture: frame.tick,
                            goal,
                            route: RoutePolicy::Fastest,
                            direction: MoveDirection::Forward,
                            facing: Some(libm::atan2(
                                goal[1] - unit.position[1],
                                goal[0] - unit.position[0],
                            )),
                        });
                    }
                } else if unit.deployment.as_ref().is_some_and(|d| d.progress >= 1.0) {
                    *mission = Mission::Serving {
                        heli: heli_id,
                        group: need.group,
                    };
                }
            }
            Mission::Serving { heli, group } => {
                let Some(unit) = frame.own.iter().find(|u| u.id == heli) else {
                    *mission = Mission::Idle;
                    return;
                };
                let needy = group_needs(frame, rules, &group, tuning)
                    .and_then(|id| frame.own.iter().find(|u| u.id == id));
                if let Some(recipient) = needy {
                    if distance(
                        [unit.position[0], unit.position[1]],
                        [recipient.position[0], recipient.position[1]],
                    ) > rules.service.radius_m
                    {
                        *mission = Mission::Inbound {
                            heli: Some(heli),
                            need: Need {
                                group,
                                unit: recipient.id,
                            },
                        };
                        continue;
                    }
                } else {
                    let post = support::support_point(
                        &frame.own.iter().collect::<Vec<_>>(),
                        rules,
                        entry,
                        tuning,
                    );
                    *mission = Mission::Returning { heli, post };
                    continue;
                }
            }
            Mission::Returning { heli, post } => {
                let Some(unit) = frame.own.iter().find(|u| u.id == heli) else {
                    *mission = Mission::Idle;
                    return;
                };
                if unit.state == contract::observation::MoveState::NoLandingSpot
                    || distance([unit.position[0], unit.position[1]], post) <= arrived_within(rules)
                {
                    if unit.goal.is_some() {
                        orders.push(Order::Stop {
                            units: vec![unit.id],
                        });
                    }
                    *mission = Mission::Idle;
                } else {
                    if unit
                        .goal
                        .is_none_or(|goal| distance(goal, post) > ARRIVAL_SLACK_M)
                        && route_time(unit.id, post).is_some()
                    {
                        orders.push(Order::Move {
                            units: vec![unit.id],
                            gesture: frame.tick,
                            goal: post,
                            route: RoutePolicy::Fastest,
                            direction: MoveDirection::Forward,
                            facing: None,
                        });
                    }
                }
            }
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_side_bears_away_from_enemy_centroid() {
        let group = Group {
            units: vec![UnitId(1), UnitId(2)],
            centroid: [100.0, 100.0],
        };
        let point = far_side(&group, &[[0.0, 100.0], [20.0, 100.0]], [100.0, 0.0], 40.0);
        assert!(point[0] > group.centroid[0]);
        assert!((point[1] - group.centroid[1]).abs() < 1e-9);
    }

    #[test]
    fn far_side_points_toward_entry_without_known_enemies() {
        let group = Group {
            units: vec![UnitId(1)],
            centroid: [100.0, 100.0],
        };
        let point = far_side(&group, &[], [100.0, 0.0], 40.0);
        assert!(point[1] < group.centroid[1]);
        assert!((point[0] - group.centroid[0]).abs() < 1e-9);
    }
}
