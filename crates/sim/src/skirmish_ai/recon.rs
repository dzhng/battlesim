//! Recon standoff geometry used by the doctrine's ordinary movement.

use contract::observation::OwnUnit;

use super::Tuning;

pub(super) fn post(
    unit: &OwnUnit,
    tuning: &Tuning,
    objective: [f64; 2],
    entry: [f64; 2],
) -> [f64; 2] {
    let dx = objective[0] - entry[0];
    let dy = objective[1] - entry[1];
    let approach = libm::hypot(dx, dy).max(1.0);
    let range = unit.sight.range.max(1.0);
    let standoff = (range * tuning.recon_sight_share).min(approach * tuning.recon_approach_share);
    [
        objective[0] - dx / approach * standoff,
        objective[1] - dy / approach * standoff,
    ]
}
