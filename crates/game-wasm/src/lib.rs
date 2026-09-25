//! Thin WASM boundary over `sim`: commands in, side-filtered observations out.
use contract::command::CommandEnvelope;
use contract::ids::Side;
use contract::map::MapDefinition;
use contract::scenario::ScenarioDefinition;
use sim::battle::{Battle, Replay};
use sim::math::v3;
use sim::publication;
use sim::world::{export, WorldGeometry};
use wasm_bindgen::prelude::*;

/// Build identity, so the browser can report which simulation it loaded.
#[wasm_bindgen]
pub fn build_id() -> String {
    format!("game-wasm {}", env!("CARGO_PKG_VERSION"))
}

/// Strides, field order and enum tags of the geometry exports.
#[wasm_bindgen]
pub fn world_layout() -> String {
    export::layout_json()
}

/// Lab-only view of authoritative world geometry: exported meshes and direct
/// queries for the geometry probe. Player routes read geometry through the
/// battle authority instead.
#[wasm_bindgen]
pub struct WorldView {
    world: WorldGeometry,
}

#[wasm_bindgen]
impl WorldView {
    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str) -> Result<WorldView, JsError> {
        let map: MapDefinition =
            serde_json::from_str(map_json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WorldView {
            world: WorldGeometry::new(&map),
        })
    }

    pub fn terrain_positions(&self) -> Vec<f32> {
        self.world.export_terrain_positions()
    }

    pub fn terrain_indices(&self) -> Vec<u32> {
        self.world.export_terrain_indices()
    }

    pub fn terrain_triangle_surfaces(&self) -> Vec<u8> {
        self.world.export_terrain_triangle_surfaces()
    }

    pub fn props(&self) -> Vec<f32> {
        self.world.export_props()
    }

    pub fn water(&self) -> Vec<f32> {
        self.world.export_water()
    }

    pub fn forests(&self) -> Vec<f32> {
        self.world.export_forests()
    }

    pub fn slope_cutoff_deg(&self) -> f64 {
        self.world.slope_cutoff_deg()
    }

    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        self.world.height_at(x, y)
    }

    pub fn surface_at(&self, x: f64, y: f64) -> Vec<f64> {
        export::surface_record(self.world.surface_at(x, y))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn raycast(
        &self,
        ox: f64,
        oy: f64,
        oz: f64,
        dx: f64,
        dy: f64,
        dz: f64,
        max_t: f64,
    ) -> Vec<f64> {
        let dir = v3(dx, dy, dz).normalized();
        export::hit_record(self.world.raycast(v3(ox, oy, oz), dir, max_t))
    }

    pub fn obstacle_revision(&self) -> f64 {
        self.world.obstacle_revision() as f64
    }
}

/// Field order and tags of a side publication.
#[wasm_bindgen]
pub fn observation_layout() -> String {
    publication::layout_json()
}

fn js_error(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_side(side: &str) -> Result<Side, JsError> {
    serde_json::from_value(serde_json::Value::String(side.to_owned())).map_err(js_error)
}

/// The battle authority behind the worker. Small rare messages cross as JSON;
/// each tick's side observation is packed into a reusable buffer the host
/// copies out of WASM memory.
#[wasm_bindgen(js_name = Battle)]
pub struct BattleHandle {
    battle: Battle,
    publication: Vec<f32>,
}

#[wasm_bindgen(js_class = Battle)]
impl BattleHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Ok(BattleHandle {
            battle: Battle::new(&setup, seed as u64),
            publication: Vec::new(),
        })
    }

    /// A battle that re-applies a recorded replay and refuses live commands.
    pub fn from_replay(scenario_json: &str, replay_json: &str) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        let replay: Replay = serde_json::from_str(replay_json).map_err(js_error)?;
        let battle = Battle::from_replay(&setup, &replay)
            .map_err(|e| JsError::new(&format!("replay does not match this scenario: {e:?}")))?;
        Ok(BattleHandle {
            battle,
            publication: Vec::new(),
        })
    }

    /// Returns the acknowledgement as JSON.
    pub fn accept(&mut self, command_json: &str) -> Result<String, JsError> {
        let command: CommandEnvelope = serde_json::from_str(command_json).map_err(js_error)?;
        serde_json::to_string(&self.battle.accept(command)).map_err(js_error)
    }

    pub fn step(&mut self) -> f64 {
        self.battle.step() as f64
    }

    pub fn tick(&self) -> f64 {
        self.battle.tick() as f64
    }

    /// State digest as 16 hex digits (a u64 does not fit a JS number).
    pub fn digest(&self) -> String {
        format!("{:016x}", self.battle.digest())
    }

    pub fn replay_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.battle.replay()).map_err(js_error)
    }

    /// Pack `side`'s observation; returns its length in f32s. Read it at
    /// `publication_ptr()` before any other call that may grow memory.
    pub fn publish(&mut self, side: &str) -> Result<usize, JsError> {
        let side = parse_side(side)?;
        publication::pack(self.battle.observe(side), &mut self.publication);
        Ok(self.publication.len())
    }

    pub fn publication_ptr(&self) -> *const f32 {
        self.publication.as_ptr()
    }
}
