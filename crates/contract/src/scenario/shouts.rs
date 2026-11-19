//! Timing for the battle barks the simulation emits.

use serde::{Deserialize, Serialize};

/// Timing data for simulation-emitted battle barks.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShoutRules {
    pub cooldown_s: f64,
    pub talk_s: f64,
}

impl ShoutRules {
    pub(super) fn check(&self) -> Result<(), String> {
        if !self.talk_s.is_finite() || self.talk_s <= 0.0 {
            return Err("shouts: talk_s must be finite and positive".into());
        }
        if self.cooldown_s > 0.0 {
            Ok(())
        } else {
            Err("shouts: cooldown_s must be positive".into())
        }
    }
}
