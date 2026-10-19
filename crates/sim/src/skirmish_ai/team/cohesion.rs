//! Group cohesion and lane movement evidence used by team planning.

use super::*;

pub fn cohesion_sample(positions: &[[f64; 2]], approach: [f64; 2], radius: f64) -> [u64; 2] {
    let inside = positions
        .iter()
        .filter(|position| distance(**position, approach) <= radius)
        .count() as u64;
    [inside, positions.len() as u64]
}

/// Units that the lane staging/advance branch actually moves as a group.
pub fn is_group_unit(rules: &Rules, unit: &OwnUnit) -> bool {
    let catalog = rules.catalog.get(unit.kind);
    !catalog.has_role("logistics")
        && !catalog.has_role("recon")
        && super::super::transport::carrier_of(unit).is_none()
        && (crate::carriage::seats(rules, unit.kind) == 0
            || !rules.catalog.mounts(unit.kind).is_empty())
}

pub fn group_positions<'a>(
    rules: &Rules,
    units: impl IntoIterator<Item = &'a OwnUnit>,
) -> Vec<[f64; 2]> {
    units
        .into_iter()
        .filter(|unit| is_group_unit(rules, unit))
        .map(|unit| [unit.position[0], unit.position[1]])
        .collect()
}

/// A push may begin once the core of the group is together. One late
/// reinforcement must not hold the whole lane at staging forever.
pub fn group_ready(positions: &[[f64; 2]], approach: [f64; 2], radius: f64) -> bool {
    let sample = cohesion_sample(positions, approach, radius);
    sample[1] >= 2 && sample[0] * 4 >= sample[1] * 3
}

pub fn lane_advancing(rules: &Rules, units: &[&OwnUnit], plan: &PlayerPlan) -> bool {
    if group_positions(rules, units.iter().copied()).is_empty() {
        return false;
    }
    units
        .iter()
        .filter(|unit| is_group_unit(rules, unit))
        .any(|unit| {
            let position = [unit.position[0], unit.position[1]];
            let current_distance = distance(position, plan.main_objective.center);
            current_distance <= plan.main_objective.radius_m
                || unit.goal.is_some_and(|goal| {
                    distance(goal, plan.main_objective.center) <= plan.main_objective.radius_m
                        || distance(goal, plan.main_objective.center) + 1.0 < current_distance
                })
        })
}
