//! The map's ground on the page's presentation (`static_world::ground`):
//! resolved from the map's exports under the biome into the terrain's and
//! grass's GPU tables and the water's cells, and its scenery placed for the
//! frames' static batches.
#[path = "terrain_partition.rs"]
mod terrain_partition;

use crate::{error, Presentation};
use presentation::presentation::Presentation as Inner;
use presentation::static_world::ground::field::Field;
use presentation::static_world::ground::mesh;
use presentation::static_world::ground::tables::Grid;
use presentation::static_world::ground::walk::Quality as SceneryQuality;
use presentation::static_world::ground::{self, biome::Biome, Ground, Source, Tuning};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// `WorldView.terrain_grid()`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridHeader {
    nx: usize,
    ny: usize,
    spacing: f64,
    page_size: usize,
    min_height: f64,
}

fn ground(inner: &Inner) -> Result<&Ground, JsError> {
    inner
        .ground()
        .ok_or_else(|| JsError::new("no ground is set"))
}

#[wasm_bindgen]
impl Presentation {
    /// The packed height pages consumed by the renderer's fog and terrain
    /// compute passes (`page_size`, directory, then height bit patterns).
    pub fn pages(&self) -> Result<Vec<u32>, JsError> {
        Ok(ground(&self.inner)?.source.grid.pages())
    }

    /// Sets the map's ground: `extents` is the physical map's `[min x, min y,
    /// max x, max y]`, the playable rectangle, then its rendered
    /// surroundings' where it has them;
    /// `grid` is `WorldView.terrain_grid()` with its page ids and heights;
    /// the streams are `WorldView`'s exports of the same names; `layout` is
    /// `world_layout()`; `biome` the biome's file and `family` the map's
    /// regional family (or none); `tuning` the fixture's `presentation` block.
    #[allow(clippy::too_many_arguments)]
    pub fn set_ground(
        &mut self,
        extents: &[f64],
        grid: &str,
        page_ids: Vec<u32>,
        heights: Vec<f32>,
        props: Vec<f32>,
        foliage: Vec<f32>,
        surface_strokes: Vec<f32>,
        surface_runs: Vec<f32>,
        surface_triangles: Vec<f32>,
        surface_boundaries: Vec<f32>,
        rivers: Vec<f32>,
        river_runs: Vec<f32>,
        swamps: Vec<f32>,
        forests: Vec<f32>,
        forest_rect_ids: Vec<u32>,
        forest_metadata: Vec<f32>,
        forest_strokes: Vec<f32>,
        forest_triangles: Vec<f32>,
        forest_boundaries: Vec<f32>,
        forest_trunk_ranges: Vec<u32>,
        layout: &str,
        biome: &str,
        family: Option<String>,
        tuning: &str,
    ) -> Result<(), JsError> {
        let g: GridHeader = serde_json::from_str(grid).map_err(error)?;
        let source = Source {
            map: extents[..4].try_into().unwrap(),
            playable: extents[4..8].try_into().unwrap(),
            rendered: extents.get(8..12).map(|r| r.try_into().unwrap()),
            grid: Grid {
                nx: g.nx,
                ny: g.ny,
                spacing: g.spacing,
                page_size: g.page_size,
                min_height: g.min_height,
                page_ids,
                heights,
            },
            props,
            kinds: ground::prop_kinds(layout).map_err(error)?,
            foliage,
            surface_strokes,
            surface_runs,
            surface_triangles,
            surface_boundaries,
            rivers,
            river_runs,
            swamps,
            forests,
            forest_rect_ids,
            forest_metadata,
            forest_strokes,
            forest_triangles,
            forest_boundaries,
            forest_trunk_ranges,
        };
        let biome = Biome::load(biome, family.as_deref()).map_err(error)?;
        self.grass = Some(biome.grass.clone());
        let value = serde_json::from_str(tuning).map_err(error)?;
        let tuning = Tuning::from_value(&value).map_err(error)?;
        self.inner.set_ground(source, biome, tuning).map_err(error)
    }

    /// Grows the grass by `rules` (JSON, the biome's `grass` row) in place of
    /// the biome's, or by the biome's again: a lab compares fields that
    /// differ in a single number.
    pub fn retune_grass(&mut self, rules: Option<String>) -> Result<(), JsError> {
        let rules = match rules {
            Some(text) => serde_json::from_str(&text).map_err(error)?,
            None => self
                .grass
                .clone()
                .ok_or_else(|| JsError::new("no ground is set"))?,
        };
        let g = self
            .inner
            .ground_mut()
            .ok_or_else(|| JsError::new("no ground is set"))?;
        g.biome.grass = rules;
        Ok(())
    }

    /// The grass appearances the field draws, in kind order (JSON).
    pub fn grass_kinds(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&ground(&self.inner)?.grass_kinds()).unwrap())
    }

    /// The appearances the scenery places, in kind order (JSON).
    pub fn scenery_kinds(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&ground(&self.inner)?.scenery_kinds()).unwrap())
    }

    /// A felled tree's stump mesh for `World.appearances` row `row` (its
    /// `draws` 6), in the renderer's world vertex; none for any other row.
    pub fn stump_mesh(&self, row: u32) -> Option<Vec<f32>> {
        self.inner.stump_mesh(row).map(<[f32]>::to_vec)
    }

    /// Places the ground's props, drawn from `art` (JSON, `PropInput`: the
    /// installed appearances a prop may be drawn as, each with its footprint,
    /// paints and size; each vehicle appearance's wreck; the stand-in's box
    /// and tints; the map's regional family; the models' tiers) under the
    /// world `layout`; drawn from the next `load_world`.
    pub fn place_props(&mut self, layout: &str, art: &str) -> Result<(), JsError> {
        let input = serde_json::from_str(art).map_err(|e| error(format!("props: {e}")))?;
        let records = ground(&self.inner)?.source.props.clone();
        self.inner
            .place_props(&records, layout, input)
            .map_err(error)
    }

    /// The props' bodies drawn, those a building draws, those no model
    /// draws, the models placed and those drawing what the side knows;
    /// empty without props.
    pub fn prop_stats(&self) -> Vec<u32> {
        self.inner
            .prop_stats()
            .map_or_else(Vec::new, |s| s.iter().map(|&n| n as u32).collect())
    }

    /// Every prop model placed (JSON): its appearance, state, position, yaw
    /// and scale.
    pub fn prop_models(&self) -> String {
        let models: Vec<_> = self
            .inner
            .props()
            .into_iter()
            .flat_map(|p| p.models())
            .map(|((name, state, _), r)| {
                serde_json::json!({
                    "appearance": name,
                    "state": state,
                    "position": [r[0], r[1], r[2]],
                    "yaw": r[3],
                    "scale": [r[12], r[13], r[14]],
                })
            })
            .collect();
        serde_json::to_string(&models).unwrap()
    }

    /// The player's scenery settings: the trees' tier thresholds over the
    /// biome's, and whether the forest floor's dressing and the tree lines'
    /// shrubs draw.
    pub fn set_scenery_quality(&mut self, tree_lod_scale: f64, dressing: bool, understorey: bool) {
        let quality = SceneryQuality {
            tree_lod_scale,
            dressing,
            understorey,
        };
        self.inner.set_scenery_quality(quality);
    }

    /// Wanted dressing cells are still to be laid: a capture frames again.
    pub fn scenery_pending(&self) -> bool {
        self.inner.scenery_pending()
    }

    /// A felled tree still falls.
    pub fn scenery_falling(&self) -> bool {
        self.inner.scenery_falling()
    }

    /// Scenery populations, query visits and known-fall work; empty with none.
    /// Doubles retain cumulative row counts beyond the u32 range.
    pub fn scenery_stats(&self) -> Vec<f64> {
        self.inner
            .scenery_stats()
            .map_or_else(Vec::new, |s| s.iter().map(|&n| n as f64).collect())
    }

    // Lab probes: the ground scenes read the ground as the material draws
    // it; no frame does.

    /// The plots (JSON): each one's kind, sRGB colour, the unit vector across
    /// its rows and its outline.
    pub fn ground_plots_json(&self) -> Result<String, JsError> {
        let plots = ground(&self.inner)?.plots.plots.iter().map(|p| {
            serde_json::json!({ "kind": p.kind, "colour": p.colour, "across": p.across, "outline": p.outline })
        });
        Ok(serde_json::Value::Array(plots.collect()).to_string())
    }

    /// The plot kind under `(x, y)`, or -1 outside the patchwork.
    pub fn ground_plot_kind_at(&self, x: f64, y: f64) -> Result<i32, JsError> {
        let plots = &ground(&self.inner)?.plots;
        Ok(plots.at(x, y).map_or(-1, |p| plots.plots[p].kind as i32))
    }

    /// The paved strokes as drawn; none where they are the export's.
    pub fn ground_drawn_strokes(&self) -> Result<Option<Vec<f32>>, JsError> {
        Ok(ground(&self.inner)?.drawn.clone())
    }

    /// How far past its edge each rule (paved, forest, water) is read by a
    /// pixel `footprint` metres wide.
    pub fn ground_reach(&self, footprint: f64) -> Result<Vec<f64>, JsError> {
        Ok(ground(&self.inner)?.reach(footprint, 0.0).to_vec())
    }

    /// Surface-field geometry and bucket index for the CPU/GPU agreement probe.
    pub fn ground_surface_field(&self) -> Result<String, JsError> {
        let field = self.field()?;
        Ok(serde_json::json!({ "records": field.records, "index": field.index }).to_string())
    }

    /// The material's surface field's grid: its low corner, finest cell,
    /// columns, rows, levels, and the widest pixel it reads exactly (infinite
    /// where its ladder ends in one cell).
    pub fn ground_field_grid(&self) -> Result<Vec<f64>, JsError> {
        Ok(self.field()?.grid.to_vec())
    }

    /// The material's surface field's paved, forest and water distances at
    /// each `(x, y, footprint)`.
    pub fn ground_distances(&self, points: &[f64]) -> Result<Vec<f64>, JsError> {
        let field = self.field()?;
        let chunks = points.as_chunks::<3>().0.iter();
        Ok(chunks.flat_map(|&p| field.distances(p)).collect())
    }
}

impl Presentation {
    fn field(&self) -> Result<Field, JsError> {
        ground(&self.inner)?.terrain_field().map_err(error)
    }
}

/// The height grid's pages as the GPU reads them (`Grid::pages`): the
/// grid's vertices across and up, its page side, the present pages' ids and
/// their heights.
#[wasm_bindgen]
pub fn terrain_heights(
    nx: usize,
    ny: usize,
    page_size: usize,
    page_ids: Vec<u32>,
    heights: Vec<f32>,
) -> Vec<u32> {
    Grid {
        nx,
        ny,
        page_size,
        page_ids,
        heights,
        ..Grid::default()
    }
    .pages()
}

/// The simulation's ground triangles as the renderer draws them
/// (`ground::mesh::terrain`): untinted, or, given the triangle surfaces
/// and the blocked flag, tinted by what ground units can enter.
#[wasm_bindgen]
pub fn terrain_mesh(
    positions: &[f32],
    indices: &[u32],
    surfaces: Option<Vec<u8>>,
    blocked: u8,
) -> Vec<f32> {
    mesh::terrain(
        positions,
        indices,
        surfaces.as_deref().map(|s| (s, blocked)),
    )
}

/// The scene assets' limits Rust holds (`ground::grass::LIMITS`): the clump
/// height variation's top and the tallest grass.
#[wasm_bindgen]
pub fn grass_limits() -> Vec<f64> {
    let limits = presentation::static_world::ground::grass::LIMITS;
    vec![limits.variation, limits.tallest]
}

/// The appearances drawing a map's props (its exported `props` under the
/// world `layout`) from `art` (`Presentation.place_props`) could need, as a
/// JSON list of names: what the models layer installs for them.
#[wasm_bindgen]
pub fn prop_appearances(layout: &str, props: &[f32], art: &str) -> Result<String, JsError> {
    let input = serde_json::from_str(art).map_err(|e| error(format!("props: {e}")))?;
    let names =
        presentation::static_world::props::appearances(props, layout, input).map_err(error)?;
    Ok(serde_json::to_string(&names).unwrap())
}
