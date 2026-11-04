//! The bounded GPU cache of the learned ground's scars: which tiles the
//! directory lists, where each varying page lies in the pool, and what a
//! renderer uploads to follow it.
use crate::records::ScarCacheParams;
use std::collections::BTreeSet;

use serde::Serialize;

use super::ground::{Changes, LearnedGround};
use super::page::{Page, CELLS, TILE};
use super::{
    hash, Ordered, DENSE, DIRECTORY_CAPACITY, NORMALIZATION_WORDS, POOL_WORDS, REGION_CELLS, RUNS,
    UNIFORM,
};

/// What a renderer writes before drawing with the scars.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScarUpload {
    /// The directory's words (two a texel, `rg32uint`) and its texture's
    /// width and height, when it changed.
    pub directory: Option<(Vec<u32>, [u32; 2])>,
    /// Pool writes, each (offset, texels): a run of texels within one row of
    /// the pool, `offset` counted row by row. Their bytes follow one another
    /// in `texels`, four a texel (`rgba8unorm`).
    pub writes: Vec<[u32; 2]>,
    pub texels: Vec<u8>,
    /// The grid, pages, clip, cache bounds and default word.
    pub params: ScarCacheParams,
}

/// What the cache holds and what its uploads wrote.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScarStats {
    pub cols: usize,
    pub rows: usize,
    pub cell_m: f64,
    pub uploads: u64,
    /// Uploads that wrote every page again (a new view, region or default
    /// word, or a compacted pool).
    pub full_uploads: u64,
    pub last_bytes: u64,
    pub total_bytes: u64,
    pub pages: usize,
    pub directory_entries: usize,
    pub pool_words: u32,
    pub default_word: u32,
    pub cache_bounds: [u32; 4],
}

/// A listed tile: one word, or a page of `words` pool words at `offset`
/// holding `count` runs (or its 256 cells when `dense`).
#[derive(Clone, Copy)]
struct Entry {
    word: Option<u32>,
    offset: Option<u32>,
    words: u32,
    count: u32,
    dense: bool,
}

/// A buddy allocator of pool words in blocks of 16 to the pool's size. Each
/// level's free blocks are taken first in, first out.
struct Pool {
    free: Vec<Ordered<u32, ()>>,
}

fn level(words: u32) -> usize {
    words.max(16).next_power_of_two().trailing_zeros() as usize
}

impl Pool {
    fn new(capacity: u32) -> Pool {
        let max = capacity.ilog2() as usize;
        let mut free: Vec<Ordered<u32, ()>> = (0..=max).map(|_| Ordered::default()).collect();
        let mut offset = 0;
        while offset < capacity {
            let l = (capacity - offset).ilog2();
            free[l as usize].insert(offset, ());
            offset += 1 << l;
        }
        Pool { free }
    }

    fn max_level(&self) -> usize {
        self.free.len() - 1
    }

    fn allocate(&mut self, words: u32) -> Option<u32> {
        let want = level(words);
        let mut found = (want..self.free.len()).find(|&l| self.free[l].len() > 0)?;
        let offset = self.free[found].first().unwrap();
        self.free[found].remove(&offset);
        while found > want {
            found -= 1;
            self.free[found].insert(offset + (1 << found), ());
        }
        Some(offset)
    }

    fn release(&mut self, mut offset: u32, words: u32) {
        let mut l = level(words);
        while l < self.max_level() {
            let buddy = offset ^ (1 << l);
            if self.free[l].remove(&buddy).is_none() {
                break;
            }
            offset = offset.min(buddy);
            l += 1;
        }
        self.free[l].insert(offset, ());
    }
}

pub struct ScarCache {
    capacity: u32,
    /// Words at the pool's start the cache never hands out (the
    /// normalization table's).
    reserved: u32,
    width: u32,
    cols: usize,
    rows: usize,
    cell_m: f64,
    /// The crater's view-ray reach, metres: a region loads this much more.
    reach_m: f64,
    regions: Vec<[f64; 4]>,
    /// What changed since the last upload, and whether since `changed`.
    pending: Changes,
    unreported: bool,
    has_marks: bool,
    global: bool,
    shape_word: u32,
    /// What the last upload was made for.
    synced_ground: bool,
    region_key: Option<([f64; 4], f64)>,
    default_word: u32,
    cache_bounds: [u32; 4],
    pages: Ordered<u32, Entry>,
    pool: Pool,
    resident: u32,
    entries: usize,
    runs: Vec<[u32; 2]>,
    stats: ScarStats,
}

impl ScarCache {
    /// The cache for the presentation's ground, in the pool renderers allocate.
    pub fn new(grid: Option<(usize, usize, f64)>) -> ScarCache {
        ScarCache::with_pool(grid, POOL_WORDS, NORMALIZATION_WORDS)
    }

    /// A cache of `capacity` pool words (a power of two), the first
    /// `reserved` held back.
    pub fn with_pool(grid: Option<(usize, usize, f64)>, capacity: u32, reserved: u32) -> ScarCache {
        let (cols, rows, cell_m) = grid.unwrap_or((0, 0, 1.0));
        let span = REGION_CELLS as f64 * cell_m;
        let (w, h) = (cols as f64 * cell_m, rows as f64 * cell_m);
        let mut regions = Vec::new();
        let mut y = 0.0;
        while y < h {
            let mut x = 0.0;
            while x < w {
                regions.push([x, y, (x + span).min(w), (y + span).min(h)]);
                x += span;
            }
            y += span;
        }
        let mut cache = ScarCache {
            capacity,
            reserved,
            width: 1 << libm::log2((capacity as f64).sqrt()).ceil() as u32,
            cols,
            rows,
            cell_m,
            reach_m: 0.0,
            regions,
            pending: Changes::NONE,
            unreported: false,
            has_marks: false,
            global: true,
            shape_word: 0,
            synced_ground: false,
            region_key: None,
            default_word: 0,
            cache_bounds: [0; 4],
            pages: Ordered::default(),
            pool: Pool::new(capacity),
            resident: 0,
            entries: 1,
            runs: Vec::with_capacity(CELLS),
            stats: ScarStats::default(),
        };
        cache.fresh_pool();
        cache.resend();
        cache
    }

    fn fresh_pool(&mut self) {
        self.pool = Pool::new(self.capacity);
        if self.reserved > 0 {
            assert_eq!(
                self.pool.allocate(self.reserved),
                Some(0),
                "the reservation starts the pool"
            );
        }
        self.resident = self.reserved;
    }

    /// The renderer holds nothing (a new one, or scars shown again): the
    /// next upload writes everything.
    pub fn resend(&mut self) {
        self.pending = Changes::All;
        self.unreported = true;
    }

    /// The crater's view-ray reach, metres (`Ground::scar_reach_m`).
    pub fn set_reach(&mut self, reach_m: f64) {
        self.reach_m = reach_m;
    }

    /// Takes what the ground learned since the last look.
    fn drain(&mut self, ground: Option<&mut LearnedGround>) {
        let Some(g) = ground else {
            (self.has_marks, self.global, self.shape_word) = (false, true, 0);
            return;
        };
        let taken = g.take_changes();
        if !taken.is_empty() {
            self.pending.merge(taken, 65536);
            self.unreported = true;
        }
        if self.unreported {
            // Every exception listed, and every varying page resident, at once?
            let shape = g.shape();
            self.has_marks = g.has_marks();
            self.global = shape.exceptions * 2 <= DIRECTORY_CAPACITY
                && shape.pool_words + self.reserved as u64 <= self.capacity as u64;
            self.shape_word = shape.word;
        }
    }

    /// Whether the scars changed since the last call (what grows on them
    /// must grow again).
    pub fn changed(&mut self, ground: Option<&mut LearnedGround>) -> bool {
        self.drain(ground);
        std::mem::take(&mut self.unreported)
    }

    /// The regions drawn one by one, each with only its own marks loaded;
    /// none when every mark fits at once.
    pub fn regions(&self) -> Vec<[f64; 4]> {
        if self.has_marks && !self.global {
            self.regions.clone()
        } else {
            Vec::new()
        }
    }

    /// What to upload before drawing `region` (metres, `[x0, y0, x1, y1]`;
    /// none for the view), clipped to it when `clipped`.
    pub fn prepare(
        &mut self,
        mut ground: Option<&mut LearnedGround>,
        region: Option<[f64; 4]>,
        clipped: bool,
    ) -> ScarUpload {
        self.drain(ground.as_deref_mut());
        let mut upload = ScarUpload::default();
        let margin = 2.0 + self.reach_m / self.cell_m;
        let cached = if self.global { None } else { region };
        self.sync(ground.as_deref(), cached, margin, &mut upload);
        let on = self.cols > 0 && self.rows > 0;
        let inverse = |n: usize| {
            if on {
                (1.0 / (n as f64 * self.cell_m)) as f32
            } else {
                0.0
            }
        };
        let clip = region.filter(|_| clipped).unwrap_or([0.0; 4]);
        upload.params = ScarCacheParams {
            grid: [
                inverse(self.cols),
                inverse(self.rows),
                self.cell_m as f32,
                on as u32 as f32,
            ],
            pages: [
                self.cols as u32,
                self.rows as u32,
                self.entries as u32 - 1,
                self.width,
            ],
            clip: clip.map(|v| v as f32),
            cache_bounds: self.cache_bounds,
            default_word: [self.default_word, 0, 0, 0],
        };
        upload
    }

    pub fn stats(&self) -> ScarStats {
        ScarStats {
            cols: self.cols,
            rows: self.rows,
            cell_m: self.cell_m,
            pages: self.pages.len(),
            directory_entries: self.entries,
            pool_words: self.resident,
            default_word: self.default_word,
            cache_bounds: self.cache_bounds,
            ..self.stats.clone()
        }
    }

    fn release(&mut self, offset: Option<u32>, words: u32) {
        self.pool.release(offset.expect("a resident page"), words);
        self.resident -= words;
    }

    fn sync(
        &mut self,
        ground: Option<&LearnedGround>,
        region: Option<[f64; 4]>,
        margin: f64,
        upload: &mut ScarUpload,
    ) -> bool {
        let changes = std::mem::replace(&mut self.pending, Changes::NONE);
        let region_key = region.map(|r| (r, margin));
        let default_word = ground.map_or(0, |_| self.shape_word);
        let reset = ground.is_some() != self.synced_ground
            || changes == Changes::All
            || region_key != self.region_key
            || default_word != self.default_word;
        if !reset && (ground.is_none() || changes.is_empty()) {
            return false;
        }
        (self.synced_ground, self.region_key, self.default_word) =
            (ground.is_some(), region_key, default_word);
        let (tiles_x, tiles_y) = (
            self.cols.div_ceil(TILE) as i64,
            self.rows.div_ceil(TILE) as i64,
        );
        let tile = |m: f64, pad: f64| ((m / self.cell_m + pad) / TILE as f64).floor() as i64;
        let [lo_x, lo_y, hi_x, hi_y] = match region {
            Some(r) => [
                tile(r[0], -margin).max(0),
                tile(r[1], -margin).max(0),
                tile(r[2], margin).min(tiles_x - 1),
                tile(r[3], margin).min(tiles_y - 1),
            ],
            None => [0, 0, tiles_x - 1, tiles_y - 1],
        };
        self.cache_bounds = [lo_x, lo_y, lo_x.max(hi_x), lo_y.max(hi_y)].map(|v| v as u32);
        let mut dirty = BTreeSet::new();
        if reset {
            self.pages = Ordered::default();
            self.fresh_pool();
            if let Some(g) = ground {
                let bounds = region.map(|_| [lo_x, lo_y, hi_x, hi_y]);
                g.exceptions(default_word, bounds, |t| {
                    dirty.insert(t);
                });
            }
        } else if let Changes::Tiles { tiles, .. } = &changes {
            for &t in tiles {
                let (x, y) = (t as i64 % tiles_x, t as i64 / tiles_x);
                if (lo_x..=hi_x).contains(&x) && (lo_y..=hi_y).contains(&y) {
                    dirty.insert(t);
                }
            }
        }
        let mut directory_changed = reset;
        let mut promoted = Vec::new();
        for &key in &dirty {
            let word = ground.map_or(Some(0), |g| g.uniform(key));
            let old = self.pages.get(&key).copied();
            let old_varying = old.filter(|o| o.word.is_none());
            if word == Some(default_word) {
                if let Some(o) = old {
                    if o.word.is_none() {
                        self.release(o.offset, o.words);
                    }
                    self.pages.remove(&key);
                    directory_changed = true;
                }
                continue;
            }
            if let Some(w) = word {
                if let Some(o) = old_varying {
                    self.release(o.offset, o.words);
                }
                if old.is_none_or(|o| o.word != Some(w)) {
                    self.pages.insert(
                        key,
                        Entry {
                            word: Some(w),
                            offset: None,
                            words: 0,
                            count: 0,
                            dense: false,
                        },
                    );
                    directory_changed = true;
                }
                continue;
            }
            let count = ground.unwrap().scar_runs(key, &mut self.runs) as u32;
            let dense = count > 128;
            let words = Page::words_for(count as usize);
            match old_varying {
                Some(o) if o.words == words => {
                    directory_changed |= o.count != count || o.dense != dense;
                    let entry = self.pages.get_mut(&key).unwrap();
                    (entry.count, entry.dense) = (count, dense);
                }
                _ => {
                    if let Some(o) = old_varying {
                        self.release(o.offset, o.words);
                    }
                    self.pages.insert(
                        key,
                        Entry {
                            word: None,
                            offset: None,
                            words,
                            count,
                            dense,
                        },
                    );
                    promoted.push(key);
                    directory_changed = true;
                }
            }
        }
        let wanted: u32 = promoted
            .iter()
            .map(|k| self.pages.get(k).unwrap().words)
            .sum();
        assert!(
            self.resident + wanted <= self.capacity,
            "ground scar words exceed the bounded resident pool"
        );
        let mut compacted = false;
        for &key in &promoted {
            let words = self.pages.get(&key).unwrap().words;
            match self.pool.allocate(words) {
                Some(offset) => {
                    self.pages.get_mut(&key).unwrap().offset = Some(offset);
                    self.resident += words;
                }
                None => {
                    // Churn scattered the free words. Repack, largest blocks
                    // first: power-of-two sizes fit whenever their sum does.
                    self.fresh_pool();
                    let varying: Vec<u32> = self
                        .pages
                        .iter()
                        .filter(|(_, e)| e.word.is_none())
                        .map(|(&k, _)| k)
                        .collect();
                    for l in (4..=self.pool.max_level()).rev() {
                        for k in &varying {
                            let entry = self.pages.get_mut(k).unwrap();
                            if level(entry.words) != l {
                                continue;
                            }
                            entry.offset = Some(
                                self.pool
                                    .allocate(entry.words)
                                    .expect("compaction fits the admitted words"),
                            );
                            self.resident += entry.words;
                        }
                    }
                    compacted = true;
                    directory_changed = true;
                    break;
                }
            }
        }
        let mut directory_bytes = 0;
        if directory_changed {
            let entries = (self.pages.len() * 2).max(1).next_power_of_two();
            assert!(
                entries <= DIRECTORY_CAPACITY,
                "ground scar directory exceeds its capacity"
            );
            let mut words = vec![0u32; entries * 2];
            let mask = entries as u32 - 1;
            for (&key, page) in self.pages.iter() {
                let mut at = hash(key, mask) as usize;
                while words[at * 2] != 0 {
                    at = (at + 1) & mask as usize;
                }
                let (flag, value) = match (page.word, page.dense) {
                    (Some(w), _) => (UNIFORM, w),
                    (None, true) => (DENSE, page.offset.unwrap()),
                    (None, false) => (RUNS, page.offset.unwrap() * 256 + page.count),
                };
                words[at * 2] = (key + 1) | flag;
                words[at * 2 + 1] = value;
            }
            let width = entries.min(1024) as u32;
            directory_bytes = words.len() as u64 * 4;
            upload.directory = Some((words, [width, entries as u32 / width]));
            self.entries = entries;
        }
        if let Some(g) = ground {
            for (&key, page) in self.pages.iter() {
                if page.word.is_some() || (!reset && !compacted && !dirty.contains(&key)) {
                    continue;
                }
                let count = g.scar_runs(key, &mut self.runs);
                let start = upload.texels.len();
                if page.dense {
                    let mut prior = 0;
                    for &[end, word] in &self.runs[..count] {
                        for _ in prior..end {
                            upload.texels.extend(word.to_le_bytes());
                        }
                        prior = end;
                    }
                } else {
                    for &[end, word] in &self.runs[..count] {
                        upload.texels.extend(end.to_le_bytes());
                        upload.texels.extend(word.to_le_bytes());
                    }
                }
                // A page lies within one row of a pool at least 256 wide;
                // a narrower pool's rows split it.
                let (mut offset, mut left) = (
                    page.offset.unwrap(),
                    ((upload.texels.len() - start) / 4) as u32,
                );
                while left > 0 {
                    let n = left.min(self.width - offset % self.width);
                    upload.writes.push([offset, n]);
                    (offset, left) = (offset + n, left - n);
                }
            }
        }
        let bytes = upload.texels.len() as u64 + directory_bytes;
        let s = &mut self.stats;
        (s.last_bytes, s.total_bytes, s.uploads) = (bytes, s.total_bytes + bytes, s.uploads + 1);
        s.full_uploads += (reset || compacted) as u64;
        true
    }
}
