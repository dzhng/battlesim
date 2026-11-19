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
//!
//! The land is a witness of its own, as the simulation's sweep treats it:
//! a crest block whose lowest land stands above every eye raises the
//! horizon of each ray that crosses it, and any later cell whose highest
//! land plus the target height stays below the crest is hidden. Every ray
//! that can sample such a cell, `d` out on a witness bearing, passes within
//! a cell's diagonal times `d_w / d` of the crest at its distance `d_w`, so
//! inside the block's 1.5-cell disk, where a whole step puts one of its
//! samples. A crest therefore needs no far cell: its hidden cell is the
//! farthest on the map, beyond the crest, below it (`crest_admits`). Crests
//! stand on a lattice `CREST_STRIDE` fog cells apart: one crest on a rise
//! hides as much as many.
use super::*;
use crate::relief::ReliefBound;

/// Tile of the relief bound.
const TILE_M: f64 = 8.0;
/// Fog cells between crest witnesses, each way.
const CREST_STRIDE: usize = 4;
/// Distances a crest tries for its hidden cell, from the far cell inward.
const CREST_SAMPLES: usize = 4;

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
    /// Index a crest witness wherever a 3x3 block's lowest land stands
    /// above every eye, the least it must clear on any rectangle, with the
    /// height every eye and target must stay below.
    pub(super) fn note_crests(&mut self) {
        let Some(s) = &self.relief else {
            return;
        };
        let mut crests = Vec::new();
        for y in (1..self.ny - 1).step_by(CREST_STRIDE) {
            for x in (1..self.nx - 1).step_by(CREST_STRIDE) {
                let k = y * self.nx + x;
                if !self.queries.work.spend(4) {
                    return;
                }
                let p = self.middle(x, y);
                let crest = self.lowest_ground(s, p, x, y) - s.eye;
                if !self.witnessed[k] && crest > 0. {
                    crests.push((k, p, Some(crest)));
                }
            }
        }
        self.commit_witnesses(crests);
    }

    /// The 3x3 fog-cell block about cell `(x, y)`, grown by a triangle's
    /// reach.
    fn block(&self, s: &Sight, x: usize, y: usize) -> [f64; 4] {
        let (x0, x1) = (x.saturating_sub(1), (x + 1).min(self.nx - 1));
        let (y0, y1) = (y.saturating_sub(1), (y + 1).min(self.ny - 1));
        [
            x0 as f64 * self.cell - s.pad,
            y0 as f64 * self.cell - s.pad,
            (x1 + 1) as f64 * self.cell + s.pad,
            (y1 + 1) as f64 * self.cell + s.pad,
        ]
    }

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
        let (x, y) = self.cell_of(witness);
        let (x0, x1) = (x.saturating_sub(1), (x + 1).min(self.nx - 1));
        let (y0, y1) = (y.saturating_sub(1), (y + 1).min(self.ny - 1));
        let top = (y0..=y1)
            .flat_map(|j| (x0..=x1).map(move |i| j * self.nx + i))
            .fold(s.canopy, |top, k| top.min(s.body_top[k]));
        let mut most = s.highest([low[0], low[1], high[0], high[1]]);
        for &(distance, spread) in &self.far_samples {
            most = most.max(s.highest(self.sample_box(low, high, direction, distance, spread)));
        }
        self.lowest_ground(s, witness, x, y) + top > most + s.eye
    }

    /// A bare crest that every eye and hidden target must stay below
    /// `crest`, at most `reach` from `low..high`, hides a cell of every
    /// observer there, as the module explains.
    pub(super) fn crest_admits(
        &self,
        crest: f64,
        reach: f64,
        low: Point,
        high: Point,
        direction: &[[f64; 2]; 2],
    ) -> bool {
        let Some(s) = &self.relief else {
            return false;
        };
        if !self.queries.work.spend(8) || crest <= self.highest(low, high) {
            return false;
        }
        // Past the crest's block and a whole step beyond it.
        let nearest = reach + 4. * self.cell;
        self.far_samples.iter().all(|&(far, spread)| {
            nearest <= far
                && (0..=CREST_SAMPLES).any(|k| {
                    let distance = far - (far - nearest) * k as f64 / CREST_SAMPLES as f64;
                    self.queries.work.spend(1)
                        && self.on_map(low, high, direction, distance, spread)
                        && crest
                            > s.highest(self.sample_box(low, high, direction, distance, spread))
                })
        })
    }

    /// The most the land or a deck can stand over `low..high`; nothing
    /// on flat land.
    pub(super) fn highest(&self, low: Point, high: Point) -> f64 {
        self.relief
            .as_ref()
            .map_or(0., |s| s.highest([low[0], low[1], high[0], high[1]]))
    }

    fn cell_of(&self, p: Point) -> (usize, usize) {
        let cell = |v: f64, n: usize| (libm::floor(v / self.cell).max(0.) as usize).min(n - 1);
        (cell(p[0], self.nx), cell(p[1], self.ny))
    }

    /// The least the land can stand over the 3x3 block about cell `(x, y)`
    /// holding `witness`, a river or swamp beneath it included.
    fn lowest_ground(&self, s: &Sight, witness: Point, x: usize, y: usize) -> f64 {
        let block = self.block(s, x, y);
        let reach = libm::hypot(block[2] - block[0], block[3] - block[1]) / 2. + s.bank_margin;
        let river = self
            .queries
            .water
            .gap_charged(witness, reach, || self.queries.work.spend(1));
        s.bound.span(block)[0] + s.swamp_low + if river < reach { s.river_low } else { 0. }
    }
}
