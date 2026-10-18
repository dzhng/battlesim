//! Player match setup and side-scoped economy publication.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::catalog::TypeIndex;
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
    pub max_players_per_side: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "UncheckedSetup")]
pub struct SkirmishSetup {
    pub rules: MatchRules,
    pub sites: SkirmishSites,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub openings: BTreeMap<String, Vec<crate::players::OpeningPurchase>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedSetup {
    rules: MatchRules,
    sites: SkirmishSites,
    #[serde(default)]
    openings: BTreeMap<String, Vec<crate::players::OpeningPurchase>>,
}

impl TryFrom<UncheckedSetup> for SkirmishSetup {
    type Error = String;
    fn try_from(value: UncheckedSetup) -> Result<Self, Self::Error> {
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
            || r.max_players_per_side == 0
        {
            return Err(
                "skirmish requires positive affordable economy, dispatch and objective rules"
                    .into(),
            );
        }
        value.sites.validate()?;
        Ok(Self {
            rules: value.rules,
            sites: value.sites,
            openings: value.openings,
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
    /// The entry chosen for this purchase while it is being reserved.
    pub entry: usize,
    pub destination: [f64; 2],
    pub spots: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingPurchase {
    pub id: PurchaseId,
    pub kind: TypeIndex,
    /// The entry chosen once, when the purchase is reserved (D37).
    #[serde(default)]
    pub entry: usize,
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
    pub players: Vec<PlayerView>,
    #[serde(skip)]
    pub ready: [bool; 2],
    pub preparation_remaining_s: f64,
    #[serde(skip)]
    pub credits: f64,
    #[serde(skip)]
    pub unit_count: u32,
    #[serde(skip)]
    pub max_units: u32,
    #[serde(skip)]
    pub pending: Vec<PendingPurchase>,
    pub objectives: Vec<ObjectiveView>,
    pub scores: [f64; 2],
    pub result: Option<MatchResult>,
    /// The complete match report is revealed once the referee finishes the match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<MatchReport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub index: crate::players::PlayerIndex,
    pub id: String,
    pub controller: crate::players::ControllerKind,
    pub ready: bool,
    pub credits: f64,
    pub unit_count: u32,
    pub cap: u32,
    pub pending: Vec<PendingPurchase>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchReport {
    pub totals: Vec<MatchTotals>,
    pub events: Vec<MatchEvent>,
}

impl MatchReport {
    pub fn side_totals(&self, players: &[crate::players::Player]) -> [MatchTotals; 2] {
        let mut totals = [MatchTotals::default(); 2];
        for (player, player_totals) in players.iter().zip(&self.totals) {
            let side = player.side.index();
            totals[side].spawned += player_totals.spawned;
            totals[side].spent += player_totals.spent;
            totals[side].killed += player_totals.killed;
            totals[side].refunded += player_totals.refunded;
        }
        totals
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<crate::players::PlayerIndex>,
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
