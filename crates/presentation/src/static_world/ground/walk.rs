//! The scenery's per-frame walk, the one both renderers draw from: which
//! trees, shrubs and pieces of dressing a view draws and casts, at what
//! tier, as the static world's batches.
//!
//! - **Trees and shrubs** (the forest, the shrubs under its tree lines, the
//!   backdrop past the map) are bucketed in square chunks. A chunk too far for
//!   its tallest to take a finer tier than the last draws whole at it; a
//!   nearer chunk's instances each take the tier their projected height
//!   gives. The forest and the tree lines' shrubs cast into the sun's
//!   cascades a tier coarser, with every chunk whose shadow can land in view;
//!   the backdrop casts nothing.
//! - **Dressing** is laid a cell of ground at a time as the camera comes near
//!   enough to see its tallest piece, a few cells a walk, and a cell draws
//!   whole at the tier its nearest point takes. Pieces on ground the side has
//!   seen cleared are left out.
//! - **Felled trees** leave the forest and fall on the clock, tipped about
//!   their stump's top and pressed flat as they land; their stumps stand
//!   where they grew. Each falling tree is two records (`felled`).
//!
//! Sim space, `f64` as the browser's walk was; records are the renderer's
//! 64 B instance: x, y, z, yaw; a population's extra; r, g, b and the leaf
//! seed (a piece of dressing: its height over the size it fades at); the
//! scale per axis and the crown's heart.
use super::biome::Felled as FelledRules;
use super::scenery::{Scenery, Size, DRESSING_CELL_M, FLOATS};
use super::Ground;
use crate::records::{CAST_PASS, FOG_FACES, NONE, VIEW_PASS};
use crate::shadow::{CasterVolumes, CASCADES};
use crate::static_world::batches::{print, List, Population, PRINT_BASIS};
use crate::static_world::index::BoundsIndex;
use crate::static_world::walk::Record;
use crate::view::{caster_tier, View, TIERS};
use std::collections::{HashMap, HashSet};

mod felled;
use felled::FelledSpatial;
pub use felled::FelledTree;

const LAST: usize = TIERS - 1;
use super::sway::SWAY;

const CHUNK_M: f64 = 128.0;
/// A whole unfiltered chunk has an identity distinct from an instance.
const WHOLE: u64 = 1 << 63;
/// A crown's heart, where a whole tree probes fog, as a share of its height.
pub const HEART: f64 = 0.62;
/// The most cells of dressing drawn at once, and laid a walk.
const DRESSING_SLOTS: usize = 128;
const DRESSING_CELLS_PER_WALK: usize = 8;
/// A piece of dressing is gone at this share of the height it starts to
/// shrink at (the renderer's vertex stage shrinks it).
const DRESSING_GONE: f64 = 0.5;

/// What draws a scenery row (`Appearance.draws`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Draws {
    Forest = 1,
    Understorey = 2,
    Backdrop = 3,
    Dressing = 4,
    /// Two records an instance: the tree, then its fall.
    Felled = 5,
    Stump = 6,
}
pub const DRAWS: [Draws; 6] = [
    Draws::Forest,
    Draws::Understorey,
    Draws::Backdrop,
    Draws::Dressing,
    Draws::Felled,
    Draws::Stump,
];

/// What a tree kind lends its fall, unscaled: its trunk's radius at the
/// stump's top, and where its leaves begin.
#[derive(Clone, Copy, Default)]
pub struct Trunk {
    pub radius: f64,
    pub crown: f64,
}

/// The tier an instance `h` metres tall takes from `distance` metres.
fn tier_for(h: f64, distance: f64, view: &View, lod: &[f64; 3]) -> usize {
    let px = h * view.pixels_per_metre / distance.max(1e-3);
    lod.iter().position(|&t| px > t).unwrap_or(LAST)
}

/// A tree's leaf-noise offset by where it stands.
fn leaf_seed(x: f32, y: f32) -> f64 {
    (x as f64 * 0.618 + y as f64 * 0.382) % 97.0
}

/// A placed tree as an instance, its kind, height and reach.
fn instance(p: &[f32; FLOATS], sizes: &[Size]) -> (Record, usize, f32, f32) {
    let kind = p[6] as usize;
    let (sxy, sz) = (p[4], p[5]);
    let h = sizes[kind].height * sz as f64;
    let mut r = [0.0; 16];
    r[..4].copy_from_slice(&p[..4]);
    r[8..11].copy_from_slice(&p[7..10]);
    r[11] = leaf_seed(p[0], p[1]) as f32;
    r[12..16].copy_from_slice(&[sxy, sxy, sz, (HEART * h) as f32]);
    (r, kind, h as f32, (sizes[kind].radius * sxy as f64) as f32)
}

struct Chunk {
    bounds: [f64; 6],
    size: f64,
    start: Vec<usize>,
    end: Vec<usize>,
}

/// One population's instances in chunk order, and what the last walk drew
/// of them: per kind and tier its view and cast lists, with their prints.
struct Pop {
    kinds: usize,
    sorted: Vec<Vec<Record>>,
    heights: Vec<Vec<f32>>,
    /// Original placement indices, in the same immutable order as sorted.
    sources: Vec<Vec<usize>>,
    examined: usize,
    chunks: Vec<Chunk>,
    chunk_index: BoundsIndex,
    candidates: Vec<usize>,
    visited: usize,
    casts: bool,
    view: Vec<Vec<Record>>,
    cast: Vec<Vec<Record>>,
    prints: Vec<u64>,
    /// Counts rebuilds, so a list's print is new after one.
    generation: u64,
}

impl Pop {
    fn new(placed: &[f32], sizes: &[Size], kinds: usize, casts: bool, generation: u64) -> Pop {
        let instances: Vec<_> = placed
            .as_chunks::<FLOATS>()
            .0
            .iter()
            .map(|p| instance(p, sizes))
            .collect();
        let mut cells: HashMap<i64, Vec<usize>> = HashMap::new();
        for (i, (r, ..)) in instances.iter().enumerate() {
            let at = |v: f32| (v as f64 / CHUNK_M).floor() as i64;
            cells
                .entry((at(r[1]) + 4096) * 8192 + at(r[0]) + 4096)
                .or_default()
                .push(i);
        }
        let mut ids: Vec<i64> = cells.keys().copied().collect();
        ids.sort();
        let (mut sorted, mut heights) = (vec![vec![]; kinds], vec![vec![]; kinds]);
        let mut sources = vec![vec![]; kinds];
        let chunks: Vec<Chunk> = ids
            .iter()
            .map(|id| {
                let start: Vec<usize> = sorted.iter().map(Vec::len).collect();
                let mut bounds = [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ];
                let mut size: f64 = 0.0;
                for &i in &cells[id] {
                    let (r, kind, h, reach) = instances[i];
                    let [x, y, z, r_, h_] = [r[0], r[1], r[2], reach, h].map(f64::from);
                    // Its box takes in the farthest the wind can carry it.
                    let (r_, h_) = (r_ + SWAY.reach(h_), h_ + SWAY.reach(h_));
                    for (k, v) in [x - r_, y - r_, z].into_iter().enumerate() {
                        bounds[k] = bounds[k].min(v);
                        bounds[k + 3] = bounds[k + 3].max(v);
                    }
                    for (k, v) in [x + r_, y + r_, z + h_].into_iter().enumerate() {
                        bounds[k] = bounds[k].min(v);
                        bounds[k + 3] = bounds[k + 3].max(v);
                    }
                    size = size.max(h_);
                    sorted[kind].push(r);
                    sources[kind].push(i);
                    heights[kind].push(h);
                }
                Chunk {
                    bounds,
                    size,
                    start,
                    end: sorted.iter().map(Vec::len).collect(),
                }
            })
            .collect();
        Pop {
            kinds,
            sorted,
            heights,
            sources,
            examined: 0,
            chunk_index: BoundsIndex::new(chunks.iter().map(|c| c.bounds)),
            chunks,
            candidates: Vec::new(),
            visited: 0,
            casts,
            view: vec![vec![]; kinds * TIERS],
            cast: vec![vec![]; kinds * TIERS * CASCADES],
            prints: vec![0; kinds * TIERS * (1 + CASCADES)],
            generation,
        }
    }

    fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>, lod: &[f64; 3]) {
        self.select_kept(view, shadow, lod, None::<fn(usize) -> bool>);
    }

    fn select_kept(
        &mut self,
        view: &View,
        shadow: Option<&CasterVolumes>,
        lod: &[f64; 3],
        keep: Option<impl Fn(usize) -> bool>,
    ) {
        let examined = std::cell::Cell::new(0);
        let filtered = keep.is_some();
        let keep = |id| {
            keep.as_ref().is_none_or(|keep| {
                examined.set(examined.get() + 1);
                keep(id)
            })
        };
        let n = self.kinds * TIERS;
        self.view
            .iter_mut()
            .chain(&mut self.cast)
            .for_each(Vec::clear);
        self.prints.fill(print(PRINT_BASIS, self.generation));
        self.candidates.clear();
        self.visited = self.chunk_index.query(
            |b| view.sees_box(b) || (self.casts && shadow.is_some_and(|s| s.intersects(b))),
            |i| self.candidates.push(i),
        );
        self.candidates.sort_unstable();
        for &c in &self.candidates {
            let chunk = &self.chunks[c];
            let mask = if self.casts {
                shadow.map_or(0, |s| s.mask(&chunk.bounds))
            } else {
                0
            };
            let whole = |out: &mut [Vec<Record>], prints: &mut [u64], at: usize| {
                for k in 0..self.kinds {
                    let list = k * TIERS + LAST;
                    if !filtered {
                        out[list].extend_from_slice(&self.sorted[k][chunk.start[k]..chunk.end[k]]);
                        prints[at + list] = print(prints[at + list], WHOLE | c as u64);
                        continue;
                    }
                    for i in chunk.start[k]..chunk.end[k] {
                        if keep(self.sources[k][i]) {
                            out[list].push(self.sorted[k][i]);
                            prints[at + list] =
                                print(prints[at + list], ((c as u64) << 32) | i as u64);
                        }
                    }
                }
            };
            if !view.sees_box(&chunk.bounds) {
                for cascade in 0..CASCADES {
                    if mask & (1 << cascade) != 0 {
                        whole(
                            &mut self.cast[cascade * n..(cascade + 1) * n],
                            &mut self.prints,
                            (cascade + 1) * n,
                        );
                    }
                }
                continue;
            }
            let distance = view.distance_to_box(&chunk.bounds);
            if tier_for(chunk.size, distance, view, lod) == LAST {
                whole(&mut self.view, &mut self.prints, 0);
                for cascade in 0..CASCADES {
                    if mask & (1 << cascade) != 0 {
                        whole(
                            &mut self.cast[cascade * n..(cascade + 1) * n],
                            &mut self.prints,
                            (cascade + 1) * n,
                        );
                    }
                }
                continue;
            }
            for k in 0..self.kinds {
                for i in chunk.start[k]..chunk.end[k] {
                    if !keep(self.sources[k][i]) {
                        continue;
                    }
                    let r = &self.sorted[k][i];
                    let size = self.heights[k][i] as f64;
                    let centre = [r[0] as f64, r[1] as f64, r[2] as f64 + size / 2.0];
                    let d = (0..3)
                        .map(|a| (centre[a] - view.eye[a]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    let t = tier_for(size, d, view, lod);
                    let mark = ((c as u64) << 32) | i as u64;
                    self.view[k * TIERS + t].push(*r);
                    self.prints[k * TIERS + t] = print(self.prints[k * TIERS + t], mark);
                    for cascade in 0..CASCADES {
                        if mask & (1 << cascade) != 0 {
                            let m = cascade * n + k * TIERS + caster_tier(t);
                            self.cast[m].push(*r);
                            self.prints[n + m] = print(self.prints[n + m], mark);
                        }
                    }
                }
            }
        }
        self.examined = examined.get();
    }

    fn each_list(&self, rows: &[u32], visit: &mut dyn FnMut(List)) {
        let n = self.kinds * TIERS;
        for (pass, lists, at) in [(VIEW_PASS, &self.view, 0), (CAST_PASS, &self.cast, n)] {
            for (key, records) in lists.iter().enumerate() {
                let (row, tier) = (rows[(key % n) / TIERS], (key % TIERS) as u32);
                visit(List {
                    row,
                    tier,
                    pass,
                    caster: List::own_caster(pass, tier),
                    cascade_mask: if pass == CAST_PASS { 1 << (key / n) } else { 0 },
                    fog: FOG_FACES,
                    records,
                    print: self.prints[at + key],
                });
            }
        }
    }

    fn placed(&self) -> usize {
        self.sorted.iter().map(Vec::len).sum()
    }
}

/// A population on the simulation's ground, rebuilt when what stands of it
/// changes (a shrub on ground it saw cleared).
struct Standing {
    placed: Vec<f32>,
    kept: Option<Vec<usize>>,
    pop: Pop,
}

impl Standing {
    fn new(placed: &[f32], sizes: &[Size], kinds: usize) -> Standing {
        Standing {
            placed: placed.to_vec(),
            kept: None,
            pop: Pop::new(placed, sizes, kinds, true, 0),
        }
    }

    /// Stands it without what `gone` names (by its index); true when that changed.
    fn restand(&mut self, sizes: &[Size], gone: impl Fn(usize, &[f32; FLOATS]) -> bool) -> bool {
        let all = self.placed.as_chunks::<FLOATS>().0;
        let kept: Vec<usize> = (0..all.len()).filter(|&i| !gone(i, &all[i])).collect();
        let same = match &self.kept {
            Some(before) => *before == kept,
            None => kept.len() == all.len(),
        };
        if same {
            return false;
        }
        let left: Vec<f32> = kept.iter().flat_map(|&i| all[i]).collect();
        self.pop = Pop::new(&left, sizes, self.pop.kinds, true, self.pop.generation + 1);
        self.kept = Some(kept);
        true
    }
}

/// A cell of dressing: where it is, its box, and its pieces once laid, per
/// kind of the field's.
struct Cell {
    at: [i64; 2],
    bounds: [f64; 6],
    laid: Option<Vec<Vec<Record>>>,
    /// Laid and holds nothing: never wanted again.
    empty: bool,
}

/// The ground the side has seen cleared: cells `cell_m` metres a side,
/// `cols` across.
#[derive(Default)]
pub struct Cleared {
    cell_m: f64,
    cols: usize,
    rows: usize,
    cells: HashSet<usize>,
}

impl Cleared {
    /// From the learned ground's runs: tile and `start + length · 256`, a
    /// tile 16 cells a side, its cells row-major.
    pub fn from_runs(runs: &[u32], cols: usize, rows: usize, cell_m: f64) -> Cleared {
        let tiles_x = cols.div_ceil(16);
        let mut cells = HashSet::new();
        for r in runs.as_chunks::<2>().0 {
            let (tile, start, len) = (r[0] as usize, (r[1] % 256) as usize, (r[1] / 256) as usize);
            for local in start..start + len {
                let (i, j) = (
                    (tile % tiles_x) * 16 + local % 16,
                    (tile / tiles_x) * 16 + local / 16,
                );
                cells.insert(j * cols + i);
            }
        }
        Cleared {
            cell_m,
            cols,
            rows,
            cells,
        }
    }

    fn at(&self, x: f64, y: f64) -> bool {
        if self.cells.is_empty() {
            return false;
        }
        let (i, j) = ((x / self.cell_m).floor(), (y / self.cell_m).floor());
        i >= 0.0
            && j >= 0.0
            && (i as usize) < self.cols
            && (j as usize) < self.rows
            && self.cells.contains(&(j as usize * self.cols + i as usize))
    }
}

/// The player's settings for the scenery: the trees' tier thresholds over
/// the biome's, and whether the dressing and the tree lines' shrubs draw.
#[derive(Clone, Copy)]
pub struct Quality {
    pub tree_lod_scale: f64,
    pub dressing: bool,
    pub understorey: bool,
}

impl Default for Quality {
    fn default() -> Quality {
        Quality {
            tree_lod_scale: 1.0,
            dressing: true,
            understorey: true,
        }
    }
}

pub struct SceneryWalk {
    pub scenery: Scenery,
    sizes: Vec<Size>,
    trunks: Vec<Trunk>,
    /// Per placed forest tree: its trunk's prop id.
    index: HashMap<u32, usize>,
    forest: Pop,
    understorey: Standing,
    backdrop: Pop,
    cells: Vec<Cell>,
    cell_index: HashMap<[i64; 2], usize>,
    /// The cells the last walk drew, nearest first, and their tiers.
    drawn: Vec<(usize, usize)>,
    dressing: Vec<Vec<Record>>,
    dressing_prints: Vec<u64>,
    /// Counts re-lays (cleared ground changed), so the prints are new.
    dressing_generation: u64,
    /// Wanted cells are still to be laid: walk again.
    pub pending: bool,
    felled: Vec<FelledTree>,
    felled_spatial: FelledSpatial,
    felled_lists: Vec<Vec<Record>>,
    stumps: Vec<Vec<Record>>,
    felled_print: u64,
    /// A tree still falls: the next frame walks again.
    pub falling: bool,
    clock: f64,
    cleared: Cleared,
    quality: Quality,
    /// Per `DRAWS` and kind: the `World.appearances` row.
    pub rows: Vec<Vec<u32>>,
    /// Per kind: a tree's stump mesh (`kinds::stump_mesh`).
    pub stump_meshes: Vec<Option<Vec<f32>>>,
}

impl SceneryWalk {
    /// The walk of `scenery`, its kinds `sizes` big with `trunks`.
    pub fn new(scenery: Scenery, sizes: Vec<Size>, trunks: Vec<Trunk>) -> SceneryWalk {
        let kinds = scenery.kinds.len();
        let backdrop = Pop::new(&scenery.backdrop, &sizes, kinds, false, 0);
        let m = DRESSING_CELL_M;
        let (past, d) = (scenery.dressing.reach_m, &scenery.dressing);
        let cells: Vec<Cell> = d
            .cells
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[i, j]| {
                let (i, j) = (i as i64, j as i64);
                Cell {
                    at: [i, j],
                    bounds: [
                        i as f64 * m - past,
                        j as f64 * m - past,
                        d.ground[0],
                        (i + 1) as f64 * m + past,
                        (j + 1) as f64 * m + past,
                        d.ground[1] + d.top_m,
                    ],
                    laid: None,
                    empty: false,
                }
            })
            .collect();
        SceneryWalk {
            index: scenery
                .forest_ids
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, i))
                .collect(),
            forest: Pop::new(&scenery.forest, &sizes, kinds, true, 0),
            understorey: Standing::new(&scenery.understorey, &sizes, kinds),
            backdrop,
            cell_index: cells.iter().enumerate().map(|(n, c)| (c.at, n)).collect(),
            cells,
            drawn: vec![],
            dressing: vec![vec![]; kinds * TIERS],
            dressing_prints: vec![0; kinds * TIERS],
            dressing_generation: 0,
            pending: false,
            felled: vec![],
            felled_spatial: FelledSpatial::default(),
            felled_lists: vec![vec![]; kinds * TIERS * (1 + CASCADES)],
            stumps: vec![vec![]; kinds * (1 + CASCADES)],
            felled_print: 0,
            falling: false,
            clock: 0.0,
            cleared: Cleared::default(),
            quality: Quality::default(),
            rows: vec![],
            stump_meshes: vec![],
            scenery,
            sizes,
            trunks,
        }
    }

    pub fn set_quality(&mut self, quality: Quality) {
        self.quality = quality;
    }

    /// Follows the ground the side has seen cleared: the shrubs and dressing
    /// standing on it leave.
    pub fn set_cleared(&mut self, cleared: Cleared) {
        self.cleared = cleared;
        let c = &self.cleared;
        self.understorey
            .restand(&self.sizes, |_, p| c.at(p[0] as f64, p[1] as f64));
        for cell in &mut self.cells {
            (cell.laid, cell.empty) = (None, false);
        }
        self.dressing_generation += 1;
    }

    /// Reconcile the side's full snapshot (one fall per prop), O(F), including
    /// equality/replacement. Geometry and hierarchy are independent of knowledge.
    pub fn set_felled(&mut self, felled: Vec<FelledTree>) -> bool {
        if felled == self.felled {
            return false;
        }
        let spatial = &mut self.felled_spatial;
        spatial.known.clear();
        spatial.latest = None;
        for (id, f) in felled.iter().enumerate() {
            spatial.reconciled += 1;
            let Some(&source) = self.index.get(&f.prop) else {
                continue;
            };
            spatial.known.insert(source, id);
            spatial.latest = Some(spatial.latest.map_or(f.fell_at, |v| v.max(f.fell_at)));
        }
        self.felled = felled;
        true
    }

    /// The presentation clock; true when a tree still falls on it.
    pub fn set_clock(&mut self, clock: f64) -> bool {
        let moved = clock != self.clock && self.falling;
        self.clock = clock;
        moved
    }

    fn select(&mut self, ground: &Ground, view: &View, shadow: Option<&CasterVolumes>) {
        let lod = ground
            .biome
            .trees
            .lod_px
            .map(|v| v * self.quality.tree_lod_scale);
        self.prepare_felled(&ground.biome.trees.felled);
        let known = &self.felled_spatial.known;
        self.forest
            .select_kept(view, shadow, &lod, Some(|id| !known.contains_key(&id)));
        if self.quality.understorey {
            self.understorey.pop.select(view, shadow, &lod);
        } else {
            self.understorey.pop.visited = 0;
        }
        self.backdrop.select(view, None, &lod);
        self.select_dressing(ground, view);
        self.select_felled(ground, view, shadow, &lod);
    }

    fn select_dressing(&mut self, ground: &Ground, view: &View) {
        self.dressing.iter_mut().for_each(Vec::clear);
        self.dressing_prints
            .fill(print(PRINT_BASIS, self.dressing_generation));
        self.pending = false;
        self.drawn.clear();
        if !self.quality.dressing || self.cells.is_empty() {
            return;
        }
        let rules = &ground.biome.forest_floor.dressing;
        let (top_m, kinds) = (
            self.scenery.dressing.top_m,
            self.scenery.dressing.kinds.clone(),
        );
        let reach = top_m * view.pixels_per_metre / (rules.fade_px * DRESSING_GONE);
        let m = DRESSING_CELL_M;
        let range =
            |e: f64| (((e - reach) / m).floor() as i64)..=(((e + reach) / m).floor() as i64);
        let mut wanted = vec![];
        for j in range(view.eye[1]) {
            for i in range(view.eye[0]) {
                let Some(&n) = self.cell_index.get(&[i, j]) else {
                    continue;
                };
                let cell = &self.cells[n];
                if cell.empty || !view.sees_box(&cell.bounds) {
                    continue;
                }
                let distance = view.distance_to_box(&cell.bounds);
                if distance < reach {
                    wanted.push((distance, n));
                }
            }
        }
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
        let slots = DRESSING_SLOTS.min(self.cells.len());
        let mut budget = DRESSING_CELLS_PER_WALK;
        let lod = rules.lod_px;
        let mut held = 0;
        for (distance, n) in wanted {
            if held == slots {
                break;
            }
            if self.cells[n].laid.is_none() {
                if budget == 0 {
                    self.pending = true;
                    continue;
                }
                budget -= 1;
                self.lay(ground, n);
                if self.cells[n].empty {
                    continue;
                }
            }
            held += 1;
            let tier = tier_for(top_m, distance, view, &lod);
            self.drawn.push((n, tier));
            for (k, pieces) in self.cells[n].laid.as_ref().unwrap().iter().enumerate() {
                let list = kinds[k] * TIERS + tier;
                self.dressing[list].extend_from_slice(pieces);
                self.dressing_prints[list] = print(self.dressing_prints[list], n as u64);
            }
        }
        // Keep what the view may come back to, and no more than it can draw.
        if self.cells.iter().filter(|c| c.laid.is_some()).count() > 4 * slots {
            let keep: HashSet<usize> = self.drawn.iter().map(|&(n, _)| n).collect();
            for (n, cell) in self.cells.iter_mut().enumerate() {
                if !keep.contains(&n) && !cell.empty {
                    cell.laid = None;
                }
            }
        }
    }

    /// Lays cell `n`'s pieces, kind by kind, leaving out those on cleared ground.
    fn lay(&mut self, ground: &Ground, n: usize) {
        let [i, j] = self.cells[n].at;
        let placed = ground.dress(&self.scenery, i, j);
        let d = &self.scenery.dressing;
        let fade = ground.biome.forest_floor.dressing.fade_px;
        let mut laid = vec![vec![]; d.kinds.len()];
        for p in placed.as_chunks::<FLOATS>().0 {
            let (mut r, kind, h, _) = instance(p, &self.sizes);
            if self.cleared.at(p[0] as f64, p[1] as f64) {
                continue;
            }
            r[11] = (h as f64 / fade) as f32;
            laid[d.kinds.iter().position(|&k| k == kind).unwrap()].push(r);
        }
        let cell = &mut self.cells[n];
        cell.empty = placed.is_empty();
        cell.laid = Some(laid);
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        let row = |d: Draws| &self.rows[d as usize - 1];
        self.forest.each_list(row(Draws::Forest), visit);
        if self.quality.understorey {
            self.understorey
                .pop
                .each_list(row(Draws::Understorey), visit);
        }
        self.backdrop.each_list(row(Draws::Backdrop), visit);
        let kinds = self.scenery.kinds.len();
        for (key, records) in self.dressing.iter().enumerate() {
            let print = self.dressing_prints[key];
            // The forest floor's dressing casts nothing.
            visit(List {
                row: row(Draws::Dressing)[key / TIERS],
                tier: (key % TIERS) as u32,
                pass: VIEW_PASS,
                caster: NONE,
                cascade_mask: 0,
                fog: FOG_FACES,
                records,
                print,
            });
        }
        for (key, records) in self.felled_lists.iter().enumerate() {
            let (pass, key) = if key < kinds * TIERS {
                (VIEW_PASS, key)
            } else {
                (CAST_PASS, key - kinds * TIERS)
            };
            let tier = (key % TIERS) as u32;
            visit(List {
                row: row(Draws::Felled)[(key / TIERS) % kinds],
                tier,
                pass,
                caster: List::own_caster(pass, tier),
                cascade_mask: if pass == CAST_PASS {
                    1 << (key / (kinds * TIERS))
                } else {
                    0
                },
                fog: FOG_FACES,
                records,
                print: self.felled_print,
            });
        }
        for kind in 0..kinds {
            visit(List {
                row: row(Draws::Stump)[kind],
                tier: 0,
                pass: VIEW_PASS,
                caster: NONE,
                cascade_mask: 0,
                fog: FOG_FACES,
                records: &self.stumps[kind],
                print: self.felled_print,
            });
            for cascade in 0..CASCADES {
                visit(List {
                    row: row(Draws::Stump)[kind],
                    tier: 0,
                    pass: CAST_PASS,
                    caster: List::own_caster(CAST_PASS, 0),
                    cascade_mask: 1 << cascade,
                    fog: FOG_FACES,
                    records: &self.stumps[(cascade + 1) * kinds + kind],
                    print: self.felled_print,
                });
            }
        }
    }

    /// Instances placed per population (forest, understorey, backdrop) and
    /// dressing cells drawn, bounds visited, then `felled_work` counters.
    pub fn stats(&self) -> [usize; 8] {
        let work = self.felled_work();
        [
            self.forest.placed() - self.felled_spatial.known.len(),
            self.understorey.pop.placed(),
            self.backdrop.placed(),
            self.drawn.len(),
            self.forest.visited
                + self.understorey.pop.visited
                + self.backdrop.visited
                + self.felled_spatial.visited,
            work[0],
            work[1],
            work[2],
        ]
    }
}

/// The scenery as a frame's static population: its walk, over its ground.
pub struct SceneryPopulation<'a> {
    pub walk: &'a mut SceneryWalk,
    pub ground: &'a Ground,
}

impl Population for SceneryPopulation<'_> {
    fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>) -> bool {
        self.walk.select(self.ground, view, shadow);
        true
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        self.walk.each_list(visit);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod spatial_tests;
