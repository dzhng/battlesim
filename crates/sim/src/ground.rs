//! The ground layer (D2, Q8): what fire and movement leave on the ground, as
//! authoritative state in fixed cells of `ground.cell_m`. Craters are the one
//! channel with a rule of its own here: infantry in one has light cover (`cover`), and a
//! vehicle driving over one is slowed slightly, in movement integration
//! only. Navigation never reads
//! craters, so a crater never rebuilds anyone's plan (landmine 3). Scorch,
//! track wear and trampling are recorded and change nothing. `cleared` marks
//! the crushed lane a vehicle knocked through trees (Q16): the world's
//! cleared mask is what forest queries read; this is its mark, learned by
//! sight and drawn.
//!
//! Bounded: cells live in tiles allocated on first mark, so storage grows with
//! the ground touched and never past the map's area. Every channel is a byte
//! that accumulates and saturates; nothing decays.
//!
//! Each side learns the layer by sight into its own [`KnownGround`] (owned by
//! its knowledge): a cell counts as seen when the fog cell holding its centre
//! is seen, at that side's fog sweep. Delivery reads the learned cells by
//! revision, never this layer.
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::cell_page::Page;

use contract::observation::VisibilityField;
use contract::scenario::{GroundRules, Rules};

use crate::digest::Digest;
use crate::math::{v2, V2, V3};
use crate::world::{SurfaceKind, WorldGeometry};

/// Cells per tile edge.
const TILE: usize = 16;
const TILE_CELLS: usize = TILE * TILE;

/// One cell's marks, each in [0, 255].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroundCell {
    pub crater: u8,
    pub scorch: u8,
    pub tracks: u8,
    pub trampled: u8,
    /// 255 where a vehicle knocked its way through trees, else 0.
    pub cleared: u8,
}

impl GroundCell {
    fn word(self) -> u64 {
        u64::from_le_bytes([
            self.crater,
            self.scorch,
            self.tracks,
            self.trampled,
            self.cleared,
            0,
            0,
            0,
        ])
    }
}

/// A cosmetic wear channel movement writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wear {
    Tracks,
    Trampled,
}

struct Tile {
    cells: Arc<Page<GroundCell>>,
    /// Digest of `cells`, refreshed by [`GroundLayer::seal`].
    hash: u64,
    /// Marked since the last seal.
    dirty: bool,
    /// The layer's edit count at this tile's latest change.
    edit: u64,
}

pub struct GroundLayer {
    cell_m: f64,
    cols: usize,
    rows: usize,
    tiles_x: usize,
    tiles: BTreeMap<usize, Box<Tile>>,
    /// Tiles marked since the last seal, in first-mark order.
    dirty: Vec<usize>,
    /// Cell changes so far: learning skips ground unedited since it last looked.
    edits: Option<Edits>,
}

/// Truth keeps one current entry per edited tile, not an unbounded edit journal.
/// A learned page store uses the same representation without truth edit tracking.
struct Edits {
    revision: u64,
    tiles: BTreeMap<u64, usize>,
}

/// Reject ground rules that would break the contract: craters are never
/// impassable.
pub fn validate(rules: &Rules) {
    let g = &rules.ground;
    assert!(g.cell_m > 0.0, "ground.cell_m must be positive");
    assert!(
        g.crater_full_depth > 0.0 && g.crater_full_depth <= u8::MAX as f64,
        "ground.crater_full_depth must be in (0, 255]"
    );
    assert!(
        g.crater_vehicle_mult > 0.0 && g.crater_vehicle_mult <= 1.0,
        "ground.crater_vehicle_mult must be in (0, 1]: craters are never impassable"
    );
    assert!(
        (0.0..=1.0).contains(&g.track_gauge),
        "ground.track_gauge is a share of the half width, in [0, 1]"
    );
    assert!(
        g.lane_margin_m >= 0.0,
        "ground.lane_margin_m must not be negative"
    );
}

impl GroundLayer {
    /// An unmarked layer over a map `width` × `depth` metres from the origin.
    pub fn new(width: f64, depth: f64, rules: &GroundRules) -> Self {
        let cell_m = rules.cell_m;
        let cols = (width / cell_m).ceil().max(1.0) as usize;
        let rows = (depth / cell_m).ceil().max(1.0) as usize;
        let tiles_x = cols.div_ceil(TILE);
        GroundLayer {
            cell_m,
            cols,
            rows,
            tiles_x,
            tiles: BTreeMap::new(),
            dirty: Vec::new(),
            edits: Some(Edits {
                revision: 0,
                tiles: BTreeMap::new(),
            }),
        }
    }

    /// An unmarked layer over the same grid.
    fn blank(&self) -> Self {
        GroundLayer {
            cell_m: self.cell_m,
            cols: self.cols,
            rows: self.rows,
            tiles_x: self.tiles_x,
            tiles: BTreeMap::new(),
            dirty: Vec::new(),
            edits: None,
        }
    }

    /// Cells across (x) and down (y).
    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cell_m(&self) -> f64 {
        self.cell_m
    }

    fn index(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let (i, j) = ((x / self.cell_m).floor(), (y / self.cell_m).floor());
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.cols && (j as usize) < self.rows)
            .then_some((i as usize, j as usize))
    }

    fn slot(&self, i: usize, j: usize) -> (usize, usize) {
        (
            (j / TILE) * self.tiles_x + i / TILE,
            (j % TILE) * TILE + i % TILE,
        )
    }

    /// The marks of the cell holding (x, y); an unmarked or off-map cell is blank.
    pub fn cell(&self, x: f64, y: f64) -> GroundCell {
        self.index(x, y).map_or_else(GroundCell::default, |(i, j)| {
            let (t, c) = self.slot(i, j);
            self.cell_at(t, c)
        })
    }

    fn mark(&mut self, i: usize, j: usize, f: impl FnOnce(&mut GroundCell)) {
        let (t, c) = self.slot(i, j);
        let mut cell = self.cell_at(t, c);
        f(&mut cell);
        self.put(t, c, cell);
    }

    /// Set cell `c` of tile `t`; returns whether it changed.
    fn put(&mut self, t: usize, c: usize, cell: GroundCell) -> bool {
        if cell == self.cell_at(t, c) {
            return false;
        }
        let tile = self.tiles.entry(t).or_insert_with(|| {
            Box::new(Tile {
                cells: Arc::new(Page::default()),
                hash: 0,
                dirty: false,
                edit: 0,
            })
        });
        Arc::make_mut(&mut tile.cells).set(c, cell);
        if let Some(edits) = &mut self.edits {
            edits.revision += 1;
            if !tile.dirty {
                edits.tiles.remove(&tile.edit);
            }
            tile.edit = edits.revision;
        }
        if !tile.dirty {
            tile.dirty = true;
            self.dirty.push(t);
        }
        true
    }

    fn changed_tiles_since(&self, base: u64) -> impl Iterator<Item = (usize, &Tile)> {
        self.edits
            .as_ref()
            .into_iter()
            .flat_map(move |edits| {
                edits
                    .tiles
                    .range((std::ops::Bound::Excluded(base), std::ops::Bound::Unbounded))
                    .map(|(_, &tile)| (tile, &*self.tiles[&tile]))
            })
            .chain(self.dirty.iter().filter_map(move |&tile| {
                let value = &*self.tiles[&tile];
                (value.edit > base).then_some((tile, value))
            }))
    }

    fn cell_at(&self, t: usize, c: usize) -> GroundCell {
        self.tiles
            .get(&t)
            .map_or_else(GroundCell::default, |tile| tile.cells.get(c))
    }

    /// How full the crater at (x, y) is, in [0, 1].
    pub fn crater_fill(&self, x: f64, y: f64, rules: &GroundRules) -> f64 {
        (self.cell(x, y).crater as f64 / rules.crater_full_depth).min(1.0)
    }

    /// Speed multiplier for a vehicle whose centre is over (x, y).
    pub fn vehicle_speed(&self, x: f64, y: f64, rules: &GroundRules) -> f64 {
        1.0 - (1.0 - rules.crater_vehicle_mult) * self.crater_fill(x, y, rules)
    }

    /// A round of blast radius `blast_radius` bursts at `at`: it digs a crater
    /// if it bursts within the crater's radius of the ground, and scorches the
    /// ground within the scorch radius likewise. Water takes no marks.
    pub fn burst(&mut self, world: &WorldGeometry, at: V3, blast_radius: f64, rules: &GroundRules) {
        let Some(ground) = world.ground_surface_at(at.x, at.y) else {
            return;
        };
        if ground.kind == SurfaceKind::Water {
            return;
        }
        let height = at.z - ground.z;
        let crater = blast_radius * rules.crater_radius_fraction;
        if height <= crater {
            let depth = rules.crater_depth_per_m * crater;
            self.stamp(at.xy(), crater, depth, |c, v| c.crater = add(c.crater, v));
        }
        let scorch = blast_radius * rules.scorch_radius_fraction;
        if height <= scorch {
            self.stamp(at.xy(), scorch, rules.scorch_per_burst, |c, v| {
                c.scorch = add(c.scorch, v)
            });
        }
    }

    /// Add `amount` at `center`, falling linearly to nothing at `radius`
    /// (by cell centre); the cell holding the centre always takes it whole.
    fn stamp(&mut self, center: V2, radius: f64, amount: f64, f: impl Fn(&mut GroundCell, f64)) {
        if amount <= 0.0 {
            return;
        }
        let Some((ci, cj)) = self.index(center.x, center.y) else {
            return;
        };
        let reach = (radius / self.cell_m).ceil() as isize;
        for dj in -reach..=reach {
            for di in -reach..=reach {
                let (i, j) = (ci as isize + di, cj as isize + dj);
                if i < 0 || j < 0 || i as usize >= self.cols || j as usize >= self.rows {
                    continue;
                }
                let (i, j) = (i as usize, j as usize);
                let mid = v2(
                    (i as f64 + 0.5) * self.cell_m,
                    (j as f64 + 0.5) * self.cell_m,
                );
                let d = if (di, dj) == (0, 0) {
                    0.0
                } else {
                    (mid - center).length()
                };
                if d < radius || d == 0.0 {
                    let v = amount * (1.0 - d / radius.max(1e-9));
                    self.mark(i, j, |c| f(c, v));
                }
            }
        }
    }

    /// A track or a soldier moved from `from` to `to`: entering a new cell
    /// adds one pass of wear there.
    pub fn wear(&mut self, from: V2, to: V2, channel: Wear, rules: &GroundRules) {
        let amount = match channel {
            Wear::Tracks => rules.tracks_per_pass,
            Wear::Trampled => rules.trampled_per_pass,
        };
        if amount <= 0.0 {
            return;
        }
        let Some((i, j)) = self.index(to.x, to.y) else {
            return;
        };
        if self.index(from.x, from.y) == Some((i, j)) {
            return;
        }
        self.mark(i, j, |c| match channel {
            Wear::Tracks => c.tracks = add(c.tracks, amount),
            Wear::Trampled => c.trampled = add(c.trampled, amount),
        });
    }

    /// The ground at `at` was cleared by a vehicle knocking through trees.
    pub fn clear(&mut self, at: V2) {
        if let Some((i, j)) = self.index(at.x, at.y) {
            self.mark(i, j, |c| c.cleared = u8::MAX);
        }
    }

    /// Refresh the digest of every tile marked since the last seal (the
    /// battle calls it once per tick, after every write).
    pub fn seal(&mut self) {
        for t in std::mem::take(&mut self.dirty) {
            let tile = self.tiles.get_mut(&t).expect("a dirty tile exists");
            Arc::make_mut(&mut tile.cells).compress();
            tile.hash = tile_hash(&tile.cells);
            if let Some(edits) = &mut self.edits {
                edits.tiles.insert(tile.edit, t);
            }
            tile.dirty = false;
        }
    }

    pub fn digest(&self, d: &mut Digest) {
        debug_assert!(self.dirty.is_empty(), "the ground layer is sealed");
        d.u64(self.allocated() as u64);
        for (&t, tile) in &self.tiles {
            d.u64(t as u64).u64(tile.hash);
        }
    }

    /// Every marked cell's lower corner and marks, in cell order within tiles.
    pub fn cells(&self) -> impl Iterator<Item = (f64, f64, GroundCell)> + '_ {
        self.tiles.iter().flat_map(move |(&t, tile)| {
            let (ti, tj) = (t % self.tiles_x, t / self.tiles_x);
            (0..TILE_CELLS).filter_map(move |c| {
                let cell = tile.cells.get(c);
                if cell == GroundCell::default() {
                    return None;
                }
                let (i, j) = (ti * TILE + c % TILE, tj * TILE + c / TILE);
                Some((i as f64 * self.cell_m, j as f64 * self.cell_m, cell))
            })
        })
    }

    /// Representation payload estimate, excluding allocator/tree-node overhead.
    /// Shared pages are counted per owner; allocator probes measure actual peaks.
    pub fn bytes(&self) -> usize {
        self.index_bytes()
            + self
                .tiles
                .values()
                .map(|tile| std::mem::size_of::<Tile>() + tile.cells.bytes())
                .sum::<usize>()
    }

    /// Dense representation payload estimate with every tile allocated;
    /// allocator/tree-node overhead is measured separately.
    pub fn bound_bytes(&self) -> usize {
        self.tiles_x
            * self.rows.div_ceil(TILE)
            * (std::mem::size_of::<Tile>()
                + std::mem::size_of::<(usize, Box<Tile>)>()
                + std::mem::size_of::<[GroundCell; TILE_CELLS]>()
                + std::mem::size_of::<Page<GroundCell>>()
                + if self.edits.is_some() {
                    std::mem::size_of::<(u64, usize)>()
                } else {
                    0
                })
    }

    fn allocated(&self) -> usize {
        self.tiles.len()
    }

    fn index_bytes(&self) -> usize {
        self.tiles.len() * std::mem::size_of::<(usize, Box<Tile>)>()
            + self.edits.as_ref().map_or(0, |edits| {
                edits.tiles.len() * std::mem::size_of::<(u64, usize)>()
            })
    }
}

/// One side's learned copy of the ground layer: each cell as it was when the
/// side last saw it, blank where it never has. It only ever catches up with
/// the layer, over ground its fog shows; ground out of sight keeps its old
/// marks here however the battle changes it (hidden information). Learning
/// a crater touches nothing else, navigation least of all (landmine 3).
///
/// Each change is stamped with the knowledge revision it happened at, so
/// delivery can hand any consumer exactly the cells changed after its cursor
/// ([`KnownGround::change_runs_since`]) without keeping a journal. Bounded like
/// the layer: marks and stamps live in tiles allocated on first learning.
pub struct KnownGround {
    cells: GroundLayer,
    /// Per tile, the revision each cell last changed at (0: never learned).
    stamps: BTreeMap<usize, Box<Stamps>>,
    /// Bumped by every learning pass that changes a cell.
    revision: u32,
    /// Hints only: unchanged source tiles and previously visible words require
    /// no cell learning. Neither cache participates in the battle digest.
    source_revision: u64,
    visibility: Vec<u32>,
    visibility_grid: (f64, usize, usize),
}

/// A lossless changed span in one16×16 tile, in original tile/cell order.
/// Local spans may cross a tile row: global cell indices use the map stride.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundRunPatch {
    pub tile: u32,
    pub start: u16,
    pub len: u16,
    pub marks: GroundCell,
}

struct Stamps {
    cells: Page<u32>,
    /// The newest of `cells`: a tile unchanged since a cursor is skipped whole.
    newest: u32,
}

impl KnownGround {
    pub(crate) fn layer(&self) -> &GroundLayer {
        &self.cells
    }

    /// Nothing learned yet, over `layer`'s grid.
    pub fn new(layer: &GroundLayer) -> Self {
        KnownGround {
            cells: layer.blank(),
            stamps: BTreeMap::new(),
            revision: 0,
            source_revision: 0,
            visibility: Vec::new(),
            visibility_grid: (0.0, 0, 0),
        }
    }

    /// The revision of the latest learning pass that changed a cell.
    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// The learned marks of the cell holding (x, y).
    pub fn cell(&self, x: f64, y: f64) -> GroundCell {
        self.cells.cell(x, y)
    }

    /// Every learned (non-blank) cell's lower corner and marks.
    pub fn cells(&self) -> impl Iterator<Item = (f64, f64, GroundCell)> + '_ {
        self.cells.cells()
    }

    /// Learn every cell of `layer` whose centre lies in a fog cell `fog`
    /// shows: a seen cell is known as it is now.
    pub fn learn(&mut self, layer: &GroundLayer, fog: &VisibilityField) {
        if layer.tiles.is_empty() {
            return;
        }
        let grid = (fog.cell_m, fog.nx as usize, fog.ny as usize);
        let fresh = self.visibility_grid != grid || self.visibility.len() != fog.bits.len();
        let base = self.source_revision;
        let next = self.revision + 1;
        let mut changed = false;
        for (tile, truth) in layer.changed_tiles_since(base) {
            changed |= self.learn_tile(layer, tile, truth, fog, next);
        }
        // On the first pass the edit index already visits every truth page.
        // Later reveals select pages from new visibility; a broad reveal walks
        // occupied pages rather than materializing a world-sized candidate list.
        if base != 0 {
            let limit = (layer.tiles.len() / 8).max(1024);
            let mut tiles = Vec::new();
            let mut broad = false;
            'words: for (word, &visible) in fog.bits.iter().enumerate() {
                let mut bits = visible & !if fresh { 0 } else { self.visibility[word] };
                while bits != 0 {
                    let cell = word * 32 + bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    if cell >= grid.1 * grid.2 {
                        break;
                    }
                    let span = |f: usize, n: usize| {
                        let edge = |f: usize| {
                            ((f as f64 * fog.cell_m / layer.cell_m - 0.5).ceil().max(0.0) as usize)
                                .min(n)
                        };
                        (edge(f), edge(f + 1))
                    };
                    let (i0, i1) = span(cell % grid.1, layer.cols);
                    let (j0, j1) = span(cell / grid.1, layer.rows);
                    if i0 >= i1 || j0 >= j1 {
                        continue;
                    }
                    for y in j0 / TILE..=(j1 - 1) / TILE {
                        for x in i0 / TILE..=(i1 - 1) / TILE {
                            if tiles.len() >= limit {
                                broad = true;
                                break 'words;
                            }
                            tiles.push(y * layer.tiles_x + x);
                        }
                    }
                }
            }
            if broad {
                for (&tile, truth) in &layer.tiles {
                    if truth.edit <= base {
                        changed |= self.learn_tile(layer, tile, truth, fog, next);
                    }
                }
            } else {
                tiles.sort_unstable();
                tiles.dedup();
                for tile in tiles {
                    if let Some(truth) = layer.tiles.get(&tile).filter(|truth| truth.edit <= base) {
                        changed |= self.learn_tile(layer, tile, truth, fog, next);
                    }
                }
            }
        }
        self.source_revision = layer.edits.as_ref().expect("truth tracks edits").revision;
        self.visibility.clone_from(&fog.bits);
        self.visibility_grid = grid;
        if changed {
            self.revision = next;
        }
        let touched = self.cells.dirty.clone();
        self.cells.seal();
        for tile in touched {
            if let Some(stamps) = self.stamps.get_mut(&tile) {
                stamps.cells.compress();
            }
            let known = self
                .cells
                .tiles
                .get_mut(&tile)
                .expect("learned tile exists");
            if let Some(truth) = layer.tiles.get(&tile) {
                if (0..TILE_CELLS).all(|cell| known.cells.get(cell) == truth.cells.get(cell)) {
                    known.cells = Arc::clone(&truth.cells);
                }
            }
        }
    }

    fn learn_tile(
        &mut self,
        layer: &GroundLayer,
        tile: usize,
        truth: &Tile,
        fog: &VisibilityField,
        next: u32,
    ) -> bool {
        let (x, y) = (tile % layer.tiles_x * TILE, tile / layer.tiles_x * TILE);
        let (width, height) = ((layer.cols - x).min(TILE), (layer.rows - y).min(TILE));
        // Match the original ceiling partition at fractional-grid boundaries.
        let fog_coord = |cell: usize| {
            let mut at = ((cell as f64 + 0.5) * layer.cell_m / fog.cell_m).floor() as usize;
            let edge = |at: usize| {
                (at as f64 * fog.cell_m / layer.cell_m - 0.5)
                    .ceil()
                    .max(0.0) as usize
            };
            if at < usize::MAX {
                if cell < edge(at) {
                    at = at.saturating_sub(1);
                } else if cell >= edge(at + 1) {
                    at += 1;
                }
            }
            at
        };
        let visible = |i: usize, j: usize| {
            if i >= fog.nx as usize || j >= fog.ny as usize {
                return false;
            }
            let cell = j * fog.nx as usize + i;
            fog.bits[cell / 32] & (1 << (cell % 32)) != 0
        };
        let seen = |i: usize, j: usize| visible(fog_coord(i), fog_coord(j));
        let (x0, x1) = (fog_coord(x), fog_coord(x + width - 1));
        let (y0, y1) = (fog_coord(y), fog_coord(y + height - 1));
        let full = x1 < fog.nx as usize
            && y1 < fog.ny as usize
            && (x1 - x0 + 1)
                .checked_mul(y1 - y0 + 1)
                .is_some_and(|cells| cells <= TILE_CELLS)
            && (y0..=y1).all(|j| (x0..=x1).all(|i| visible(i, j)));
        if full {
            let old = self.cells.tiles.get(&tile);
            if old.is_some_and(|old| Arc::ptr_eq(&old.cells, &truth.cells)) {
                return false;
            }
            let stamps = self.stamps.entry(tile).or_insert_with(|| {
                Box::new(Stamps {
                    cells: Page::default(),
                    newest: 0,
                })
            });
            let mut changed = false;
            match old {
                None => {
                    stamps.cells = truth.cells.map_values(|marks| {
                        if marks == GroundCell::default() {
                            0
                        } else {
                            next
                        }
                    });
                    changed = true;
                }
                Some(old)
                    if width == TILE
                        && height == TILE
                        && old.cells.uniform().is_some()
                        && truth.cells.uniform().is_some()
                        && old.cells.uniform() != truth.cells.uniform() =>
                {
                    stamps.cells = Page::Uniform(next);
                    changed = true;
                }
                Some(old) => {
                    for j in 0..height {
                        for i in 0..width {
                            let cell = j * TILE + i;
                            if old.cells.get(cell) != truth.cells.get(cell) {
                                stamps.cells.set(cell, next);
                                changed = true;
                            }
                        }
                    }
                }
            }
            if changed {
                stamps.newest = next;
                stamps.cells.compress();
                self.cells.tiles.insert(
                    tile,
                    Box::new(Tile {
                        cells: Arc::clone(&truth.cells),
                        hash: if truth.dirty {
                            tile_hash(&truth.cells)
                        } else {
                            truth.hash
                        },
                        dirty: false,
                        edit: 0,
                    }),
                );
            }
            return changed;
        }
        let mut changed = false;
        for j in 0..height {
            for i in 0..width {
                if seen(x + i, y + j) {
                    let cell = j * TILE + i;
                    if self.cells.put(tile, cell, truth.cells.get(cell)) {
                        let stamps = self.stamps.entry(tile).or_insert_with(|| {
                            Box::new(Stamps {
                                cells: Page::default(),
                                newest: 0,
                            })
                        });
                        stamps.cells.set(cell, next);
                        stamps.newest = next;
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Every cell learned or changed after revision `base`, as its grid index
    /// and marks, tile by tile. From 0 it is everything learned.
    pub fn change_runs_since(&self, base: u32) -> impl Iterator<Item = GroundRunPatch> + '_ {
        self.stamps
            .iter()
            .take(if base < self.revision { usize::MAX } else { 0 })
            .filter(move |(_, stamps)| stamps.newest > base)
            .flat_map(move |(&tile, stamps)| {
                let values = &self
                    .cells
                    .tiles
                    .get(&tile)
                    .expect("a learned tile exists")
                    .cells;
                let mut next = 0;
                std::iter::from_fn(move || {
                    if next == 0 {
                        if let (Some(stamp), Some(marks)) =
                            (stamps.cells.uniform(), values.uniform())
                        {
                            next = TILE_CELLS;
                            return (stamp > base).then_some(GroundRunPatch {
                                tile: tile as u32,
                                start: 0,
                                len: TILE_CELLS as u16,
                                marks,
                            });
                        }
                    }
                    while next < TILE_CELLS && stamps.cells.get(next) <= base {
                        next += 1;
                    }
                    if next == TILE_CELLS {
                        return None;
                    }
                    let start = next;
                    let marks = values.get(next);
                    next += 1;
                    while next < TILE_CELLS
                        && stamps.cells.get(next) > base
                        && values.get(next) == marks
                    {
                        next += 1;
                    }
                    Some(GroundRunPatch {
                        tile: tile as u32,
                        start: start as u16,
                        len: (next - start) as u16,
                        marks,
                    })
                })
            })
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.revision as u64);
        self.cells.digest(d);
    }

    /// Representation payload estimate; includes shared pages per owner,
    /// excludes allocator/tree-node overhead.
    pub fn bytes(&self) -> usize {
        self.cells.bytes()
            + self.stamps.len() * std::mem::size_of::<(usize, Box<Stamps>)>()
            + self
                .stamps
                .values()
                .map(|s| std::mem::size_of::<Stamps>() + s.cells.bytes())
                .sum::<usize>()
            + self.visibility.capacity() * std::mem::size_of::<u32>()
    }
}

/// A tile's content hash: FNV-1a over one cell per 64-bit word, so sealing
/// a busy tick stays cheap. Only its equality matters to the digest.
fn tile_hash(cells: &Page<GroundCell>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for c in 0..TILE_CELLS {
        h = (h ^ cells.get(c).word()).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Saturating add of a rounded amount.
fn add(v: u8, amount: f64) -> u8 {
    (v as f64 + amount.round()).min(u8::MAX as f64) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn rules() -> GroundRules {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
        serde_json::from_value(fixture["ground"].clone()).unwrap()
    }

    fn digest(known: &KnownGround) -> u64 {
        let mut d = Digest::default();
        known.digest(&mut d);
        d.finish()
    }

    #[test]
    fn learning_matches_ceiling_partitions_through_edits_reveals_and_grid_changes() {
        // Reference the original fog-cell ceiling partition. Keep explicit cell
        // values/stamps, independently of page sharing and candidate selection.
        for width in [17, 1040] {
            let height = 19;
            let mut truth = GroundLayer::new(width as f64, height as f64, &rules());
            for j in 0..height {
                for i in 0..width {
                    truth.mark(i, j, |cell| cell.crater = 1 + ((i + j) % 13) as u8);
                }
            }
            truth.seal();
            let mut known = KnownGround::new(&truth);
            let mut values = vec![GroundCell::default(); width * height];
            let mut stamps = vec![0; width * height];
            let mut revision = 0;
            for (step, fog_m) in [2.3, 2.3, 2.3, 1.0, 0.3, 8.1, 8.1].into_iter().enumerate() {
                if step == 1 || step == 5 {
                    // Direct API edits deliberately remain unsealed at learning.
                    for (i, j) in [(0, 0), (16, 18), (width - 1, 1)] {
                        truth.mark(i, j, |cell| cell.scorch = 71 + step as u8);
                    }
                }
                let (nx, ny) = (
                    (width as f64 / fog_m).ceil() as u32,
                    (height as f64 / fog_m).ceil() as u32,
                );
                let mut fog = VisibilityField {
                    cell_m: fog_m,
                    nx,
                    ny,
                    bits: vec![0; (nx as usize * ny as usize).div_ceil(32)],
                };
                for f in 0..nx as usize * ny as usize {
                    if step >= 3 || (f + step) % 3 == 0 {
                        fog.bits[f / 32] |= 1 << (f % 32);
                    }
                }
                let mut changed = false;
                for f in 0..nx as usize * ny as usize {
                    if fog.bits[f / 32] & (1 << (f % 32)) == 0 {
                        continue;
                    }
                    let edge = |f: usize, n: usize| {
                        ((f as f64 * fog_m - 0.5).ceil().max(0.0) as usize).min(n)
                    };
                    for j in edge(f / nx as usize, height)..edge(f / nx as usize + 1, height) {
                        for i in edge(f % nx as usize, width)..edge(f % nx as usize + 1, width) {
                            let c = j * width + i;
                            let value = truth.cell(i as f64 + 0.5, j as f64 + 0.5);
                            if value != values[c] {
                                values[c] = value;
                                stamps[c] = revision + 1;
                                changed = true;
                            }
                        }
                    }
                }
                revision += u32::from(changed);
                known.learn(&truth, &fog);
                assert_eq!(known.revision(), revision, "width={width} step={step}");
                for base in 0..=revision {
                    let actual: BTreeMap<_, _> = known
                        .change_runs_since(base)
                        .flat_map(|run| {
                            (run.start..run.start + run.len)
                                .map(move |local| ((run.tile as usize, local as usize), run.marks))
                        })
                        .collect();
                    let expected: BTreeMap<_, _> = (0..height)
                        .flat_map(|j| (0..width).map(move |i| (i, j)))
                        .filter(|&(i, j)| stamps[j * width + i] > base)
                        .map(|(i, j)| (truth.slot(i, j), values[j * width + i]))
                        .collect();
                    assert_eq!(actual, expected, "width={width} step={step} base={base}");
                }
                truth.seal();
            }
        }
    }

    #[test]
    fn empty_large_ground_does_not_allocate_for_untouched_cells() {
        let layer = GroundLayer::new(18_000.0, 18_000.0, &rules());
        let known = KnownGround::new(&layer);
        assert!(
            layer.bytes() + known.bytes() < 4096,
            "untouched ground allocated {} bytes",
            layer.bytes() + known.bytes()
        );
        assert_eq!(layer.cell(17_999.5, 17_999.5), GroundCell::default());
    }

    #[test]
    fn a_fully_learned_uniform_page_keeps_old_values_after_hidden_truth_edits() {
        let mut truth = GroundLayer::new(16.0, 16.0, &rules());
        for j in 0..16 {
            for i in 0..16 {
                truth.mark(i, j, |cell| {
                    cell.crater = 19;
                    cell.cleared = 255;
                });
            }
        }
        truth.seal();
        let mut known = KnownGround::new(&truth);
        let visible = VisibilityField {
            cell_m: 16.0,
            nx: 1,
            ny: 1,
            bits: vec![1],
        };
        known.learn(&truth, &visible);
        assert!(
            truth.bytes() + known.bytes() < 1024,
            "uniform touched pages must not require a dense cell or revision copy"
        );
        let runs: Vec<_> = known.change_runs_since(0).collect();
        assert_eq!(runs.len(), 1);
        assert_eq!((runs[0].tile, runs[0].start, runs[0].len), (0, 0, 256));
        assert_eq!(runs[0].marks, truth.cell(15.5, 15.5));
        assert_eq!(known.change_runs_since(known.revision()).count(), 0);
        let old = digest(&known);
        truth.mark(15, 15, |cell| cell.crater = 73);
        truth.seal();
        known.learn(
            &truth,
            &VisibilityField {
                bits: vec![0],
                ..visible.clone()
            },
        );
        assert_eq!(known.cell(15.5, 15.5).crater, 19);
        assert_eq!(digest(&known), old);
        known.learn(&truth, &visible);
        assert_eq!(known.cell(15.5, 15.5).crater, 73);
        let delta: Vec<_> = known.change_runs_since(1).collect();
        assert_eq!(delta.len(), 1);
        assert_eq!((delta[0].tile, delta[0].start, delta[0].len), (0, 255, 1));
        assert_eq!(delta[0].marks.crater, 73);
    }

    #[test]
    fn uniform_run_export_keeps_partial_edge_holes_out_of_the_patch() {
        let mut layer = GroundLayer::new(17.0, 17.0, &rules());
        for j in 0..17 {
            for i in 0..17 {
                layer.mark(i, j, |cell| cell.cleared = 255);
            }
        }
        layer.seal();
        let mut known = KnownGround::new(&layer);
        known.learn(
            &layer,
            &VisibilityField {
                cell_m: 32.0,
                nx: 1,
                ny: 1,
                bits: vec![1],
            },
        );
        let runs: Vec<_> = known.change_runs_since(0).collect();
        let ids: BTreeSet<_> = runs
            .iter()
            .flat_map(|run| {
                (run.start..run.start + run.len).map(move |cell| {
                    let (ti, tj) = (run.tile as usize % 2, run.tile as usize / 2);
                    (tj * TILE + cell as usize / TILE) * 17 + ti * TILE + cell as usize % TILE
                })
            })
            .collect();
        assert_eq!(ids, (0..289).collect());
        assert_eq!(runs.iter().map(|run| run.len as usize).sum::<usize>(), 289);
        assert!(runs.iter().all(|run| run.marks.cleared == 255));
        assert!(known
            .change_runs_since(0)
            .any(|run| run.tile == 0 && run.len == 256));
        assert!(known
            .change_runs_since(0)
            .filter(|run| run.tile == 1)
            .all(|run| run.len == 1));
    }

    #[test]
    fn learned_cells_are_digested_and_only_seen_fog_cells_are_learned() {
        let rules = rules();
        let mut layer = GroundLayer::new(64.0, 64.0, &rules);
        layer.wear(v2(3.5, 3.5), v2(20.5, 4.5), Wear::Tracks, &rules);
        layer.wear(v2(3.5, 3.5), v2(40.5, 40.5), Wear::Tracks, &rules);
        layer.seal();
        let mut known = KnownGround::new(&layer);
        let blank = digest(&known);
        // 8 m fog over 64 m: only fog cell (2, 0), x 16-24 and y 0-8, is seen.
        let fog = VisibilityField {
            cell_m: 8.0,
            nx: 8,
            ny: 8,
            bits: vec![1 << 2, 0],
        };
        known.learn(&layer, &fog);
        assert!(layer.cell(20.5, 4.5).tracks > 0);
        assert_eq!(known.cell(20.5, 4.5), layer.cell(20.5, 4.5));
        assert_eq!(known.cell(40.5, 40.5), GroundCell::default());
        assert_eq!(known.revision(), 1);
        assert_ne!(digest(&known), blank, "learned cells are knowledge state");
    }
}
