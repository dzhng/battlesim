//! A unit type's optional abilities (`Capabilities`), each present only on
//! the types that have them, and the ability records they hold.
use super::{ActiveProtection, FlareCapability, SmokeCapability};
use serde::{Deserialize, Serialize};

/// Optional abilities, present only on the types that have them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Carries infantry squads in an enclosed hull (D1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carriage: Option<Carriage>,
    /// Sets up in place before it can serve (L01), and packs up to move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<Deploy>,
    /// Serves nearby units from a finite stock (L05); needs `deploy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supply: Option<Supply>,
    /// A finite defensive interceptor, independent of offensive mounts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_protection: Option<ActiveProtection>,
    /// An aircraft's fuel, afterburner and withdrawal refund
    /// (`AirCapability`); needs air mobility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub air: Option<AirCapability>,
    /// Emits a radar signature, independently of sight reach.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub radar_emitter: bool,
    /// Decoys ejected against an incoming decoyable seeker, a charge each.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flares: Option<FlareCapability>,
    /// A radar warning receiver: it finds enemy radars that emit, by
    /// bearing and range, never identifying them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar_receiver: Option<RadarReceiver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke: Option<SmokeCapability>,
}

impl Capabilities {
    /// What makes this set of abilities inadmissible for a type that does or
    /// doesn't fly (`air`): supply serves only once set up, and an aircraft
    /// sets up by landing, so it authors a drop shorter than its set-up and
    /// nothing that drives authors one.
    pub(super) fn problem(&self, air: bool) -> Option<&'static str> {
        if self.supply.is_some() && self.deploy.is_none() {
            return Some("supply serves only when deployed: it needs deploy");
        }
        let deploy = self.deploy?;
        match deploy.drop_seconds {
            None if air => Some("an aircraft with deploy needs capabilities.deploy.drop_seconds"),
            Some(_) if !air => Some("capabilities.deploy.drop_seconds requires air mobility"),
            Some(drop) if !(drop.is_finite() && drop > 0.0 && drop < deploy.seconds) => {
                Some("capabilities.deploy.drop_seconds must be positive and shorter than seconds")
            }
            _ => None,
        }
    }
}

/// A hull's passenger capacity. Only the authored capacities are admitted so
/// the card and simulation share a small, legible set of carriage sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Carriage {
    pub seats: u32,
}

/// How far a radar warning receiver finds an emitting radar.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarReceiver {
    pub range_m: f64,
}

/// The explicit afterburner tradeoff. Both multipliers are authored.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AfterburnerCapability {
    pub speed_factor: f64,
    pub fuel_factor: f64,
}

/// Health/ammunition/fuel shares of an aircraft's withdrawal refund.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundWeights {
    pub health: f64,
    pub ammunition: f64,
    pub fuel: f64,
}

/// An aircraft's fuel, optional afterburner and withdrawal refund. Optional
/// on any air-mobile row (the roster's helicopters author none); a row that
/// authors it must be complete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirCapability {
    pub fuel_capacity_s: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afterburner: Option<AfterburnerCapability>,
    pub refund: RefundWeights,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deploy {
    /// Time to pack up before moving.
    pub pack_seconds: f64,
    /// Full setup duration.
    pub seconds: f64,
    /// Portion of setup spent descending to the ground (aircraft only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_seconds: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    /// Its stock at the start, unless the scenario says otherwise.
    pub stock: u32,
}
