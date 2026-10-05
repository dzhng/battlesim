//! Ground a vehicle cannot drive into: a pocket is open ground closed off on
//! its side's known grid (a lawn walled round with no gate it fits), found
//! by flooding the cells its footprint fits from a candidate destination.
//! A flood that runs out of cells within [`POCKET_CELLS`] has found a
//! pocket; one that does not is open ground a route can be searched across.
//! The grid's own moves decide it: a vehicle steps between orthogonal
//! neighbours it fits, and a diagonal needs both of them, so four-way
//! flooding finds exactly what a route search can reach.
use std::cell::RefCell;
use std::collections::BTreeMap;

use super::{Mobility, Mover, NavGrid, NAV_CELL_M, START_REACH_M};
use crate::math::V2;

/// The most cells a flood visits before it calls the ground open: a 128 m
/// square, larger than any yard or lawn a town closes off.
const POCKET_CELLS: usize = 4096;

/// How far from where it was sent a vehicle's destination may move to get
/// out of a pocket: across the widest pocket a click lands in the middle of.
pub(super) const POCKET_REACH_M: f64 = 48.0;

/// What one order's placement has learned of the pockets on its side's
/// grid, per footprint (what a class can shove, and how wide it is).
#[derive(Default)]
pub struct Pockets {
    found: RefCell<BTreeMap<(u8, u64), Flooded>>,
}

/// The cells flooded for one footprint, each with the flood that reached
/// it, and whether that flood ran out of cells (a pocket).
#[derive(Default)]
struct Flooded {
    flood: BTreeMap<usize, usize>,
    closed: Vec<bool>,
}

impl NavGrid {
    /// Whether a vehicle `m` standing at `from` can drive to `to` on this
    /// grid, judged by the pocket `to` lies in, between the cells a route
    /// search would start and end in (without paying a route's work). Ground the flood calls open
    /// is taken as reachable, and so is everything when `from` is wedged
    /// where it fits no cell: the route search then has the last word.
    pub(super) fn reaches(&self, from: V2, to: V2, m: &Mobility, pockets: &Pockets) -> bool {
        let Some(start) = self.nearest_fit(from, Mover::free(m), START_REACH_M, false) else {
            return true;
        };
        let Some(end) = self.nearest_fit(to, Mover::free(m), NAV_CELL_M * 3.0, false) else {
            return false;
        };
        if !self.base.terrain.connected(start, end, self.nx) {
            return false;
        }
        let key = (self.stopping(m.push), m.half_width_m.to_bits());
        let mut found = pockets.found.borrow_mut();
        let flooded = found.entry(key).or_default();
        let flood = match flooded.flood.get(&end) {
            Some(&flood) => flood,
            None => self.flood(end, m, flooded),
        };
        !flooded.closed[flood] || flooded.flood.get(&start) == Some(&flood)
    }

    /// Flood the cells `m` fits from `seed`, recording each in `flooded`;
    /// the new flood's index.
    fn flood(&self, seed: usize, m: &Mobility, flooded: &mut Flooded) -> usize {
        let id = flooded.closed.len();
        let mut queue = std::collections::VecDeque::from([seed]);
        flooded.flood.insert(seed, id);
        let mut visited = 1;
        // Meeting an earlier flood means meeting ground it called open: a
        // pocket's flood took in every cell that touches it.
        let mut open = false;
        while let Some(k) = queue.pop_front() {
            if open || visited >= POCKET_CELLS {
                open = true;
                break;
            }
            let (i, j) = ((k % self.nx) as isize, (k / self.nx) as isize);
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let Some(next) = self.index(i + di, j + dj) else {
                    continue;
                };
                if let Some(&other) = flooded.flood.get(&next) {
                    open |= other != id;
                    continue;
                }
                if !self.fits_off_centre(next, Mover::free(m), 0.0, false) {
                    continue;
                }
                flooded.flood.insert(next, id);
                visited += 1;
                queue.push_back(next);
            }
        }
        flooded.closed.push(!open);
        id
    }
}
