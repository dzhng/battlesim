//! Each side's learned copy of the ground layer, and the change runs
//! delivery reads from it.
use super::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn rules() -> GroundRules {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/rules.json")).unwrap();
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
        let game: contract::scenario::Rules =
            serde_json::from_value(crate::fixtures::test_rules()).unwrap();
        let map = serde_json::from_value(serde_json::json!({
            "size":[64,64],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &game);
        layer.wear(&world, v2(3.5, 3.5), v2(20.5, 4.5), Wear::Tracks, &rules);
        layer.wear(&world, v2(3.5, 3.5), v2(40.5, 40.5), Wear::Tracks, &rules);
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
