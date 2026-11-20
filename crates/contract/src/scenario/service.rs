//! Supply service rates.
use serde::{Deserialize, Serialize};

/// Supply rates; stock and deployment belong to unit capabilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRules {
    /// Service reach from a deployed truck.
    pub radius_m: f64,
    pub vehicle_hp_per_s: f64,
    pub stock_per_hp: u32,
    pub soldier_replacement_s: f64,
    pub stock_per_soldier: u32,
    /// One flare restored every `flare_service_s`, each costing
    /// `stock_per_flare`, for any unit that carries flares and stands still
    /// in reach (a jet never does: it withdraws to rearm).
    pub flare_service_s: f64,
    pub stock_per_flare: u32,
}
