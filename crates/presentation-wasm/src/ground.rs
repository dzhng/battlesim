//! The ground's shapes for the page (`presentation::static_world::ground`): forests,
//! plots, drawn strokes, surface fields and water cells, as flat arrays.
use presentation::static_world::ground::{self, Source};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Ground(ground::Ground);

#[wasm_bindgen]
impl Ground {
    /// The ground of a map: `extents` is the physical map's
    /// `[min x, min y, max x, max y]`, then its rendered surroundings' where
    /// it has them; the streams are `WorldView`'s exports of the same names;
    /// `buildings` their centres (`x, y` each); `biome` the resolved biome in
    /// the map's regional finish (JSON).
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extents: &[f64],
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
        buildings: Vec<f32>,
        biome: &str,
    ) -> Result<Ground, JsError> {
        let biome =
            serde_json::from_str(biome).map_err(|e| JsError::new(&format!("biome: {e}")))?;
        let source = Source {
            map: extents[..4].try_into().unwrap(),
            rendered: extents.get(4..8).map(|r| r.try_into().unwrap()),
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
            buildings,
        };
        ground::Ground::new(source, &biome)
            .map(Ground)
            .map_err(|e| JsError::new(&e))
    }

    /// Per forest, 14 numbers: canopy, its trunks' first and one-past
    /// source prop, its kind (0 rectangle, 1 stroke, 2 polygon), a
    /// rectangle's `x, y, w, h` (NaN for the rest), then the float ranges
    /// `[start, end)` of its strokes, triangles and boundaries in the
    /// forest streams.
    pub fn forests(&self) -> Vec<f64> {
        let mut out = vec![];
        for f in &self.0.forests {
            let [first, end] = f.trunks.map(f64::from);
            out.extend([f.canopy as f64, first, end, f.kind as u8 as f64]);
            out.extend(f.rect.map_or([f64::NAN; 4], |r| r.map(f64::from)));
            for r in [&f.strokes, &f.triangles, &f.boundaries] {
                out.extend([r.start as f64, r.end as f64]);
            }
        }
        out
    }

    /// The plots' split region and the tree's depth.
    pub fn plot_region(&self) -> Vec<f64> {
        let p = &self.0.plots;
        [&p.region[..], &[p.depth as f64]].concat()
    }

    /// The split tree's nodes, `nx, ny, c, front, back` each.
    pub fn plot_nodes(&self) -> Vec<f64> {
        self.0.plots.nodes.clone()
    }

    /// Per plot, 8 numbers: its kind, sRGB colour, the unit vector across
    /// its rows, and its outline's range `[start, end)` in `plot_outlines`.
    pub fn plots(&self) -> Vec<f64> {
        let mut at = 0;
        let mut out = vec![];
        for p in &self.0.plots.plots {
            out.push(p.kind as f64);
            out.extend(p.colour);
            out.extend(p.across);
            out.extend([at as f64, (at + p.outline.len()) as f64]);
            at += p.outline.len();
        }
        out
    }

    pub fn plot_outlines(&self) -> Vec<f64> {
        self.0
            .plots
            .plots
            .iter()
            .flat_map(|p| p.outline.iter().copied())
            .collect()
    }

    /// The paved strokes as drawn; none where they are the source's.
    pub fn drawn_strokes(&self) -> Option<Vec<f32>> {
        self.0.drawn.clone()
    }

    /// The surface field, with the strokes as drawn or as exported. `reach`
    /// holds how far the paved, forest and water rules are read at each
    /// level's pixel (2 m, doubling a level), three numbers a level.
    pub fn field(&self, drawn: bool, reach: &[f64]) -> Result<SurfaceField, JsError> {
        self.0
            .field(drawn, reach.as_chunks().0)
            .map(SurfaceField)
            .map_err(|e| JsError::new(&e))
    }

    pub fn water_cells(&self) -> Vec<f32> {
        self.0.water_cells()
    }
}

#[wasm_bindgen]
pub struct SurfaceField(ground::field::Field);

#[wasm_bindgen]
impl SurfaceField {
    pub fn records(&self) -> Vec<f32> {
        self.0.records.clone()
    }

    pub fn index(&self) -> Vec<u32> {
        self.0.index.clone()
    }

    /// The grid's origin `x, y`, finest cell side, columns, rows, levels,
    /// and the widest pixel read exactly.
    pub fn grid(&self) -> Vec<f64> {
        self.0.grid.to_vec()
    }
}
