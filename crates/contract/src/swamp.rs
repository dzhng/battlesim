//! A swamp: standing shallow water in a dip it carves itself (decisions 1,
//! 21, 22 and 30 of `specs/done/water-and-wetlands`).
//!
//! Its outline is a polygon. Inside it the ground lowers by up to `depth_m`,
//! easing in over [`EDGE_CELLS`] height samples from the outline, and the
//! water stands level with the land it was cut from. One number, how far a
//! point lies inside the outline, gives the membership every rule reads and
//! the depth the terrain is cut to. A road is never cut: the simulation ends
//! the cut at a road's side as it does at the outline.
use crate::forest::body_contains;
use crate::ground::{edges, limits, polygon_contains, segment_distance, GroundShape};
use crate::map::{relief_height, MapDefinition, Relief};
use crate::river::{section, steepest_grade};
use serde::{Deserialize, Serialize};

/// The dip eases in over this many height samples from its edge, so a mover
/// wading in meets no slope the grid draws steeper than its gentlest grade
/// (decision 21).
pub const EDGE_CELLS: f64 = 3.0;
/// A swamp holding a disc this many height samples across is big enough to
/// carve, wherever the grid falls on it (decision 22).
pub const MIN_ACROSS_CELLS: f64 = 3.0;
/// The steepest the dip's easing curve ([`Swamp::cut`]) falls, against the
/// depth over the edge's width.
const EASE_STEEPEST: f64 = 1.5;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swamp {
    /// Its outline: a polygon ([`validate`] refuses a stroke).
    pub shape: GroundShape,
    /// How far its bed lies under its water, once the edge has eased in.
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub depth_m: f64,
}

/// How wide a swamp's easing edge is on a height grid of `height_grid_m`.
pub fn edge_m(height_grid_m: f64) -> f64 {
    EDGE_CELLS * height_grid_m
}

/// The deepest a swamp may be on that grid under that slope cutoff: its
/// easing edge then falls no steeper than [`steepest_grade`].
pub fn deepest_m(height_grid_m: f64, slope_cutoff_deg: f64) -> f64 {
    steepest_grade(slope_cutoff_deg) * edge_m(height_grid_m) / EASE_STEEPEST
}

impl Swamp {
    fn ring(&self) -> &[[f64; 2]] {
        match &self.shape {
            GroundShape::Polygon { ring } => ring,
            GroundShape::Stroke { .. } => panic!("an admitted swamp is a polygon"),
        }
    }

    /// How far `p` lies inside the outline, in metres; negative outside.
    /// The swamp is the closed set `inside >= 0`.
    pub fn inside(&self, p: [f64; 2]) -> f64 {
        let ring = self.ring();
        let edge = edges(ring)
            .map(|(a, b)| segment_distance(*a, *b, p))
            .fold(f64::INFINITY, f64::min);
        if polygon_contains(ring, p) {
            edge
        } else {
            -edge
        }
    }

    /// The height its water stands at: the land's, under its outline, which
    /// [`validate`] holds level.
    pub fn surface_z(&self, relief: &[Relief]) -> f64 {
        let [x, y] = self.ring()[0];
        relief_height(relief, x, y)
    }

    /// How far the ground is cut `inside` metres from where the cut starts,
    /// over an edge `edge_m` wide: nothing at the start, the whole depth past
    /// the edge, easing in and out (smoothstep) between.
    pub fn cut(&self, inside: f64, edge_m: f64) -> f64 {
        let t = (inside / edge_m).clamp(0.0, 1.0);
        self.depth_m * t * t * (3.0 - 2.0 * t)
    }

    /// Its bounds `[x, y, max_x, max_y]`, grown by `margin`.
    pub fn limits(&self, margin: f64) -> [f64; 4] {
        limits(self.ring(), margin)
    }
}

/// What must stand on uncut ground within `reach` of `bounds`: each
/// building part, and each bridge deck lengthened by an `edge`'s width at
/// both ends. Each is what it is, its centre, half extents and yaw.
fn footings(
    map: &MapDefinition,
    bounds: [f64; 4],
    edge: f64,
    reach: f64,
) -> Vec<(String, [f64; 2], [f64; 2], f64)> {
    let near = |center: [f64; 2], half: [f64; 2]| {
        let reach = libm::hypot(half[0], half[1]) + reach;
        center[0] + reach >= bounds[0]
            && center[0] - reach <= bounds[2]
            && center[1] + reach >= bounds[1]
            && center[1] - reach <= bounds[3]
    };
    let mut footings = Vec::new();
    for building in &map.buildings {
        for part in &building.geometry.parts {
            let half = [part.half_extents[0], part.half_extents[1]];
            if near(part.center, half) {
                let what = format!("the building of owner {}", building.owner);
                footings.push((what, part.center, half, part.yaw));
            }
        }
    }
    for (b, bridge) in map.bridges.iter().enumerate() {
        let half = [bridge.half_extents[0] + edge, bridge.half_extents[1]];
        if near(bridge.center, half) {
            let what = format!("bridges[{b}]'s approach");
            footings.push((what, bridge.center, half, bridge.yaw));
        }
    }
    footings
}

/// What a map's swamps have to be for the terrain to carry them, checked at
/// every height sample where the cut or the triangles it moves reach:
/// - a polygon, with a finite positive depth;
/// - an edge no steeper than [`steepest_grade`]: the depth is checked
///   against the edge's width;
/// - big enough to carve: some height sample lies as far inside its outline
///   as one does in any swamp [`MIN_ACROSS_CELLS`] across;
/// - on level land, so its water lies flat at the land's height;
/// - an edge's width (and a sample more) clear of every river's water and
///   of the bank a river cuts;
/// - off every building and every bridge deck and its approach (an edge's
///   width past each end): no triangle under them is cut.
pub fn validate(map: &MapDefinition) -> Result<(), String> {
    let grid = map.height_grid_m;
    let edge = edge_m(grid);
    // A cut sample moves every triangle it is a corner of.
    let triangle = grid * std::f64::consts::SQRT_2;
    let [nx, ny] = map.height_grid_size();
    for (k, swamp) in map.swamps.iter().enumerate() {
        if !matches!(swamp.shape, GroundShape::Polygon { .. }) {
            return Err(format!(
                "swamps[{k}] is a stroke; a swamp is an area, outlined by a polygon"
            ));
        }
        if !swamp.depth_m.is_finite() || swamp.depth_m <= 0.0 {
            return Err(format!("swamps[{k}] needs a finite positive depth"));
        }
        if swamp.depth_m > deepest_m(grid, map.slope_cutoff_deg) {
            return Err(format!(
                "swamps[{k}] is {} m deep over its {edge} m edge: steeper than the map's {}° slope cutoff allows",
                swamp.depth_m, map.slope_cutoff_deg
            ));
        }
        let level = swamp.surface_z(&map.relief);
        let bounds = swamp.limits(edge + grid);
        let span = |lo: f64, hi: f64, n: usize| {
            let first = (lo / grid).floor().max(0.0) as usize;
            let last = ((hi / grid).ceil().max(0.0) as usize).min(n - 1);
            first..=last
        };
        let footings = footings(map, bounds, edge, triangle);
        let mut deepest = f64::NEG_INFINITY;
        for j in span(bounds[1], bounds[3], ny) {
            for i in span(bounds[0], bounds[2], nx) {
                let p = [i as f64 * grid, j as f64 * grid];
                let inside = swamp.inside(p);
                deepest = deepest.max(inside);
                let at = format!("swamps[{k}] at ({}, {})", p[0], p[1]);
                if inside > -(edge + grid) {
                    let land = map.relief_height(p[0], p[1]);
                    for river in &map.rivers {
                        let cut = river.samples().windows(2).any(|pair| {
                            let s = section(&pair[0], &pair[1], p);
                            s.inside_m >= 0.0
                                || s.carved(land, river.surface_z(), 0.0, |e| {
                                    map.relief_height(e[0], e[1])
                                }) < land
                        });
                        if cut {
                            return Err(format!("{at} lies within its edge's width of a river"));
                        }
                    }
                }
                if inside > -triangle && map.relief_height(p[0], p[1]) != level {
                    return Err(format!(
                        "{at} lies on uneven land: its water would not lie level"
                    ));
                }
                if inside <= 0.0 {
                    continue;
                }
                for (what, center, half, yaw) in &footings {
                    if body_contains(*center, *half, *yaw, p, triangle) {
                        return Err(format!("{at} cuts the ground under {what}"));
                    }
                }
            }
        }
        if deepest < (MIN_ACROSS_CELLS / 2.0 - std::f64::consts::FRAC_1_SQRT_2) * grid {
            return Err(format!(
                "swamps[{k}] is too small: the {grid} m height grid carves no swamp under {} m across",
                MIN_ACROSS_CELLS * grid
            ));
        }
    }
    Ok(())
}
