//! The side's learned ground and the GPU's cache of its scars:
//! [`LearnedGround`] keeps what the side's publications taught it, cell by
//! cell, and [`ScarCache`] turns it into what a renderer uploads as it is:
//! the directory's words, the pool's texels and `ScarCacheParams`. The shared
//! `scars.wgsl` shader reads them
//! (`scarEntry`, `scarCell`); a renderer derives nothing.
//!
//! A tile the directory lists is either one word (every cell alike) or a
//! page in the pool: its runs (exclusive end, word pairs) or, past 128 runs,
//! its 256 cells. A tile it does not list inside the cache's bounds is the
//! common word. When every exception fits, the whole ground is cached at
//! once; otherwise each drawn region loads its own, with the filter's halo.
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

mod cache;
mod ground;
mod page;

pub use cache::{ScarCache, ScarStats, ScarUpload};
pub use ground::{Changes, LearnedGround, Shape};
pub use page::TILE;

/// A directory entry's flags over its tile key + 1.
pub const UNIFORM: u32 = 0x8000_0000;
pub const RUNS: u32 = 0x4000_0000;
pub const DENSE: u32 = 0x2000_0000;
pub const KEY_MASK: u32 = 0x1fff_ffff;
/// The most directory entries the cache holds.
pub const DIRECTORY_CAPACITY: usize = 1 << 22;
/// The pool texture (`rgba8unorm`, one word a texel).
pub const POOL_WIDTH: u32 = 2048;
pub const POOL_HEIGHT: u32 = 1024;
pub const POOL_WORDS: u32 = POOL_WIDTH * POOL_HEIGHT;
/// The pool's first words: the shader's table from a filtered byte (in
/// sixteenths) to its fraction of a full mark, as f32 bits.
pub const NORMALIZATION_WORDS: u32 = 4096;
/// The side of a region drawn alone when the marks don't fit at once, cells.
pub const REGION_CELLS: usize = 1024;
/// A grid axis past this many cells loses Q8 positions in the shader's f32.
pub const MAX_AXIS: usize = (1 << 24) / 256;

/// The normalization table's texels, as the pool's first bytes.
pub fn normalization() -> Vec<u8> {
    (0..NORMALIZATION_WORDS)
        .flat_map(|q| ((q.min(4080) as f64 / 4080.0) as f32).to_le_bytes())
        .collect()
}

/// The pixels of a `size` target that a scar region (metres, `[x0, y0, x1,
/// y1]`) can cover, through the map's box (`shadow::map_box`, its heights)
/// and the frame's `view_proj`: `[x, y, width, height]`, two pixels wider
/// each side, or none when the region is off screen or behind the eye. A
/// box crossing the eye's plane has no bound on screen, and a map without a
/// box has no heights to bound it: the whole target.
pub fn scissor(
    region: [f64; 4],
    map_box: Option<&[f64; 6]>,
    view_proj: &[f32; 16],
    size: [u32; 2],
) -> Option<[u32; 4]> {
    let Some(map_box) = map_box else {
        return Some([0, 0, size[0], size[1]]);
    };
    let m = view_proj.map(f64::from);
    let [w, h] = size.map(f64::from);
    let (mut front, mut lo, mut hi) = (0, [f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for x in [region[0], region[2]] {
        for y in [region[1], region[3]] {
            for z in [map_box[2], map_box[5]] {
                let clip = |r: usize| m[r] * x + m[4 + r] * y + m[8 + r] * z + m[12 + r];
                let cw = clip(3);
                if cw <= 0.0 {
                    continue;
                }
                front += 1;
                for axis in 0..2 {
                    let ndc = clip(axis) / cw;
                    lo[axis] = lo[axis].min(ndc);
                    hi[axis] = hi[axis].max(ndc);
                }
            }
        }
    }
    if front == 0 {
        return None;
    }
    if front < 8 {
        return Some([0, 0, size[0], size[1]]);
    }
    let x = (((lo[0] + 1.0) * w / 2.0).floor() - 2.0).max(0.0);
    let y = (((1.0 - hi[1]) * h / 2.0).floor() - 2.0).max(0.0);
    let right = (((hi[0] + 1.0) * w / 2.0).ceil() + 2.0).min(w);
    let bottom = (((1.0 - lo[1]) * h / 2.0).ceil() + 2.0).min(h);
    (right > x && bottom > y).then_some([
        x as u32,
        y as u32,
        (right - x) as u32,
        (bottom - y) as u32,
    ])
}

/// The directory's hash of a tile key.
pub fn hash(key: u32, mask: u32) -> u32 {
    (key.wrapping_mul(crate::seed::GOLDEN) ^ (key >> 16)) & mask
}

/// A map iterated in insertion order: setting a key keeps its place, and a
/// key removed and set again goes last. The cache's directory and pool
/// follow this order, so it is part of what they hold.
pub(crate) struct Ordered<K, V> {
    at: HashMap<K, (u64, V)>,
    order: BTreeMap<u64, K>,
    next: u64,
}

impl<K, V> Default for Ordered<K, V> {
    fn default() -> Self {
        Ordered {
            at: HashMap::new(),
            order: BTreeMap::new(),
            next: 0,
        }
    }
}

impl<K: Hash + Eq + Copy, V> Ordered<K, V> {
    pub fn len(&self) -> usize {
        self.at.len()
    }
    pub fn get(&self, k: &K) -> Option<&V> {
        self.at.get(k).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.at.get_mut(k).map(|(_, v)| v)
    }
    pub fn insert(&mut self, k: K, v: V) {
        match self.at.get_mut(&k) {
            Some(slot) => slot.1 = v,
            None => {
                self.order.insert(self.next, k);
                self.at.insert(k, (self.next, v));
                self.next += 1;
            }
        }
    }
    pub fn get_or_insert_with(&mut self, k: K, v: impl FnOnce() -> V) -> &mut V {
        if !self.at.contains_key(&k) {
            self.insert(k, v());
        }
        self.get_mut(&k).unwrap()
    }
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let (seq, v) = self.at.remove(k)?;
        self.order.remove(&seq);
        Some(v)
    }
    pub fn first(&self) -> Option<K> {
        self.order.values().next().copied()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.order.values().map(|k| (k, &self.at[k].1))
    }
}
