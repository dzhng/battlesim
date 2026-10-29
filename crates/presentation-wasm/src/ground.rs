//! The map's ground on the page's presentation (`static_world::ground`):
//! resolved from the map's exports under the biome into the terrain's and
//! grass's GPU tables and the water's cells, and its scenery placed for the
//! frames' static batches.
#[path = "terrain_partition.rs"]
mod terrain_partition;

use crate::{error, Presentation};
use presentation::presentation::Presentation as Inner;
use presentation::shadow;
use presentation::static_world::ground::field::Field;
use presentation::static_world::ground::grass::{
    blade_vertices, pack_shapes, strip_indices, KindMesh, PackedShapes, Quality, TierMesh, Window,
    FIELD_SEGMENTS, LIMITS,
};
use presentation::static_world::ground::mesh;
use presentation::static_world::ground::tables::{Grid, TerrainTables};
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

    /// The terrain's GPU tables (`GroundTables`).
    pub fn ground_terrain(&self) -> Result<GroundTables, JsError> {
        let g = ground(&self.inner)?;
        Ok(GroundTables(
            g.terrain_tables(g.terrain_field().map_err(error)?),
        ))
    }

    /// The plots' records, with or without their texture.
    pub fn ground_plots(&self, textured: bool) -> Result<Vec<u32>, JsError> {
        Ok(ground(&self.inner)?.plot_records(textured))
    }

    /// The water's surface mesh (`ground::mesh::water` over the ground's
    /// water cells).
    pub fn ground_water_mesh(&self) -> Result<Vec<f32>, JsError> {
        Ok(mesh::water(&ground(&self.inner)?.water_cells()))
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

    /// `GrassParams` for the kinds `shapes` packed (in `grass_kinds` order),
    /// under the scene assets' limits and the player's grass density (a
    /// scale on the densest stand, 1 the biome's), its per-frame fields zero
    /// (`grass_frame_params` writes them).
    pub fn grass_params(&self, shapes: &GrassShapes, density: f64) -> Result<Vec<u32>, JsError> {
        ground(&self.inner)?
            .grass_params(&shapes.0.facts, &LIMITS, &Quality { density })
            .map_err(error)
    }

    /// The window of tiles the grass grows over for the last frame's camera:
    /// its corner x and y, tiles across and up, reach, and the far corner x
    /// and y; empty before a frame.
    pub fn grass_window(&self) -> Result<Vec<f64>, JsError> {
        ground(&self.inner)?;
        let Some((w, _)) = self.inner.grass_window() else {
            return Ok(Vec::new());
        };
        let [.., x1, y1] = w.bounds();
        Ok(vec![
            w.x0,
            w.y0,
            f64::from(w.tiles_x),
            f64::from(w.tiles_y),
            w.reach,
            x1,
            y1,
        ])
    }

    /// Writes the last frame's fields into `GrassParams`' `words`
    /// (`ground::grass::frame_params`) for the window `grass_window` gave.
    pub fn grass_frame_params(&self, words: &mut [u32], window: &[f64]) {
        let window = Window {
            x0: window[0],
            y0: window[1],
            tiles_x: window[2] as u32,
            tiles_y: window[3] as u32,
            reach: window[4],
        };
        self.inner.grass_frame_params(words, &window);
    }

    /// The props' footprints where no grass grows, by grid cell.
    pub fn grass_props(&self) -> Result<Vec<u32>, JsError> {
        Ok(ground(&self.inner)?.grass_props())
    }

    /// The appearances the scenery places, in kind order (JSON).
    pub fn scenery_kinds(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&ground(&self.inner)?.scenery_kinds()).unwrap())
    }

    /// Places the ground's scenery, given each of its kinds' (`scenery_kinds`)
    /// finest tier in the renderer's world vertex, one after another,
    /// `floats` floats each; drawn from the next `load_world`.
    pub fn place_scenery(&mut self, meshes: &[f32], floats: &[u32]) -> Result<(), JsError> {
        let mut at = 0;
        let finest: Vec<&[f32]> = floats
            .iter()
            .map(|&n| {
                let mesh = &meshes[at..at + n as usize];
                at += n as usize;
                mesh
            })
            .collect();
        self.inner.place_scenery(&finest).map_err(error)
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

/// The map's skirt (`ground::mesh::skirt`).
#[wasm_bindgen]
pub fn skirt_mesh(positions: &[f32]) -> Vec<f32> {
    mesh::skirt(positions)
}

/// The land past the map's edge (`ground::mesh::backdrop`) round `map_box`.
#[wasm_bindgen]
pub fn backdrop_mesh(map_box: &[f64], reach_m: f64) -> Vec<f32> {
    mesh::backdrop(&std::array::from_fn(|i| map_box[i]), reach_m)
}

/// The box round a world mesh (`shadow::map_box`), none for an empty one.
#[wasm_bindgen]
pub fn map_box(mesh: &[f32]) -> Option<Vec<f64>> {
    shadow::map_box(mesh).map(|b| b.to_vec())
}

/// The pixels a scar region can cover on a `width` × `height` target
/// through the map's box, if it has one (`scars::scissor`): `[x, y, width,
/// height]`, empty when it is off screen.
#[wasm_bindgen]
pub fn scar_scissor(
    region: &[f64],
    map_box: Option<Box<[f64]>>,
    view_proj: &[f32],
    width: u32,
    height: u32,
) -> Vec<u32> {
    let view_proj = view_proj.first_chunk().expect("a 4×4 matrix");
    let map_box = map_box.map(|b| std::array::from_fn(|i| b[i]));
    presentation::scars::scissor(
        std::array::from_fn(|i| region[i]),
        map_box.as_ref(),
        view_proj,
        [width, height],
    )
    .map_or_else(Vec::new, |r| r.to_vec())
}

/// The installed grass kinds' meshes packed for the field
/// (`ground::grass::pack_shapes`), and their facts for `Ground.grass_params`.
#[wasm_bindgen]
pub struct GrassShapes(PackedShapes);

#[wasm_bindgen]
impl GrassShapes {
    /// Packs the kinds `names` (JSON, in `Ground.grass_kinds` order): each
    /// kind's first state's tier count (`tiers`) and top (`tops`), and per
    /// tier its vertex count (`vertices`); the tiers' positions, normals,
    /// uvs and colours one after another. Refuses a kind whose tiers are not
    /// blade strips, by name.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        names: &str,
        tiers: &[u32],
        tops: &[f64],
        vertices: &[u32],
        positions: &[f32],
        normals: &[i16],
        uvs: &[f32],
        colors: &[u8],
    ) -> Result<GrassShapes, JsError> {
        let names: Vec<String> = serde_json::from_str(names).map_err(error)?;
        let mut at = 0;
        let mut v = 0;
        let mut kinds = Vec::with_capacity(names.len());
        for (k, name) in names.iter().enumerate() {
            let mut list = Vec::with_capacity(tiers[k] as usize);
            for _ in 0..tiers[k] {
                let n = vertices[at] as usize;
                list.push(TierMesh {
                    positions: &positions[v * 3..(v + n) * 3],
                    normals: &normals[v * 4..(v + n) * 4],
                    uvs: &uvs[v * 2..(v + n) * 2],
                    colors: &colors[v * 4..(v + n) * 4],
                });
                (at, v) = (at + 1, v + n);
            }
            kinds.push(KindMesh {
                name,
                tiers: list,
                top: tops[k],
            });
        }
        Ok(GrassShapes(pack_shapes(&kinds).map_err(error)?))
    }

    /// Every kind's two field tiers as clump vertices.
    pub fn shapes(&self) -> Vec<f32> {
        self.0.shapes.clone()
    }

    /// The most blades any kind has: the index lists' blades.
    pub fn max_blades(&self) -> u32 {
        self.0.max_blades
    }
}

/// The field's two tiers' segments a blade, near then far.
#[wasm_bindgen]
pub fn grass_field_segments() -> Vec<u32> {
    FIELD_SEGMENTS.to_vec()
}

/// Vertices of one blade of `segments` segments.
#[wasm_bindgen]
pub fn grass_blade_vertices(segments: u32) -> u32 {
    blade_vertices(segments)
}

/// The index list of `blades` blade strips of `segments` segments.
#[wasm_bindgen]
pub fn grass_strip_indices(blades: u32, segments: u32) -> Vec<u16> {
    strip_indices(blades, segments)
}

/// The scene assets' limits Rust holds (`ground::grass::LIMITS`): the clump
/// height variation's top and the tallest grass.
#[wasm_bindgen]
pub fn grass_limits() -> Vec<f64> {
    vec![LIMITS.variation, LIMITS.tallest]
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
