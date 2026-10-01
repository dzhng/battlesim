//! What rivers and bridges do to the land's height.
//!
//! In the water the ground is the river's cross-section (`contract::river`).
//! On the bank it is the land, cut down by however far the cross-section
//! still lies below the land at the water's edge: the cut is deepest at the
//! waterline, where the ground meets the surface exactly, and gone where the
//! bank's grade has climbed back to the edge's height. Land that rises
//! beside a river keeps its shape; only the strip beside the water moves.
//!
//! Along a bridge's approach the cross-section is as steep as the grid can
//! draw under the slope cutoff (`contract::river::steepest_grade`), so the
//! cut is over by the deck's end and the deck is stepped onto from the
//! land's own height.
use super::surfaces::SurfaceIndex;
use super::terrain::HEIGHT_PAGE_SIZE;
use crate::math::{v2, Obb2, V2};
use contract::ground::limits;
use contract::map::MapDefinition;
use contract::river::steepest_grade;

pub(super) struct Carve<'a> {
    map: &'a MapDefinition,
    ground: &'a SurfaceIndex,
    /// The steepest grade a ramp takes.
    steepest: f64,
    /// The farthest any bank is cut from its water's edge.
    reach: f64,
    approaches: Vec<Approach>,
    regions: Vec<[f64; 4]>,
}

/// Where a bridge is come onto: its deck, lengthened at both ends by the
/// bank's reach, and a height sample wider each side so the grid carries the
/// deck's whole width. Beside it the ramp eases back to the river's own bank
/// over `ease_m`.
struct Approach {
    footprint: Obb2,
    ease_m: f64,
}

/// The ramp's sides ease over at least this many height samples, and this
/// many times the bank's reach: the tilt that adds along the bank then keeps
/// the grid's triangles under the slope cutoff.
const EASE_SAMPLES: f64 = 3.0;
const EASE_REACHES: f64 = 2.5;

impl<'a> Carve<'a> {
    /// `relief` is each relief feature's box and the most it adds to the land.
    pub fn new(
        map: &'a MapDefinition,
        ground: &'a SurfaceIndex,
        relief: &[([f64; 4], f64)],
    ) -> Self {
        let grid = map.height_grid_m;
        let piece_m = grid * HEIGHT_PAGE_SIZE as f64;
        let mut reach: f64 = 0.0;
        let mut regions = Vec::new();
        let mut approaches = Vec::new();
        for river in ground.rivers() {
            // In pieces no longer than a height page, so a long diagonal
            // stretch claims the strip beside it and not the box round it.
            for pair in river.samples().windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let length = (v2(b.xy[0], b.xy[1]) - v2(a.xy[0], a.xy[1])).length();
                let pieces = (length / piece_m).ceil().max(1.0) as usize;
                let grade = (a.depth_m / a.half_width_m).min(b.depth_m / b.half_width_m);
                let half = a.half_width_m.max(b.half_width_m);
                for k in 0..pieces {
                    let at = |u: f64| [0, 1].map(|i| a.xy[i] + (b.xy[i] - a.xy[i]) * u);
                    let ends = [
                        at(k as f64 / pieces as f64),
                        at((k + 1) as f64 / pieces as f64),
                    ];
                    // The land at the water's edge stands no higher than the
                    // relief that reaches the edge adds up to.
                    let [x0, y0, x1, y1] = limits(&ends, half);
                    let edge_land: f64 = relief
                        .iter()
                        .filter(|(r, _)| overlaps(*r, [x0, y0, x1 - x0, y1 - y0]))
                        .map(|(_, peak)| peak.max(0.0))
                        .sum();
                    let bank = ((edge_land - river.surface_z()) / grade).max(0.0);
                    reach = reach.max(bank);
                    // A sampled change moves the adjacent triangle cell too.
                    let [x0, y0, x1, y1] = limits(&ends, half + bank + grid);
                    regions.push([x0, y0, x1 - x0, y1 - y0]);
                }
            }
        }
        if !ground.rivers().is_empty() {
            let gentlest = ground
                .rivers()
                .iter()
                .flat_map(|r| r.samples())
                .map(|s| s.depth_m / s.half_width_m)
                .fold(f64::INFINITY, f64::min);
            let lowest = ground
                .rivers()
                .iter()
                .map(|r| r.surface_z())
                .fold(f64::INFINITY, f64::min);
            approaches = map
                .bridges
                .iter()
                .map(|b| {
                    let land = b
                        .ends()
                        .map(|end| {
                            map.relief_height(
                                (end[0][0] + end[1][0]) / 2.0,
                                (end[0][1] + end[1][1]) / 2.0,
                            )
                        })
                        .into_iter()
                        .fold(f64::NEG_INFINITY, f64::max);
                    let bank = ((land - lowest) / gentlest).max(0.0);
                    Approach {
                        footprint: Obb2 {
                            center: v2(b.center[0], b.center[1]),
                            yaw: b.yaw,
                            half: v2(b.half_extents[0] + bank, b.half_extents[1] + grid),
                        },
                        ease_m: (EASE_SAMPLES * grid).max(EASE_REACHES * bank),
                    }
                })
                .collect();
        }
        Self {
            map,
            ground,
            steepest: steepest_grade(map.slope_cutoff_deg),
            reach,
            approaches,
            regions,
        }
    }

    /// Where the carving can change the land.
    pub fn regions(&self) -> &[[f64; 4]] {
        &self.regions
    }

    /// The carved height at `p`, where the land stands `natural` high.
    pub fn height(&self, natural: f64, p: V2) -> f64 {
        if self.ground.rivers().is_empty() {
            return natural;
        }
        // On an approach, bed and bank alike take the steepest grade: still
        // one plane through the waterline, so a ramp never stands in the water.
        let least_grade = self
            .approaches
            .iter()
            .map(|a| self.steepest * (1.0 - a.footprint.distance(p) / a.ease_m))
            .fold(0.0, f64::max);
        let mut h = natural;
        for (river, section) in self.ground.river_sections(p, self.reach) {
            let surface_z = self.ground.rivers()[river].surface_z();
            h = h.min(section.carved(natural, surface_z, least_grade, |edge| {
                self.map.relief_height(edge[0], edge[1])
            }));
        }
        h
    }
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[0] + b[2] && b[0] <= a[0] + a[2] && a[1] <= b[1] + b[3] && b[1] <= a[1] + a[3]
}
