//! Where swamps form: how often, how big, how deep, and what they keep
//! clear of (decisions 5, 21, 22 and 30 of `specs/done/water-and-wetlands`).
use super::super::{outline_ok, refusal, OutlineShape, Range, Tolerance};
use crate::layout::MapType;
use crate::Diagnostic;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Swamps come in pairs, one in each half and of one size, so the halves
/// hold the same wet ground. Every length is metres, the same at every size.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swamps {
    /// Each map type's chance that a seed has swamps at all, like its
    /// `river_chance`.
    pub chance: BTreeMap<MapType, f64>,
    /// How many pairs a seed with swamps draws.
    pub pairs: [u32; 2],
    /// A swamp's size: the radius of the circle of its area, before its
    /// aspect squeezes it.
    pub radius_m: Range,
    pub aspect: Range,
    pub outline_noise: f64,
    pub outline_points: u32,
    /// How far its bed lies under its water: chosen for how a soldier
    /// looks wading in it (decision 30).
    pub depth_m: f64,
    /// With a river, a swamp's edge lies within this of the water's, on
    /// flat ground beside it; with none, it may lie anywhere.
    pub river_reach_m: f64,
    /// Least dry ground between a swamp's edge and a river's water.
    pub river_gap_m: f64,
    /// Least ground between a swamp and a settlement's outline, another
    /// swamp, a bridge deck lengthened by its approach, a road's edge and a
    /// wood.
    pub settlement_gap_m: f64,
    pub swamp_gap_m: f64,
    pub bridge_gap_m: f64,
    pub road_gap_m: f64,
    pub wood_gap_m: f64,
    /// Attempts at each swamp before its pair is given up.
    pub attempts: u32,
    /// How far the halves' swamp areas may differ.
    pub fairness: Tolerance,
}

impl Swamps {
    /// A swamp's shape: an ellipse with its edge drawn in and out.
    pub fn shape(&self) -> OutlineShape {
        OutlineShape {
            exponent: 2.0,
            noise: self.outline_noise,
            points: self.outline_points,
        }
    }

    /// What the presets refuse: a swamp the terrain could not carve
    /// (`contract::swamp::validate`), or one no rule can place.
    /// `bank_m` is how far a river's bank cuts into the land.
    pub(super) fn errors(
        &self,
        height_grid_m: f64,
        slope_cutoff_deg: f64,
        bank_m: f64,
    ) -> Vec<Diagnostic> {
        let finite = |v: f64| v.is_finite() && v >= 0.0;
        let ordered = |r: Range| r.iter().all(|v| v.is_finite() && *v > 0.0) && r[0] <= r[1];
        // The narrowest swamp drawn, across its short axis, holds a disc
        // the contract carves.
        let narrowest = 2.0 * self.radius_m[0] * self.aspect[0];
        let carvable = (contract::swamp::MIN_ACROSS_CELLS + 1.0) * height_grid_m;
        let checks = [
            (
                MapType::ALL
                    .iter()
                    .all(|t| self.chance.get(t).is_some_and(|c| (0.0..=1.0).contains(c))),
                "chance",
                "every map type has a swamp chance between 0 and 1",
            ),
            (
                self.pairs[0] <= self.pairs[1] && self.attempts > 0,
                "pairs",
                "pairs are an ordered count, and attempts positive",
            ),
            (
                ordered(self.radius_m)
                    && ordered(self.aspect)
                    && self.aspect[1] <= 1.0
                    && outline_ok(self.outline_noise, self.outline_points),
                "radius_m",
                "swamp sizes and outline must be finite, ordered and positive",
            ),
            (
                narrowest >= carvable,
                "radius_m",
                "the narrowest swamp drawn is too small for the height grid to carve",
            ),
            (
                self.depth_m > 0.0
                    && self.depth_m <= contract::swamp::deepest_m(height_grid_m, slope_cutoff_deg),
                "depth_m",
                "a swamp is deeper than its edge can ease in under the slope cutoff",
            ),
            (
                [
                    self.river_reach_m,
                    self.river_gap_m,
                    self.settlement_gap_m,
                    self.swamp_gap_m,
                    self.bridge_gap_m,
                    self.road_gap_m,
                    self.wood_gap_m,
                ]
                .into_iter()
                .all(finite)
                    && self.river_gap_m < self.river_reach_m,
                "river_gap_m",
                "swamp gaps are finite and not negative, and a river's gap is inside its reach",
            ),
            (
                self.river_gap_m >= contract::swamp::edge_m(height_grid_m) + height_grid_m + bank_m,
                "river_gap_m",
                "a swamp's edge and a sample more must stay clear of the bank a river cuts",
            ),
            (
                finite(self.fairness.rel) && finite(self.fairness.abs),
                "fairness",
                "fairness allowances are finite and not negative",
            ),
        ];
        checks
            .into_iter()
            .filter(|(ok, ..)| !ok)
            .map(|(_, field, message)| refusal(format!("$.presets.swamps.{field}"), message.into()))
            .collect()
    }
}
