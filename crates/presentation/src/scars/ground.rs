//! The side's learned ground, from its publications' patches: a new epoch
//! replaces what was learned with its snapshot, and each delta continues it.
//! Only touched tiles are kept; every other tile is blank.
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use sim::publication::reader::GroundPatch;

use super::page::{Edit, Page, CELLS, TILE};
use super::Ordered;

/// What changed since it was last taken: every tile, or these tiles (one
/// entry per changed run) over `cells` cells.
#[derive(Clone, Debug, PartialEq)]
pub enum Changes {
    All,
    Tiles { cells: usize, tiles: Vec<u32> },
}

impl Changes {
    pub const NONE: Changes = Changes::Tiles {
        cells: 0,
        tiles: Vec::new(),
    };

    pub fn is_empty(&self) -> bool {
        matches!(self, Changes::Tiles { cells: 0, .. })
    }

    /// `other` after these; past `bound` cells, everything.
    pub fn merge(&mut self, other: Changes, bound: usize) {
        match (&mut *self, other) {
            (Changes::All, _) => {}
            (_, Changes::All) => *self = Changes::All,
            (
                Changes::Tiles { cells, tiles },
                Changes::Tiles {
                    cells: n,
                    tiles: more,
                },
            ) => {
                if *cells + n > bound {
                    *self = Changes::All;
                } else {
                    *cells += n;
                    tiles.extend(more);
                }
            }
        }
    }
}

/// The scar cache's view of the ground's tiles by their visible words.
pub struct Shape {
    /// The word most tiles share (a blank tile's is 0).
    pub word: u32,
    /// Tiles that are not `word`.
    pub exceptions: usize,
    /// Pool words every varying tile takes.
    pub pool_words: u64,
}

pub struct LearnedGround {
    pub cell_m: f64,
    pub cols: usize,
    pub rows: usize,
    tiles_x: usize,
    tiles_y: usize,
    tiles: BTreeMap<u32, Page>,
    /// One bit per tile never learned.
    holes: Vec<u32>,
    marked_pages: usize,
    varying_words: u64,
    /// Tiles by the word they are uniformly, in the order each word first
    /// appeared (the shape's ties go to the first).
    uniform: Ordered<u32, HashSet<u32>>,
    varying: HashSet<u32>,
    cleared: usize,
    pub epoch: f64,
    pub side: Option<String>,
    pub revision: f64,
    changes: Changes,
}

impl LearnedGround {
    pub fn new(cell_m: f64, cols: usize, rows: usize) -> LearnedGround {
        let (tiles_x, tiles_y) = (cols.div_ceil(TILE), rows.div_ceil(TILE));
        LearnedGround {
            cell_m,
            cols,
            rows,
            tiles_x,
            tiles_y,
            tiles: BTreeMap::new(),
            holes: holes(tiles_x * tiles_y),
            marked_pages: 0,
            varying_words: 0,
            uniform: Ordered::default(),
            varying: HashSet::new(),
            cleared: 0,
            epoch: 0.0,
            side: None,
            revision: 0.0,
            changes: Changes::NONE,
        }
    }

    pub fn tiles(&self) -> [usize; 2] {
        [self.tiles_x, self.tiles_y]
    }

    /// Whether any learned cell is marked or cleared.
    pub fn has_marks(&self) -> bool {
        self.marked_pages > 0
    }

    /// Cells the side has seen cleared.
    pub fn cleared_count(&self) -> usize {
        self.cleared
    }

    /// Forgets what was learned (the side observed changes): blank until
    /// the next epoch's snapshot.
    pub fn forget(&mut self) {
        *self = LearnedGround {
            epoch: self.epoch,
            changes: Changes::All,
            ..LearnedGround::new(self.cell_m, self.cols, self.rows)
        };
    }

    /// Applies a publication's patch: false for one from an older epoch.
    pub fn apply(&mut self, patch: &GroundPatch) -> Result<bool, String> {
        if patch.epoch < self.epoch {
            return Ok(false);
        }
        if patch.epoch > self.epoch {
            if !patch.full {
                return Err(format!(
                    "ground epoch {} opened without a full snapshot",
                    patch.epoch
                ));
            }
            *self = LearnedGround {
                epoch: patch.epoch,
                side: Some(patch.side.to_string()),
                changes: Changes::All,
                ..LearnedGround::new(self.cell_m, self.cols, self.rows)
            };
        } else if patch.full || patch.base_revision != self.revision {
            return Err(format!(
                "ground patch {}→{} does not follow revision {}",
                patch.base_revision, patch.revision, self.revision
            ));
        }
        let mut edits = Vec::with_capacity(CELLS);
        let mut prior = None;
        for run in patch.runs.as_chunks::<4>().0 {
            let tile = run[0] as u32;
            if prior != Some(tile) {
                if let Some(p) = prior {
                    self.finish(p, &edits);
                }
                edits.clear();
                prior = Some(tile);
            }
            let (span, marks, rest) = (run[1] as usize, run[2] as u32, run[3] as u32);
            let edit = Edit {
                start: span % 256,
                len: span / 256,
                word: marks | ((rest & 0xffff) << 16),
                cleared: (rest >> 16) as u8,
            };
            self.touch(tile, edit.len);
            edits.push(edit);
        }
        if let Some(p) = prior {
            self.finish(p, &edits);
        }
        self.revision = patch.revision;
        Ok(true)
    }

    /// Allocates a tile on its first edit and notes the change.
    fn touch(&mut self, tile: u32, len: usize) {
        if let Entry::Vacant(blank) = self.tiles.entry(tile) {
            blank.insert(Page::blank());
            self.holes[tile as usize / 32] &= !(1 << (tile % 32));
            self.uniform
                .get_or_insert_with(0, HashSet::new)
                .insert(tile);
        }
        // Past a sixteenth of the map (at most 65,536 cells), everything.
        let bound = 65536f64.min((self.cols * self.rows) as f64 / 16.0);
        if let Changes::Tiles { cells, tiles } = &mut self.changes {
            if (*cells + len) as f64 > bound {
                self.changes = Changes::All;
            } else {
                *cells += len;
                tiles.push(tile);
            }
        }
    }

    fn finish(&mut self, tile: u32, edits: &[Edit]) {
        let (x, y) = (tile as usize % self.tiles_x, tile as usize / self.tiles_x);
        let (width, height) = (
            TILE.min(self.cols - x * TILE),
            TILE.min(self.rows - y * TILE),
        );
        let page = self.tiles.get_mut(&tile).unwrap();
        let marked = page.marked();
        self.cleared = (self.cleared as i64 + page.apply(edits)) as usize;
        self.marked_pages = self.marked_pages + page.marked() as usize - marked as usize;
        let (before, words_before) = (page.uniform, page.scar_words);
        page.uniform = page.projected_uniform(width, height);
        page.scar_words = match page.uniform {
            Some(_) => 0,
            None => Page::words_for(page.scar_runs(&mut Vec::new())),
        };
        self.varying_words = self.varying_words + page.scar_words as u64 - words_before as u64;
        let after = page.uniform;
        if after != before {
            match before {
                None => {
                    self.varying.remove(&tile);
                }
                Some(word) => {
                    let group = self.uniform.get_mut(&word).unwrap();
                    group.remove(&tile);
                    if group.is_empty() {
                        self.uniform.remove(&word);
                    }
                }
            }
            match after {
                None => {
                    self.varying.insert(tile);
                }
                Some(word) => {
                    self.uniform
                        .get_or_insert_with(word, HashSet::new)
                        .insert(tile);
                }
            }
        }
    }

    /// The tiles changed since the last call.
    pub fn take_changes(&mut self) -> Changes {
        std::mem::replace(&mut self.changes, Changes::NONE)
    }

    fn page(&self, i: usize, j: usize) -> Option<&Page> {
        if i >= self.cols || j >= self.rows {
            return None;
        }
        self.tiles
            .get(&((j / TILE * self.tiles_x + i / TILE) as u32))
    }

    /// Cell (i, j)'s marks as drawn (clearing as full track wear).
    pub fn marks(&self, i: usize, j: usize) -> [u8; 4] {
        self.page(i, j).map_or([0; 4], |p| {
            let (mut marks, cleared) = p.cell(j % TILE * TILE + i % TILE);
            marks[2] = marks[2].max(cleared);
            marks
        })
    }

    pub fn is_cleared(&self, i: usize, j: usize) -> bool {
        self.page(i, j)
            .is_some_and(|p| p.cell(j % TILE * TILE + i % TILE).1 > 0)
    }

    /// The visible word all of a tile's cells share, if they do; an
    /// unlearned tile's is 0.
    pub fn uniform(&self, tile: u32) -> Option<u32> {
        self.tiles.get(&tile).map_or(Some(0), |p| p.uniform)
    }

    /// A tile's runs as the GPU reads them; an unlearned tile is one blank run.
    pub fn scar_runs(&self, tile: u32, out: &mut Vec<[u32; 2]>) -> usize {
        match self.tiles.get(&tile) {
            Some(p) => p.scar_runs(out),
            None => {
                out.clear();
                out.push([CELLS as u32, 0]);
                1
            }
        }
    }

    pub fn shape(&self) -> Shape {
        let count = self.tiles_x * self.tiles_y;
        let blank = |u: &Ordered<u32, HashSet<u32>>| u.get(&0).map_or(0, HashSet::len);
        let (mut word, mut largest) = (0, count - self.tiles.len() + blank(&self.uniform));
        for (&candidate, tiles) in self.uniform.iter() {
            if candidate != 0 && tiles.len() > largest {
                (word, largest) = (candidate, tiles.len());
            }
        }
        Shape {
            word,
            exceptions: count - largest,
            pool_words: self.varying_words,
        }
    }

    /// Visits every tile whose visible word is not `word`: within the tile
    /// bounds `[x0, y0, x1, y1]` (inclusive) when given, else everywhere.
    pub fn exceptions(&self, word: u32, bounds: Option<[i64; 4]>, mut visit: impl FnMut(u32)) {
        if let Some([x0, y0, x1, y1]) = bounds {
            let (x1, y1) = (
                x1.min(self.tiles_x as i64 - 1),
                y1.min(self.tiles_y as i64 - 1),
            );
            for y in y0.max(0)..=y1 {
                for x in x0.max(0)..=x1 {
                    let tile = (y * self.tiles_x as i64 + x) as u32;
                    if self.uniform(tile) != Some(word) {
                        visit(tile);
                    }
                }
            }
            return;
        }
        self.varying.iter().for_each(|&t| visit(t));
        for (&value, tiles) in self.uniform.iter() {
            if value != word {
                tiles.iter().for_each(|&t| visit(t));
            }
        }
        if word != 0 {
            for (at, &bits) in self.holes.iter().enumerate() {
                let mut bits = bits;
                while bits != 0 {
                    visit(at as u32 * 32 + bits.trailing_zeros());
                    bits &= bits - 1;
                }
            }
        }
    }

    /// The cleared spans in tile order: (tile, start + length × 256) pairs.
    pub fn cleared_runs(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for (&tile, page) in &self.tiles {
            page.cleared_spans(|start, len| out.extend([tile, (start + len * 256) as u32]));
        }
        out
    }

    /// Every marked or cleared cell in row-major order: (cell index, its
    /// marks as drawn, a byte each) pairs.
    pub fn marked_cells(&self) -> Vec<u32> {
        let mut out = Vec::new();
        let marked: Vec<(u32, &Page)> = self
            .tiles
            .iter()
            .filter(|(_, p)| p.marked())
            .map(|(&t, p)| (t, p))
            .collect();
        for row in
            marked.chunk_by(|a, b| a.0 as usize / self.tiles_x == b.0 as usize / self.tiles_x)
        {
            let ty = row[0].0 as usize / self.tiles_x;
            for y in 0..TILE.min(self.rows - ty * TILE) {
                let j = ty * TILE + y;
                for &(tile, page) in row {
                    let x0 = tile as usize % self.tiles_x * TILE;
                    for x in 0..TILE.min(self.cols - x0) {
                        let (mut marks, cleared) = page.cell(y * TILE + x);
                        if marks == [0; 4] && cleared == 0 {
                            continue;
                        }
                        marks[2] = marks[2].max(cleared);
                        out.extend([(j * self.cols + x0 + x) as u32, u32::from_le_bytes(marks)]);
                    }
                }
            }
        }
        out
    }
}

/// Every tile a hole.
fn holes(count: usize) -> Vec<u32> {
    let mut words = vec![u32::MAX; count.div_ceil(32)];
    if !count.is_multiple_of(32) {
        *words.last_mut().unwrap() = u32::MAX >> (32 - count % 32);
    }
    words
}
