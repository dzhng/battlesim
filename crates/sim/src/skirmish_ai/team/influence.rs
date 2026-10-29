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
    let values = vec![0.0; resolution * resolution];
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
    let mut grid = InfluenceGrid {
        resolution,
        min,
        max,
        values,
    };
    for (position, strength) in strengths {
        let [x, y] = grid.cell([position[0], position[1]]);
        grid.values[y * resolution + x] += strength;
    }
    if let Some(previous) =
        previous.filter(|old| old.resolution == resolution && old.min == min && old.max == max)
    {
        for (value, old) in grid.values.iter_mut().zip(&previous.values) {
            *value = tuning.influence_blend * *old + (1.0 - tuning.influence_blend) * *value;
        }
    }
    grid
}

impl InfluenceGrid {
    /// The grid's cell holding `point`, as column and row.
    fn cell(&self, point: [f64; 2]) -> [usize; 2] {
        std::array::from_fn(|axis| {
            (((point[axis] - self.min[axis]) / (self.max[axis] - self.min[axis]))
                * self.resolution as f64)
                .floor()
                .clamp(0.0, self.resolution as f64 - 1.0) as usize
        })
    }

    /// The strength in the cell holding `point`.
    pub fn at(&self, point: [f64; 2]) -> f64 {
        let [x, y] = self.cell(point);
        self.values[y * self.resolution + x]
    }

    /// The strength on `objective`: each cell its square (the circle's
    /// bounds) reaches, weighted by the share of the square it holds. An
    /// objective wholly inside a cell reads that cell; one near a cell's
    /// edge reads the cells either side in proportion, so what stands on it
    /// counts wherever the edges fall.
    pub fn on(&self, objective: &ObjectiveSite) -> f64 {
        let (c, r) = (objective.center, objective.radius_m);
        let [x0, y0] = self.cell([c[0] - r, c[1] - r]);
        let [x1, y1] = self.cell([c[0] + r, c[1] + r]);
        // The share of the square's span along `axis` that cell `k` holds;
        // the outer cells reach on past the grid, as `cell` clamps to them.
        let share = |axis: usize, k: usize| {
            let side = (self.max[axis] - self.min[axis]) / self.resolution as f64;
            let lo = match k {
                0 => f64::NEG_INFINITY,
                _ => self.min[axis] + side * k as f64,
            };
            let hi = match k + 1 == self.resolution {
                true => f64::INFINITY,
                false => self.min[axis] + side * (k + 1) as f64,
            };
            ((c[axis] + r).min(hi) - (c[axis] - r).max(lo)).max(0.0) / (2.0 * r)
        };
        (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .map(|(x, y)| self.values[y * self.resolution + x] * share(0, x) * share(1, y))
            .sum()
    }
}
