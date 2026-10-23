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
    serde_json::to_string(&sim::skirmish::setup_for(&players, sites)).map_err(js_error)
}
