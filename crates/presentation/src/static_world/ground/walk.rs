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
use crate::static_world::batches::{print, List, Population, CAST_PASS, VIEW_PASS};
use crate::static_world::walk::{casts_into, Record, Shadow};
use crate::view::View;
use std::collections::{HashMap, HashSet};
use std::f64::consts::FRAC_PI_2;

pub const TIERS: usize = 4;
const LAST: usize = TIERS - 1;
const CHUNK_M: f64 = 128.0;
/// A crown's heart, where a whole tree probes fog, as a share of its height.
const HEART: f64 = 0.62;
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
    chunks: Vec<Chunk>,
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
        let chunks = ids
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
            chunks,
            casts,
            view: vec![vec![]; kinds * TIERS],
            cast: vec![vec![]; kinds * TIERS],
            prints: vec![0; kinds * TIERS * 2],
            generation,
        }
    }

    fn select(&mut self, view: &View, shadow: Option<&Shadow>, lod: &[f64; 3]) {
        let n = self.kinds * TIERS;
        self.view
            .iter_mut()
            .chain(&mut self.cast)
            .for_each(Vec::clear);
        self.prints.fill(self.generation);
        let shadow = shadow.filter(|_| self.casts);
        for (c, chunk) in self.chunks.iter().enumerate() {
            let whole = |out: &mut Vec<Vec<Record>>, prints: &mut [u64], at: usize| {
                for k in 0..self.kinds {
                    let list = k * TIERS + LAST;
                    out[list].extend_from_slice(&self.sorted[k][chunk.start[k]..chunk.end[k]]);
                    prints[at + list] = print(prints[at + list], c as u64);
                }
            };
            if !view.sees_box(&chunk.bounds) {
                if casts_into(view, shadow, &chunk.bounds, chunk.size) {
                    whole(&mut self.cast, &mut self.prints, n);
                }
                continue;
            }
            let distance = view.distance_to_box(&chunk.bounds);
            if tier_for(chunk.size, distance, view, lod) == LAST {
                whole(&mut self.view, &mut self.prints, 0);
                if shadow.is_some_and(|s| distance <= s.reach) {
                    whole(&mut self.cast, &mut self.prints, n);
                }
                continue;
            }
            for k in 0..self.kinds {
                for i in chunk.start[k]..chunk.end[k] {
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
                    if self.casts {
                        let m = k * TIERS + (t + 1).min(LAST);
                        self.cast[m].push(*r);
                        self.prints[n + m] = print(self.prints[n + m], mark);
                    }
                }
            }
        }
    }

    fn each_list(&self, rows: &[u32], visit: &mut dyn FnMut(List)) {
        let n = self.kinds * TIERS;
        for (pass, lists, at) in [(VIEW_PASS, &self.view, 0), (CAST_PASS, &self.cast, n)] {
            for (key, records) in lists.iter().enumerate() {
                let (row, tier) = (rows[key / TIERS], (key % TIERS) as u32);
                visit(List {
                    row,
                    tier,
                    pass,
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
/// changes (a tree the side saw fall, a shrub on ground it saw cleared).
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

/// A tree the side saw fall: its trunk's prop, the way it fell, and the
/// presentation second it began to.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FelledTree {
    pub prop: u32,
    pub toward: [f64; 2],
    pub fell_at: f64,
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
    forest: Standing,
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
            forest: Standing::new(&scenery.forest, &sizes, kinds),
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
            felled_lists: vec![vec![]; kinds * TIERS * 2],
            stumps: vec![vec![]; kinds],
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

    /// Follows the trees the side saw fall; true when that changed.
    pub fn set_felled(&mut self, felled: Vec<FelledTree>) -> bool {
        if felled == self.felled {
            return false;
        }
        let ids: HashSet<u32> = felled.iter().map(|f| f.prop).collect();
        let forest_ids = &self.scenery.forest_ids;
        self.forest
            .restand(&self.sizes, |i, _| ids.contains(&forest_ids[i]));
        self.felled = felled;
        true
    }

    /// The presentation clock; true when a tree still falls on it.
    pub fn set_clock(&mut self, clock: f64) -> bool {
        let moved = clock != self.clock && self.falling;
        self.clock = clock;
        moved
    }

    fn select(&mut self, ground: &Ground, view: &View, shadow: Option<&Shadow>) {
        let lod = ground
            .biome
            .trees
            .lod_px
            .map(|v| v * self.quality.tree_lod_scale);
        self.forest.pop.select(view, shadow, &lod);
        if self.quality.understorey {
            self.understorey.pop.select(view, shadow, &lod);
        }
        self.backdrop.select(view, None, &lod);
        self.select_dressing(ground, view);
        self.select_felled(ground, view, &lod);
    }

    fn select_dressing(&mut self, ground: &Ground, view: &View) {
        self.dressing.iter_mut().for_each(Vec::clear);
        self.dressing_prints.fill(self.dressing_generation);
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

    fn select_felled(&mut self, ground: &Ground, view: &View, lod: &[f64; 3]) {
        // What the last walk listed: a view that moves a tree to another
        // tier writes its lists anew.
        let before = std::mem::take(&mut self.felled_lists);
        self.felled_lists = vec![vec![]; before.len()];
        self.stumps.iter_mut().for_each(Vec::clear);
        self.falling = false;
        let rules = &ground.biome.trees.felled;
        let forest = self.scenery.forest.as_chunks::<FLOATS>().0;
        for f in &self.felled {
            let Some(&i) = self.index.get(&f.prop) else {
                continue;
            };
            let p = &forest[i];
            let (tree, kind, h, reach) = instance(p, &self.sizes);
            let (sxy, sz) = (p[4] as f64, p[5] as f64);
            let age = self.clock - f.fell_at;
            self.falling |= age < rules.fall_s + rules.settle_s;
            let t = &self.trunks[kind];
            let pose = fell_pose(
                rules,
                age,
                h as f64,
                reach as f64,
                t.radius * sxy,
                t.crown * sz,
            );
            let mut fall = tree;
            fall[4..8]
                .copy_from_slice(&[f.toward[0], f.toward[1], pose[0], pose[1]].map(|v| v as f32));
            let mut rest = [0.0; 16];
            for (r, v) in rest.iter_mut().zip(&pose[2..]) {
                *r = *v as f32;
            }
            let d = crate::world::length([0, 1, 2].map(|a| tree[a] as f64 - view.eye[a]));
            let list = &mut self.felled_lists[kind * TIERS + tier_for(h as f64, d, view, lod)];
            list.extend([fall, rest]);
            let mut stump = tree;
            stump[11] = 0.0;
            stump[14..16].copy_from_slice(&[1.0, (0.5 * rules.stump_height_m) as f32]);
            self.stumps[kind].push(stump);
        }
        // Casters draw a tier coarser.
        let kinds = self.scenery.kinds.len();
        for k in 0..kinds {
            for t in 0..TIERS {
                let view_list = self.felled_lists[k * TIERS + t].clone();
                self.felled_lists[kinds * TIERS + k * TIERS + (t + 1).min(LAST)].extend(view_list);
            }
        }
        if self.felled_lists != before {
            self.felled_print += 1;
        }
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        let row = |d: Draws| &self.rows[d as usize - 1];
        self.forest.pop.each_list(row(Draws::Forest), visit);
        if self.quality.understorey {
            self.understorey
                .pop
                .each_list(row(Draws::Understorey), visit);
        }
        self.backdrop.each_list(row(Draws::Backdrop), visit);
        let kinds = self.scenery.kinds.len();
        for (key, records) in self.dressing.iter().enumerate() {
            let print = self.dressing_prints[key];
            visit(List {
                row: row(Draws::Dressing)[key / TIERS],
                tier: (key % TIERS) as u32,
                pass: VIEW_PASS,
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
            visit(List {
                row: row(Draws::Felled)[key / TIERS],
                tier: (key % TIERS) as u32,
                pass,
                records,
                print: self.felled_print,
            });
        }
        for (kind, records) in self.stumps.iter().enumerate() {
            for pass in [VIEW_PASS, CAST_PASS] {
                visit(List {
                    row: row(Draws::Stump)[kind],
                    tier: 0,
                    pass,
                    records,
                    print: self.felled_print,
                });
            }
        }
    }

    /// Instances placed per population (forest, understorey, backdrop) and
    /// dressing cells drawn by the last walk.
    pub fn stats(&self) -> [usize; 4] {
        [
            self.forest.pop.placed(),
            self.understorey.pop.placed(),
            self.backdrop.placed(),
            self.drawn.len(),
        ]
    }
}

/// A felled tree's pose `age` seconds after it began to fall, `height` tall
/// with a crown `reach` round its trunk, trunk radius `trunk` and crown base
/// `crown` (scaled): its tip angle and hinge, then the press up-and-down and
/// across of what lies past its trunk, the trunk, how far it has landed and
/// the crown base.
fn fell_pose(
    rules: &FelledRules,
    age: f64,
    height: f64,
    reach: f64,
    trunk: f64,
    crown: f64,
) -> [f64; 7] {
    let hinge = rules.stump_height_m;
    let length = height - hinge;
    let rest = FRAC_PI_2 + libm::asin((trunk / length.max(1e-3)).min(1.0));
    let angle = if age <= 0.0 {
        0.0
    } else if age < rules.fall_s {
        rest * (age / rules.fall_s).powi(2)
    } else {
        let u = if rules.settle_s > 0.0 {
            (age - rules.fall_s) / rules.settle_s
        } else {
            1.0
        };
        if u >= 1.0 {
            rest
        } else {
            rest - rules.settle_deg * std::f64::consts::PI / 180.0
                * libm::sin(std::f64::consts::PI * u)
                * (1.0 - u)
        }
    };
    let u = ((angle / rest - 0.5) / 0.5).clamp(0.0, 1.0);
    let landed = u * u * (3.0 - 2.0 * u);
    let axis = trunk * (1.0 - ((crown - hinge) / length).clamp(0.0, 1.0));
    let squash = ((rules.rest_height_m - axis) / reach.max(1e-3)).clamp(0.02, 1.0);
    [
        angle,
        hinge,
        1.0 + (squash - 1.0) * landed,
        1.0 + (rules.rest_spread - 1.0) * landed,
        trunk,
        landed,
        crown,
    ]
}

/// The scenery as a frame's static population: its walk, over its ground.
pub struct SceneryPopulation<'a> {
    pub walk: &'a mut SceneryWalk,
    pub ground: &'a Ground,
}

impl Population for SceneryPopulation<'_> {
    fn select(&mut self, view: &View, shadow: Option<&Shadow>) -> bool {
        self.walk.select(self.ground, view, shadow);
        true
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        self.walk.each_list(visit);
    }
}

#[cfg(test)]
mod tests {
    use super::{fell_pose, FelledRules};

    fn rules() -> FelledRules {
        serde_json::from_value(serde_json::json!({
            "fall_s": 2, "settle_s": 0.5, "settle_deg": 5, "rest_height_m": 1,
            "rest_spread": 0.6, "stump_height_m": 0.4, "cut": [0.4, 0.3, 0.2]
        }))
        .unwrap()
    }

    /// A tree 11 m tall, its crown 4.2 m round from 4 m up, its trunk 0.45 m.
    fn pose(age: f64) -> [f64; 7] {
        fell_pose(&rules(), age, 11.0, 4.2, 0.45, 4.0)
    }

    #[test]
    fn a_tree_tips_over_gathering_speed_strikes_at_fall_s_rocks_and_lies() {
        let r = rules();
        let rest = pose(1000.0)[0];
        assert!(rest > std::f64::consts::FRAC_PI_2);
        assert_eq!([pose(-1.0)[0], pose(0.0)[0]], [0.0, 0.0]);
        let (mut last, mut step) = (0.0, 0.0);
        for k in 1..=20 {
            let angle = pose(k as f64 / 20.0 * r.fall_s)[0];
            assert!(angle - last > step - 1e-12, "it never slows while it falls");
            (step, last) = (angle - last, angle);
        }
        assert!((pose(r.fall_s)[0] - rest).abs() < 1e-12);
        let rock = r.settle_deg.to_radians();
        for k in 0..50 {
            let angle = pose(r.fall_s + k as f64 / 50.0 * r.settle_s)[0];
            assert!(
                angle <= rest && angle > rest - rock - 1e-9,
                "it rocks back no more than settle_deg"
            );
        }
        assert_eq!(pose(r.fall_s + r.settle_s)[0], rest);
    }

    #[test]
    fn a_tree_lands_in_its_second_half_smoothly_and_lies_pressed_and_narrowed() {
        let r = rules();
        let landed = |age: f64| pose(age)[5];
        assert_eq!(landed(0.0), 0.0);
        // Half way over it has not begun to land.
        assert_eq!(landed(r.fall_s * 0.5f64.sqrt() * 0.99), 0.0);
        let mut last = 0.0;
        for k in 0..=40 {
            let l = landed(k as f64 / 40.0 * r.fall_s);
            assert!(l >= last && l - last < 0.2, "it lands smoothly");
            last = l;
        }
        let [_, hinge, squash, spread, trunk, landed, crown] = pose(1000.0);
        assert_eq!(
            [hinge, trunk, landed, crown],
            [r.stump_height_m, 0.45, 1.0, 4.0]
        );
        // Lying, its crown narrows to rest_spread, and is pressed so that its
        // farthest leaf above its axis keeps under rest_height_m.
        assert!((spread - r.rest_spread).abs() < 1e-12);
        let axis = trunk * (1.0 - (crown - hinge) / (11.0 - hinge));
        assert!(axis + squash * 4.2 <= r.rest_height_m + 1e-9);
        // Upright, nothing is pressed.
        let upright = pose(0.0);
        assert_eq!([upright[2], upright[3]], [1.0, 1.0]);
    }
}
