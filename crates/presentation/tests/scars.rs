//! The side's learned ground and its scar cache. What a renderer uploads is
//! what the browser uploaded when it owned them (`GroundView`, `ScarSync`),
//! word for word, over the streams in `tests/fixtures/scar-vectors.json`
//! (written by the browser's code before it moved here: epochs, deltas,
//! dense, run and uniform pages, a common word, pool pressure, the regional
//! path). Every cell read back through the uploaded directory and pool, as
//! the shader reads it, is the cell the side learned.
use presentation::scars::{
    hash, Changes, LearnedGround, ScarCache, ScarUpload, DENSE, KEY_MASK, UNIFORM,
};
use serde_json::Value;
use sim::publication::reader::GroundPatch;

fn patch(epoch: f64, base: f64, revision: f64, full: bool, runs: &[u32]) -> GroundPatch {
    GroundPatch {
        epoch,
        side: "blue".into(),
        base_revision: base,
        revision,
        full,
        runs: runs.iter().map(|&v| v as f32).collect(),
    }
}

/// Runs for single cells (index, crater, scorch, tracks, trampled, cleared),
/// in tile order.
fn cells(cols: usize, cells: &[(usize, [u32; 5])]) -> Vec<u32> {
    let tiles_x = cols.div_ceil(16);
    let mut runs: Vec<(u32, u32, u32, u32)> = cells
        .iter()
        .map(|&(index, m)| {
            let (i, j) = (index % cols, index / cols);
            (
                (j / 16 * tiles_x + i / 16) as u32,
                (j % 16 * 16 + i % 16) as u32,
                m[0] | m[1] << 8,
                m[2] | m[3] << 8 | m[4] << 16,
            )
        })
        .collect();
    runs.sort();
    runs.iter()
        .flat_map(|&(t, c, a, b)| [t, c + 256, a, b])
        .collect()
}

/// What the GPU holds, as the uploads left it.
struct Mirror {
    pool: Vec<u8>,
    directory: Vec<u32>,
    params: [u32; 20],
}

impl Mirror {
    fn new(words: usize) -> Mirror {
        Mirror {
            pool: vec![0; words * 4],
            directory: vec![0; 2],
            params: [0; 20],
        }
    }

    fn take(&mut self, u: &ScarUpload) {
        if let Some((words, [w, h])) = &u.directory {
            assert_eq!(words.len() as u32, w * h * 2);
            self.directory = words.clone();
        }
        let mut at = 0;
        for &[offset, texels] in &u.writes {
            let (o, n) = (offset as usize * 4, texels as usize * 4);
            self.pool[o..o + n].copy_from_slice(&u.texels[at..at + n]);
            at += n;
        }
        assert_eq!(at, u.texels.len());
        self.params
            .copy_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(&u.params)));
    }

    fn word(&self, w: usize) -> u32 {
        u32::from_le_bytes(self.pool[w * 4..w * 4 + 4].try_into().unwrap())
    }

    /// Cell (i, j) as `scarEntry` and `scarCell` read it.
    fn cell(&self, i: usize, j: usize) -> [u8; 4] {
        let p = &self.params;
        let (tx, ty) = ((i / 16) as u32, (j / 16) as u32);
        if tx < p[12] || ty < p[13] || tx > p[14] || ty > p[15] {
            return [0; 4];
        }
        let key = ty * p[4].div_ceil(16) + tx;
        let mut at = hash(key, p[6]);
        let mut e = [UNIFORM, p[16]];
        while self.directory[at as usize * 2] != 0 {
            if self.directory[at as usize * 2] & KEY_MASK == key + 1 {
                e = [
                    self.directory[at as usize * 2],
                    self.directory[at as usize * 2 + 1],
                ];
                break;
            }
            at = (at + 1) & p[6];
        }
        let local = (j % 16 * 16 + i % 16) as u32;
        let word = if e[0] & UNIFORM != 0 {
            e[1]
        } else if e[0] & DENSE != 0 {
            self.word((e[1] + local) as usize)
        } else {
            let (offset, count) = (e[1] >> 8, e[1] & 255);
            let (mut lo, mut hi) = (0, count);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if self.word((offset + mid * 2) as usize) & 0xffff <= local {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            self.word((offset + lo * 2 + 1) as usize)
        };
        word.to_le_bytes()
    }

    /// Every cell of the tiles around each of `tiles` reads as learned.
    fn matches(&self, ground: &LearnedGround, tiles: &[u32]) {
        let tiles_x = ground.cols.div_ceil(16);
        for &t in tiles {
            let (x, y) = (t as usize % tiles_x * 16, t as usize / tiles_x * 16);
            for j in y.saturating_sub(2)..(y + 18).min(ground.rows) {
                for i in x.saturating_sub(2)..(x + 18).min(ground.cols) {
                    assert_eq!(self.cell(i, j), ground.marks(i, j), "cell ({i}, {j})");
                }
            }
        }
    }

    /// Every marked cell reads as learned.
    fn matches_marked(&self, ground: &LearnedGround) {
        for pair in ground.marked_cells().chunks(2) {
            let (i, j) = (
                pair[0] as usize % ground.cols,
                pair[0] as usize / ground.cols,
            );
            assert_eq!(self.cell(i, j), pair[1].to_le_bytes(), "cell ({i}, {j})");
        }
    }
}

fn tile_of(cols: usize, i: usize, j: usize) -> u32 {
    (j / 16 * cols.div_ceil(16) + i / 16) as u32
}

// --- the browser's vectors ---------------------------------------------------

fn fnv(words: &[u32], bytes: &[u8]) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .chain(bytes.iter().copied())
    {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    format!("{h:08x}")
}

fn digest(u: &ScarUpload) -> String {
    let mut words = bytemuck::cast_slice::<_, u32>(bytemuck::bytes_of(&u.params)).to_vec();
    match &u.directory {
        Some((d, size)) => words.extend([1].iter().chain(size).chain(d)),
        None => words.push(0),
    }
    words.push(u.writes.len() as u32);
    words.extend(u.writes.concat());
    fnv(&words, &u.texels)
}

/// The browser's synthetic generators, for the streams too large to keep.
fn dense(tile: u32) -> Vec<u32> {
    (0..256)
        .flat_map(|c| [tile, c + 256, 1 + c % 200, c * 7 % 256])
        .collect()
}

fn generated(name: &str) -> Vec<u32> {
    let tiles_x = 18000u32.div_ceil(16);
    match name {
        "pool-1" => (0..126_000)
            .flat_map(|t| [t, 100 * 256, 5, 0, t, 100 + 156 * 256, 9, 0])
            .collect(),
        free if free.starts_with("pool-free-") => {
            let k: u32 = free["pool-free-".len()..].parse().unwrap();
            (k * 500..k * 500 + 500)
                .step_by(2)
                .flat_map(|t| [t, 65536, 5, 0])
                .collect()
        }
        dense_step if dense_step.starts_with("pool-dense-") => {
            let k: u32 = dense_step["pool-dense-".len()..].parse().unwrap();
            (30_000 + k * 100..30_100 + k * 100)
                .flat_map(dense)
                .collect()
        }
        "block" => (100..191)
            .flat_map(|y| (30..121).map(move |x| y * tiles_x + x))
            .flat_map(dense)
            .collect(),
        delta => {
            let step: u32 = delta.strip_prefix("delta-").unwrap().parse().unwrap();
            let mut picked = std::collections::BTreeMap::new();
            for k in 0..50 {
                let tile =
                    (100 + (k * 37 + step * 11) % 91) * tiles_x + 30 + (k * 53 + step * 7) % 91;
                picked.insert(tile, (k + step) % 3);
            }
            picked
                .into_iter()
                .flat_map(|(t, kind)| match kind {
                    0 => vec![t, 65536, 19 + 29 * 256, 31],
                    1 => vec![t, 65536, 0, 0],
                    _ => dense(t),
                })
                .collect()
        }
    }
}

#[test]
fn uploads_equal_the_browsers_word_for_word() {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/scar-vectors.json"
    );
    let vectors: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let reach = vectors["reach"].as_f64().unwrap();
    let mut prepares = 0;
    for stream in vectors["streams"].as_array().unwrap() {
        let name = stream["name"].as_str().unwrap();
        let (cols, rows) = (
            stream["cols"].as_u64().unwrap() as usize,
            stream["rows"].as_u64().unwrap() as usize,
        );
        let mut ground = LearnedGround::new(1.0, cols, rows);
        let mut cache = ScarCache::new(Some((cols, rows, 1.0)));
        cache.set_reach(reach);
        for (s, step) in stream["steps"].as_array().unwrap().iter().enumerate() {
            let p = &step["patch"];
            let runs: Vec<u32> = match p["generated"].as_str() {
                Some(g) => generated(g),
                None => p["runs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
            };
            let mut patch = patch(
                p["epoch"].as_f64().unwrap(),
                p["baseRevision"].as_f64().unwrap(),
                p["revision"].as_f64().unwrap(),
                p["full"].as_bool().unwrap(),
                &runs,
            );
            patch.side = p["side"].as_str().unwrap().into();
            assert!(ground.apply(&patch).unwrap(), "{name} step {s}");
            assert_eq!(
                fnv(&ground.cleared_runs(), &[]),
                step["cleared"],
                "{name} step {s}"
            );
            assert_eq!(
                ground.cleared_count() as u64,
                step["clearedCount"].as_u64().unwrap()
            );
            if let Some(marked) = step.get("marked") {
                assert_eq!(&fnv(&ground.marked_cells(), &[]), marked, "{name} step {s}");
            }
            for (f, frame) in step["frames"].as_array().unwrap().iter().enumerate() {
                let at = format!("{name} step {s} frame {f}");
                if frame["op"] == "resend" {
                    cache.resend();
                }
                assert_eq!(cache.changed(Some(&mut ground)), frame["changed"], "{at}");
                let regions = cache.regions();
                assert_eq!(
                    regions.len() as u64,
                    frame["regions"].as_u64().unwrap(),
                    "{at}"
                );
                let g: Vec<f64> = frame["grass"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect();
                let mut got = vec![digest(&cache.prepare(
                    Some(&mut ground),
                    Some([g[0], g[1], g[2], g[3]]),
                    false,
                ))];
                if regions.is_empty() {
                    got.push(digest(&cache.prepare(Some(&mut ground), None, false)));
                }
                for r in regions {
                    got.push(digest(&cache.prepare(Some(&mut ground), Some(r), true)));
                }
                let want: Vec<&str> = frame["digests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| d.as_str().unwrap())
                    .collect();
                assert_eq!(got, want, "{at}");
                prepares += got.len();
            }
        }
    }
    assert!(prepares > 1500, "{prepares} prepares");
}

// --- the learned ground ------------------------------------------------------

#[test]
fn epochs_replace_deltas_continue_and_gaps_are_refused() {
    let mut g = LearnedGround::new(1.0, 4, 4);
    assert!(g
        .apply(&patch(
            1.0,
            0.0,
            2.0,
            true,
            &cells(4, &[(5, [10, 0, 0, 0, 0])])
        ))
        .unwrap());
    assert!(g
        .apply(&patch(
            1.0,
            2.0,
            3.0,
            false,
            &cells(4, &[(6, [0, 20, 30, 40, 0])])
        ))
        .unwrap());
    assert_eq!(g.marks(1, 1), [10, 0, 0, 0]);
    assert_eq!(g.marks(2, 1), [0, 20, 30, 40]);
    // A delta that skips a revision means a lost patch; a new epoch opens
    // only with a snapshot.
    assert!(g.apply(&patch(1.0, 4.0, 5.0, false, &[])).is_err());
    assert!(g.apply(&patch(2.0, 3.0, 4.0, false, &[])).is_err());
    assert!(g
        .apply(&patch(
            2.0,
            0.0,
            7.0,
            true,
            &cells(4, &[(0, [1, 1, 1, 1, 0])])
        ))
        .unwrap());
    assert_eq!(g.marked_cells(), vec![0, u32::from_le_bytes([1, 1, 1, 1])]);
    // An older epoch's patch arriving late changes nothing.
    assert!(!g
        .apply(&patch(
            1.0,
            3.0,
            4.0,
            false,
            &cells(4, &[(9, [9, 9, 9, 9, 0])])
        ))
        .unwrap());
    assert_eq!(g.marks(1, 2), [0; 4]);
}

#[test]
fn changes_are_exact_and_everything_after_a_snapshot_or_past_a_bound() {
    let mut g = LearnedGround::new(1.0, 64, 64);
    g.apply(&patch(
        1.0,
        0.0,
        1.0,
        true,
        &cells(64, &[(3, [5, 0, 0, 0, 0])]),
    ))
    .unwrap();
    assert_eq!(g.take_changes(), Changes::All);
    g.apply(&patch(
        1.0,
        1.0,
        2.0,
        false,
        &cells(64, &[(7, [1, 0, 0, 0, 0])]),
    ))
    .unwrap();
    g.apply(&patch(
        1.0,
        2.0,
        3.0,
        false,
        &cells(64, &[(80, [1, 0, 0, 0, 0])]),
    ))
    .unwrap();
    assert_eq!(
        g.take_changes(),
        Changes::Tiles {
            cells: 2,
            tiles: vec![0, 1]
        }
    );
    assert_eq!(g.take_changes(), Changes::NONE);
    // Nobody took them: past a sixteenth of the map it is everything again.
    let many: Vec<_> = (0..300).map(|k| (k, [2, 0, 0, 0, 0])).collect();
    g.apply(&patch(1.0, 3.0, 4.0, false, &cells(64, &many)))
        .unwrap();
    assert_eq!(g.take_changes(), Changes::All);
}

#[test]
fn clearing_draws_as_full_wear_and_its_spans_follow_deltas_and_epochs() {
    let mut g = LearnedGround::new(1.0, 18000, 18000);
    let tile = 1125 * 1125 - 1;
    g.apply(&patch(
        1.0,
        0.0,
        1.0,
        true,
        &[
            0,
            65536,
            19 + 29 * 256,
            31 + 43 * 256 + 255 * 65536,
            tile,
            255 + 256,
            73,
            255 * 65536,
        ],
    ))
    .unwrap();
    assert!(g.has_marks());
    assert_eq!(g.marks(15, 15), [19, 29, 255, 43]);
    assert!(g.is_cleared(15, 15) && !g.is_cleared(16, 16));
    assert_eq!(g.marks(17999, 17999), [73, 0, 255, 0]);
    assert_eq!(g.cleared_count(), 257);
    g.apply(&patch(1.0, 1.0, 2.0, false, &[0, 17 + 256, 911 % 256, 0]))
        .unwrap();
    assert_eq!(g.marks(1, 1), [(911 % 256) as u8, 0, 0, 0]);
    assert_eq!(g.marks(2, 1)[0], 19);
    assert_eq!(g.cleared_count(), 256);
    assert_eq!(
        g.cleared_runs(),
        vec![0, 17 * 256, 0, 18 + 238 * 256, tile, 255 + 256]
    );
    let mut red = patch(2.0, 0.0, 1.0, true, &[tile, 255 + 256, 31, 0]);
    red.side = "red".into();
    g.apply(&red).unwrap();
    assert_eq!(g.side.as_deref(), Some("red"));
    assert_eq!(g.marks(0, 0), [0; 4]);
    assert_eq!(g.marks(17999, 17999)[0], 31);
    assert_eq!(g.cleared_count(), 0);
    assert!(g.cleared_runs().is_empty());
}

#[test]
fn marked_cells_are_sparse_and_row_major_at_full_extent() {
    let mut g = LearnedGround::new(1.0, 18000, 18000);
    let edge = 18000 * 18000 - 1;
    let runs = cells(
        18000,
        &[
            (edge, [17, 29, 31, 43, 0]),
            (16, [3, 5, 7, 11, 0]),
            (18000, [1, 0, 0, 0, 0]),
        ],
    );
    g.apply(&patch(1.0, 0.0, 1.0, true, &runs)).unwrap();
    let marked = g.marked_cells();
    let indices: Vec<u32> = marked.iter().step_by(2).copied().collect();
    assert_eq!(indices, vec![16, 18000, edge as u32]);
    assert_eq!(g.marks(17999, 17999), [17, 29, 31, 43]);
    assert_eq!(g.marks(9000, 9000), [0; 4]);
}

#[test]
fn the_shape_counts_holes_edge_tiles_and_varying_exceptions() {
    // 33 x 17: three tiles across, two down; the edge tiles hold one column
    // or one row of real cells.
    let mut g = LearnedGround::new(1.0, 33, 17);
    let mut runs = vec![0, 65536, 19, 0, 1, 65536, 19, 0];
    runs.extend((0..16).flat_map(|row| [2, row * 16 + 256, 19, 0]));
    runs.extend([3, 16 * 256, 19, 0, 4, 16 * 256, 19, 0]);
    g.apply(&patch(1.0, 0.0, 1.0, true, &runs)).unwrap();
    let shape = g.shape();
    assert_eq!((shape.word, shape.exceptions), (19, 1));
    let mut exceptions = vec![];
    g.exceptions(19, None, |t| exceptions.push(t));
    assert_eq!(exceptions, vec![5], "the one tile never learned");
    g.apply(&patch(
        1.0,
        1.0,
        2.0,
        false,
        &[0, 256, 20, 0, 5, 256, 19, 0],
    ))
    .unwrap();
    let mut exceptions = vec![];
    g.exceptions(19, None, |t| exceptions.push(t));
    assert_eq!(exceptions, vec![0]);
}

#[test]
fn a_page_reads_as_its_visible_runs_and_takes_pool_words_while_it_varies() {
    let mut g = LearnedGround::new(1.0, 32, 16);
    g.apply(&patch(
        1.0,
        0.0,
        1.0,
        true,
        &[
            0,
            65536,
            19,
            255 * 65536,
            1,
            256,
            37,
            255 * 65536,
            1,
            1 + 255 * 256,
            19,
            255 * 65536,
        ],
    ))
    .unwrap();
    let mut runs = vec![];
    assert_eq!(g.scar_runs(1, &mut runs), 2);
    assert_eq!(runs, vec![[1, 37 + 255 * 65536], [256, 19 + 255 * 65536]]);
    assert_eq!(g.shape().pool_words, 16);
    g.apply(&patch(1.0, 1.0, 2.0, false, &[1, 256, 19, 255 * 65536]))
        .unwrap();
    assert_eq!(g.shape().pool_words, 0);
}

// --- the cache ---------------------------------------------------------------

/// A ground with `runs` learned, its cache, and the mirror after one upload.
fn cached(cols: usize, rows: usize, runs: &[u32]) -> (LearnedGround, ScarCache, Mirror) {
    let mut g = LearnedGround::new(1.0, cols, rows);
    g.apply(&patch(1.0, 0.0, 1.0, true, runs)).unwrap();
    let mut cache = ScarCache::new(Some((cols, rows, 1.0)));
    let mut mirror = Mirror::new(2048 * 1024);
    assert!(cache.changed(Some(&mut g)));
    mirror.take(&cache.prepare(Some(&mut g), None, false));
    (g, cache, mirror)
}

#[test]
fn edge_scars_at_full_extent_read_back_across_page_joins_from_a_small_upload() {
    let at = [(0, 0), (15, 15), (16, 16), (17, 15), (17999, 17999)];
    let marks: Vec<_> = at
        .iter()
        .enumerate()
        .map(|(k, &(i, j))| {
            let k = k as u32;
            (
                j * 18000 + i,
                [37 + k * 31, 11 + k * 17, 23 + k * 19, 3 + k * 23, 0],
            )
        })
        .collect();
    let (g, cache, mirror) = cached(18000, 18000, &cells(18000, &marks));
    let tiles: Vec<u32> = at.iter().map(|&(i, j)| tile_of(18000, i, j)).collect();
    mirror.matches(&g, &tiles);
    assert!(cache.stats().last_bytes < 256_000);
}

#[test]
fn deltas_rewrite_only_their_pages_and_an_unchanged_frame_uploads_nothing() {
    let (mut g, mut cache, mut mirror) =
        cached(600, 440, &cells(600, &[(80 * 600 + 32, [19, 0, 0, 0, 0])]));
    let full = cache.stats();
    for r in 2..40u32 {
        let index = (r as usize * 977) % (600 * 440);
        let runs = cells(
            600,
            &[
                (index, [r * 5 % 256, r, 7, 0, 0]),
                (index + 1, [r, 0, 0, 0, 255]),
            ],
        );
        g.apply(&patch(1.0, r as f64 - 1.0, r as f64, false, &runs))
            .unwrap();
        assert!(cache.changed(Some(&mut g)));
        let u = cache.prepare(Some(&mut g), None, false);
        assert!(u.texels.len() <= 2048, "only the changed pages");
        mirror.take(&u);
        mirror.matches_marked(&g);
    }
    assert_eq!(cache.stats().full_uploads, full.full_uploads);
    assert!(!cache.changed(Some(&mut g)));
    let u = cache.prepare(Some(&mut g), None, false);
    assert!(u.directory.is_none() && u.writes.is_empty());
}

#[test]
fn uniform_tiles_stay_directory_words_until_a_cell_differs() {
    let (mut g, mut cache, mut mirror) = cached(
        32,
        16,
        &[
            0,
            65536,
            19 + 29 * 256,
            31 + 43 * 256,
            1,
            65536,
            73 + 91 * 256,
            101 + 113 * 256,
        ],
    );
    assert_eq!(
        cache.stats().pool_words,
        4096,
        "nothing past the normalization table"
    );
    mirror.matches(&g, &[0, 1]);
    g.apply(&patch(
        1.0,
        1.0,
        2.0,
        false,
        &[0, 15 + 8 * 16 + 256, 181, 0],
    ))
    .unwrap();
    cache.changed(Some(&mut g));
    let u = cache.prepare(Some(&mut g), None, false);
    assert!(!u.writes.is_empty(), "the tile became a page");
    mirror.take(&u);
    mirror.matches(&g, &[0, 1]);
}

#[test]
fn a_common_word_lists_only_its_exceptions_whatever_region_is_drawn() {
    let mut runs = vec![0, 65536, 19, 0, 1, 65536, 19, 0];
    runs.extend((0..16).flat_map(|row| [2, row * 16 + 256, 19, 0]));
    runs.extend([3, 16 * 256, 19, 0, 4, 16 * 256, 19, 0]);
    let (mut g, mut cache, mut mirror) = cached(33, 17, &runs);
    assert_eq!(mirror.params[16], 19, "the default word");
    assert!(cache.stats().directory_entries < 8);
    assert_eq!(mirror.cell(0, 0), [19, 0, 0, 0]);
    assert_eq!(mirror.cell(32, 16), [0; 4], "the unlearned corner tile");
    g.apply(&patch(
        1.0,
        1.0,
        2.0,
        false,
        &[0, 256, 20, 0, 5, 256, 19, 0],
    ))
    .unwrap();
    cache.changed(Some(&mut g));
    mirror.take(&cache.prepare(Some(&mut g), None, false));
    mirror.matches(&g, &[0, 5]);
    // The cache is global here, so a region changes nothing it holds.
    mirror.take(&cache.prepare(Some(&mut g), Some([0.0, 0.0, 16.0, 16.0]), false));
    assert_eq!(mirror.cell(32, 16), [19, 0, 0, 0]);
}

#[test]
fn churn_compacts_the_pool_and_keeps_the_normalization_words() {
    // A 1,024-word pool, its first 512 held back.
    let mut g = LearnedGround::new(1.0, 832, 16);
    let mut runs = vec![];
    for tile in 0..52 {
        if tile < 32 {
            runs.extend([
                tile,
                136 * 256,
                255,
                0,
                tile,
                136 + 256,
                19,
                0,
                tile,
                137 + 119 * 256,
                255,
                0,
            ]);
        } else {
            runs.extend([tile, 65536, 255, 0]);
        }
    }
    g.apply(&patch(1.0, 0.0, 1.0, true, &runs)).unwrap();
    let mut cache = ScarCache::with_pool(Some((832, 16, 1.0)), 1024, 512);
    let mut mirror = Mirror::new(1024);
    let table: Vec<u8> = (0..512 * 4).map(|i| (i % 251) as u8).collect();
    mirror.pool[..table.len()].copy_from_slice(&table);
    cache.changed(Some(&mut g));
    mirror.take(&cache.prepare(Some(&mut g), None, false));
    let before = cache.stats().full_uploads;
    let mut delta: Vec<u32> = (0..256)
        .flat_map(|c| [0, c + 256, 50 + c % 200, 0])
        .collect();
    delta.extend((1..30).step_by(2).flat_map(|t| [t, 136 + 256, 255, 0]));
    g.apply(&patch(1.0, 1.0, 2.0, false, &delta)).unwrap();
    cache.changed(Some(&mut g));
    mirror.take(&cache.prepare(Some(&mut g), None, false));
    assert_eq!(
        cache.stats().full_uploads,
        before + 1,
        "the pool was repacked"
    );
    assert_eq!(&mirror.pool[..table.len()], &table[..]);
    mirror.matches(&g, &(0..52).collect::<Vec<_>>());
}

#[test]
fn too_many_varying_pages_draw_region_by_region_each_bounded() {
    let tiles_x = 18000u32.div_ceil(16);
    let runs: Vec<u32> = (100..191)
        .flat_map(|y| (30..121).map(move |x| y * tiles_x + x))
        .flat_map(dense)
        .collect();
    let mut g = LearnedGround::new(1.0, 18000, 18000);
    g.apply(&patch(1.0, 0.0, 1.0, true, &runs)).unwrap();
    let mut cache = ScarCache::new(Some((18000, 18000, 1.0)));
    cache.changed(Some(&mut g));
    let regions = cache.regions();
    assert_eq!(regions.len(), 18 * 18);
    let mut mirror = Mirror::new(2048 * 1024);
    let region = [1024.0, 2048.0, 2048.0, 3072.0];
    assert!(regions.contains(&region));
    let u = cache.prepare(Some(&mut g), Some(region), true);
    assert!(u.texels.len() < 8 << 20);
    mirror.take(&u);
    assert_eq!(
        &mirror.params[8..12],
        &region.map(|v| (v as f32).to_bits()),
        "clipped to it"
    );
    // The region's own tiles and its filter halo read as learned.
    let inside: Vec<u32> = (128..192)
        .flat_map(|y| (64..121).map(move |x| y * tiles_x + x))
        .collect();
    mirror.matches(&g, &inside);
    // Erasing the block brings the whole view back.
    let erase: Vec<u32> = (100..191)
        .flat_map(|y| (30..121).map(move |x| y * tiles_x + x))
        .flat_map(|t| [t, 65536, 0, 0])
        .collect();
    g.apply(&patch(1.0, 1.0, 2.0, false, &erase)).unwrap();
    cache.changed(Some(&mut g));
    assert!(cache.regions().is_empty());
}

#[test]
fn a_new_renderer_is_sent_everything_again() {
    let (mut g, mut cache, _) = cached(600, 440, &cells(600, &[(80 * 600 + 32, [19, 3, 0, 0, 0])]));
    assert!(!cache.changed(Some(&mut g)));
    cache.resend();
    assert!(
        cache.changed(Some(&mut g)),
        "what grows on the scars regrows"
    );
    let mut fresh = Mirror::new(2048 * 1024);
    let u = cache.prepare(Some(&mut g), None, false);
    assert!(u.directory.is_some() && !u.writes.is_empty());
    fresh.take(&u);
    fresh.matches_marked(&g);
}

/// A camera over a 100 m square: x and y map straight onto the screen,
/// every point at `w` (nothing in front when `w` ≤ 0).
fn overhead(w: f32) -> [f32; 16] {
    let mut m = [0.0; 16];
    [m[0], m[5], m[12], m[13], m[14], m[15]] = [0.02, 0.02, -1.0, -1.0, 0.5, 1.0];
    m.iter_mut().for_each(|v| *v *= w);
    m
}

#[test]
fn a_region_draws_only_where_it_is_on_screen() {
    use presentation::scars::scissor;
    let map = [0.0, 0.0, -5.0, 100.0, 100.0, 20.0];
    let size = [1000, 500];
    // The lower-left quarter of the map: the screen's lower-left quarter,
    // two pixels wider each side.
    assert_eq!(
        scissor([0.0, 0.0, 50.0, 50.0], Some(&map), &overhead(1.0), size),
        Some([0, 248, 502, 252])
    );
    // Past the screen's edge, or behind the eye: nothing to draw.
    assert_eq!(
        scissor([150.0, 0.0, 200.0, 50.0], Some(&map), &overhead(1.0), size),
        None
    );
    assert_eq!(
        scissor([0.0, 0.0, 50.0, 50.0], Some(&map), &overhead(-1.0), size),
        None
    );
    // Crossing the eye's plane: the whole screen, as nothing bounds it.
    let mut across = overhead(1.0);
    across[11] = -0.1; // w falls with height: the box's top is behind the eye
    assert_eq!(
        scissor([0.0, 0.0, 50.0, 50.0], Some(&map), &across, size),
        Some([0, 0, 1000, 500])
    );
    // A map with no box: nothing bounds the region either.
    assert_eq!(
        scissor([0.0, 0.0, 50.0, 50.0], None, &overhead(1.0), size),
        Some([0, 0, 1000, 500])
    );
}
