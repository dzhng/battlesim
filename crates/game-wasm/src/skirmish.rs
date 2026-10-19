//! Skirmish setup and the size rule, for pages that author or seat a
//! skirmish. The rules stay owned by `sim::skirmish` and
//! `contract::skirmish_sizes`; these calls only cross the boundary.
use crate::js_error;
use contract::players::{Player, Seating};
use wasm_bindgen::prelude::*;

/// Synthetic skirmish entry and objective sites for a `[width, depth]` map
/// with no generated ones. The geometry and ordering stay owned by
/// `sim::skirmish`.
#[wasm_bindgen]
pub fn synthetic_sites(size_json: &str, entries_per_side: u32) -> Result<String, JsError> {
    let size: [f64; 2] = serde_json::from_str(size_json).map_err(js_error)?;
    serde_json::to_string(&sim::skirmish::synthetic_sites(
        size,
        entries_per_side as usize,
    ))
    .map_err(js_error)
}

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

/// What a side of `players_per_side` players gets, and how many a side may
/// have: `{"max_players_per_side": n, "tier": SizeTier}`, the tier giving
/// the smallest map size, its extent and the roads a side enters by. Pass
/// the larger side's count: both sides get its roads.
#[wasm_bindgen]
pub fn skirmish_size_rule(players_per_side: u32) -> String {
    serde_json::json!({
        "max_players_per_side": sim::skirmish::starter_rules().max_players_per_side,
        "tier": contract::skirmish_sizes::tier(players_per_side as usize),
    })
    .to_string()
}
