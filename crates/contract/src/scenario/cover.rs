//! Infantry cover rules and their admission.

use super::ranges::{nonnegative, positive, share};
use super::{CoverTier, RulesError};
use serde::{Deserialize, Serialize};

/// Incoming spread multiplier per cover tier (Q5: cover works through scatter only).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverTiers {
    pub light: f64,
    pub medium: f64,
    pub heavy: f64,
}

impl CoverTiers {
    pub fn spread(&self, tier: CoverTier) -> f64 {
        match tier {
            CoverTier::Light => self.light,
            CoverTier::Medium => self.medium,
            CoverTier::Heavy => self.heavy,
        }
    }
}

/// Infantry cover (D3–D6, Q5, Q7, Q11, Q20, Q24): a hard-coded game rule for
/// soldiers only. When a round is aimed at a soldier, the strongest cover
/// body within `reach_m` of him that lies between him and the shooter widens
/// that round's spread by its tier; a crater he stands in is light cover.
/// A garrison keeps its building shelter instead (Q22, the named exception):
/// wider spread and fewer damaging fragments, never less damage per hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRules {
    pub tiers: CoverTiers,
    // Which bodies cover, and how well, is a prop type's `cover_tier`
    // column; a live vehicle covers by its weight class (Q24).
    /// Ground cover: a crater at least `crater_min_fill` full.
    pub crater: CoverTier,
    pub crater_min_fill: f64,
    /// A body within this of a soldier can cover him (Q20).
    pub reach_m: f64,
    /// A squad's area reaches this far beyond half its spread: its
    /// soldiers seek cover anywhere inside it (Q7).
    pub search_m: f64,
    /// A soldier who cannot engage steps out, best cover first, within this
    /// far; beyond it, to the nearest place in the area he can engage from
    /// (D3, Q8).
    pub step_out_m: f64,
    /// A soldier who found nowhere to fight from searches again once the
    /// enemy has moved this far, or he has, or his side learns of a body or
    /// a crater within his search.
    pub step_out_retry_m: f64,
    /// A squad re-resolves its cover at most this often (Q11).
    pub reresolve_s: f64,
    /// A threat bearing swing that re-resolves cover (Q11).
    pub swing_deg: f64,
    /// A covering vehicle that moves this far re-resolves its users (Q24).
    pub vehicle_moved_m: f64,
    /// A soldier out on his lean fires at most this long before he tucks
    /// back in: he steps out, fires a burst, and ducks back.
    pub lean_burst_s: f64,
    /// How long he stays tucked in before leaning out again.
    pub lean_tuck_s: f64,
    /// How long he stays out after his last round from there: the firing
    /// pose's hold, which the pose driver reads too.
    pub lean_hold_s: f64,
    /// The farthest he leans from where he stands: a corner further off is
    /// a walk, the cover search's business.
    pub lean_max_m: f64,
    /// How far beyond his body radius a lean point keeps from the footprint.
    pub lean_clear_m: f64,
    /// A lean point is claimed like a place: none within this of another
    /// soldier's place or lean point.
    pub lean_apart_m: f64,
    /// A spot stands this far off the face it hides behind, beyond the
    /// soldier's own radius.
    pub standoff_m: f64,
    /// A face counts as turned away from the threat when the cosine between
    /// its normal and the threat's direction is at most this: one the threat
    /// sees nearly edge-on hides nobody.
    pub away_cos: f64,
    /// The cover search looks this far beyond the squad's area.
    pub search_slack_m: f64,
    pub building_spread_multiplier: f64,
    pub building_fragment_probability_multiplier: f64,
}

impl CoverRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive(
            "cover",
            &[
                ("reach_m", self.reach_m),
                ("search_m", self.search_m),
                ("step_out_m", self.step_out_m),
                ("reresolve_s", self.reresolve_s),
                ("vehicle_moved_m", self.vehicle_moved_m),
                ("lean_burst_s", self.lean_burst_s),
                ("lean_tuck_s", self.lean_tuck_s),
                ("crater_min_fill", self.crater_min_fill),
                ("lean_hold_s", self.lean_hold_s),
                ("lean_max_m", self.lean_max_m),
                ("lean_apart_m", self.lean_apart_m),
            ],
        )?;
        nonnegative(
            "cover",
            &[
                ("lean_clear_m", self.lean_clear_m),
                ("standoff_m", self.standoff_m),
                ("search_slack_m", self.search_slack_m),
            ],
        )?;
        if !(-1.0..=0.0).contains(&self.away_cos) {
            return Err(RulesError::new("cover.away_cos", "must lie in [-1, 0]"));
        }
        if !self.building_spread_multiplier.is_finite() || self.building_spread_multiplier < 1.0 {
            return Err(RulesError::new(
                "cover.building_spread_multiplier",
                "must be finite and at least 1",
            ));
        }
        share(
            "cover.building_fragment_probability_multiplier",
            self.building_fragment_probability_multiplier,
            true,
        )?;
        let mut previous = 1.0;
        for (name, value) in [
            ("light", self.tiers.light),
            ("medium", self.tiers.medium),
            ("heavy", self.tiers.heavy),
        ] {
            if !value.is_finite() || value < previous {
                return Err(RulesError::new(
                    format!("cover.tiers.{name}"),
                    "must widen spread by at least 1, heavier tiers no less",
                ));
            }
            previous = value;
        }
        Ok(())
    }
}
