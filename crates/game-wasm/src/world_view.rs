//! The page's view of a map's static geometry (`WorldView`) and the layout
//! of its exports (`world_layout`).
use contract::map::MapDefinition;
use contract::scenario::Rules;
use sim::math::v3;
use sim::world::{export, WorldGeometry};
use wasm_bindgen::prelude::*;

use crate::js_error;

/// Strides, field order and enum tags of the geometry exports, with the
/// prop types' body columns and appearance bindings, for `rules_json` (a scenario's rules:
/// the fixture with its catalog).
#[wasm_bindgen]
pub fn world_layout(rules_json: &str) -> Result<String, JsError> {
    let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
    sim::admit(&rules).map_err(js_error)?;
    Ok(export::layout_json(&rules.catalog))
}

/// The page's view of a map's static geometry, built by the simulation's own
/// world code: exported meshes and direct queries (picking, ground height,
/// surface, learned foliage, camera clearance, lab probes). It holds no
/// navigation and no battle state.
#[wasm_bindgen]
pub struct WorldView {
    world: WorldGeometry,
    extents: contract::map::MapExtents,
}

#[wasm_bindgen]
impl WorldView {
    /// The map's geometry as the simulation builds it under `rules_json` (a
    /// scenario's rules: the fixture with its catalog).
    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str, rules_json: &str) -> Result<WorldView, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        sim::admit(&rules).map_err(js_error)?;
        Ok(WorldView {
            extents: map.extents(),
            world: WorldGeometry::new(&map, &rules),
        })
    }

    /// Map-owned landscape bounds; the display environment is separate.
    pub fn extents(&self) -> String {
        serde_json::to_string(&self.extents).expect("finite map extents")
    }

    /// Sampled surface descriptor; absent vertex pages have height zero.
    pub fn terrain_grid(&self) -> String {
        self.world.export_terrain_grid()
    }

    pub fn terrain_page_ids(&self) -> Vec<u32> {
        self.world.export_terrain_page_ids()
    }

    pub fn terrain_heights(&self) -> Vec<f32> {
        self.world.export_terrain_heights()
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

    /// Public immutable building/template references; geometry stays in props.
    pub fn buildings(&self) -> String {
        self.world.export_buildings()
    }

    pub fn props(&self) -> Vec<f32> {
        self.world.export_props()
    }

    pub fn rivers(&self) -> Vec<f32> {
        self.world.export_rivers()
    }

    pub fn river_runs(&self) -> Vec<f32> {
        self.world.export_river_runs()
    }

    pub fn swamps(&self) -> Vec<f32> {
        self.world.export_swamps()
    }

    pub fn forests(&self) -> Vec<f32> {
        self.world.export_forests()
    }

    pub fn forest_trunk_ranges(&self) -> Vec<u32> {
        self.world.export_forest_trunk_ranges()
    }

    pub fn forest_rect_ids(&self) -> Vec<u32> {
        self.world.export_forest_rect_ids()
    }
    pub fn forest_metadata(&self) -> Vec<f32> {
        self.world.export_forest_metadata()
    }
    pub fn forest_strokes(&self) -> Vec<f32> {
        self.world.export_forest_strokes()
    }
    pub fn forest_triangles(&self) -> Vec<f32> {
        self.world.export_forest_triangles()
    }
    pub fn forest_boundaries(&self) -> Vec<f32> {
        self.world.export_forest_boundaries()
    }

    /// Sparse foliage: `[nx, ny, cell_m]`, then non-open
    /// `[column, row, canopy_m, depth_per_m]` records in row order.
    pub fn foliage(&self) -> Vec<f32> {
        self.world.export_foliage()
    }

    /// The public static foliage minus only the ground clearing this side learned.
    /// Sorted pairs hold16×16tile ID and local start+length*256. Query only
    /// forest cells through the borrowed spans; never rebuild a cell mask.
    pub fn foliage_cleared(&self, cleared_runs: &[u32], cols: u32, cell_m: f64) -> Vec<f32> {
        self.world.export_foliage_cleared(|x, y| {
            let (i, j) = ((x / cell_m).floor(), (y / cell_m).floor());
            if i < 0.0 || j < 0.0 || i >= cols as f64 {
                return false;
            }
            let (i, j) = (i as u32, j as u32);
            let tile = j / 16 * cols.div_ceil(16) + i / 16;
            let cell = j % 16 * 16 + i % 16;
            let (mut lo, mut hi) = (0, cleared_runs.len() / 2);
            while lo < hi {
                let mid = (lo + hi) / 2;
                let (key, start) = (cleared_runs[mid * 2], cleared_runs[mid * 2 + 1] % 256);
                if key < tile || (key == tile && start <= cell) {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo == 0 {
                return false;
            }
            let (key, span) = (cleared_runs[(lo - 1) * 2], cleared_runs[(lo - 1) * 2 + 1]);
            key == tile && cell < span % 256 + span / 256
        })
    }

    pub fn surface_strokes(&self) -> Vec<f32> {
        self.world.export_surface_strokes()
    }

    pub fn surface_runs(&self) -> Vec<f32> {
        self.world.export_surface_runs()
    }

    pub fn surface_triangles(&self) -> Vec<f32> {
        self.world.export_surface_triangles()
    }

    pub fn surface_boundaries(&self) -> Vec<f32> {
        self.world.export_surface_boundaries()
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
}
