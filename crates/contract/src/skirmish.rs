//! Player match setup and side-scoped economy publication.
use serde::{Deserialize, Serialize};

use crate::catalog::{Faction, TypeIndex};
use crate::encounter::SkirmishSites;
use crate::ids::{Side, Tick, UnitId};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRules {
    pub credits_per_minute: u32,
    pub starting_minutes: u32,
    pub preparation_s: u32,
    pub max_units: u32,
    pub dispatch_interval_s: u32,
    /// How long a side must hold an objective alone to take it.
    pub capture_s: u32,
    /// How long a side holding a majority of the objectives alone takes to
    /// win; holding fewer, its score climbs more slowly.
    pub majority_s: u32,
    /// The score a side wins at.
    pub victory_score: f64,
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
        if value.ai_side == Some(Side::Neutral) {
            return Err("skirmish does not support neutral ai_side".into());
        }
        let r = &value.rules;
        if r.credits_per_minute == 0
            || r.starting_minutes == 0
            || r.max_units == 0
            || r.dispatch_interval_s == 0
            || r.capture_s == 0
            || r.majority_s == 0
            || !(r.victory_score.is_finite() && r.victory_score > 0.0)
            || u64::from(r.credits_per_minute)
                .checked_mul(u64::from(r.starting_minutes))
                .and_then(|n| n.checked_mul(1_000_000))
                .is_none()
        {
            return Err(
                "skirmish requires positive affordable economy, dispatch and objective rules"
                    .into(),
            );
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

/// An admitted purchase placement: the unit type, where it will be sent
/// (a ground unit's point snapped to the nearest ground it stands on, as a
/// move's is), and where a squad's soldiers would stand round it (empty for
/// a vehicle), as [`crate::command::MoveDestination::spots`] lays them out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PurchasePreview {
    pub kind: TypeIndex,
    pub destination: [f64; 2],
    pub spots: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingPurchase {
    pub id: PurchaseId,
    pub kind: TypeIndex,
    pub destination: [f64; 2],
    pub confirmed_tick: Tick,
    pub blocked: bool,
    /// The tick a bought aircraft reaches the map, once its wait off the
    /// map has begun (D41); None for anything that enters through the queue.
    pub arrival_tick: Option<Tick>,
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
    /// The complete match report is revealed once the referee finishes the match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<MatchReport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchReport {
    pub totals: [MatchTotals; 2],
    pub events: Vec<MatchEvent>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MatchTotals {
    pub spawned: u32,
    pub spent: u32,
    pub killed: u32,
    pub refunded: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchUnit {
    pub side: Side,
    pub id: UnitId,
    pub kind: TypeIndex,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatchEvent {
    Spawn {
        tick: Tick,
        unit: MatchUnit,
        cost: u32,
    },
    Refund {
        tick: Tick,
        unit: MatchUnit,
        credits: f64,
    },
    Kill {
        tick: Tick,
        victim: MatchUnit,
        victim_cost: u32,
        killer: Option<MatchUnit>,
        killer_cost: Option<u32>,
    },
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
