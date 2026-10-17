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

/// Number of shared entry arms for a side with this many players.
pub fn entry_count(players: usize) -> usize {
    match players {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 3,
    }
}

/// Validate the topology shared by authored bundles and skirmish requests.
pub fn validate_players(players: &[Player]) -> Result<(), String> {
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
        if player.side == Side::Neutral {
            return Err("skirmish players cannot be neutral".into());
        }
        if player.faction.is_none() {
            return Err(format!(
                "side {:?} player {} needs a faction",
                player.side, player.id
            ));
        }
        if player.controller == ControllerKind::Human && !player.human_capable {
            return Err(format!("human player {} must be human_capable", player.id));
        }
    }
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

/// Resolve the faction assigned to a combatant side.
pub fn side_faction(players: &[Player], side: Side) -> Option<Faction> {
    players
        .iter()
        .find(|player| player.side == side)
        .and_then(|player| player.faction)
}

/// Entry arms per side: both sides get the larger side's count, so every
/// map stays mirror-fair however unevenly the players are split.
pub fn entries_per_side(players: &[Player]) -> usize {
    Side::COMBATANTS
        .into_iter()
        .map(|side| entry_count(players.iter().filter(|p| p.side == side).count()))
        .max()
        .unwrap_or(0)
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
