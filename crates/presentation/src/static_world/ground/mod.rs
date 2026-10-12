//! The ground as both clients draw it, resolved from the map's exported
//! primitives (`sim::world::export`) under the biome: the forests as shapes,
//! the patchwork of plots, the paved strokes as drawn through towns, the
//! surface fields the terrain material and the scenery read distances
//! through, the water surface's cells, the terrain's and grass's GPU tables
//! and the scenery, in the float order the page's TypeScript used.
pub mod biome;
pub mod field;
pub mod forests;
pub mod grass;
mod js;
pub mod plots;
pub mod scenery;
mod shapes;
mod strokes;
pub mod tables;
#[cfg(test)]
mod tests;
pub mod tuning;
mod water;

use biome::{Biome, AREA_KINDS};
use field::Field;
use forests::Forest;
use plots::Plots;
use serde::Deserialize;
use tables::Grid;
// The exported records' strides (`sim::world::export::layout_json`).
use sim::world::export::SURFACE_STROKE_STRIDE as SURFACE_STROKE;
use sim::world::export::{AREA_STRIDE as AREA, FOREST_STROKE_STRIDE as FOREST_STROKE};
use sim::world::export::{PROP_STRIDE as PROP, RIVER_STRIDE as RIVER};
use sim::world::export::{SURFACE_BOUNDARY_STRIDE as SURFACE_BOUNDARY, SURFACE_TRIANGLE_STRIDE};
use SURFACE_TRIANGLE_STRIDE as SURFACE_TRIANGLE;
const RUN: usize = 4;
const FOREST_TRIANGLE: usize = 7;
const FOREST_BOUNDARY: usize = 5;
/// A stroke's stretch is cut square at its end `a` (`b`).
const CUT_A: u32 = 1;
const CUT_B: u32 = 2;

/// All the paved kinds but paving are carriageways.
fn is_road(tag: f32) -> bool {
    (0.0..3.0).contains(&tag)
}

/// `records` as `N`-float records.
fn chunks<const N: usize>(records: &[f32]) -> &[[f32; N]] {
    records.as_chunks().0
}

/// What draws a prop kind, where the ground cares: a building's template,
/// or its forest's trees (the layout's `propAppearance[kind].drawn_by`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DrawnBy {
    Building,
    Forest,
}

/// The world layout's (`sim::world::export::layout_json`) prop kinds, as
/// far as the ground reads them: what draws each.
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

/// What draws each prop kind of the world layout `layout` (JSON), in kind order.
pub fn drawn_by(layout: &str) -> Result<Vec<Option<DrawnBy>>, String> {
    let layout: Layout = serde_json::from_str(layout).map_err(|e| format!("layout: {e}"))?;
    Ok(layout
        .prop_kinds
        .iter()
        .map(
            |k| match layout.prop_appearance.get(k).and_then(|a| a.as_ref()) {
                Some(a) if a.drawn_by == "building" => Some(DrawnBy::Building),
                Some(a) if a.drawn_by == "forest" => Some(DrawnBy::Forest),
                _ => None,
            },
        )
        .collect())
}

/// What the ground is resolved from: the physical map `[min x, min y, max x,
/// max y]`, its own rendered surroundings where it has them, its height
/// grid, its exported streams as `WorldView` names them, and per prop kind
/// what draws it.
#[derive(Default)]
pub struct Source {
    pub map: [f64; 4],
    pub rendered: Option<[f64; 4]>,
    pub grid: Grid,
    pub props: Vec<f32>,
    pub drawn_by: Vec<Option<DrawnBy>>,
    pub foliage: Vec<f32>,
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
}

impl Source {
    fn drawn_by(&self, prop: &[f32; PROP]) -> Option<DrawnBy> {
        self.drawn_by.get(prop[2] as usize).copied().flatten()
    }

    /// The buildings' centres.
    fn buildings(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        let props = self.props.as_chunks::<PROP>().0.iter();
        props
            .filter(|p| self.drawn_by(p) == Some(DrawnBy::Building))
            .map(|p| [p[3], p[4]])
    }
}

/// `presentation`'s blocks the ground reads (`tuning`).
#[derive(Clone, Deserialize)]
pub struct Tuning {
    pub terrain: tuning::TerrainTuning,
    pub grass: tuning::GrassTuning,
}

pub struct Ground {
    pub source: Source,
    pub biome: Biome,
    pub tuning: Tuning,
    pub forests: Vec<Forest>,
    pub plots: Plots,
    /// The paved strokes as drawn; `None` where they are the source's.
    pub drawn: Option<Vec<f32>>,
}

impl Ground {
    /// The ground of `source` under `biome` (in its map's regional finish).
    pub fn new(source: Source, biome: Biome, tuning: Tuning) -> Result<Ground, String> {
        let forests = forests::forests(&source)?;
        let plots = plots::plots(&source, &forests::strip_runs(&source, &forests), &biome)?;
        let built = |x, y| {
            plots
                .at(x, y)
                .is_some_and(|p| plots.plots[p].kind == plots.settlement)
        };
        let drawn = strokes::drawn(&source.surface_strokes, &biome, built);
        Ok(Ground {
            source,
            biome,
            tuning,
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
