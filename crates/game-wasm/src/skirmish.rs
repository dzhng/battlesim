//! Skirmish setup, for pages that author or seat a skirmish. The rules stay
//! owned by `sim::skirmish`; these calls only cross the boundary.
use crate::js_error;
use contract::players::{Player, Seating};
use wasm_bindgen::prelude::*;

/// The whole skirmish setup (`SkirmishSetup`: match rules, sites and empty
/// openings) for an authored economy map of `[width, depth]` seating
/// `players_json`: `sim::skirmish::setup_for` over the synthetic sites, with
/// as many roads a side as the players need. The players must form a valid
/// skirmish.
#[wasm_bindgen]
pub fn synthetic_skirmish_setup(size_json: &str, players_json: &str) -> Result<String, JsError> {
    let size: [f64; 2] = serde_json::from_str(size_json).map_err(js_error)?;
    let players: Vec<Player> = serde_json::from_str(players_json).map_err(js_error)?;
    contract::players::validate_players(&players, Seating::Skirmish).map_err(js_error)?;
    let sites = sim::skirmish::synthetic_sites(size, contract::players::entries_per_side(&players));
    serde_json::to_string(&sim::skirmish::setup_for(
        &players,
        sites,
        sim::skirmish::starter_rules(),
    ))
    .map_err(js_error)
}

/// Each entry's lane, in list order, as `{ side, lane, holder }` (the
/// player id holding it, or null), and `opening`: the entry index a seated
/// player's view opens on, or null. `sim::entries` owns both; the page
/// never numbers lanes itself.
#[wasm_bindgen]
pub fn skirmish_lanes(sites_json: &str, players_json: &str) -> Result<String, JsError> {
    let sites: contract::encounter::SkirmishSites =
        serde_json::from_str(sites_json).map_err(js_error)?;
    let players: Vec<Player> = serde_json::from_str(players_json).map_err(js_error)?;
    serde_json::to_string(&lanes(&sites, &players)).map_err(js_error)
}

fn lanes(sites: &contract::encounter::SkirmishSites, players: &[Player]) -> serde_json::Value {
    use sim::entries::{self, EntryIndex};
    let sides: Vec<_> = players.iter().map(|player| player.side).collect();
    let homes = entries::homes(sites, &sides);
    let entries: Vec<_> = (0..sites.entries.len())
        .map(|index| {
            let holder = homes
                .iter()
                .position(|home| *home == Some(EntryIndex(index)))
                .map(|player| players[player].id.clone());
            serde_json::json!({
                "side": sites.entries[index].side,
                "lane": entries::lane_number(sites, EntryIndex(index)).0,
                "holder": holder,
            })
        })
        .collect();
    serde_json::json!({
        "entries": entries,
        "opening": entries::opening(sites, players).map(|index| index.0),
    })
}
