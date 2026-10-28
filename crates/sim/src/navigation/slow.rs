//! How slow the ground is, tile by tile, for a search's estimate.
use super::cells::{Cell, Cells};
use super::{slope_multiplier, Mobility, NAV_CELL_M, TILE_SAMPLES, TILE_SIDE};
use crate::math::V2;

/// How slow the ground is, a tile at a time: what a search's estimate of the
/// way still to go reads, so a wood or a hillside ahead is not taken for
/// open ground. Counts, so the order cells are read in never shows.
#[derive(Clone)]
pub(super) struct SlowGround {
    tiles_x: usize,
    /// Per tile, how many of its cells are forest off any road.
    pub(super) forest: Vec<u16>,
    /// Per tile, how much longer its cells' slopes make a crossing than flat
    /// ground would, summed over its cells in 1/1024ths.
    pub(super) steep: Vec<u32>,
}

impl SlowGround {
    const TILE_M: f64 = TILE_SIDE as f64 * NAV_CELL_M;

    pub(super) fn new(cells: &Cells) -> Self {
        let tiles_x = cells.nx.div_ceil(TILE_SIDE);
        let tiles = tiles_x * cells.ny.div_ceil(TILE_SIDE);
        let mut slow = SlowGround {
            tiles_x,
            forest: vec![0; tiles],
            steep: vec![0; tiles],
        };
        for (at, cell) in cells.stored() {
            let tile = (at / cells.nx / TILE_SIDE) * tiles_x + at % cells.nx / TILE_SIDE;
            slow.forest[tile] += Self::forest(cell);
            slow.steep[tile] += Self::steep(cell);
        }
        slow
    }

    fn forest(cell: &Cell) -> u16 {
        u16::from(cell.forest && cell.road_factor == 0.0)
    }

    fn steep(cell: &Cell) -> u32 {
        let longer = 1.0 / slope_multiplier(cell.slope_deg) - 1.0;
        (longer * 1024.0).round() as u32
    }

    /// The cell at `at`, on a grid `nx` wide, is now `new` instead of `old`.
    pub(super) fn replace(&mut self, at: usize, nx: usize, old: &Cell, new: &Cell) {
        let tile = (at / nx / TILE_SIDE) * self.tiles_x + at % nx / TILE_SIDE;
        self.forest[tile] = self.forest[tile] - Self::forest(old) + Self::forest(new);
        self.steep[tile] = self.steep[tile] - Self::steep(old) + Self::steep(new);
    }

    /// How many times longer than over open, flat ground the straight way
    /// from `a` to `b` takes `m`, judged by the tiles it crosses.
    pub(super) fn stretch(&self, a: V2, b: V2, m: &Mobility) -> f64 {
        let samples = (((b - a).length() / Self::TILE_M).ceil() as usize).clamp(1, 32);
        let mut total = 0.0;
        for k in 0..samples {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let at = |v: f64| (v / Self::TILE_M).floor().max(0.0) as usize;
            let tile = (at(p.y) * self.tiles_x + at(p.x)).min(self.forest.len() - 1);
            let forest = self.forest[tile] as f64 / TILE_SAMPLES as f64;
            let steep = 1.0 + self.steep[tile] as f64 / (1024.0 * TILE_SAMPLES as f64);
            total += steep * (1.0 - forest + forest / m.forest_multiplier);
        }
        total / samples as f64
    }
}
