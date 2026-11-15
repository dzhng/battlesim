//! Riders' rules: boarding and unloading a carrier, and the forced exit
//! from a destroyed host.
use serde::{Deserialize, Serialize};

/// Timing and reach for squads boarding a carrier.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriageRules {
    pub board_s: f64,
    pub reach_m: f64,
    pub unload_search_m: f64,
    /// The fastest a ground carrier may roll while squads climb on or off
    /// it, m/s: about a soldier's jog.
    pub climb_speed_mps: f64,
}

impl CarriageRules {
    pub(super) fn check(&self) -> Result<(), String> {
        for (name, value) in [
            ("board_s", self.board_s),
            ("reach_m", self.reach_m),
            ("unload_search_m", self.unload_search_m),
            ("climb_speed_mps", self.climb_speed_mps),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("carriage.{name} must be finite and positive"));
            }
        }
        Ok(())
    }
}

/// The physical kind of host a forced passenger exit comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BailOutHost {
    Building,
    Ground,
    Air,
}

/// Injury and placement rules for one kind of bail-out host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BailOutRow {
    #[serde(default)]
    pub squad_loss: Option<f64>,
    pub death: f64,
    pub hp_lost: [f64; 2],
    pub search_m: f64,
}
