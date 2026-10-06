//! What a battle is prepared from: where its map comes from, which encounter
//! is laid on it, and the two seeds a player may share. Preparation resolves
//! the map through the one map owner (`maps`), then lays the encounter; the
//! battle runs on the scenario that makes and never asks where it came from.
use crate::identity::Seed;
use crate::maps::{MapId, MapSource};
use serde::{Deserialize, Serialize};

/// The largest battle seed: the battle's constructor takes its seed across
/// the JavaScript boundary as a number, which holds whole numbers exactly
/// only this far (`Number.MAX_SAFE_INTEGER`).
pub const BATTLE_SEED_MAX: u64 = (1 << 53) - 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareBattleRequest {
    pub map_source: MapSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skirmish: Option<[crate::catalog::Faction; 2]>,
    /// Which encounter. On a generated map it is a recipe of
    /// `fixtures/encounters.json`, which the planner places; on a catalogue
    /// map it is the map's saved encounter of that name
    /// (`encounters/<recipe_id>.json`), which was placed when it was saved.
    pub recipe_id: String,
    /// The planner's seed, apart from the map's. A saved encounter does not
    /// read it.
    pub encounter_seed: Seed,
    /// The battle's own random input, at most `BATTLE_SEED_MAX`.
    pub battle_seed: u64,
}

/// Why a request cannot be prepared: the field at fault, as a JSON path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RequestDiagnostic {
    pub code: &'static str,
    pub feature: Option<String>,
    pub location: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RequestOutcome {
    Ok { request: Box<PrepareBattleRequest> },
    Error { diagnostics: Vec<RequestDiagnostic> },
}

fn refused(location: &str, message: String) -> RequestDiagnostic {
    RequestDiagnostic {
        code: "invalid_request",
        feature: None,
        location: location.into(),
        message,
    }
}

impl PrepareBattleRequest {
    /// A request as it crosses a JSON boundary, or what is wrong with it. A
    /// seed must be canonical u64 decimal text; a number is refused, because
    /// the sender may already have rounded it.
    pub fn from_json(json: &str) -> Result<Self, RequestDiagnostic> {
        let request: Self =
            serde_json::from_str(json).map_err(|error| refused("$", error.to_string()))?;
        MapId::new(&request.recipe_id).map_err(|_| {
            refused(
                "$.recipe_id",
                "an encounter is named by one lowercase word".into(),
            )
        })?;
        if request.skirmish.is_some()
            && !matches!(&request.map_source,
            MapSource::Generated { request: map } if map.profile == crate::generation::GenerationProfile::Skirmish)
        {
            return Err(refused(
                "$.map_source",
                "a faction skirmish requires generated skirmish geography".into(),
            ));
        }
        if request.battle_seed > BATTLE_SEED_MAX {
            return Err(refused(
                "$.battle_seed",
                format!("the battle seed must be a whole number from 0 to {BATTLE_SEED_MAX}"),
            ));
        }
        Ok(request)
    }
}

/// `PrepareBattleRequest::from_json` across a JSON boundary: the request in
/// its canonical form, or the diagnostic.
pub fn check_request_json(json: &str) -> String {
    let outcome = match PrepareBattleRequest::from_json(json) {
        Ok(request) => RequestOutcome::Ok {
            request: Box::new(request),
        },
        Err(diagnostic) => RequestOutcome::Error {
            diagnostics: vec![diagnostic],
        },
    };
    serde_json::to_string(&outcome).expect("a request outcome is plain data")
}
