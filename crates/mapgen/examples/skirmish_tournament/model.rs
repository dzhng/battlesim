use contract::catalog::Faction;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Corpus {
    pub(crate) revision: u32,
    pub(crate) rows: Vec<Row>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    pub(crate) id: String,
    pub(crate) map: MapSource,
    pub(crate) seed: u64,
    pub(crate) horizon_ticks: u64,
    pub(crate) no_progress_ticks: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum MapSource {
    Saved(String),
    Generated(GeneratedMap),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GeneratedMap {
    pub(crate) kind: String,
    #[serde(rename = "type")]
    pub(crate) map_type: String,
    pub(crate) size: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Complete,
    Refused,
    Timeout,
    NoProgress,
}

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) id: String,
    pub(crate) seed: u64,
    pub(crate) max_battle_seconds: u64,
    pub(crate) battle_seconds: f64,
    pub(crate) status: Status,
    pub(crate) ticks: u64,
    pub(crate) battle_digest: u64,
    pub(crate) refused_orders: u32,
    /// Blue's and Red's refused orders.
    pub(crate) refused_by_side: [u32; 2],
    pub(crate) accepted_orders: u32,
    pub(crate) attempted_orders: u32,
    pub(crate) policy: &'static str,
    pub(crate) factions: [Faction; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diagnostic: Option<String>,
    pub(crate) result: Option<contract::skirmish::MatchResult>,
    pub(crate) objective_first_owner_ticks: std::collections::BTreeMap<String, u64>,
    pub(crate) first_contact_tick: Option<u64>,
    pub(crate) recon_sightings: u32,
    pub(crate) doctrine_pivots: [u32; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) map_identity: Option<String>,
    pub(crate) site_count: u32,
    pub(crate) replay_digest: Option<String>,
    pub(crate) replay_match: Option<bool>,
    pub(crate) credit_spend: [u64; 2],
    /// Each side's purchases accepted before the battle starts.
    pub(crate) opening: [Vec<String>; 2],
    pub(crate) living_cost: [u64; 2],
    pub(crate) recon_range_m: f64,
    /// Seconds each unit kind needing a clear launch path stood held for it
    /// (`BlockedTrajectory`), both sides summed.
    pub(crate) launcher_held_s: std::collections::BTreeMap<String, f64>,
    pub(crate) air: Option<[super::runner::AirSide; 2]>,
}

pub(crate) type MovementSignature = Vec<(u32, u64, u64, Option<(u64, u64)>)>;
pub(crate) type ProgressSignature = (u32, u32, u64, u64, u64, MovementSignature);
