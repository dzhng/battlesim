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

mod known;
pub use known::{GroundRunPatch, KnownGround};

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
        g.mark_reference_blast_m > 0.0,
        "ground.mark_reference_blast_m must be positive"
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
    /// ground within the scorch radius likewise. Standing water, a river's
    /// or a swamp's, takes no marks.
    pub fn burst(&mut self, world: &WorldGeometry, at: V3, blast_radius: f64, rules: &GroundRules) {
        let Some(ground) = world.ground_surface_at(at.x, at.y) else {
            return;
        };
        if ground.standing_water() {
            return;
        }
        let height = at.z - ground.z;
        // A burst smaller than the reference marks less. Its crater's depth
        // already grows with its radius (`crater_depth_per_m`), and the ground
        // it throws up goes as the area, so the depth takes the share squared;
        // scorch is burnt area added once per burst, so it takes the share
        // itself. A small shell leaves a scorched pock, not a soil ring.
        let share = (blast_radius / rules.mark_reference_blast_m).min(1.0);
        let crater = blast_radius * rules.crater_radius_fraction;
        if height <= crater {
            let depth = rules.crater_depth_per_m * crater * share * share;
            self.stamp(at.xy(), crater, depth, |c, v| c.crater = add(c.crater, v));
        }
        let scorch = blast_radius * rules.scorch_radius_fraction;
        if height <= scorch {
            self.stamp(at.xy(), scorch, rules.scorch_per_burst * share, |c, v| {
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

    /// Each tread in `before` moved to its place in `after` (the same treads,
    /// in order): wear the ground it entered.
    pub fn tread(
        &mut self,
        world: &WorldGeometry,
        before: Vec<(V2, Wear)>,
        after: Vec<(V2, Wear)>,
        rules: &GroundRules,
    ) {
        for ((from, channel), (to, _)) in before.into_iter().zip(after) {
            self.wear(world, from, to, channel, rules);
        }
    }

    /// A track or a soldier moved from `from` to `to`: entering a new cell
    /// adds one pass of wear there. Only ground it stands on takes the wear:
    /// a bridge deck carries it over whatever lies below, and standing water
    /// takes no marks.
    pub fn wear(
        &mut self,
        world: &WorldGeometry,
        from: V2,
        to: V2,
        channel: Wear,
        rules: &GroundRules,
    ) {
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
        let on_ground = world
            .surface_at(to.x, to.y)
            .is_some_and(|s| s.kind != SurfaceKind::Bridge && !s.standing_water());
        if !on_ground {
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
