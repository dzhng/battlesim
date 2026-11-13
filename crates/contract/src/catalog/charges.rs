//! Finite charges a unit carries apart from its weapon mounts, each spent
//! one at a time and restored by supply service.
use super::Hull;
use crate::smoke::SmokeCloudSpec;
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

fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

impl FlareCapability {
    pub fn validate(&self, hull: Option<&Hull>) -> Result<(), &'static str> {
        if hull.is_none() {
            return Err("capabilities.flares needs a hull: flares are dispensed from a vehicle or an airframe");
        }
        if self.capacity == 0 || !positive(self.reload_s) {
            return Err("capabilities.flares needs positive capacity and reload_s");
        }
        Ok(())
    }
}

impl ActiveProtection {
    pub fn validate(&self, hull: Option<&Hull>) -> Result<(), &'static str> {
        if hull.is_none() {
            return Err("active protection requires a hull");
        }
        if self.capacity == 0
            || self.stock_per_charge == 0
            || ![self.cooldown_s, self.standoff_m, self.service_s]
                .into_iter()
                .all(positive)
        {
            return Err(
                "active protection needs finite positive timings, reach, capacity and stock price",
            );
        }
        Ok(())
    }
}

/// Smoke charges, each popping a cloud centred where the unit stands
/// (`crate::smoke`), refilled one per delivery like flares. Unlike flares
/// it needs no hull: a squad throws its smoke.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeCapability {
    pub capacity: u32,
    /// Seconds of service a charge takes to restore.
    pub service_s: f64,
    /// The stock a restored charge costs.
    pub stock_per_charge: u32,
    pub cloud: SmokeCloudSpec,
}

impl SmokeCapability {
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity == 0 {
            return Err("smoke capacity must be at least 1".into());
        }
        if !positive(self.service_s) {
            return Err("smoke service_s must be positive".into());
        }
        // A price of 0 would let a truck give smoke away.
        if self.stock_per_charge == 0 {
            return Err("smoke stock_per_charge must be at least 1".into());
        }
        self.cloud.validate()
    }
}
