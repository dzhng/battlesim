//! Where rolling hills rise: how often, how many, how big and how high,
//! and what they keep clear of.
use super::super::{refusal, Range};
use crate::layout::MapType;
use crate::Diagnostic;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Hills come in groups of low rounded rises (`Relief::Ridge`), each group
/// mirrored through the map's centre so the halves hold the same ground.
/// Every length is metres, the same at every size.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hills {
    /// Per map type: the chance a seed has hills at all, and how many
    /// mirrored group pairs it draws per km² of map. A type not listed has
    /// none: a city stays flat.
    pub types: BTreeMap<MapType, HillDensity>,
    /// A group's rises stand within this of its middle.
    pub group_radius_m: Range,
    /// How many rises a group draws, each with its mirror.
    pub rises: [u32; 2],
    /// A rise's radius is drawn from `radius_m` and narrowed until it fits
    /// its place; its peak is that radius times a grade drawn from
    /// `peak_grade`, so broad rises stand tall and every side is as gentle.
    pub radius_m: Range,
    pub peak_grade: Range,
    /// The land never stands higher than this, wherever rises overlap.
    pub max_height_m: f64,
    /// Least ground between a rise's foot and a settlement's outline, a
    /// river's water, a swamp, a bridge deck lengthened by its approach,
    /// and a skirmish objective's or farm's site.
    pub settlement_gap_m: f64,
    pub river_gap_m: f64,
    pub swamp_gap_m: f64,
    pub bridge_gap_m: f64,
    pub objective_gap_m: f64,
    /// Attempts at each group's middle, and at each of its rises.
    pub attempts: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HillDensity {
    pub chance: f64,
    pub pairs_per_km2: f64,
}

/// `peak * (1 - (d/r)^2)^2` is steepest at `d = r/sqrt(3)`, where its slope
/// is this times `peak / r`.
pub const RIDGE_STEEPEST: f64 = 1.539_600_717_839_002;

impl Hills {
    /// What the presets refuse: hills no rule can place, or whose sides a
    /// mover could not climb even where `max_height_m` of rises overlap
    /// at the narrowest radius.
    pub(super) fn errors(&self, slope_cutoff_deg: f64, bank_m: f64) -> Vec<Diagnostic> {
        let finite = |v: f64| v.is_finite() && v >= 0.0;
        let ordered = |r: Range| r.iter().all(|v| v.is_finite() && *v > 0.0) && r[0] <= r[1];
        let steepest = RIDGE_STEEPEST * self.max_height_m / self.radius_m[0];
        let checks = [
            (
                self.types
                    .values()
                    .all(|t| (0.0..=1.0).contains(&t.chance) && finite(t.pairs_per_km2)),
                "types",
                "every chance is between 0 and 1 and every density finite",
            ),
            (
                ordered(self.group_radius_m)
                    && ordered(self.radius_m)
                    && ordered(self.peak_grade)
                    && self.rises[0] <= self.rises[1]
                    && self.attempts > 0,
                "radius_m",
                "hill sizes are finite, ordered and positive, and attempts positive",
            ),
            (
                self.max_height_m.is_finite()
                    && self.peak_grade[1] * self.radius_m[1] <= self.max_height_m,
                "max_height_m",
                "no rise is drawn higher than the land may stand",
            ),
            (
                steepest.is_finite() && steepest < libm::tan(slope_cutoff_deg.to_radians()) / 2.0,
                "max_height_m",
                "a hill's side is steeper than half the slope cutoff",
            ),
            (
                [
                    self.settlement_gap_m,
                    self.river_gap_m,
                    self.swamp_gap_m,
                    self.bridge_gap_m,
                    self.objective_gap_m,
                ]
                .into_iter()
                .all(finite)
                    && self.river_gap_m >= bank_m,
                "river_gap_m",
                "hill gaps are finite and not negative, and a river's clears its bank",
            ),
        ];
        checks
            .into_iter()
            .filter(|(ok, ..)| !ok)
            .map(|(_, field, message)| refusal(format!("$.presets.hills.{field}"), message.into()))
            .collect()
    }
}
