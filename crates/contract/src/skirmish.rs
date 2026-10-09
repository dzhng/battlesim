//! Player match setup and side-scoped economy publication.
use serde::{Deserialize, Serialize};

use crate::catalog::{Faction, TypeIndex};
use crate::encounter::SkirmishSites;
use crate::ids::{Side, Tick};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRules {
    pub credits_per_minute: u32,
    pub starting_minutes: u32,
    pub preparation_s: u32,
    pub max_units: u32,
    pub dispatch_interval_s: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "UncheckedSetup")]
pub struct SkirmishSetup {
    pub factions: [Faction; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_side: Option<Side>,
    pub rules: MatchRules,
    pub sites: SkirmishSites,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedSetup {
    factions: [Faction; 2],
    #[serde(default)]
    ai_side: Option<Side>,
    rules: MatchRules,
    sites: SkirmishSites,
}

impl TryFrom<UncheckedSetup> for SkirmishSetup {
    type Error = String;
    fn try_from(value: UncheckedSetup) -> Result<Self, Self::Error> {
        let r = &value.rules;
        if r.credits_per_minute == 0
            || r.starting_minutes == 0
            || r.max_units == 0
            || r.dispatch_interval_s == 0
            || u64::from(r.credits_per_minute)
                .checked_mul(u64::from(r.starting_minutes))
                .and_then(|n| n.checked_mul(1_000_000))
                .is_none()
        {
            return Err("skirmish requires positive affordable economy and dispatch rules".into());
        }
        value.sites.validate()?;
        Ok(Self {
            factions: value.factions,
            ai_side: value.ai_side,
            rules: value.rules,
            sites: value.sites,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Preparation,
    Active,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
/// Side-scoped reservation handle, independent of physical unit IDs.
pub struct PurchaseId(pub u32);

/// An admitted purchase placement: the unit type, and where a squad's
/// soldiers would stand round the destination (empty for a vehicle), as
/// [`crate::command::MoveDestination::spots`] lays them out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PurchasePreview {
    pub kind: TypeIndex,
    pub spots: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingPurchase {
    pub id: PurchaseId,
    pub kind: TypeIndex,
    pub destination: [f64; 2],
    pub confirmed_tick: Tick,
    pub blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkirmishView {
    pub phase: Phase,
    pub ready: [bool; 2],
    pub preparation_remaining_s: f64,
    pub credits: f64,
    pub occupied_slots: u32,
    pub max_units: u32,
    pub pending: Vec<PendingPurchase>,
    pub objectives: Vec<ObjectiveView>,
    pub scores: [f64; 2],
    pub result: Option<MatchResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveView {
    pub id: String,
    pub center: [f64; 2],
    pub radius_m: f64,
    pub owner: Option<Side>,
    pub capturing: Option<Side>,
    pub capture_progress: f64,
    pub contested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatchResult {
    Winner { side: Side },
    Draw,
}
