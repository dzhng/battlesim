//! Finite charges a unit carries apart from its weapon mounts, each spent
//! one at a time and restored by supply service.
use serde::{Deserialize, Serialize};

/// Finite flare stores and their reload interval.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlareCapability {
    pub capacity: u32,
    pub reload_s: f64,
}

/// Active protection: interceptor charges, each with a cooldown and a standoff.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveProtection {
    /// What the card's protection row reads: a concise label
    /// (`crate::labels`).
    pub name: String,
    /// Its generated icon, `assets/icons/weapons/<icon>.svg`.
    pub icon: String,
    pub capacity: u32,
    pub cooldown_s: f64,
    pub standoff_m: f64,
    pub service_s: f64,
    pub stock_per_charge: u32,
}
