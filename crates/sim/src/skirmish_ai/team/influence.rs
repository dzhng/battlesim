//! The influence grid: each side's strength over the battlefield, in the
//! one measure of value (D56, `value.rs`).

use super::value::{Measure, Threat};
use super::*;

pub(super) fn build_influence(
    rules: &Rules,
    frame: &ObservationFrame,
    sites: &SkirmishSites,
    previous: Option<&InfluenceGrid>,
    tuning: TeamTuning,
) -> InfluenceGrid {
    let resolution = tuning.grid_resolution.max(1);
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for point in sites
        .entries
        .iter()
        .map(|entry| entry.center)
        .chain(sites.objectives.iter().map(|objective| objective.center))
    {
        for axis in 0..2 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    for axis in 0..2 {
        if !min[axis].is_finite() {
            min[axis] = 0.0;
            max[axis] = 1.0;
        }
        if (max[axis] - min[axis]).abs() < 1.0 {
            max[axis] = min[axis] + 1.0;
        }
    }
    let mut values = vec![0.0; resolution * resolution];
    // Each side's units count their strength against the other's: ours
    // against the enemies identified, theirs against our own kinds.
    let measure = Measure::new(rules, frame);
    let ours = Threat::of(rules, frame.own.iter().map(|unit| unit.kind));
    let strengths = frame
        .own
        .iter()
        .map(|unit| (unit.position, measure.own(unit.kind)))
        .chain(
            frame
                .identified
                .iter()
                .map(|unit| (unit.position, -measure.against(unit.kind, &ours))),
        );
    for (position, strength) in strengths {
        add_strength(
            &mut values,
            resolution,
            min,
            max,
            [position[0], position[1]],
            strength,
        );
    }
    if let Some(previous) =
        previous.filter(|old| old.resolution == resolution && old.min == min && old.max == max)
    {
        for (value, old) in values.iter_mut().zip(&previous.values) {
            *value = tuning.influence_blend * *old + (1.0 - tuning.influence_blend) * *value;
        }
    }
    InfluenceGrid {
        resolution,
        min,
        max,
        values,
    }
}

fn add_strength(
    values: &mut [f64],
    resolution: usize,
    min: [f64; 2],
    max: [f64; 2],
    point: [f64; 2],
    strength: f64,
) {
    let x = (((point[0] - min[0]) / (max[0] - min[0])) * resolution as f64)
        .floor()
        .clamp(0.0, resolution as f64 - 1.0) as usize;
    let y = (((point[1] - min[1]) / (max[1] - min[1])) * resolution as f64)
        .floor()
        .clamp(0.0, resolution as f64 - 1.0) as usize;
    values[y * resolution + x] += strength;
}

impl InfluenceGrid {
    pub fn at(&self, point: [f64; 2]) -> f64 {
        let x = (((point[0] - self.min[0]) / (self.max[0] - self.min[0])) * self.resolution as f64)
            .floor()
            .clamp(0.0, self.resolution as f64 - 1.0) as usize;
        let y = (((point[1] - self.min[1]) / (self.max[1] - self.min[1])) * self.resolution as f64)
            .floor()
            .clamp(0.0, self.resolution as f64 - 1.0) as usize;
        self.values[y * self.resolution + x]
    }
}
