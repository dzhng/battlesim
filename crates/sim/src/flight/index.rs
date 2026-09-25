//! Uniform XY bucket grid over bodies' swept footprints, rebuilt each tick, so a
//! round only tests bodies near its path. A coarse cell only narrows the
//! candidates; every hit is still the exact swept test.
use super::Body;
use crate::math::{v2, V2};

const CELL_M: f64 = 16.0;

#[derive(Default)]
pub(super) struct BodyGrid {
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl BodyGrid {
    pub fn rebuild(&mut self, width: f64, depth: f64, bodies: &[Body]) {
        let nx = (width / CELL_M).ceil().max(1.0) as usize;
        let ny = (depth / CELL_M).ceil().max(1.0) as usize;
        if (nx, ny) != (self.nx, self.ny) {
            *self = BodyGrid {
                nx,
                ny,
                cells: vec![Vec::new(); nx * ny],
            };
        }
        for cell in &mut self.cells {
            cell.clear();
        }
        for (i, body) in bodies.iter().enumerate() {
            let r = body.footprint_radius();
            let (a, b) = (body.from.base.xy(), body.to.base.xy());
            let lo = v2(a.x.min(b.x) - r, a.y.min(b.y) - r);
            let hi = v2(a.x.max(b.x) + r, a.y.max(b.y) + r);
            let (i0, i1, j0, j1) = self.range(lo, hi);
            for j in j0..=j1 {
                for c in i0..=i1 {
                    self.cells[j * self.nx + c].push(i as u32);
                }
            }
        }
    }

    fn range(&self, lo: V2, hi: V2) -> (usize, usize, usize, usize) {
        let clamp = |v: f64, n: usize| ((v / CELL_M).floor().max(0.0) as usize).min(n - 1);
        (
            clamp(lo.x, self.nx),
            clamp(hi.x, self.nx),
            clamp(lo.y, self.ny),
            clamp(hi.y, self.ny),
        )
    }

    /// Indices of bodies whose swept footprint may reach the XY box, ascending.
    pub fn query(&self, lo: V2, hi: V2, out: &mut Vec<u32>) {
        out.clear();
        if self.cells.is_empty() {
            return;
        }
        let (i0, i1, j0, j1) = self.range(lo, hi);
        for j in j0..=j1 {
            for i in i0..=i1 {
                out.extend_from_slice(&self.cells[j * self.nx + i]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}
