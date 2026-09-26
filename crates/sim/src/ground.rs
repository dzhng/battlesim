//! The ground layer (D2, Q8): what fire and movement leave on the ground, as
//! authoritative state in fixed cells of `ground.cell_m`. Craters are the one
//! channel with a rule: infantry in one has partial cover (it joins the
//! strongest-source rule in `damage`), and a vehicle driving over one is
//! slowed slightly, in movement integration only. Navigation never reads
//! craters, so a crater never rebuilds anyone's plan (landmine 3). Scorch,
//! track wear and trampling are recorded and change nothing.
//!
//! Bounded: cells live in tiles allocated on first mark, so storage grows with
//! the ground touched and never past the map's area. Every channel is a byte
//! that accumulates and saturates; nothing decays.
use contract::scenario::{CoverRules, GroundRules, Rules};

use crate::digest::Digest;
use crate::math::{v2, V2, V3};
use crate::world::{SurfaceKind, WorldGeometry};

/// Cells per tile edge.
const TILE: usize = 16;
const TILE_CELLS: usize = TILE * TILE;
/// Ground-cover strength at a forest's first metre (`damage::ground_cover`):
/// a full crater must stay below it.
pub const FOREST_EDGE_COVER: f64 = 0.4;

/// One cell's marks, each in [0, 255].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroundCell {
    pub crater: u8,
    pub scorch: u8,
    pub tracks: u8,
    pub trampled: u8,
}

impl GroundCell {
    fn word(self) -> u32 {
        u32::from_le_bytes([self.crater, self.scorch, self.tracks, self.trampled])
    }
}

/// A cosmetic wear channel movement writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wear {
    Tracks,
    Trampled,
}

struct Tile {
    cells: [GroundCell; TILE_CELLS],
    /// Digest of `cells`, refreshed by [`GroundLayer::seal`].
    hash: u64,
    /// Marked since the last seal.
    dirty: bool,
}

pub struct GroundLayer {
    cell_m: f64,
    cols: usize,
    rows: usize,
    tiles_x: usize,
    tiles: Vec<Option<Box<Tile>>>,
    /// Tiles marked since the last seal, in first-mark order.
    dirty: Vec<usize>,
}

/// Reject ground rules that would break the contract: craters weaker cover
/// than any forest or building, never impassable.
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
        g.crater_cover > 0.0 && g.crater_cover < FOREST_EDGE_COVER,
        "ground.crater_cover must be in (0, {FOREST_EDGE_COVER}): weaker than any forest"
    );
    let CoverRules {
        forest_spread_multiplier: fs,
        forest_fragment_probability_multiplier: ff,
        building_spread_multiplier: bs,
        building_fragment_probability_multiplier: bf,
    } = rules.cover;
    let b = rules.buildings.cover_strength;
    assert!(
        1.0 + (fs - 1.0) * g.crater_cover < 1.0 + (bs - 1.0) * b
            && 1.0 + (ff - 1.0) * g.crater_cover > 1.0 + (bf - 1.0) * b,
        "a full crater must cover less than a building"
    );
}

impl GroundLayer {
    /// An unmarked layer over a map `width` × `depth` metres from the origin.
    pub fn new(width: f64, depth: f64, rules: &GroundRules) -> Self {
        let cell_m = rules.cell_m;
        let cols = (width / cell_m).ceil().max(1.0) as usize;
        let rows = (depth / cell_m).ceil().max(1.0) as usize;
        let tiles_x = cols.div_ceil(TILE);
        let tiles_y = rows.div_ceil(TILE);
        GroundLayer {
            cell_m,
            cols,
            rows,
            tiles_x,
            tiles: (0..tiles_x * tiles_y).map(|_| None).collect(),
            dirty: Vec::new(),
        }
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
        let before = self.cell_at(t, c);
        let mut cell = before;
        f(&mut cell);
        if cell == before {
            return;
        }
        let tile = self.tiles[t].get_or_insert_with(|| {
            Box::new(Tile {
                cells: [GroundCell::default(); TILE_CELLS],
                hash: 0,
                dirty: false,
            })
        });
        tile.cells[c] = cell;
        if !tile.dirty {
            tile.dirty = true;
            self.dirty.push(t);
        }
    }

    fn cell_at(&self, t: usize, c: usize) -> GroundCell {
        self.tiles[t]
            .as_ref()
            .map_or_else(GroundCell::default, |tile| tile.cells[c])
    }

    /// How full the crater at (x, y) is, in [0, 1].
    pub fn crater_fill(&self, x: f64, y: f64, rules: &GroundRules) -> f64 {
        (self.cell(x, y).crater as f64 / rules.crater_full_depth).min(1.0)
    }

    /// Ground-cover strength a crater gives infantry at `p`, on the forest's scale.
    pub fn crater_cover(&self, p: V3, rules: &GroundRules) -> f64 {
        rules.crater_cover * self.crater_fill(p.x, p.y, rules)
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

    /// Refresh the digest of every tile marked since the last seal (the
    /// battle calls it once per tick, after every write).
    pub fn seal(&mut self) {
        for t in std::mem::take(&mut self.dirty) {
            let tile = self.tiles[t].as_mut().expect("a dirty tile exists");
            tile.hash = tile_hash(&tile.cells);
            tile.dirty = false;
        }
    }

    pub fn digest(&self, d: &mut Digest) {
        debug_assert!(self.dirty.is_empty(), "the ground layer is sealed");
        d.u64(self.allocated() as u64);
        for (t, tile) in self.tiles.iter().enumerate() {
            if let Some(tile) = tile {
                d.u64(t as u64).u64(tile.hash);
            }
        }
    }

    /// Every marked cell's lower corner and marks, in cell order within tiles.
    pub fn cells(&self) -> impl Iterator<Item = (f64, f64, GroundCell)> + '_ {
        self.tiles.iter().enumerate().flat_map(move |(t, tile)| {
            let (ti, tj) = (t % self.tiles_x, t / self.tiles_x);
            tile.iter().flat_map(move |tile| {
                tile.cells
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| **cell != GroundCell::default())
                    .map(move |(c, cell)| {
                        let (i, j) = (ti * TILE + c % TILE, tj * TILE + c / TILE);
                        (i as f64 * self.cell_m, j as f64 * self.cell_m, *cell)
                    })
            })
        })
    }

    /// Bytes the layer holds now.
    pub fn bytes(&self) -> usize {
        self.index_bytes() + self.allocated() * std::mem::size_of::<Tile>()
    }

    /// The most the layer can ever hold: every tile of the map allocated.
    pub fn bound_bytes(&self) -> usize {
        self.index_bytes() + self.tiles.len() * std::mem::size_of::<Tile>()
    }

    fn allocated(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }

    fn index_bytes(&self) -> usize {
        self.tiles.len() * std::mem::size_of::<Option<Box<Tile>>>()
    }
}

/// A tile's content hash: FNV-1a over two cells per 64-bit word, so sealing
/// a busy tick stays cheap. Only its equality matters to the digest.
fn tile_hash(cells: &[GroundCell; TILE_CELLS]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for pair in cells.chunks_exact(2) {
        let w = pair[0].word() as u64 | (pair[1].word() as u64) << 32;
        h = (h ^ w).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Saturating add of a rounded amount.
fn add(v: u8, amount: f64) -> u8 {
    (v as f64 + amount.round()).min(u8::MAX as f64) as u8
}
