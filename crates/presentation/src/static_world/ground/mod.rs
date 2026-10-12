//! The ground's shapes as both clients draw them, resolved from the map's
//! exported primitives (`sim::world::export`): the forests as shapes, the
//! patchwork of plots, the paved strokes as drawn through towns, the surface
//! fields the terrain material and the scenery read distances through, and
//! the water surface's cells, in the float order the page's TypeScript used.
pub mod biome;
pub mod field;
pub mod forests;
mod js;
pub mod plots;
mod shapes;
mod strokes;
mod water;

use biome::Biome;
use field::Field;
use forests::Forest;
use plots::Plots;
// The exported records' strides (`sim::world::export::layout_json`).
use sim::world::export::{AREA_STRIDE as AREA, FOREST_STROKE_STRIDE as FOREST_STROKE};
use sim::world::export::{RIVER_STRIDE as RIVER, SURFACE_BOUNDARY_STRIDE as SURFACE_BOUNDARY};
use sim::world::export::{SURFACE_STROKE_STRIDE as SURFACE_STROKE, SURFACE_TRIANGLE_STRIDE};
use SURFACE_TRIANGLE_STRIDE as SURFACE_TRIANGLE;
const RUN: usize = 4;
const FOREST_TRIANGLE: usize = 7;
const FOREST_BOUNDARY: usize = 5;
/// A stroke's stretch is cut square at its end `a` (`b`).
const CUT_A: u32 = 1;
const CUT_B: u32 = 2;
/// The kinds a paved record's tag names, in the layout's order; all but
/// paving are carriageways.
const AREA_KINDS: [&str; 4] = ["road", "country_road", "dirt_track", "paving"];

fn is_road(tag: f32) -> bool {
    (0.0..3.0).contains(&tag)
}

/// `records` as `N`-float records.
fn chunks<const N: usize>(records: &[f32]) -> &[[f32; N]] {
    records.as_chunks().0
}

/// What the ground is resolved from: the physical map `[min x, min y, max x,
/// max y]`, its own rendered surroundings where it has them, its exported
/// streams as `WorldView` names them, and its buildings' centres (`x, y`).
#[derive(Default)]
pub struct Source {
    pub map: [f64; 4],
    pub rendered: Option<[f64; 4]>,
    pub surface_strokes: Vec<f32>,
    pub surface_runs: Vec<f32>,
    pub surface_triangles: Vec<f32>,
    pub surface_boundaries: Vec<f32>,
    pub rivers: Vec<f32>,
    pub river_runs: Vec<f32>,
    pub forests: Vec<f32>,
    pub forest_rect_ids: Vec<u32>,
    pub forest_metadata: Vec<f32>,
    pub forest_strokes: Vec<f32>,
    pub forest_triangles: Vec<f32>,
    pub forest_boundaries: Vec<f32>,
    pub forest_trunk_ranges: Vec<u32>,
    pub buildings: Vec<f32>,
}

pub struct Ground {
    pub source: Source,
    pub forests: Vec<Forest>,
    pub plots: Plots,
    /// The paved strokes as drawn; `None` where they are the source's.
    pub drawn: Option<Vec<f32>>,
}

impl Ground {
    /// The ground of `source` under `biome` (in its map's regional finish).
    pub fn new(source: Source, biome: &Biome) -> Result<Ground, String> {
        let forests = forests::forests(&source)?;
        let plots = plots::plots(&source, &forests::strip_runs(&source, &forests), biome)?;
        let built = |x, y| {
            plots
                .at(x, y)
                .is_some_and(|p| plots.plots[p].kind == plots.settlement)
        };
        let drawn = strokes::drawn(&source.surface_strokes, biome, built);
        Ok(Ground {
            source,
            forests,
            plots,
            drawn,
        })
    }

    /// The surface field with the strokes as drawn, or as the source has
    /// them, read out to `reach` (`field::field`).
    pub fn field(&self, drawn: bool, reach: &[[f64; 3]]) -> Result<Field, String> {
        let strokes = self
            .drawn
            .as_ref()
            .filter(|_| drawn)
            .unwrap_or(&self.source.surface_strokes);
        field::field(&self.source, &self.forests, strokes, reach)
    }

    pub fn water_cells(&self) -> Vec<f32> {
        water::cells(&self.source.rivers)
    }
}
