use super::RulesError;
use serde::{Deserialize, Serialize};

/// Tuning for automatic target distribution between otherwise equal targets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetingRules {
    /// How much farther one target counts for each other friendly unit engaging it.
    pub engaged_distance_penalty: f64,
    /// The maximum seeded distance variation around the unmodified distance.
    pub distance_spread: f64,
}

impl TargetingRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        if !self.engaged_distance_penalty.is_finite() || self.engaged_distance_penalty < 0.0 {
            return Err(RulesError::new(
                "targeting.engaged_distance_penalty",
                "must be finite and nonnegative",
            ));
        }
        if !self.distance_spread.is_finite() || self.distance_spread < 0.0 {
            return Err(RulesError::new(
                "targeting.distance_spread",
                "must be finite and nonnegative",
            ));
        }
        if self.distance_spread >= 1.0 {
            return Err(RulesError::new(
                "targeting.distance_spread",
                "must be less than 1",
            ));
        }
        Ok(())
    }
}
