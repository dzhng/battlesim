//! A unit type's optional abilities (`Capabilities`), each present only on
//! the types that have them, and the ability records they hold.
use super::{ActiveProtection, FlareCapability, Mobility, SmokeCapability};
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

impl super::UnitType {
    /// What makes these abilities inadmissible: supply serves only once set
    /// up, and a landing aircraft needs a shorter drop and a rotor radius for
    /// its clear spot; ground movers do not author a drop.
    pub(super) fn capability_problem(&self) -> Option<&'static str> {
        let capabilities = &self.capabilities;
        let mobility = &self.mobility;
        if capabilities.supply.is_some() && capabilities.deploy.is_none() {
            return Some("supply serves only when deployed: it needs deploy");
        }
        let deploy = capabilities.deploy?;
        if matches!(mobility.flight(), Some(super::FlightModel::FixedWing)) {
            return Some("an aircraft with deploy requires rotor flight");
        }
        let rotor = match mobility {
            Mobility::Air { rotor_radius_m, .. } => Some(*rotor_radius_m),
            _ => None,
        };
        match (deploy.drop_seconds, rotor) {
            (None, Some(_)) => {
                Some("an aircraft with deploy needs capabilities.deploy.drop_seconds")
            }
            (Some(_), None) => Some("capabilities.deploy.drop_seconds requires air mobility"),
            (Some(drop), _) if !(drop.is_finite() && drop > 0.0 && drop < deploy.seconds) => {
                Some("capabilities.deploy.drop_seconds must be positive and shorter than seconds")
            }
            (_, Some(None)) => Some("mobility: an aircraft with deploy needs rotor_radius_m"),
            _ if self.landing().is_some_and(|landing| {
                !(landing.rotor_radius_m.is_finite() && landing.rotor_radius_m > 0.0)
            }) =>
            {
                Some("mobility: rotor_radius_m must be positive")
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
