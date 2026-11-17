//! Emergency supply-helicopter planning for the skirmish doctrine.
//!
//! The battle still owns landing, service and movement.  This module only
//! decides when a combat group needs help, where its far side is, and which
//! ordinary orders keep one flying supplier on that mission.

use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::ids::UnitId;
use contract::observation::{ObservationFrame, OwnUnit};
use contract::scenario::Rules;
use contract::weapons::AmmoCapacity;

use super::purchase::drives;
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
    let centroid = |units: &[UnitId]| {
        if units.is_empty() {
            return [0.0; 2];
        }
        let sum = units.iter().fold([0.0; 2], |mut sum, id| {
            if let Some(unit) = frame.own.iter().find(|unit| unit.id == *id) {
                sum[0] += unit.position[0];
                sum[1] += unit.position[1];
            }
            sum
        });
        [sum[0] / units.len() as f64, sum[1] / units.len() as f64]
    };
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
                    centroid: centroid(&units),
                    units,
                });
            }
        }
    }
    let mut main: Vec<_> = all.difference(&detached).copied().collect();
    main.sort_unstable();
    if !main.is_empty() {
        groups.push(Group {
            centroid: centroid(&main),
            units: main,
        });
    }
    if groups.is_empty() && !all.is_empty() {
        let units: Vec<_> = all.into_iter().collect();
        groups.push(Group {
            centroid: centroid(&units),
            units,
        });
    }
    groups
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Mission {
    #[default]
    Idle,
    Inbound {
        heli: Option<UnitId>,
        group: Group,
        goal: [f64; 2],
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
        .filter(|u| {
            let t = rules.catalog.get(u.kind);
            t.has_role("logistics") && t.capabilities.supply.is_some()
        })
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
                route_time(supplier, point).is_some_and(|seconds| {
                    seconds.is_finite() && seconds >= 0.0 && seconds < tuning.emergency_reach_s
                })
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
    match mission.clone() {
        Mission::Idle => {}
        Mission::Inbound { heli, group, goal } => {
            let heli = heli.or_else(|| flying_supplier(frame, rules).map(|u| u.id));
            let Some(heli_id) = heli else {
                return;
            };
            let Some(unit) = frame.own.iter().find(|u| u.id == heli_id) else {
                *mission = Mission::Idle;
                return;
            };
            *mission = Mission::Inbound {
                heli: Some(heli_id),
                group: group.clone(),
                goal,
            };
            if unit.goal.is_none() && distance([unit.position[0], unit.position[1]], goal) > 8.0 {
                if route_time(unit.id, goal).is_some_and(|t| t.is_finite() && t >= 0.0) {
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
            } else if unit.deployment.as_ref().is_some_and(|d| d.progress >= 1.0)
                && group_needs(frame, rules, &group, tuning).is_some()
            {
                *mission = Mission::Serving {
                    heli: heli_id,
                    group,
                };
            }
        }
        Mission::Serving { heli, group } => {
            let Some(unit) = frame.own.iter().find(|u| u.id == heli) else {
                *mission = Mission::Idle;
                return;
            };
            if group_needs(frame, rules, &group, tuning).is_none() {
                let post = support::support_point(
                    &frame.own.iter().collect::<Vec<_>>(),
                    rules,
                    entry,
                    tuning,
                );
                *mission = Mission::Returning { heli, post };
            } else if unit.deployment.as_ref().is_none_or(|d| d.progress < 1.0) {
                // Landing and setup are automatic; leave the heli's existing
                // movement/landing state alone while it serves this mission.
            }
        }
        Mission::Returning { heli, post } => {
            let Some(unit) = frame.own.iter().find(|u| u.id == heli) else {
                *mission = Mission::Idle;
                return;
            };
            if distance([unit.position[0], unit.position[1]], post) > 8.0 {
                if unit.goal.is_none()
                    && route_time(unit.id, post).is_some_and(|t| t.is_finite() && t >= 0.0)
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
            } else if unit.deployment.as_ref().is_some_and(|d| d.progress >= 1.0) {
                *mission = Mission::Idle;
            }
        }
    }
}

pub(super) fn flying_supplier<'a>(
    frame: &'a ObservationFrame,
    rules: &Rules,
) -> Option<&'a OwnUnit> {
    frame.own.iter().find(|u| {
        let t = rules.catalog.get(u.kind);
        t.has_role("logistics")
            && t.capabilities.supply.is_some()
            && t.mobility.flight().is_some()
            && !drives(rules, u.kind)
    })
}

pub(super) fn is_flying_supplier(rules: &Rules, kind: contract::catalog::TypeIndex) -> bool {
    let t = rules.catalog.get(kind);
    t.has_role("logistics")
        && t.capabilities.supply.is_some()
        && t.mobility.flight().is_some()
        && !drives(rules, kind)
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
