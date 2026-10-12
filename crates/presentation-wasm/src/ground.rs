//! The ground for the page (`presentation::static_world::ground`): the map's
//! exports resolved under the biome into the terrain's and grass's GPU
//! tables, the water's cells and the scenery, as flat arrays.
use presentation::static_world::ground::biome::{Biome, GrassRules};
use presentation::static_world::ground::field::Field;
use presentation::static_world::ground::grass::{KindFacts, Limits};
use presentation::static_world::ground::scenery::{Scenery, Size};
use presentation::static_world::ground::tables::{Grid, TerrainTables};
use presentation::static_world::ground::{self, DrawnBy, Source, Tuning};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

fn error(e: impl ToString) -> JsError {
    JsError::new(&e.to_string())
}

#[wasm_bindgen]
pub struct Ground {
    inner: ground::Ground,
    scenery: Option<Scenery>,
    /// The material's surface field, built once for the probes.
    field: std::cell::OnceCell<Field>,
    /// The biome's own grass rules, while a lab grows by others.
    grass: GrassRules,
}

/// `WorldView.terrain_grid()`, and the layout's prop kinds.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridHeader {
    nx: usize,
    ny: usize,
    spacing: f64,
    page_size: usize,
    min_height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Layout {
    prop_kinds: Vec<String>,
    prop_appearance: std::collections::HashMap<String, Option<PropAppearance>>,
}

#[derive(Deserialize)]
struct PropAppearance {
    drawn_by: String,
}

#[wasm_bindgen]
impl Ground {
    /// The ground of a map: `extents` is the physical map's `[min x, min y,
    /// max x, max y]`, then its rendered surroundings' where it has them;
    /// `grid` is `WorldView.terrain_grid()` with its page ids and heights;
    /// the streams are `WorldView`'s exports of the same names; `layout` is
    /// `world_layout()`; `biome` the biome's file and `family` the map's
    /// regional family (or none); `tuning` the fixture's `presentation` block.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
    ) -> Result<Ground, JsError> {
        let g: GridHeader = serde_json::from_str(grid).map_err(error)?;
        let layout: Layout = serde_json::from_str(layout).map_err(error)?;
        let drawn_by = layout.prop_kinds.iter().map(|k| {
            match layout
                .prop_appearance
                .get(k)
                .and_then(|a| a.as_ref())
                .map(|a| a.drawn_by.as_str())
            {
                Some("building") => Some(DrawnBy::Building),
                Some("forest") => Some(DrawnBy::Forest),
                _ => None,
            }
        });
        let source = Source {
            map: extents[..4].try_into().unwrap(),
            rendered: extents.get(4..8).map(|r| r.try_into().unwrap()),
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
            drawn_by: drawn_by.collect(),
            foliage,
            surface_strokes,
            surface_runs,
            surface_triangles,
            surface_boundaries,
            rivers,
            river_runs,
            forests,
            forest_rect_ids,
            forest_metadata,
            forest_strokes,
            forest_triangles,
            forest_boundaries,
            forest_trunk_ranges,
        };
        let biome = Biome::load(biome, family.as_deref()).map_err(error)?;
        let tuning: Tuning =
            serde_json::from_str(tuning).map_err(|e| error(format!("tuning: {e}")))?;
        let inner = ground::Ground::new(source, biome, tuning).map_err(error)?;
        Ok(Ground {
            grass: inner.biome.grass.clone(),
            inner,
            scenery: None,
            field: Default::default(),
        })
    }

    /// The terrain's GPU tables (`GroundTables`).
    pub fn terrain(&self) -> Result<GroundTables, JsError> {
        Ok(GroundTables(self.inner.terrain_tables(
            self.inner.terrain_field().map_err(error)?,
        )))
    }

    /// The plots' records, with or without their texture.
    pub fn plots(&self, textured: bool) -> Vec<u32> {
        self.inner.plot_records(textured)
    }

    pub fn water_cells(&self) -> Vec<f32> {
        self.inner.water_cells()
    }

    /// Grows the grass by `rules` (JSON, the biome's `grass` row) in place of
    /// the biome's, or by the biome's again: a lab compares fields that
    /// differ in a single number.
    pub fn retune_grass(&mut self, rules: Option<String>) -> Result<(), JsError> {
        self.inner.biome.grass = match rules {
            Some(json) => serde_json::from_str(&json).map_err(error)?,
            None => self.grass.clone(),
        };
        Ok(())
    }

    /// The grass appearances the field draws, in kind order (JSON).
    pub fn grass_kinds(&self) -> String {
        serde_json::to_string(&self.inner.grass_kinds()).unwrap()
    }

    /// `GrassParams` for the kinds' mesh facts, six numbers each in kind
    /// order (highest vertex, top, blades, root width, near and far base),
    /// under the scene assets' height variation and tallest grass.
    pub fn grass_params(
        &self,
        facts: &[f64],
        variation: f64,
        tallest: f64,
    ) -> Result<Vec<u32>, JsError> {
        let facts: Vec<KindFacts> = facts
            .as_chunks::<6>()
            .0
            .iter()
            .map(|f| KindFacts {
                mesh_height: f[0],
                top: f[1],
                blades: f[2],
                root_width: f[3],
                bases: [f[4] as u32, f[5] as u32],
            })
            .collect();
        self.inner
            .grass_params(&facts, &Limits { variation, tallest })
            .map_err(error)
    }

    /// The props' footprints where no grass grows, by grid cell.
    pub fn grass_props(&self) -> Vec<u32> {
        self.inner.grass_props()
    }

    /// The appearances the scenery places, in kind order (JSON).
    pub fn scenery_kinds(&self) -> String {
        serde_json::to_string(&self.inner.scenery_kinds()).unwrap()
    }

    /// Places the scenery, its kinds sized `sizes` (height, reach each).
    pub fn place_scenery(&mut self, sizes: &[f64]) {
        let sizes: Vec<Size> = sizes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|s| Size {
                height: s[0],
                radius: s[1],
            })
            .collect();
        self.scenery = Some(self.inner.scenery(&sizes));
    }

    fn placed(&self) -> &Scenery {
        self.scenery.as_ref().expect("the scenery is placed")
    }

    /// A placed population: 0 forest, 1 understorey, 2 backdrop.
    pub fn trees(&self, population: u32) -> Vec<f32> {
        let s = self.placed();
        [&s.forest, &s.understorey, &s.backdrop][population as usize].clone()
    }

    /// Per forest tree, the trunk prop it stands on.
    pub fn forest_ids(&self) -> Vec<u32> {
        self.placed().forest_ids.clone()
    }

    /// The dressing: its cell's side, capacity, top, reach, the ground's
    /// lowest and highest points, then the kinds it lays.
    pub fn dressing(&self) -> Vec<f64> {
        let d = &self.placed().dressing;
        let head = [
            ground::scenery::DRESSING_CELL_M,
            d.capacity as f64,
            d.top_m,
            d.reach_m,
            d.ground[0],
            d.ground[1],
        ];
        head.into_iter()
            .chain(d.kinds.iter().map(|&k| k as f64))
            .collect()
    }

    /// The cells a forest reaches, `i, j` pairs.
    pub fn dressing_cells(&self) -> Vec<i32> {
        self.placed().dressing.cells.clone()
    }

    /// The pieces of dressing cell `(i, j)`.
    pub fn dress(&self, i: i32, j: i32) -> Vec<f32> {
        self.inner.dress(self.placed(), i as i64, j as i64)
    }

    // Lab probes: the ground scenes read the ground as the material draws
    // it; no frame does.

    /// The plots (JSON): each one's kind, sRGB colour, the unit vector across
    /// its rows and its outline.
    pub fn plots_json(&self) -> String {
        let plots = self.inner.plots.plots.iter().map(|p| {
            serde_json::json!({ "kind": p.kind, "colour": p.colour, "across": p.across, "outline": p.outline })
        });
        serde_json::Value::Array(plots.collect()).to_string()
    }

    /// The plot kind under `(x, y)`, or -1 outside the patchwork.
    pub fn plot_kind_at(&self, x: f64, y: f64) -> i32 {
        let plots = &self.inner.plots;
        plots.at(x, y).map_or(-1, |p| plots.plots[p].kind as i32)
    }

    /// The paved strokes as drawn; none where they are the export's.
    pub fn drawn_strokes(&self) -> Option<Vec<f32>> {
        self.inner.drawn.clone()
    }

    /// How far past its edge each rule (paved, forest, water) is read by a
    /// pixel `footprint` metres wide.
    pub fn reach(&self, footprint: f64) -> Vec<f64> {
        self.inner.reach(footprint, 0.0).to_vec()
    }

    /// The material's surface field's grid: its low corner, finest cell,
    /// columns, rows, levels, and the widest pixel it reads exactly (infinite
    /// where its ladder ends in one cell).
    pub fn field_grid(&self) -> Result<Vec<f64>, JsError> {
        Ok(self.probed()?.grid.to_vec())
    }

    /// The material's surface field's paved, forest and water distances at
    /// each `(x, y, footprint)`.
    pub fn distances(&self, points: &[f64]) -> Result<Vec<f64>, JsError> {
        let field = self.probed()?;
        Ok(points
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&p| field.distances(p))
            .collect())
    }
}

impl Ground {
    fn probed(&self) -> Result<&Field, JsError> {
        if self.field.get().is_none() {
            let _ = self.field.set(self.inner.terrain_field().map_err(error)?);
        }
        Ok(self.field.get().unwrap())
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

#[wasm_bindgen]
pub struct GroundTables(TerrainTables);

#[wasm_bindgen]
impl GroundTables {
    pub fn nodes(&self) -> Vec<u32> {
        self.0.nodes.clone()
    }
    pub fn plots(&self) -> Vec<u32> {
        self.0.plots.clone()
    }
    /// `TerrainParams` but its frame's `view`.
    pub fn params(&self) -> Vec<u32> {
        self.0.params.clone()
    }
    /// `ScarParams`' look, from `full` on.
    pub fn scars(&self) -> Vec<u32> {
        self.0.scars.clone()
    }
    pub fn surfaces(&self) -> Vec<f32> {
        self.0.surfaces.clone()
    }
    pub fn surface_index(&self) -> Vec<u32> {
        self.0.surface_index.clone()
    }
    pub fn water(&self) -> Vec<f32> {
        self.0.water.clone()
    }
}
