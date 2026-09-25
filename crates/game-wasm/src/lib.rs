//! Thin WASM boundary over `sim`: commands in, side-filtered observations out.
use contract::map::MapDefinition;
use sim::math::v3;
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
