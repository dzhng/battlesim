//! How slow the ground is, tile by tile, for a search's estimate.
use contract::movement::{Locomotion, SpeedFactorTable};

use super::cells::{Cell, Cells};
use super::{Mobility, NAV_CELL_M, TILE_SAMPLES, TILE_SIDE};
use crate::math::V2;

/// How slow the ground is, a tile at a time and for each locomotion: what a
/// search's estimate of the way still to go reads, so a wood or a hillside
/// ahead is not taken for open ground. Counts, so the order cells are read
/// in never shows. Each cell off any road is counted once, by what slows
/// it: forest, swamp, or both, where the slower of the two applies.
#[derive(Clone)]
pub(super) struct SlowGround {
    tiles_x: usize,
    pub(super) speed_factors: SpeedFactorTable,
    /// Per tile and locomotion, how many of its cells off any road are
    /// forest only, swamp only, and both.
    pub(super) ground: Vec<[[u16; 3]; 3]>,
    /// Per tile and locomotion, how much longer its cells' slopes make a
    /// crossing than flat ground would, summed over its cells in 1/1024ths.
    pub(super) steep: Vec<[u32; 3]>,
}

impl SlowGround {
    const TILE_M: f64 = TILE_SIDE as f64 * NAV_CELL_M;

    pub(super) fn new(cells: &Cells, speed_factors: SpeedFactorTable) -> Self {
        let tiles_x = cells.nx.div_ceil(TILE_SIDE);
        let tiles = tiles_x * cells.ny.div_ceil(TILE_SIDE);
        let mut slow = SlowGround {
            tiles_x,
            speed_factors,
            ground: vec![[[0; 3]; 3]; tiles],
            steep: vec![[0; 3]; tiles],
        };
        for (at, cell) in cells.stored() {
            let tile = (at / cells.nx / TILE_SIDE) * tiles_x + at % cells.nx / TILE_SIDE;
            slow.count(tile, cell, 1);
        }
        slow
    }

    /// Adds `cell` to, or with `-1` takes it from, `tile`'s counts.
    fn count(&mut self, tile: usize, cell: &Cell, by: i32) {
        for l in Locomotion::ALL {
            let k = l as usize;
            if let Some(class) = self.class(cell, l) {
                let n = &mut self.ground[tile][k][class];
                *n = (*n as i32 + by) as u16;
            }
            let longer = by as i64 * self.steep_of(cell, l) as i64;
            let steep = &mut self.steep[tile][k];
            *steep = (*steep as i64 + longer) as u32;
        }
    }

    /// Which count a cell joins for locomotion `l`: forest only, swamp only
    /// or both, off any road; none on a road or open ground.
    fn class(&self, cell: &Cell, l: Locomotion) -> Option<usize> {
        let off_road = cell
            .paved
            .is_none_or(|kind| self.speed_factors.of(l).surfaces.of(kind) == 0.0);
        match (off_road, cell.forest, cell.swamp) {
            (true, true, false) => Some(0),
            (true, false, true) => Some(1),
            (true, true, true) => Some(2),
            _ => None,
        }
    }

    fn steep_of(&self, cell: &Cell, l: Locomotion) -> u32 {
        let longer = 1.0 / self.speed_factors.of(l).slope.factor(cell.slope_deg) - 1.0;
        (longer * 1024.0).round() as u32
    }

    /// The cell at `at`, on a grid `nx` wide, is now `new` instead of `old`.
    pub(super) fn replace(&mut self, at: usize, nx: usize, old: &Cell, new: &Cell) {
        let tile = (at / nx / TILE_SIDE) * self.tiles_x + at % nx / TILE_SIDE;
        self.count(tile, old, -1);
        self.count(tile, new, 1);
    }

    /// How many times longer than over open, flat ground the straight way
    /// from `a` to `b` takes `m`, judged by the tiles it crosses.
    pub(super) fn stretch(&self, a: V2, b: V2, m: &Mobility) -> f64 {
        let samples = (((b - a).length() / Self::TILE_M).ceil() as usize).clamp(1, 32);
        let (l, f) = (m.locomotion as usize, &m.speed_factors);
        let mut total = 0.0;
        for k in 0..samples {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let at = |v: f64| (v / Self::TILE_M).floor().max(0.0) as usize;
            let tile = (at(p.y) * self.tiles_x + at(p.x)).min(self.ground.len() - 1);
            let [forest, swamp, both] =
                self.ground[tile][l].map(|n| n as f64 / TILE_SAMPLES as f64);
            let steep = 1.0 + self.steep[tile][l] as f64 / (1024.0 * TILE_SAMPLES as f64);
            let slowest = f.forest.min(f.swamp);
            total += steep
                * (1.0 - forest - swamp - both
                    + forest / f.forest
                    + swamp / f.swamp
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
        let mut speed_factors: SpeedFactorTable = serde_json::from_value(
            crate::fixtures::test_game()["movement"]["speed_factors"].clone(),
        )
        .unwrap();
        speed_factors.tracked.forest = 0.5;
        speed_factors.tracked.swamp = 0.25;
        let slow = SlowGround::new(&cells, speed_factors);
        let m = Mobility {
            off_road_mps: 5.0,
            road_mps: 10.0,
            locomotion: Locomotion::Tracked,
            speed_factors: speed_factors.tracked,
            half_width_m: 1.0,
            class: contract::map::MoverClass::Vehicle,
            push: contract::scenario::PushClass::Heavy,
            drive: None,
        };
        let stretch = slow.stretch(v2(1.0, 1.0), v2(60.0, 1.0), &m);
        assert!((stretch - 4.0).abs() < 1e-12, "{stretch}");
    }
}
