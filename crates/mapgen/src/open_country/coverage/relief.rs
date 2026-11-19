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
//! height field's triangles too, and over small boxes from the height grid's
//! own samples (`Sight::land`).
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
//!
//! Foliage below the eyes is a witness too, looking down (`falls_into`).
//! The sweep counts foliage at a sample whenever the ray's sight there,
//! `eye + max(horizon, slope) * d`, is below the canopy; one occupied step
//! already stops the ray short of its far cell. The slope term is the
//! sample's own ground plus the target height, always below the canopy.
//! The horizon term is set by earlier samples: one a fraction `t` of the
//! way, standing `h`, lifts the sight at the sample to `eye + (h - eye) /
//! t`. Bounding that over bands of `t` from the first whole step to the
//! block bounds every ray, so a block whose lowest canopy stands above the
//! bound hides the far cell from every eye in the rectangle. The bound
//! needs the eye's own land read exactly: over small boxes the land is
//! read at the height grid's samples, as the field interpolates them.
use super::*;
use crate::layout::geometry::Grid;
use crate::relief::ReliefBound;
use contract::map::Relief;

/// Tile of the relief bound.
const TILE_M: f64 = 8.0;
/// Fog cells between crest witnesses, each way.
const CREST_STRIDE: usize = 4;
/// Distances a crest tries for its hidden cell, from the far cell inward.
const CREST_SAMPLES: usize = 4;
/// Bands the way from an eye down to a foliage witness is cut into.
const FALL_BANDS: usize = 12;
/// The most times a band the sight may rise too high in is halved.
const FALL_SPLITS: usize = 2;
/// The most height-grid cells a side a box's land is read exactly over.
const EXACT_CELLS: f64 = 8.0;
/// The bucket the relief features are indexed by, for exact reads.
const FEATURE_CELL_M: f64 = 64.0;

pub(super) struct Sight {
    bound: ReliefBound,
    /// Each relief feature, indexed by the box it adds height in, for
    /// exact reads.
    relief: Vec<Relief>,
    features: Grid,
    /// The height grid's spacing, and its diagonal: how far a triangle's
    /// corners lie.
    grid: f64,
    pad: f64,
    /// The most a river can lower the land, near one only, and a swamp
    /// anywhere (its water is not in the river index).
    river_low: f64,
    swamp_low: f64,
    bank_margin: f64,
    /// The lowest eye of any ground observer, over its own ground.
    eye_low: f64,
    /// The highest eye or target over its ground.
    eye: f64,
    canopy: f64,
    /// Each deck's box and height; whether no deck stands on relief, so a
    /// deck and a rise never stack.
    decks: Vec<([f64; 4], f64)>,
    decks_flat: bool,
    /// Per fog cell, the lowest top of any body stamped there.
    body_top: Vec<f64>,
    /// The last rectangle asked about, with its highest land and lowest
    /// eye: one rectangle is asked of every witness near it in turn.
    asked: core::cell::Cell<Option<([f64; 4], [f64; 2])>>,
    /// Per fog cell, the lowest ground of its 3x3 block, once read.
    blocks: Vec<core::cell::Cell<f64>>,
}

impl Sight {
    pub(super) fn new(
        map: &MapDefinition,
        river_low: f64,
        bank_margin: f64,
        [eye_low, eye]: [f64; 2],
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
        let mut features = Grid::new(map.size, FEATURE_CELL_M);
        for (i, f) in map.relief.iter().enumerate() {
            let b = f
                .footprint()
                .0
                .map_or([0., 0., map.size[0], map.size[1]], |r| {
                    [r[0], r[1], r[0] + r[2], r[1] + r[3]]
                });
            features.insert(b, i as u32);
        }
        Self {
            bound,
            relief: map.relief.clone(),
            features,
            grid: map.height_grid_m,
            pad,
            river_low,
            swamp_low: -map.swamps.iter().map(|s| s.depth_m).fold(0., f64::max),
            bank_margin,
            eye_low,
            eye,
            canopy,
            decks,
            decks_flat,
            body_top: vec![f64::INFINITY; cells],
            asked: core::cell::Cell::new(None),
            blocks: (0..cells)
                .map(|_| core::cell::Cell::new(f64::NAN))
                .collect(),
        }
    }

    pub(super) fn note_body(&mut self, k: usize, top: f64) {
        self.body_top[k] = self.body_top[k].min(top);
    }

    /// The least and most the relief stands over the height field's
    /// triangles touching `b`, before rivers and swamps lower it. Over a
    /// small box this is exact: a triangle stands between its corners'
    /// samples, read here as the field reads them. Over a larger one it is
    /// the tile bound over the box grown by a triangle's reach.
    fn land(&self, b: [f64; 4]) -> [f64; 2] {
        let g = self.grid;
        let (i0, j0) = (libm::floor(b[0] / g), libm::floor(b[1] / g));
        let (i1, j1) = (libm::ceil(b[2] / g), libm::ceil(b[3] / g));
        if (i1 - i0).max(j1 - j0) > EXACT_CELLS {
            let p = self.pad;
            return self.bound.span([b[0] - p, b[1] - p, b[2] + p, b[3] + p]);
        }
        // Each feature once, though it may stand in several buckets.
        let mut near: Vec<u32> = Vec::new();
        self.features.any([i0 * g, j0 * g, i1 * g, j1 * g], |i| {
            near.push(i);
            false
        });
        near.sort_unstable();
        near.dedup();
        let mut out = [f64::INFINITY, f64::NEG_INFINITY];
        for j in j0 as i64..=j1 as i64 {
            for i in i0 as i64..=i1 as i64 {
                let (x, y) = (i as f64 * g, j as f64 * g);
                let h: f64 = near
                    .iter()
                    .map(|&f| self.relief[f as usize].height_at(x, y))
                    .sum();
                out = [out[0].min(h), out[1].max(h)];
            }
        }
        out
    }

    /// The most the land or a deck can stand anywhere in `b`.
    fn highest(&self, b: [f64; 4]) -> f64 {
        let land = self.land(b)[1];
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
                let crest = self.block_ground(s, x, y) - s.eye;
                if !self.witnessed[k] && crest > 0. {
                    crests.push((k, p, Some(crest)));
                }
            }
        }
        self.commit_witnesses(crests);
    }

    /// The 3x3 fog-cell block about cell `(x, y)`.
    fn block(&self, x: usize, y: usize) -> [f64; 4] {
        let (x0, x1) = (x.saturating_sub(1), (x + 1).min(self.nx - 1));
        let (y0, y1) = (y.saturating_sub(1), (y + 1).min(self.ny - 1));
        [
            x0 as f64 * self.cell,
            y0 as f64 * self.cell,
            (x1 + 1) as f64 * self.cell,
            (y1 + 1) as f64 * self.cell,
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
        let block = self.block(x, y);
        let ground = self.block_ground(s, x, y);
        let [mut most, _] = self.asked(s, low, high);
        for &(distance, spread) in &self.far_samples {
            most = most.max(s.highest(self.sample_box(low, high, direction, distance, spread)));
        }
        ground + top > most + s.eye
            || (top == s.canopy && self.falls_into(s, block, ground + top, low, high))
    }

    /// Every ray from an eye in `low..high` that samples the foliage block
    /// `block` does so below `canopy`, its lowest canopy, as the module
    /// explains. The way there is cut into bands by the fraction `t` of a
    /// ray's way to its sample; land or a body standing `h` in a band lifts
    /// the ray's sight at the sample to at most `eye + (h - eye) / t`. A
    /// band that lifts it too high is halved, `FALL_SPLITS` times at most.
    fn falls_into(&self, s: &Sight, block: [f64; 4], canopy: f64, low: Point, high: Point) -> bool {
        let rect = [low[0], low[1], high[0], high[1]];
        let [_, eye] = self.asked(s, low, high);
        let far = libm::hypot(
            (rect[2] - block[0]).max(block[2] - rect[0]),
            (rect[3] - block[1]).max(block[3] - rect[1]),
        );
        let at = |t: f64| core::array::from_fn::<f64, 4, _>(|i| rect[i] + (block[i] - rect[i]) * t);
        // Whether no land or body in the band `t..next` lifts the sight to
        // the canopy: rising ground lifts it most from the band's near end,
        // falling ground from its far end.
        let clear = |t: f64, next: f64| {
            let (a, b) = (at(t), at(next));
            let area = [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3]),
            ];
            let most = s.highest(area).max(self.body_top_within(s, area));
            eye + (most - eye) / if most > eye { t } else { next } < canopy
        };
        // A ray's first sample is a whole step out.
        let first = (self.cell / far).min(1.);
        let ratio = libm::pow(1. / first, 1. / FALL_BANDS as f64);
        // Nearest the eye last in, so first out: those fail most often.
        let mut bands: Vec<(f64, f64, usize)> = (0..FALL_BANDS)
            .rev()
            .map(|k| {
                let next = if k + 1 == FALL_BANDS {
                    1.
                } else {
                    first * libm::pow(ratio, (k + 1) as f64)
                };
                (first * libm::pow(ratio, k as f64), next, 0)
            })
            .collect();
        while let Some((t, next, splits)) = bands.pop() {
            if !self.queries.work.spend(4) {
                return false;
            }
            if clear(t, next) {
                continue;
            }
            if splits == FALL_SPLITS {
                return false;
            }
            let middle = libm::sqrt(t * next);
            bands.extend([(middle, next, splits + 1), (t, middle, splits + 1)]);
        }
        true
    }

    /// The most any body near `area` can stand, by its index bounds;
    /// nothing where none is near.
    fn body_top_within(&self, s: &Sight, area: [f64; 4]) -> f64 {
        let q = &self.queries;
        let mut most = f64::NEG_INFINITY;
        q.body_index.any_charged(
            area,
            |i| {
                let b = &q.bodies[i as usize];
                let r = libm::hypot(b.half_extents[0], b.half_extents[1]);
                let c = b.center;
                let base = b
                    .base_z
                    .unwrap_or_else(|| s.highest([c[0] - r, c[1] - r, c[0] + r, c[1] + r]));
                most = most.max(base + 2. * b.half_extents[2]);
                false
            },
            || q.work.spend(1),
        );
        most
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

    /// The highest land or deck over `low..high`, and its lowest eye.
    fn asked(&self, s: &Sight, low: Point, high: Point) -> [f64; 2] {
        let rect = [low[0], low[1], high[0], high[1]];
        if let Some((_, known)) = s.asked.get().filter(|(asked, _)| *asked == rect) {
            return known;
        }
        let known = [
            s.highest(rect),
            self.lowest_ground(s, scale(add(low, high), 0.5), rect) + s.eye_low,
        ];
        s.asked.set(Some((rect, known)));
        known
    }

    /// The lowest ground of the 3x3 block about cell `(x, y)`, read once.
    fn block_ground(&self, s: &Sight, x: usize, y: usize) -> f64 {
        let cell = &s.blocks[y * self.nx + x];
        if cell.get().is_nan() {
            cell.set(self.lowest_ground(s, self.middle(x, y), self.block(x, y)));
        }
        cell.get()
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

    /// The least the land can stand over `area` about `middle`, a river or
    /// swamp beneath it included.
    fn lowest_ground(&self, s: &Sight, middle: Point, area: [f64; 4]) -> f64 {
        let reach = libm::hypot(area[2] - area[0], area[3] - area[1]) / 2. + s.pad + s.bank_margin;
        let river = self
            .queries
            .water
            .gap_charged(middle, reach, || self.queries.work.spend(1));
        s.land(area)[0] + s.swamp_low + if river < reach { s.river_low } else { 0. }
    }
}
