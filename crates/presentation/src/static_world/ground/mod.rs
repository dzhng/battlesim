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
pub mod kinds;
pub mod mesh;
pub mod partition;
pub mod plots;
pub mod puddles;
pub mod scenery;
mod shapes;
mod strokes;
pub mod sway;
pub mod tables;
#[cfg(test)]
mod tests;
pub mod tuning;
pub mod walk;
pub mod water;

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
pub const CUT_A: u32 = 1;
pub const CUT_B: u32 = 2;

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
/// far as the ground and sight read them: what draws each, and which hide
/// what lies behind them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Layout {
    prop_kinds: Vec<String>,
    prop_appearance: std::collections::HashMap<String, Option<PropAppearance>>,
    #[serde(default)]
    occluding_prop_kinds: Vec<String>,
}

#[derive(Deserialize)]
struct PropAppearance {
    drawn_by: String,
}

/// The world layout's prop kinds, in kind order: their names, what draws
/// each, and which occlude sight.
#[derive(Clone, Debug, Default)]
pub struct PropKinds {
    pub names: Vec<String>,
    pub drawn_by: Vec<Option<DrawnBy>>,
    pub occludes: Vec<bool>,
}

/// The prop kinds of the world layout `layout` (JSON).
pub fn prop_kinds(layout: &str) -> Result<PropKinds, String> {
    let layout: Layout = serde_json::from_str(layout).map_err(|e| format!("layout: {e}"))?;
    let drawn_by = layout
        .prop_kinds
        .iter()
        .map(
            |k| match layout.prop_appearance.get(k).and_then(|a| a.as_ref()) {
                Some(a) if a.drawn_by == "building" => Some(DrawnBy::Building),
                Some(a) if a.drawn_by == "forest" => Some(DrawnBy::Forest),
                _ => None,
            },
        )
        .collect();
    let occludes = layout
        .prop_kinds
        .iter()
        .map(|k| layout.occluding_prop_kinds.contains(k))
        .collect();
    Ok(PropKinds {
        names: layout.prop_kinds,
        drawn_by,
        occludes,
    })
}

/// What the ground is resolved from: the physical map `[min x, min y, max x,
/// max y]`, the playable rectangle as the map's extents give it, its own
/// rendered surroundings where it has them, its height grid, its exported
/// streams as `WorldView` names them, and its prop kinds.
#[derive(Default)]
pub struct Source {
    pub map: [f64; 4],
    pub playable: [f64; 4],
    pub rendered: Option<[f64; 4]>,
    pub grid: Grid,
    pub props: Vec<f32>,
    pub kinds: PropKinds,
    pub foliage: Vec<f32>,
    pub surface_strokes: Vec<f32>,
    pub surface_runs: Vec<f32>,
    pub surface_triangles: Vec<f32>,
    pub surface_boundaries: Vec<f32>,
    pub rivers: Vec<f32>,
    pub river_runs: Vec<f32>,
    /// Every swamp's outline, edge by edge (`SWAMP_STRIDE`).
    pub swamps: Vec<f32>,
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
        self.kinds.drawn_by.get(prop[2] as usize).copied().flatten()
    }

    /// The buildings' centres.
    fn buildings(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        let props = self.props.as_chunks::<PROP>().0.iter();
        props
            .filter(|p| self.drawn_by(p) == Some(DrawnBy::Building))
            .map(|p| [p[3], p[4]])
    }
}

/// The authored ground settings (`tuning`), and fog of war's
/// sight over it.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    pub terrain: tuning::TerrainTuning,
    pub fog_geometry: crate::fog::sight::Geometry,
}

impl Tuning {
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let tuning: Self = crate::settings::read(value, "ground tuning")?;
        tuning.fog_geometry.check()?;
        crate::settings::validate::ground(&tuning.terrain)?;
        Ok(tuning)
    }
}

/// What the ground of `world` is resolved from, the map's `extents` and its
/// world `layout` (`sim::world::export::layout_json`).
pub fn source(
    world: &sim::world::WorldGeometry,
    extents: &contract::map::MapExtents,
    layout: &str,
) -> Result<Source, String> {
    let header: serde_json::Value = serde_json::from_str(&world.export_terrain_grid())
        .map_err(|e| format!("terrain grid: {e}"))?;
    let n = |k: &str| header[k].as_f64().unwrap_or(0.0);
    Ok(Source {
        map: extents.physical,
        playable: extents.playable,
        // The page hands the rendered surroundings only where they reach
        // past the map (`resolveGround`).
        rendered: (extents.rendered[0] < 0.0).then_some(extents.rendered),
        grid: Grid {
            nx: n("nx") as usize,
            ny: n("ny") as usize,
            spacing: n("spacing"),
            page_size: n("pageSize") as usize,
            min_height: n("minHeight"),
            page_ids: world.export_terrain_page_ids(),
            heights: world.export_terrain_heights(),
        },
        props: world.export_props(),
        kinds: prop_kinds(layout)?,
        foliage: world.export_foliage(),
        surface_strokes: world.export_surface_strokes(),
        surface_runs: world.export_surface_runs(),
        surface_triangles: world.export_surface_triangles(),
        surface_boundaries: world.export_surface_boundaries(),
        rivers: world.export_rivers(),
        river_runs: world.export_river_runs(),
        swamps: world.export_swamps(),
        forests: world.export_forests(),
        forest_rect_ids: world.export_forest_rect_ids(),
        forest_metadata: world.export_forest_metadata(),
        forest_strokes: world.export_forest_strokes(),
        forest_triangles: world.export_forest_triangles(),
        forest_boundaries: world.export_forest_boundaries(),
        forest_trunk_ranges: world.export_forest_trunk_ranges(),
    })
}

pub struct Ground {
    pub source: Source,
    pub biome: Biome,
    pub tuning: Tuning,
    pub forests: Vec<Forest>,
    pub plots: Plots,
    /// Its puddles (`puddles::PuddleGround::puddles`).
    pub puddles: Vec<puddles::Puddle>,
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
        // A puddle lies in no other water.
        let s = &source;
        let water = water::StandingWater::new(&s.rivers, &s.swamps, &[], &s.grid, &biome);
        let puddles = puddles::PuddleGround {
            source: s,
            strokes: drawn.as_deref().unwrap_or(&s.surface_strokes),
            forests: &forests,
            biome: &biome,
            in_water: &|x, y| water.at(x, y).is_some(),
        }
        .puddles();
        Ok(Ground {
            source,
            biome,
            tuning,
            forests,
            plots,
            puddles,
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

    /// The standing water of its rivers, swamps and puddles
    /// (`water::StandingWater`).
    pub fn standing_water(&self) -> water::StandingWater<'_> {
        let s = &self.source;
        water::StandingWater::new(&s.rivers, &s.swamps, &self.puddles, &s.grid, &self.biome)
    }

    /// The water surface's cells (`StandingWater::cells`), each with the
    /// look its water draws: a row of `TerrainParams.waterLooks`.
    pub fn water_cells(&self) -> Vec<f32> {
        self.standing_water().cells()
    }
}
