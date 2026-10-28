//! How slow the ground is, tile by tile, for a search's estimate.
use super::cells::{Cell, Cells};
use super::{slope_multiplier, Mobility, NAV_CELL_M, TILE_SAMPLES, TILE_SIDE};
use crate::math::V2;

/// How slow the ground is, a tile at a time: what a search's estimate of the
/// way still to go reads, so a wood or a hillside ahead is not taken for
/// open ground. Counts, so the order cells are read in never shows. Each
/// cell off any road is counted once, by what slows it: forest, swamp, or
/// both, where the slower of the two applies.
#[derive(Clone)]
pub(super) struct SlowGround {
    tiles_x: usize,
    /// Per tile, how many of its cells off any road are forest only, swamp
    /// only, and both.
    pub(super) ground: Vec<[u16; 3]>,
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
            ground: vec![[0; 3]; tiles],
            steep: vec![0; tiles],
        };
        for (at, cell) in cells.stored() {
            let tile = (at / cells.nx / TILE_SIDE) * tiles_x + at % cells.nx / TILE_SIDE;
            if let Some(class) = Self::class(cell) {
                slow.ground[tile][class] += 1;
            }
            slow.steep[tile] += Self::steep(cell);
        }
        slow
    }

    /// Which count a cell joins: forest only, swamp only or both, off any
    /// road; none on a road or open ground.
    fn class(cell: &Cell) -> Option<usize> {
        match (cell.road_factor == 0.0, cell.forest, cell.swamp) {
            (true, true, false) => Some(0),
            (true, false, true) => Some(1),
            (true, true, true) => Some(2),
            _ => None,
        }
    }

    fn steep(cell: &Cell) -> u32 {
        let longer = 1.0 / slope_multiplier(cell.slope_deg) - 1.0;
        (longer * 1024.0).round() as u32
    }

    /// The cell at `at`, on a grid `nx` wide, is now `new` instead of `old`.
    pub(super) fn replace(&mut self, at: usize, nx: usize, old: &Cell, new: &Cell) {
        let tile = (at / nx / TILE_SIDE) * self.tiles_x + at % nx / TILE_SIDE;
        if let Some(class) = Self::class(old) {
            self.ground[tile][class] -= 1;
        }
        if let Some(class) = Self::class(new) {
            self.ground[tile][class] += 1;
        }
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
            let tile = (at(p.y) * self.tiles_x + at(p.x)).min(self.ground.len() - 1);
            let [forest, swamp, both] = self.ground[tile].map(|n| n as f64 / TILE_SAMPLES as f64);
            let steep = 1.0 + self.steep[tile] as f64 / (1024.0 * TILE_SAMPLES as f64);
            let slowest = m.forest_multiplier.min(m.swamp_multiplier);
            total += steep
                * (1.0 - forest - swamp - both
                    + forest / m.forest_multiplier
                    + swamp / m.swamp_multiplier
                    + both / slowest);
        }
        total / samples as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::v2;

    /// Ground both wooded and wet is as slow as the slower of the two, once:
    /// the estimate never multiplies them.
    #[test]
    fn a_wood_in_a_swamp_slows_the_estimate_by_the_slower_factor_once() {
        let mut cells = Cells::new(true, TILE_SIDE, TILE_SIDE);
        for at in 0..TILE_SAMPLES {
            let cell = Cell {
                forest: true,
                swamp: true,
                ..cells.default_at(at)
            };
            cells.set(at, cell);
        }
        let slow = SlowGround::new(&cells);
        let m = Mobility {
            off_road_mps: 5.0,
            road_mps: 10.0,
            forest_multiplier: 0.5,
            swamp_multiplier: 0.25,
            half_width_m: 1.0,
            class: contract::map::MoverClass::Vehicle,
            push: contract::scenario::PushClass::Heavy,
            drive: None,
        };
        let stretch = slow.stretch(v2(1.0, 1.0), v2(60.0, 1.0), &m);
        assert!((stretch - 4.0).abs() < 1e-12, "{stretch}");
    }
}
