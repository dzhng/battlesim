//! The certificate on land with relief. On flat land one global margin
//! holds every witness: the lowest land plus the canopy stands above the
//! highest eye. With rises that cannot hold everywhere, so each witness
//! earns it locally, for the one rectangle it is asked about: the least the
//! foliage or body over its block can stand (the lowest the land there can
//! be, plus the lowest top over the block) must stand above the most any
//! observer in the rectangle or any far sample of it can stand (the highest
//! the land there can be, or a deck's, plus the eye). Any ray between them
//! then meets the witness below its top, or meets the land first; either
//! hides the far cell. Heights come from per-tile bounds (`crate::relief`)
//! over boxes grown by a height-grid diagonal, so they hold for the sampled
//! height field's triangles too.
use super::*;
use crate::relief::ReliefBound;

/// Tile of the relief bound.
const TILE_M: f64 = 64.0;

pub(super) struct Sight {
    bound: ReliefBound,
    /// The height grid's diagonal: how far a triangle's corners lie.
    pad: f64,
    /// The most a river can lower the land, near one only, and a swamp
    /// anywhere (its water is not in the river index).
    river_low: f64,
    swamp_low: f64,
    bank_margin: f64,
    eye: f64,
    canopy: f64,
    /// Each deck's box and height; whether no deck stands on relief, so a
    /// deck and a rise never stack.
    decks: Vec<([f64; 4], f64)>,
    decks_flat: bool,
    /// Per fog cell, the lowest top of any body stamped there.
    body_top: Vec<f64>,
}

impl Sight {
    pub(super) fn new(
        map: &MapDefinition,
        river_low: f64,
        bank_margin: f64,
        eye: f64,
        canopy: f64,
        cells: usize,
    ) -> Self {
        let bound = ReliefBound::new(&map.relief, map.size, TILE_M);
        let pad = map.height_grid_m * core::f64::consts::SQRT_2;
        let decks: Vec<_> = map
            .bridges
            .iter()
            .map(|b| {
                let r = libm::hypot(b.half_extents[0], b.half_extents[1]);
                let c = b.center;
                ([c[0] - r, c[1] - r, c[0] + r, c[1] + r], b.deck_z)
            })
            .collect();
        let decks_flat = decks.iter().all(|(b, _)| {
            bound.span([b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]) == [0.0, 0.0]
        });
        Self {
            bound,
            pad,
            river_low,
            swamp_low: -map.swamps.iter().map(|s| s.depth_m).fold(0., f64::max),
            bank_margin,
            eye,
            canopy,
            decks,
            decks_flat,
            body_top: vec![f64::INFINITY; cells],
        }
    }

    pub(super) fn note_body(&mut self, k: usize, top: f64) {
        self.body_top[k] = self.body_top[k].min(top);
    }

    /// The most the land or a deck can stand anywhere in `b`.
    fn highest(&self, b: [f64; 4]) -> f64 {
        let p = self.pad;
        let land = self.bound.span([b[0] - p, b[1] - p, b[2] + p, b[3] + p])[1];
        let deck = self
            .decks
            .iter()
            .filter(|(d, _)| d[0] <= b[2] && d[2] >= b[0] && d[1] <= b[3] && d[3] >= b[1])
            .map(|(_, z)| *z)
            .fold(0., f64::max);
        if self.decks_flat {
            land.max(deck)
        } else {
            land + deck
        }
    }
}

impl Coverage<'_> {
    /// `witness` hides a far cell of every observer in `low..high` over the
    /// relief, as the module explains; `direction` and the far samples are
    /// `admits`'s.
    pub(super) fn relief_admits(
        &self,
        witness: Point,
        low: Point,
        high: Point,
        direction: &[[f64; 2]; 2],
    ) -> bool {
        let Some(s) = &self.relief else {
            return true;
        };
        if !self.queries.work.spend(8) {
            return false;
        }
        let cell = |v: f64, n: usize| (libm::floor(v / self.cell).max(0.) as usize).min(n - 1);
        let (x, y) = (cell(witness[0], self.nx), cell(witness[1], self.ny));
        let (x0, x1) = (x.saturating_sub(1), (x + 1).min(self.nx - 1));
        let (y0, y1) = (y.saturating_sub(1), (y + 1).min(self.ny - 1));
        let mut top = s.canopy;
        for j in y0..=y1 {
            for i in x0..=x1 {
                top = top.min(s.body_top[j * self.nx + i]);
            }
        }
        let block = [
            x0 as f64 * self.cell - s.pad,
            y0 as f64 * self.cell - s.pad,
            (x1 + 1) as f64 * self.cell + s.pad,
            (y1 + 1) as f64 * self.cell + s.pad,
        ];
        let reach = libm::hypot(block[2] - block[0], block[3] - block[1]) / 2. + s.bank_margin;
        let river = self
            .queries
            .water
            .gap_charged(witness, reach, || self.queries.work.spend(1));
        let ground =
            s.bound.span(block)[0] + s.swamp_low + if river < reach { s.river_low } else { 0. };
        let mut most = s.highest([low[0], low[1], high[0], high[1]]);
        for (distance, margin) in &self.far_samples {
            let t = [0, 1].map(|i| {
                [
                    low[i] + distance * direction[i][0] - margin,
                    high[i] + distance * direction[i][1] + margin,
                ]
            });
            most = most.max(s.highest([t[0][0], t[1][0], t[0][1], t[1][1]]));
        }
        ground + top > most + s.eye
    }
}
