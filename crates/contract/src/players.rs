//! Player seats shared by authored scenarios and skirmish matches.

use crate::catalog::Faction;
use crate::ids::Side;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerKind {
    Human,
    ActiveAi,
    PassiveAi,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub id: String,
    pub side: Side,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Faction>,
    pub controller: ControllerKind,
    #[serde(default)]
    pub human_capable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningPurchase {
    pub variant: String,
    pub destination: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerIndex(pub u16);

/// Which seating a player list must satisfy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seating {
    /// An authored scenario: a Neutral player may hold the world's props and
    /// civilians, with no faction and no human (D54).
    Authored,
    /// A skirmish: only combatants, at least one a side, one faction a side.
    Skirmish,
}

/// The one validator of a player list, for authored bundles, skirmish
/// scenarios and preparation requests alike.
pub fn validate_players(players: &[Player], seating: Seating) -> Result<(), String> {
    if players.is_empty() {
        return Err("player list must not be empty".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for player in players {
        if player.id.trim().is_empty() {
            return Err("player id must not be empty".into());
        }
        if !ids.insert(player.id.as_str()) {
            return Err(format!("duplicate player id {}", player.id));
        }
        if player.controller == ControllerKind::Human && !player.human_capable {
            return Err(format!("human player {} must be human_capable", player.id));
        }
        match (player.side, player.faction) {
            (Side::Neutral, _) if seating == Seating::Skirmish => {
                return Err("skirmish players cannot be neutral".into())
            }
            (Side::Neutral, Some(_)) => {
                return Err(format!(
                    "neutral player {} cannot carry a faction",
                    player.id
                ))
            }
            (Side::Neutral, None) if player.human_capable => {
                return Err(format!(
                    "neutral player {} cannot be human-capable",
                    player.id
                ))
            }
            (Side::Blue | Side::Red, None) => {
                return Err(format!(
                    "side {:?} player {} needs a faction",
                    player.side, player.id
                ))
            }
            _ => {}
        }
    }
    if seating == Seating::Skirmish {
        for side in Side::COMBATANTS {
            let side_players: Vec<_> = players.iter().filter(|p| p.side == side).collect();
            if side_players.is_empty() {
                return Err(format!("side {side:?} must have at least one player"));
            }
            let faction = side_players[0].faction;
            if side_players.iter().any(|p| p.faction != faction) {
                return Err(format!("side {side:?} must use one faction"));
            }
        }
    }
    Ok(())
}

/// Validate the fixture-controlled player cap for each combatant side.
pub fn validate_players_per_side(
    players: &[Player],
    max_players_per_side: usize,
) -> Result<(), String> {
    for side in Side::COMBATANTS {
        let count = players.iter().filter(|player| player.side == side).count();
        if count > max_players_per_side {
            return Err(format!(
                "side {side:?} exceeds max_players_per_side {max_players_per_side}"
            ));
        }
    }
    Ok(())
}

/// Entry arms per side: both sides get the larger side's count, so every
/// map stays mirror-fair however unevenly the players are split.
pub fn entries_per_side(players: &[Player]) -> usize {
    let larger = Side::COMBATANTS
        .into_iter()
        .map(|side| players.iter().filter(|p| p.side == side).count())
        .max()
        .unwrap_or(0);
    if larger == 0 {
        return 0;
    }
    crate::skirmish_sizes::tier(larger).entry_count
}

/// Validate that each side's map arms number `entries_per_side`.
pub fn validate_entry_counts(
    players: &[Player],
    entries: &[crate::encounter::EntrySite],
) -> Result<(), String> {
    let wanted = entries_per_side(players);
    for side in Side::COMBATANTS {
        let entries_on_side = entries.iter().filter(|entry| entry.side == side).count();
        if entries_on_side != wanted {
            return Err(format!(
                "side {side:?} has {entries_on_side} entries; the players need {wanted} per side"
            ));
        }
    }
    Ok(())
}

impl crate::scenario::ScenarioDefinition {
    /// A skirmish's contract: who may play, that its players buy every force
    /// (nothing is authored), that each opening names a player, and that the
    /// map has the roads the players need.
    pub fn validate_player_contract(&self) -> Result<(), String> {
        let Some(skirmish) = &self.skirmish else {
            return Ok(());
        };
        validate_players(&self.players, Seating::Skirmish)?;
        validate_players_per_side(&self.players, skirmish.rules.max_players_per_side as usize)?;
        if !self.units.is_empty()
            || !self.events.is_empty()
            || !self.scripts.is_empty()
            || self.opponent.is_some()
            || self.encounter.is_some()
        {
            return Err(
                "a skirmish carries no authored forces, events, scripts, opponent or \
                 encounter: its players buy what it fields"
                    .into(),
            );
        }
        for player_id in skirmish.openings.keys() {
            if !self.players.iter().any(|player| player.id == *player_id) {
                return Err(format!(
                    "opening {player_id} does not name a skirmish player"
                ));
            }
        }
        validate_entry_counts(&self.players, &skirmish.sites.entries)
    }
}
