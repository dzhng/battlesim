//! Worker-owned physical preparation, adopted by the battle authority.
use super::*;

/// A worker-owned world: skirmish admission lends it, battle startup consumes it.
#[wasm_bindgen]
pub struct PreparedWorld {
    map: MapDefinition,
    rules: Rules,
    prepared: sim::encounter::PreparedMap,
    map_digest: u64,
    rules_digest: u64,
}

#[wasm_bindgen]
impl PreparedWorld {
    /// Final skirmish placement/route admission on this prepared physical world.
    pub fn admit_skirmish(&self, sites_json: &str, players_json: &str) -> Result<String, JsError> {
        let sites = serde_json::from_str(sites_json).map_err(js_error)?;
        let players: Vec<contract::players::Player> =
            serde_json::from_str(players_json).map_err(js_error)?;
        let admitted = sim::map_analysis::admit_skirmish(
            &self.prepared.queries(&self.map, &sites),
            &self.rules,
            &players,
        )
        .map_err(js_error)?;
        serde_json::to_string(&admitted).map_err(js_error)
    }
    /// Build the complete generated skirmish scenario from this prepared
    /// world's own map and rules serialization.
    pub fn skirmish_scenario(
        &self,
        sites_json: &str,
        players_json: &str,
    ) -> Result<String, JsError> {
        let sites: contract::encounter::EncounterSites =
            serde_json::from_str(sites_json).map_err(js_error)?;
        let players: Vec<contract::players::Player> =
            serde_json::from_str(players_json).map_err(js_error)?;
        let setup = sim::skirmish::setup_for(
            &players,
            sites
                .skirmish
                .ok_or_else(|| JsError::new("skirmish sites are missing"))?,
        );
        setup.sites.validate().map_err(js_error)?;
        serde_json::to_string(&ScenarioDefinition {
            players,
            skirmish: Some(setup),
            map: self.map.clone(),
            rules: self.rules.clone(),
            units: vec![],
            events: vec![],
            scripts: vec![],
            authored_scripts: vec![],
            victory: None,
            opponent: None,
            encounter: None,
        })
        .map_err(js_error)
    }

    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str, rules_json: &str) -> Result<PreparedWorld, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        Self::build(map, rules)
    }

    pub fn from_scenario(scenario_json: &str) -> Result<PreparedWorld, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Self::build(setup.map, setup.rules)
    }

    /// Admit a self-contained download without consulting fixtures or a map generator.
    pub fn from_bundle(bundle_json: &str) -> Result<PreparedWorld, JsError> {
        let bundle = admit_bundle(bundle_json).map_err(js_error)?;
        let scenario: ScenarioDefinition =
            serde_json::from_str(&bundle.scenario_json().map_err(js_error)?).map_err(js_error)?;
        Self::build(scenario.map, scenario.rules)
    }

    pub fn extents(&self) -> String {
        serde_json::to_string(&self.map.extents()).expect("finite map extents")
    }

    pub fn into_battle(self, scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup = self.scenario(scenario_json)?;
        Ok(BattleHandle {
            battle: Battle::from_prepared(&setup, seed as u64, self.prepared),
            publisher: Publisher::new(),
        })
    }

    pub fn into_replay(
        self,
        scenario_json: &str,
        replay_json: &str,
    ) -> Result<BattleHandle, JsError> {
        let setup = self.scenario(scenario_json)?;
        let replay: Replay = serde_json::from_str(replay_json).map_err(js_error)?;
        let battle = Battle::from_prepared_replay(&setup, &replay, self.prepared)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(BattleHandle {
            battle,
            publisher: Publisher::new(),
        })
    }
}

fn identity_digest<T: serde::Serialize>(value: &T) -> Result<u64, JsError> {
    Ok(sim::digest::of_str(
        &serde_json::to_string(value).map_err(js_error)?,
    ))
}

impl PreparedWorld {
    pub(crate) fn build(map: MapDefinition, rules: Rules) -> Result<Self, JsError> {
        sim::admit(&rules).map_err(js_error)?;
        let map_digest = identity_digest(&map)?;
        let rules_digest = identity_digest(&rules)?;
        let prepared = sim::encounter::PreparedMap::new(&map, &rules);
        Ok(Self {
            map,
            rules,
            prepared,
            map_digest,
            rules_digest,
        })
    }

    fn scenario(&self, scenario_json: &str) -> Result<ScenarioDefinition, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        sim::admit(&setup.rules).map_err(js_error)?;
        if identity_digest(&setup.map)? != self.map_digest
            || identity_digest(&setup.rules)? != self.rules_digest
        {
            return Err(JsError::new("prepared map and rules do not match scenario"));
        }
        setup.validate_player_contract().map_err(js_error)?;
        Ok(setup)
    }
}
