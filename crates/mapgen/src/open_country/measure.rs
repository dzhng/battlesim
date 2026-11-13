//! The open country of a finished plan, read from its geometry alone: what
//! stands outside the settlements, by half, held to the presets' evenness.
use super::*;

/// One thing's amount in each half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Split {
    pub top: f64,
    pub bottom: f64,
    pub fair: bool,
}

/// What the open country holds, read from the plan's geometry alone.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CountryMetrics {
    /// Buildings outside every settlement's outline.
    pub homes: Split,
    /// Metres of tree line.
    pub tree_line_m: Split,
    /// Small woods: larger than a tree's plot, smaller than any wood.
    pub copses: Split,
    /// Single trees and the clumps beside yards.
    pub trees: Split,
    /// Bodies outside every settlement's outline.
    pub cover: Split,
}

/// Whether `p` lies in no settlement's outline.
fn outside(plan: &MapPlan, limits: &[[f64; 4]], p: Point) -> bool {
    !plan
        .settlements
        .iter()
        .zip(limits)
        .any(|(settlement, [x0, y0, x1, y1])| {
            p[0] >= *x0
                && p[0] <= *x1
                && p[1] >= *y0
                && p[1] <= *y1
                && polygon_contains(&settlement.outline, p)
        })
}

fn outline_limits(plan: &MapPlan) -> Vec<[f64; 4]> {
    plan.settlements
        .iter()
        .map(|settlement| contract::ground::limits(&settlement.outline, 0.0))
        .collect()
}

/// The parcels outside every settlement's outline: the yards of the homes
/// in open country.
pub fn country_lots(plan: &MapPlan) -> impl Iterator<Item = &LotPlan> {
    let limits = outline_limits(plan);
    plan.lots
        .iter()
        .filter(move |lot| outside(plan, &limits, lot.ring[0]))
}

/// The open country of a plan, by half, held to the presets' evenness.
pub fn measure(plan: &MapPlan, presets: &PresetDefinitions) -> CountryMetrics {
    let rules = &presets.open_country;
    let middle = plan.size[1] / 2.0;
    let limits = outline_limits(plan);
    let wood_floor = presets.wood_floor_m2();
    let mut tally = [[0.0; 2]; 5];
    let mut count = |row: usize, p: Point, amount: f64| {
        tally[row][usize::from(p[1] < middle)] += amount;
    };
    for building in &plan.buildings {
        let [x, y, _] = building.frame.translation;
        if outside(plan, &limits, [x, y]) {
            count(0, [x, y], 1.0);
        }
    }
    for forest in &plan.forests {
        match &forest.shape {
            GroundShape::Stroke { centerline, .. } => {
                for pair in centerline.samples().windows(2) {
                    count(
                        1,
                        scale(add(pair[0], pair[1]), 0.5),
                        distance(pair[0], pair[1]),
                    );
                }
            }
            GroundShape::Polygon { ring } => {
                let size = crate::layout::geometry::area(ring);
                let middle = crate::layout::geometry::centroid(ring);
                if size < rules.copses.area_m2[0] {
                    count(3, middle, 1.0);
                } else if size < wood_floor {
                    count(2, middle, 1.0);
                }
            }
        }
    }
    for prop in &plan.props {
        if outside(plan, &limits, prop.center) {
            count(4, prop.center, 1.0);
        }
    }
    let e = &rules.fairness;
    let split = |row: usize, least: f64| {
        let [top, bottom] = tally[row];
        Split {
            top,
            bottom,
            fair: even(e.rel, least, top, bottom),
        }
    };
    CountryMetrics {
        homes: split(0, e.homes),
        tree_line_m: split(1, e.tree_line_m),
        copses: split(2, e.copses),
        trees: split(3, e.trees),
        cover: split(4, e.cover),
    }
}

pub(super) fn even(rel: f64, least: f64, top: f64, bottom: f64) -> bool {
    (top - bottom).abs() <= (rel * (top + bottom)).max(least)
}
