//! Fog of war's sight: what the sight-light compute (`fogTerrain`,
//! `fogMerge`, `fogCull`, `fogWhole`) and every fogged material read, decided
//! here for both clients, which upload it as it is (spike 02's technique).
//!
//! - **The static world** sight is cut by ([`SightWorld`]): the simulation's
//!   terrain grid (the ground's height pages, which the renderer already
//!   holds), its playable rectangle, the sensor numbers fog shares with the
//!   simulation's sweep, and the map's occluding props.
//! - **The side's foliage**: the simulation's export, less what the side has
//!   seen cleared (the client hands it over; the rule is the simulation
//!   world's).
//! - **The side's sight**, per publication: every own eye at the published
//!   tick (never interpolated), and the occluders it knows stand: the map's
//!   occluding props less those it has seen fall, plus the obstacles, ruins
//!   and wrecks it has learned. A prop the side has not learned never
//!   occludes, so fog never shows what the side cannot know.
//!
//! Each eye keeps a horizon map, `azimuth_bins × radial_bins` words: a
//! coarse terrain march, then a merge at full azimuth that meets only the
//! known occluders within reach. New eyes, and eyes an occluder change
//! touches, rebuild at once; moved eyes `rebuild_eyes_per_frame` at a time,
//! oldest first. The structures that take fog whole are flagged after the
//! maps or the eyes change. [`FogSight::prepare`] is one drawn frame's
//! uploads and dispatches ([`FogUpload`]).
//!
//! `tests/fog_sight.rs` holds every upload to the recorded vectors word for
//! word.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sim::publication::reader::{KnownProp, Sight};
use sim::world::export::PROP_STRIDE;

pub use super::upload::{FogUpload, WholePass};
use crate::records::{Camera, FogBox, FogEye, FogParams};
use crate::static_world::ground::Source;

/// A map prop's id is `lo + hi · 2^16` (the layout's `limbBits`).
const LIMB: f64 = 65536.0;

/// Foliage depth is stored per bin in steps of this, in 8 bits.
pub const FOLIAGE_STEP: f64 = 0.005;
pub const FOLIAGE_QUANTA: u32 = 255;
pub const FOLIAGE_MAX: f64 = FOLIAGE_QUANTA as f64 * FOLIAGE_STEP;
/// The cull tile extent and log-spaced samples per sight ray.
pub const TILE_PX: u32 = 16;
pub const RADIAL_BINS: u32 = 64;
/// Words a row of the `wholes` texture holds.
pub const WHOLE_TEXTURE_WIDTH: usize = 256;
/// Words per structure box in the `wholes` texture: centre x, y; cos and
/// sin of its yaw; half extents along and across it; base and top (f32).
pub const WHOLE_BOX_WORDS: usize = 8;
/// The whole-fog grid's smallest cell, metres, and most cells a side.
const WHOLE_CELL_M: f64 = 8.0;
const WHOLE_GRID_MAX: f64 = 512.0;
/// The angular sectors a rebuilt eye's occluders are listed by.
pub const HORIZON_SECTORS: usize = 64;
/// The build compute's workgroup, and the most workgroups a dispatch takes.
pub const BUILD_WORKGROUP: usize = 64;
const MAX_WORKGROUPS: usize = 65535;

/// `presentation.fog_geometry`: the sight lights' resolution and budgets.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    /// Rays per eye of the full map, where buildings are met exactly.
    pub azimuth_bins: u32,
    /// Rays per eye of the coarse terrain march.
    pub terrain_azimuth_bins: u32,
    pub first_bin_m: f64,
    /// The terrain march's step: `fraction` of the distance, within `[min, max]` m.
    pub terrain_step_m: [f64; 2],
    pub terrain_step_fraction: f64,
    /// Eyes a tile's list holds; more are dropped.
    pub tile_eyes_max: u32,
    /// A face probes this far outside itself, along its normal.
    pub face_probe_m: f64,
    /// A roof or canopy top above an eye looks this far toward it for seen air.
    pub roof_reach_m: f64,
    /// Moved eyes rebuilt per frame; new eyes are always built at once.
    pub rebuild_eyes_per_frame: u32,
    /// A structure taking fog whole is sampled this far apart.
    pub whole_step_m: f64,
}

impl Geometry {
    /// Refuses a geometry the passes cannot run, by its field.
    pub fn check(&self) -> Result<(), String> {
        let whole = |v: u32, name: &str, min: u32| {
            if v < min {
                Err(format!("fog_geometry.{name} must be an integer ≥ {min}"))
            } else {
                Ok(())
            }
        };
        whole(self.azimuth_bins, "azimuth_bins", 8)?;
        whole(self.terrain_azimuth_bins, "terrain_azimuth_bins", 8)?;
        whole(self.tile_eyes_max, "tile_eyes_max", 1)?;
        whole(self.rebuild_eyes_per_frame, "rebuild_eyes_per_frame", 1)?;
        if !(self.first_bin_m > 0.0 && self.face_probe_m > 0.0 && self.terrain_step_fraction > 0.0)
        {
            return Err(
                "fog_geometry: first_bin_m, face_probe_m and terrain_step_fraction must be > 0"
                    .into(),
            );
        }
        if self.roof_reach_m.is_nan() || self.roof_reach_m < 0.0 {
            return Err("fog_geometry.roof_reach_m must be ≥ 0".into());
        }
        if self.whole_step_m.is_nan() || self.whole_step_m < 0.25 {
            return Err("fog_geometry.whole_step_m must be ≥ 0.25".into());
        }
        let [lo, hi] = self.terrain_step_m;
        if !(lo > 0.0 && hi >= lo) {
            return Err("fog_geometry.terrain_step_m must be 0 < min ≤ max".into());
        }
        Ok(())
    }
}

/// The sensor rules fog shares with the simulation's sweep (`rules.sensors`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Sensors {
    pub fog_target_height_m: f64,
    pub foliage_full_block: f64,
    /// Least room a line of sight needs between occluders on its two sides.
    pub min_sight_gap_m: f64,
}

/// An oriented box that hides what lies behind it, and takes fog whole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Occluder {
    pub x: f64,
    pub y: f64,
    pub yaw: f64,
    /// Half extents along and across the heading.
    pub hx: f64,
    pub hy: f64,
    /// Heights of its base and its top.
    pub base: f64,
    pub top: f64,
}

impl Occluder {
    /// Its value, for telling a box that moved from one that stayed (−0
    /// and +0 alike, as the numbers' text is).
    fn key(&self) -> [u64; 7] {
        [
            self.x, self.y, self.hx, self.hy, self.yaw, self.base, self.top,
        ]
        .map(|v| (v + 0.0).to_bits())
    }
}

/// The static world sight is cut by: the height grid's size, the playable
/// rectangle, the sensor numbers, and the map's occluding props (with each
/// one's map id) and which prop kinds occlude.
#[derive(Clone, Debug)]
pub struct SightWorld {
    pub nx: u32,
    pub ny: u32,
    pub spacing: f64,
    /// As the map's extents give it (`[x, y, width, height]` from the origin).
    pub playable: [f64; 4],
    pub sensors: Sensors,
    pub map: Vec<Occluder>,
    pub ids: Vec<f64>,
    pub occluding: HashSet<String>,
}

impl SightWorld {
    /// The world of the ground `source`, under `sensors`.
    pub fn of(source: &Source, sensors: Sensors) -> SightWorld {
        let mut map = Vec::new();
        let mut ids = Vec::new();
        for p in source.props.as_chunks::<PROP_STRIDE>().0 {
            // idLo, idHi, kind, x, y, yaw, hx, hy, hz, baseZ, wreckOf.
            if !source
                .kinds
                .occludes
                .get(p[2] as usize)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            ids.push(p[0] as f64 + p[1] as f64 * LIMB);
            map.push(Occluder {
                x: p[3] as f64,
                y: p[4] as f64,
                yaw: p[5] as f64,
                hx: p[6] as f64,
                hy: p[7] as f64,
                base: p[9] as f64,
                top: p[9] as f64 + 2.0 * p[8] as f64,
            });
        }
        let kinds = &source.kinds;
        SightWorld {
            nx: source.grid.nx as u32,
            ny: source.grid.ny as u32,
            spacing: source.grid.spacing,
            playable: source.playable,
            sensors,
            map,
            ids,
            occluding: kinds
                .names
                .iter()
                .zip(&kinds.occludes)
                .filter(|(_, &o)| o)
                .map(|(n, _)| n.clone())
                .collect(),
        }
    }

    /// Refuses sensor numbers the maps cannot store.
    pub fn check(&self) -> Result<(), String> {
        // Foliage saturates at 8 bits: the full block must fall inside that.
        if self.sensors.foliage_full_block > FOLIAGE_MAX {
            return Err(format!("fog: foliage_full_block must be ≤ {FOLIAGE_MAX}"));
        }
        Ok(())
    }
}

/// A prop the side knows, as sight reads it.
pub struct KnownBox<'a> {
    pub kind: &'a str,
    pub center: [f64; 2],
    pub yaw: f64,
    pub half: [f64; 3],
    pub base_z: f64,
    /// The map prop it is (or replaces): that one no longer stands as mapped.
    pub authored: Option<f64>,
    pub destroyed: bool,
}

impl<'a> KnownBox<'a> {
    pub fn published(k: &'a KnownProp) -> Self {
        KnownBox {
            kind: &k.kind,
            center: k.center,
            yaw: k.yaw,
            half: k.half,
            base_z: k.base_z,
            authored: k.authored_prop,
            destroyed: k.destroyed,
        }
    }
}

/// A sight shape's reach multipliers ahead, abeam and behind.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Shape {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
}

/// One own eye at the published tick.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Eye {
    /// Stable per unit and eye index (`<unit>:<eye>`), so an eye that did
    /// not move keeps its map.
    pub key: String,
    pub position: [f64; 3],
    pub forward: f64,
    pub shape: Shape,
    pub range: f64,
}

impl Eye {
    /// How far it reaches in any direction: its map's radius.
    pub fn reach(&self) -> f64 {
        let s = self.shape;
        self.range * s.front.max(s.side).max(s.rear)
    }

    /// Every eye of the side's own units, each `(id, sight)` at the
    /// published tick: a garrison gives one per facade it holds.
    pub fn of_units<'a>(units: impl IntoIterator<Item = (f64, &'a Sight)>) -> Vec<Eye> {
        units
            .into_iter()
            .flat_map(|(id, sight)| {
                sight
                    .eyes
                    .iter()
                    .enumerate()
                    .map(move |(k, &position)| Eye {
                        key: format!("{id}:{k}"),
                        position,
                        forward: sight.forward,
                        shape: Shape {
                            front: sight.shape.front,
                            side: sight.shape.side,
                            rear: sight.shape.rear,
                        },
                        range: sight.range,
                    })
            })
            .collect()
    }
}

/// The structures that take fog whole, as `wholes` texture words: a grid of
/// cells over their footprints (each `start << 8 | count` into the item
/// lists), the lists, the boxes (`WHOLE_BOX_WORDS` each), then a seen flag
/// per box from `flags_row` on.
#[derive(Clone, Debug, Default)]
pub struct Wholes {
    pub words: Vec<u32>,
    pub rows: u32,
    pub flags_row: u32,
    pub flag_rows: u32,
    pub count: u32,
    pub nx: u32,
    pub ny: u32,
    pub cell_m: f64,
    pub origin: [f64; 2],
    pub items_base: u32,
    pub boxes_base: u32,
}

impl Wholes {
    fn float(&self, at: usize) -> f64 {
        f32::from_bits(self.words[at]) as f64
    }

    /// The box `index`'s centre and half extents, as the GPU reads them.
    fn box_at(&self, index: u32) -> ([f64; 2], [f64; 2]) {
        let b = self.boxes_base as usize + index as usize * WHOLE_BOX_WORDS;
        (
            [self.float(b), self.float(b + 1)],
            [self.float(b + 4), self.float(b + 5)],
        )
    }
}

fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// The lookup grid over `boxes`, each footprint grown by `margin`.
pub fn whole_words(boxes: &[Occluder], margin: f64) -> Result<Wholes, String> {
    let n = boxes.len();
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    // Each grown box's own bounds: grown in its frame, then turned.
    let extents: Vec<[f64; 2]> = boxes
        .iter()
        .map(|b| {
            let c = libm::cos(b.yaw).abs();
            let s = libm::sin(b.yaw).abs();
            let ex = (b.hx + margin) * c + (b.hy + margin) * s;
            let ey = (b.hx + margin) * s + (b.hy + margin) * c;
            min_x = min_x.min(b.x - ex);
            min_y = min_y.min(b.y - ey);
            max_x = max_x.max(b.x + ex);
            max_y = max_y.max(b.y + ey);
            [ex, ey]
        })
        .collect();
    if n == 0 {
        (min_x, min_y, max_x, max_y) = (0.0, 0.0, 1.0, 1.0);
    }
    let cell = WHOLE_CELL_M.max((max_x - min_x).max(max_y - min_y) / WHOLE_GRID_MAX);
    // One more than the far edge's cell: a point exactly on it still has one.
    let nx = ((max_x - min_x) / cell).floor() as usize + 1;
    let ny = ((max_y - min_y) / cell).floor() as usize + 1;
    let clamp = |v: f64, hi: usize| (v.floor().max(0.0) as usize).min(hi);
    // Each box's cell range, then the lists by counting.
    let ranges: Vec<[usize; 4]> = boxes
        .iter()
        .zip(&extents)
        .map(|(b, &[ex, ey])| {
            [
                clamp((b.x - ex - min_x) / cell, nx - 1),
                clamp((b.x + ex - min_x) / cell, nx - 1),
                clamp((b.y - ey - min_y) / cell, ny - 1),
                clamp((b.y + ey - min_y) / cell, ny - 1),
            ]
        })
        .collect();
    let mut counts = vec![0u32; nx * ny];
    for r in &ranges {
        for j in r[2]..=r[3] {
            for k in r[0]..=r[1] {
                counts[j * nx + k] += 1;
            }
        }
    }
    let mut items = 0usize;
    for &count in &counts {
        if count > 255 {
            return Err("fog: more than 255 structures share a whole-fog cell".into());
        }
        items += count as usize;
    }
    let items_base = nx * ny;
    let boxes_base = items_base + items;
    let w = WHOLE_TEXTURE_WIDTH;
    let flags_row = (boxes_base + n * WHOLE_BOX_WORDS).div_ceil(w);
    let flag_rows = n.div_ceil(w).max(1);
    let mut words = vec![0u32; (flags_row + flag_rows) * w];
    let mut next = vec![0usize; nx * ny];
    let mut start = 0usize;
    for (c, &count) in counts.iter().enumerate() {
        words[c] = ((start as u32) << 8) | count;
        next[c] = items_base + start;
        start += count as usize;
    }
    for (i, r) in ranges.iter().enumerate() {
        for j in r[2]..=r[3] {
            for k in r[0]..=r[1] {
                words[next[j * nx + k]] = i as u32;
                next[j * nx + k] += 1;
            }
        }
    }
    for (i, b) in boxes.iter().enumerate() {
        let at = boxes_base + i * WHOLE_BOX_WORDS;
        let row = [
            b.x,
            b.y,
            libm::cos(b.yaw),
            libm::sin(b.yaw),
            b.hx,
            b.hy,
            b.base,
            b.top,
        ];
        for (k, v) in row.into_iter().enumerate() {
            words[at + k] = (v as f32).to_bits();
        }
    }
    Ok(Wholes {
        words,
        rows: (flags_row + flag_rows) as u32,
        flags_row: flags_row as u32,
        flag_rows: flag_rows as u32,
        count: n as u32,
        nx: nx as u32,
        ny: ny as u32,
        cell_m: cell,
        origin: [min_x, min_y],
        items_base: items_base as u32,
        boxes_base: boxes_base as u32,
    })
}

/// How far the broad phase grows past `reach`: eight f32 rounding units of
/// the largest number it measures.
fn broad_pad(grid: &Wholes, [x, y, _]: [f64; 3], reach: f64) -> f64 {
    let [ox, oy] = grid.origin;
    let (nx, ny, cell) = (grid.nx as f64, grid.ny as f64, grid.cell_m);
    8.0 * 2f64.powi(-23)
        * [
            x.abs(),
            y.abs(),
            reach,
            ox.abs() + nx * cell,
            oy.abs() + ny * cell,
        ]
        .into_iter()
        .fold(1.0, f64::max)
}

/// The boxes of `grid` whose bounding circle comes within `reach` (and
/// `query_margin` more, to find them) of `position`, in index order. The
/// broad phase grows by eight f32 rounding units, as the GPU reads f32.
pub fn in_reach(grid: &Wholes, position: [f64; 3], reach: f64, query_margin: f64) -> Vec<u32> {
    if grid.count == 0 {
        return Vec::new();
    }
    let [x, y, _] = position;
    let [ox, oy] = grid.origin;
    let (nx, ny, cell) = (grid.nx as f64, grid.ny as f64, grid.cell_m);
    let pad = broad_pad(grid, position, reach);
    let query = reach + query_margin;
    let lo_x = ((x - query - pad - ox) / cell).floor().max(0.0);
    let hi_x = (nx - 1.0).min(((x + query + pad - ox) / cell).floor());
    let lo_y = ((y - query - pad - oy) / cell).floor().max(0.0);
    let hi_y = (ny - 1.0).min(((y + query + pad - oy) / cell).floor());
    // A box spans several cells: each is judged once (`seen`), and those in
    // reach are marked (`marked`), read back in index order from the words
    // touched: a bit per box, no sort of the boxes themselves.
    let words = (grid.count as usize).div_ceil(64);
    let (mut seen, mut marked) = (vec![0u64; words], vec![0u64; words]);
    let mut touched: Vec<usize> = Vec::new();
    let mut j = lo_y;
    while j <= hi_y {
        let mut i = lo_x;
        while i <= hi_x {
            let cell_word = grid.words[j as usize * grid.nx as usize + i as usize];
            let start = grid.items_base as usize + (cell_word >> 8) as usize;
            for &index in &grid.words[start..start + (cell_word & 255) as usize] {
                let (word, bit) = ((index / 64) as usize, 1u64 << (index % 64));
                if seen[word] & bit != 0 {
                    continue;
                }
                if seen[word] == 0 {
                    touched.push(word);
                }
                seen[word] |= bit;
                let (center, half) = grid.box_at(index);
                let radius = length(half[0], half[1]);
                let (dx, dy) = (x - center[0], y - center[1]);
                if dx * dx + dy * dy <= (reach + radius + pad) * (reach + radius + pad) {
                    marked[word] |= bit;
                }
            }
            i += 1.0;
        }
        j += 1.0;
    }
    touched.sort_unstable();
    let mut found = Vec::new();
    for word in touched {
        let mut bits = marked[word];
        while bits != 0 {
            found.push(word as u32 * 64 + bits.trailing_zeros());
            bits &= bits - 1;
        }
    }
    found
}

/// Of `wide`, an eye's boxes found with a query margin, those `in_reach`
/// finds with none: the same circle test, over a larger square of cells.
fn within(grid: &Wholes, wide: &[u32], position: [f64; 3], reach: f64) -> Vec<u32> {
    let pad = broad_pad(grid, position, reach);
    let [x, y, _] = position;
    wide.iter()
        .copied()
        .filter(|&index| {
            let (center, half) = grid.box_at(index);
            let radius = length(half[0], half[1]);
            let (dx, dy) = (x - center[0], y - center[1]);
            dx * dx + dy * dy <= (reach + radius + pad) * (reach + radius + pad)
        })
        .collect()
}

/// The rebuilt eyes' occluder lists for the merge: per eye `[eye index,
/// sector table offset, sector count]`, then each eye's sector table of
/// `[candidate offset, count]` pairs, then the candidates. A box is listed
/// in every sector its bounding circle (grown by `margin`) may cross, one
/// more at either edge for f32 ray angles; all of them when the eye stands
/// inside it. Each list keeps the boxes' order, so equal slopes keep the
/// first box's jump.
pub fn horizon_records(
    grid: &Wholes,
    eyes: &[([f64; 3], f64)],
    indices: &[u32],
    margin: f64,
) -> Vec<u32> {
    let found: Vec<Vec<u32>> = eyes
        .iter()
        .map(|&(p, reach)| in_reach(grid, p, reach, 0.0))
        .collect();
    horizon_records_of(grid, eyes, &found, indices, margin)
}

/// `horizon_records` over each eye's boxes in reach, `found`.
fn horizon_records_of(
    grid: &Wholes,
    eyes: &[([f64; 3], f64)],
    found: &[Vec<u32>],
    indices: &[u32],
    margin: f64,
) -> Vec<u32> {
    let scale = HORIZON_SECTORS as f64 / (std::f64::consts::PI * 2.0);
    let sectors = HORIZON_SECTORS as i64;
    let spans: Vec<(&Vec<u32>, Vec<[i64; 2]>)> = eyes
        .iter()
        .zip(found)
        .map(|(&(position, _), found)| {
            let (px, py) = (position[0] as f32 as f64, position[1] as f32 as f64);
            let range = found
                .iter()
                .map(|&index| {
                    let (center, half) = grid.box_at(index);
                    let distance = length(px - center[0], py - center[1]);
                    let radius = length(half[0], half[1]) + margin;
                    let (mut lo, mut hi) = (0, sectors - 1);
                    if distance > radius * (1.0 + 8.0 * 2f64.powi(-23)) {
                        let angle = libm::atan2(center[1] - py, center[0] - px);
                        let half = libm::asin((radius / distance).min(1.0));
                        let pi = std::f64::consts::PI;
                        let a = ((angle - half + pi) * scale).floor() as i64 - 1;
                        let z = ((angle + half + pi) * scale).floor() as i64 + 1;
                        if z - a + 1 < sectors {
                            (lo, hi) = (a, z);
                        }
                    }
                    [lo, hi]
                })
                .collect();
            (found, range)
        })
        .collect();
    let headers = eyes.len() * 3;
    let table_words = eyes.len() * HORIZON_SECTORS * 2;
    let candidates: usize = spans
        .iter()
        .flat_map(|(_, r)| r.iter().map(|[lo, hi]| (hi - lo + 1) as usize))
        .sum();
    let mut records = vec![0u32; headers + table_words + candidates];
    let sector = |s: i64| s.rem_euclid(sectors) as usize;
    let mut at = headers + table_words;
    for (r, (found, range)) in spans.iter().enumerate() {
        let table = headers + r * HORIZON_SECTORS * 2;
        records[r * 3..r * 3 + 3].copy_from_slice(&[
            indices[r],
            table as u32,
            HORIZON_SECTORS as u32,
        ]);
        let mut sizes = [0usize; HORIZON_SECTORS];
        for &[lo, hi] in range {
            for s in lo..=hi {
                sizes[sector(s)] += 1;
            }
        }
        let mut cursor = [0usize; HORIZON_SECTORS];
        for s in 0..HORIZON_SECTORS {
            records[table + s * 2] = at as u32;
            records[table + s * 2 + 1] = sizes[s] as u32;
            cursor[s] = at;
            at += sizes[s];
        }
        for (&index, &[lo, hi]) in found.iter().zip(range) {
            for s in lo..=hi {
                records[cursor[sector(s)]] = index;
                cursor[sector(s)] += 1;
            }
        }
    }
    records
}

/// The whole-fog pass's rows: per reachable structure, in box order,
/// `[box, eyes offset, eye count]`, then each row's eyes in eye order.
/// Returns the row count and the words.
pub fn whole_records(
    grid: &Wholes,
    eyes: &[([f64; 3], f64)],
    query_margin: f64,
) -> (u32, Vec<u32>) {
    whole_records_kept(grid, eyes, query_margin, &mut Kept::default())
}

/// Each eye's structures in reach on one grid, by where it stands and how
/// far it sees, as the last whole-fog pass found them.
type Kept = HashMap<[u64; 3], Arc<Vec<u32>>>;

/// `whole_records`, an eye standing where it stood at the last call (on
/// the same grid, `kept`) reusing its list rather than searching again:
/// most have not moved. The records are the same either way.
fn whole_records_kept(
    grid: &Wholes,
    eyes: &[([f64; 3], f64)],
    query_margin: f64,
    kept: &mut Kept,
) -> (u32, Vec<u32>) {
    let mut lists = Kept::new();
    let found: Vec<Arc<Vec<u32>>> = eyes
        .iter()
        .map(|&(p, reach)| {
            let key = [p[0].to_bits(), p[1].to_bits(), reach.to_bits()];
            lists
                .entry(key)
                .or_insert_with(|| {
                    kept.get(&key)
                        .cloned()
                        .unwrap_or_else(|| Arc::new(in_reach(grid, p, reach, query_margin)))
                })
                .clone()
        })
        .collect();
    *kept = lists;
    let boxes = grid.count as usize;
    let mut eyes_of = vec![0u32; boxes];
    let mut pairs = 0usize;
    for list in &found {
        for &index in list.iter() {
            eyes_of[index as usize] += 1;
            pairs += 1;
        }
    }
    let count = eyes_of.iter().filter(|&&n| n > 0).count();
    let mut words = vec![0u32; count * 3 + pairs];
    let mut cursor = vec![0usize; boxes];
    let (mut row, mut at) = (0usize, count * 3);
    for b in 0..boxes {
        if eyes_of[b] == 0 {
            continue;
        }
        words[row * 3..row * 3 + 3].copy_from_slice(&[b as u32, at as u32, eyes_of[b]]);
        cursor[b] = at;
        at += eyes_of[b] as usize;
        row += 1;
    }
    for (e, list) in found.iter().enumerate() {
        for &index in list.iter() {
            words[cursor[index as usize]] = e as u32;
            cursor[index as usize] += 1;
        }
    }
    (count as u32, words)
}

/// A known box: its map prop's index, which stays the same box for the
/// world's life, or a box the side learned.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Identity {
    Map(usize),
    Learned,
}

/// The eyes whose maps an occluder change touches: a box removed touches
/// the eyes that reached it, a box added the eyes that reach it. A map box
/// kept is unchanged; a learned box that reappears with the same numbers
/// is too.
fn affected_eyes(
    before: &[(Identity, Occluder)],
    before_grid: &Wholes,
    after: &[(Identity, Occluder)],
    after_grid: &Wholes,
    eyes: &[([f64; 3], f64)],
) -> Vec<usize> {
    let map_ids = |boxes: &[(Identity, Occluder)]| -> HashSet<usize> {
        boxes
            .iter()
            .filter_map(|(id, _)| match id {
                Identity::Map(i) => Some(*i),
                Identity::Learned => None,
            })
            .collect()
    };
    let (kept, had) = (map_ids(after), map_ids(before));
    let stays =
        |id: &Identity, other: &HashSet<usize>| matches!(id, Identity::Map(i) if other.contains(i));
    let gone: Vec<usize> = (0..before.len())
        .filter(|&i| !stays(&before[i].0, &kept))
        .collect();
    let come: Vec<usize> = (0..after.len())
        .filter(|&i| !stays(&after[i].0, &had))
        .collect();
    let gone_keys: HashSet<[u64; 7]> = gone.iter().map(|&i| before[i].1.key()).collect();
    let come_keys: HashSet<[u64; 7]> = come.iter().map(|&i| after[i].1.key()).collect();
    let removed: HashSet<u32> = gone
        .into_iter()
        .filter(|&i| !come_keys.contains(&before[i].1.key()))
        .map(|i| i as u32)
        .collect();
    let added: HashSet<u32> = come
        .into_iter()
        .filter(|&i| !gone_keys.contains(&after[i].1.key()))
        .map(|i| i as u32)
        .collect();
    if removed.is_empty() && added.is_empty() {
        return Vec::new();
    }
    (0..eyes.len())
        .filter(|&i| {
            let (p, reach) = eyes[i];
            (!removed.is_empty()
                && in_reach(before_grid, p, reach, 0.0)
                    .iter()
                    .any(|b| removed.contains(b)))
                || (!added.is_empty()
                    && in_reach(after_grid, p, reach, 0.0)
                        .iter()
                        .any(|b| added.contains(b)))
        })
        .collect()
}

/// One eye's map slot: where its map was built from, and to what reach
/// (none until built).
#[derive(Clone, Debug)]
struct Slot {
    key: String,
    slot: u32,
    built: Option<[f64; 3]>,
    reach: f64,
    eye: Eye,
}

/// Fog of war's sight over one world for one side.
pub struct FogSight {
    geometry: Geometry,
    world: Option<SightWorld>,
    /// The side's foliage: the header (`nx, ny, cell_m`), then records.
    foliage: Vec<f32>,
    foliage_sent: bool,
    translucent_lift_m: f64,
    /// The occluders the side knows, and their lookup grid.
    occluders: Option<Vec<(Identity, Occluder)>>,
    occluders_sent: bool,
    wholes: Wholes,
    wholes_dirty: bool,
    /// Each eye's structures in reach at the last whole-fog pass, on `wholes`.
    kept: Kept,
    max_occluder_radius: f64,
    /// Whether the side's sight is set (fog is drawn).
    sighted: bool,
    capacity: u32,
    /// Map slots in the order they were first taken (`Map` insertion order).
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The latest sight's eyes, as indices into `slots`.
    order: Vec<usize>,
    /// Moved eyes waiting for a rebuild, oldest first.
    queue: Vec<String>,
    tiles_x: u32,
    pub rebuilt: u32,
    pub rebuilt_total: u64,
}

impl FogSight {
    pub fn new(geometry: Geometry) -> Result<Self, String> {
        geometry.check()?;
        Ok(FogSight {
            geometry,
            world: None,
            foliage: Vec::new(),
            foliage_sent: false,
            translucent_lift_m: 0.0,
            occluders: None,
            occluders_sent: false,
            wholes: whole_words(&[], 0.0)?,
            wholes_dirty: false,
            kept: Kept::new(),
            max_occluder_radius: 0.0,
            sighted: false,
            capacity: 0,
            slots: Vec::new(),
            free: Vec::new(),
            order: Vec::new(),
            queue: Vec::new(),
            tiles_x: 0,
            rebuilt: 0,
            rebuilt_total: 0,
        })
    }

    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    pub fn world(&self) -> Option<&SightWorld> {
        self.world.as_ref()
    }

    /// Every map rebuilt from scratch (a new world, or a cost probe).
    pub fn invalidate(&mut self) {
        for s in &mut self.slots {
            s.built = None;
        }
        self.queue.clear();
    }

    /// A new world: its foliage `foliage`; every map rebuilds.
    pub fn set_world(&mut self, world: SightWorld, foliage: Vec<f32>) -> Result<(), String> {
        world.check()?;
        self.world = Some(world);
        self.occluders = None;
        self.set_foliage(foliage);
        Ok(())
    }

    /// The side's foliage changed (trees on ground it saw cleared).
    pub fn set_foliage(&mut self, foliage: Vec<f32>) {
        self.translucent_lift_m = foliage
            .iter()
            .skip(5)
            .step_by(4)
            .fold(0.0, |m, &v| m.max(v as f64));
        self.foliage = foliage;
        self.foliage_sent = false;
        self.invalidate();
    }

    /// The renderer holds nothing: the next upload sends the world and the
    /// knowledge again.
    pub fn resend(&mut self) {
        self.foliage_sent = false;
        self.occluders_sent = false;
        self.wholes_dirty = true;
        self.invalidate();
    }

    /// The occluders the side knows stand: the map's occluding props less
    /// those `known` says no longer stand as mapped, plus the known props
    /// that occlude and are not destroyed.
    fn known_occluders(world: &SightWorld, known: &[KnownBox]) -> Vec<(Identity, Occluder)> {
        let fallen: HashSet<u64> = known
            .iter()
            .filter_map(|k| k.authored.map(f64::to_bits))
            .collect();
        let mut out: Vec<(Identity, Occluder)> = world
            .map
            .iter()
            .enumerate()
            .filter(|(i, _)| !fallen.contains(&world.ids[*i].to_bits()))
            .map(|(i, b)| (Identity::Map(i), *b))
            .collect();
        for k in known {
            if !world.occluding.contains(k.kind) || k.destroyed {
                continue;
            }
            out.push((
                Identity::Learned,
                Occluder {
                    x: k.center[0],
                    y: k.center[1],
                    yaw: k.yaw,
                    hx: k.half[0],
                    hy: k.half[1],
                    base: k.base_z,
                    top: k.base_z + 2.0 * k.half[2],
                },
            ));
        }
        out
    }

    /// The side's sight: its `eyes`, and, when its knowledge changed, what
    /// it knows (`known`; none keeps the last).
    pub fn set_sight(&mut self, eyes: &[Eye], known: Option<&[KnownBox]>) -> Result<(), String> {
        let Some(world) = &self.world else {
            return Ok(());
        };
        self.sighted = true;
        self.wholes_dirty = true;
        if let Some(known) = known.or(self.occluders.is_none().then_some(&[])) {
            let next = Self::known_occluders(world, known);
            self.set_occluders(next)?;
        }
        let live: HashSet<&str> = eyes.iter().map(|e| e.key.as_str()).collect();
        let mut freed = Vec::new();
        self.slots.retain(|s| {
            let keep = live.contains(s.key.as_str());
            if !keep {
                freed.push(s.slot);
            }
            keep
        });
        self.free.extend(freed);
        self.queue.retain(|k| live.contains(k.as_str()));
        let needed = eyes.len() as u32;
        if needed > self.capacity {
            // Slots are renumbered densely into the grown maps.
            self.free.clear();
            self.capacity = needed.max(self.capacity * 2).max(4);
            self.invalidate();
            for (i, s) in self.slots.iter_mut().enumerate() {
                s.slot = i as u32;
            }
            let kept = self.slots.len() as u32;
            self.free.extend((kept..self.capacity).rev());
        }
        let index: HashMap<String, usize> = self
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| (s.key.clone(), i))
            .collect();
        let mut order = Vec::with_capacity(eyes.len());
        for eye in eyes {
            match index.get(&eye.key) {
                None => {
                    let slot = self.free.pop().ok_or("fog: no free map slot")?;
                    order.push(self.slots.len());
                    self.slots.push(Slot {
                        key: eye.key.clone(),
                        slot,
                        built: None,
                        reach: eye.reach(),
                        eye: eye.clone(),
                    });
                }
                Some(&i) => {
                    let s = &mut self.slots[i];
                    let moved = s
                        .built
                        .is_some_and(|b| b != eye.position || s.reach != eye.reach());
                    s.eye = eye.clone();
                    if moved && !self.queue.contains(&eye.key) {
                        self.queue.push(eye.key.clone());
                    }
                    order.push(i);
                }
            }
        }
        self.order = order;
        Ok(())
    }

    /// No sight: fog is off and the world is drawn clear.
    pub fn clear_sight(&mut self) {
        self.sighted = false;
    }

    fn set_occluders(&mut self, next: Vec<(Identity, Occluder)>) -> Result<(), String> {
        let boxes: Vec<Occluder> = next.iter().map(|(_, b)| *b).collect();
        let next_wholes = whole_words(&boxes, 2.0 * self.geometry.face_probe_m)?;
        let built: Vec<usize> = (0..self.slots.len())
            .filter(|&i| self.slots[i].built.is_some())
            .collect();
        let eyes: Vec<([f64; 3], f64)> = built
            .iter()
            .map(|&i| (self.slots[i].built.unwrap(), self.slots[i].reach))
            .collect();
        let before = self.occluders.take().unwrap_or_default();
        for i in affected_eyes(&before, &self.wholes, &next, &next_wholes, &eyes) {
            self.slots[built[i]].built = None;
        }
        let slots = &self.slots;
        self.queue
            .retain(|k| slots.iter().any(|s| &s.key == k && s.built.is_some()));
        self.max_occluder_radius = boxes.iter().fold(0.0, |m, b| m.max(length(b.hx, b.hy)));
        self.occluders = Some(next);
        self.occluders_sent = false;
        self.wholes = next_wholes;
        self.kept.clear();
        Ok(())
    }

    /// This frame's rebuilds: every eye never built, then moved eyes up to
    /// the budget, oldest first.
    fn pick_rebuilds(&mut self) -> Vec<usize> {
        let mut picked: Vec<usize> = (0..self.order.len())
            .filter(|&i| self.slots[self.order[i]].built.is_none())
            .collect();
        let mut budget = self.geometry.rebuild_eyes_per_frame;
        let index: HashMap<&str, usize> = self
            .order
            .iter()
            .enumerate()
            .map(|(i, &s)| (self.slots[s].key.as_str(), i))
            .collect();
        let mut taken = 0;
        while budget > 0 && taken < self.queue.len() {
            let i = index[self.queue[taken].as_str()];
            taken += 1;
            if !picked.contains(&i) {
                picked.push(i);
            }
            budget -= 1;
        }
        self.queue.drain(..taken);
        for &i in &picked {
            let s = &mut self.slots[self.order[i]];
            s.built = Some(s.eye.position);
            s.reach = s.eye.reach();
        }
        picked
    }

    fn eye_record(s: &Slot) -> FogEye {
        let p = s.built.unwrap_or(s.eye.position);
        FogEye {
            position: p.map(|v| v as f32),
            reach: s.reach as f32,
            forward: s.eye.forward as f32,
            front: s.eye.shape.front as f32,
            side: s.eye.shape.side as f32,
            rear: s.eye.shape.rear as f32,
            range: s.eye.range as f32,
            slot: s.slot,
            pad0: 0,
            pad1: 0,
        }
    }

    /// Where each eye's map stands and how far it reaches, in sight order.
    fn eye_rows(&self) -> Vec<([f64; 3], f64)> {
        self.order
            .iter()
            .map(|&i| {
                let s = &self.slots[i];
                (s.built.unwrap_or(s.eye.position), s.reach)
            })
            .collect()
    }

    fn active(&self) -> bool {
        self.world.is_some() && self.sighted
    }

    /// One frame's fog for `camera` (none: a probe's, which keeps the last
    /// frame's tiles) over a `width`×`height` target, with `probes` points
    /// to probe.
    pub fn prepare(
        &mut self,
        camera: Option<&Camera>,
        width: u32,
        height: u32,
        probes: u32,
    ) -> Result<FogUpload, String> {
        let g = &self.geometry;
        let bins = self.capacity * RADIAL_BINS;
        let mut up = FogUpload {
            map_words: [bins * g.azimuth_bins, bins * g.terrain_azimuth_bins],
            ..FogUpload::default()
        };
        if camera.is_some() {
            self.tiles_x = width.div_ceil(TILE_PX);
            up.tiles = [self.tiles_x, height.div_ceil(TILE_PX), g.tile_eyes_max];
        }
        let active = self.active();
        up.active = active;
        if !self.foliage_sent && self.world.is_some() {
            self.foliage_sent = true;
            up.foliage = Some(if self.foliage.len() > 3 {
                self.foliage[3..].to_vec()
            } else {
                vec![0.0; 4]
            });
        }
        if !self.occluders_sent {
            if let Some(occluders) = &self.occluders {
                self.occluders_sent = true;
                up.occluders = Some(
                    occluders
                        .iter()
                        .map(|(_, b)| FogBox {
                            center: [b.x as f32, b.y as f32],
                            half: [b.hx as f32, b.hy as f32],
                            yaw: b.yaw as f32,
                            top: b.top as f32,
                            base: b.base as f32,
                            pad1: 0.0,
                        })
                        .collect(),
                );
                up.wholes = Some(self.wholes.words.clone());
            }
        }
        let picked = if active {
            self.pick_rebuilds()
        } else {
            Vec::new()
        };
        self.rebuilt = picked.len() as u32;
        self.rebuilt_total += picked.len() as u64;
        if active {
            up.eyes = self
                .order
                .iter()
                .map(|&i| Self::eye_record(&self.slots[i]))
                .collect();
            if !picked.is_empty() {
                let rows = self.eye_rows();
                let eyes: Vec<_> = picked.iter().map(|&i| rows[i]).collect();
                let indices: Vec<u32> = picked.iter().map(|&i| i as u32).collect();
                let margin = self.world.as_ref().unwrap().sensors.min_sight_gap_m / 2.0;
                // One search per rebuilt eye: its structures within the
                // whole-fog margin (kept for that pass), of which those
                // within its reach alone are the merge's candidates.
                let found: Vec<Vec<u32>> = eyes
                    .iter()
                    .map(|&(p, reach)| {
                        let key = [p[0].to_bits(), p[1].to_bits(), reach.to_bits()];
                        let wide = self
                            .kept
                            .entry(key)
                            .or_insert_with(|| {
                                Arc::new(in_reach(&self.wholes, p, reach, self.max_occluder_radius))
                            })
                            .clone();
                        within(&self.wholes, &wide, p, reach)
                    })
                    .collect();
                up.rebuild = horizon_records_of(&self.wholes, &eyes, &found, &indices, margin);
                let dispatch = |bins: u32| {
                    let n = (picked.len() * bins as usize).div_ceil(BUILD_WORKGROUP);
                    if n > MAX_WORKGROUPS {
                        Err(format!(
                            "fog: {} eyes exceed one rebuild dispatch",
                            picked.len()
                        ))
                    } else {
                        Ok(n as u32)
                    }
                };
                up.rebuild_dispatch = [
                    dispatch(self.geometry.terrain_azimuth_bins)?,
                    dispatch(self.geometry.azimuth_bins)?,
                ];
            }
        }
        up.params = self.params(camera, width, height, picked.len() as u32, probes);
        if !picked.is_empty() {
            self.wholes_dirty = true;
        }
        let has_occluders = self.occluders.as_ref().is_some_and(|o| !o.is_empty());
        if camera.is_some() && active && self.wholes_dirty && has_occluders {
            // Flag each structure seen or not, against every eye in reach.
            self.wholes_dirty = false;
            let (count, items) = whole_records_kept(
                &self.wholes,
                &self.eye_rows(),
                self.max_occluder_radius,
                &mut self.kept,
            );
            let n = count as usize;
            up.whole = Some(WholePass {
                count,
                items,
                uniform: [self.geometry.whole_step_m as f32, count as f32, 0.0, 0.0],
                dispatch: [
                    n.min(MAX_WORKGROUPS) as u32,
                    n.div_ceil(MAX_WORKGROUPS) as u32,
                ],
                flags_row: self.wholes.flags_row,
                flag_rows: self.wholes.flag_rows,
            });
        }
        Ok(up)
    }

    fn params(
        &self,
        camera: Option<&Camera>,
        width: u32,
        height: u32,
        rebuild: u32,
        probes: u32,
    ) -> FogParams {
        let g = &self.geometry;
        let world = self.world.as_ref();
        let active = self.active();
        let f = &self.foliage;
        let foliage = f.len() > 3;
        let w = &self.wholes;
        FogParams {
            inv_view_proj: camera.map_or([0.0; 16], |c| c.inv_view_proj),
            camera_eye: camera.map_or([0.0; 3], |c| c.eye),
            translucent_lift_m: self.translucent_lift_m as f32,
            azimuth_bins: g.azimuth_bins,
            terrain_azimuth_bins: g.terrain_azimuth_bins,
            tiles_x: self.tiles_x,
            tile_eyes_max: g.tile_eyes_max,
            eye_count: self.order.len() as u32,
            foliage_nx: if foliage { f[0] as u32 } else { 0 },
            foliage_ny: if foliage { f[1] as u32 } else { 0 },
            height_nx: world.map_or(2, |w| w.nx),
            height_ny: world.map_or(2, |w| w.ny),
            rebuild_count: rebuild,
            enabled: u32::from(active),
            probe_count: probes,
            first_bin_m: g.first_bin_m as f32,
            target_height_m: world.map_or(0.0, |w| w.sensors.fog_target_height_m) as f32,
            foliage_cell_m: if foliage { f[2] } else { 1.0 },
            foliage_full_block: world.map_or(1.0, |w| w.sensors.foliage_full_block) as f32,
            min_sight_gap_m: world.map_or(0.0, |w| w.sensors.min_sight_gap_m) as f32,
            pad0: [0.0; 3],
            playable: world.map_or([0.0; 4], |w| w.playable.map(|v| v as f32)),
            height_spacing: world.map_or(1.0, |w| w.spacing) as f32,
            face_probe_m: g.face_probe_m as f32,
            width: if camera.is_some() { width as f32 } else { 0.0 },
            height: if camera.is_some() { height as f32 } else { 0.0 },
            step_min_m: g.terrain_step_m[0] as f32,
            step_max_m: g.terrain_step_m[1] as f32,
            step_fraction: g.terrain_step_fraction as f32,
            roof_reach_m: g.roof_reach_m as f32,
            whole_count: if active { w.count } else { 0 },
            whole_nx: w.nx,
            whole_ny: w.ny,
            whole_cell_m: w.cell_m as f32,
            whole_origin: w.origin.map(|v| v as f32),
            whole_items_base: w.items_base,
            whole_boxes_base: w.boxes_base,
            whole_flags_base: w.flags_row * WHOLE_TEXTURE_WIDTH as u32,
            pad1: [0.0; 3],
        }
    }

    /// What the fog holds, for the lab's readout.
    pub fn stats(&self) -> FogStats {
        let g = &self.geometry;
        let r = RADIAL_BINS as u64;
        FogStats {
            enabled: self.active(),
            eyes: self.order.len(),
            slots: self.capacity,
            occluders: self.occluders.as_ref().map_or(0, Vec::len),
            rebuilt: self.rebuilt,
            pending: self.queue.len(),
            rebuilt_total: self.rebuilt_total,
            map_bytes: self.capacity as u64 * g.azimuth_bins as u64 * r * 4,
            terrain_map_bytes: self.capacity as u64 * g.terrain_azimuth_bins as u64 * r * 4,
        }
    }

    /// The latest sight's eyes.
    pub fn eyes(&self) -> Vec<Eye> {
        self.order
            .iter()
            .map(|&i| self.slots[i].eye.clone())
            .collect()
    }

    /// The occluders the side knows, in the order the structures' flags are.
    pub fn occluders(&self) -> Vec<Occluder> {
        self.occluders
            .as_ref()
            .map_or_else(Vec::new, |o| o.iter().map(|(_, b)| *b).collect())
    }
}

/// What the fog holds and did last frame.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FogStats {
    pub enabled: bool,
    pub eyes: usize,
    /// Map slots allocated.
    pub slots: u32,
    pub occluders: usize,
    /// Eye maps rebuilt last frame, and moved eyes still waiting their turn.
    pub rebuilt: u32,
    pub pending: usize,
    pub rebuilt_total: u64,
    pub map_bytes: u64,
    pub terrain_map_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::publication::reader::SightShape;

    const WALL: &str = "wall";

    fn boxed(x: f64, y: f64, hx: f64, hy: f64, yaw: f64) -> Occluder {
        Occluder {
            x,
            y,
            yaw,
            hx,
            hy,
            base: 0.0,
            top: 8.0,
        }
    }
    fn at(x: f64, y: f64) -> Occluder {
        boxed(x, y, 2.0, 3.0, 0.0)
    }
    fn known(b: &Occluder) -> KnownBox<'static> {
        KnownBox {
            kind: WALL,
            center: [b.x, b.y],
            yaw: b.yaw,
            half: [b.hx, b.hy, (b.top - b.base) / 2.0],
            base_z: b.base,
            authored: None,
            destroyed: false,
        }
    }
    fn geometry() -> Geometry {
        Geometry {
            azimuth_bins: 8,
            terrain_azimuth_bins: 8,
            first_bin_m: 1.0,
            terrain_step_m: [1.0, 5.0],
            terrain_step_fraction: 0.1,
            tile_eyes_max: 8,
            face_probe_m: 0.1,
            roof_reach_m: 0.0,
            rebuild_eyes_per_frame: 1,
            whole_step_m: 1.0,
        }
    }
    fn world(map: Vec<Occluder>, ids: Vec<f64>) -> SightWorld {
        SightWorld {
            nx: 2,
            ny: 2,
            spacing: 5000.0,
            playable: [0.0, 0.0, 5000.0, 5000.0],
            sensors: Sensors {
                fog_target_height_m: 0.0,
                foliage_full_block: 1.0,
                min_sight_gap_m: 0.0,
            },
            map,
            ids,
            occluding: [WALL.to_owned(), "ruin".to_owned()].into(),
        }
    }
    fn eye(key: &str, x: f64) -> Eye {
        Eye {
            key: key.into(),
            position: [x, 0.0, 2.0],
            forward: 0.0,
            shape: Shape {
                front: 1.0,
                side: 1.0,
                rear: 1.0,
            },
            range: 100.0,
        }
    }

    #[test]
    fn eye_candidates_come_once_each_in_index_order() {
        let boxes = [
            at(1000.0, 1000.0),
            at(105.0, 0.0),
            at(0.0, 0.0),
            boxed(150.0, 0.0, 60.0, 2.0, 0.6),
            boxed(-104.0, 0.0, 4.0, 3.0, 0.0),
            at(0.0, 1000.0),
        ];
        let grid = whole_words(&boxes, 0.2).unwrap();
        assert_eq!(in_reach(&grid, [0.0, 0.0, 2.0], 100.0, 0.0), [2, 3, 4]);
        assert_eq!(in_reach(&grid, [1000.0, 1000.0, 2.0], 100.0, 0.0), [0]);
        assert!(in_reach(&grid, [-5000.0, -5000.0, 2.0], 100.0, 0.0).is_empty());
        let none = whole_words(&[], 0.2).unwrap();
        assert!(in_reach(&none, [0.0, 0.0, 2.0], 100.0, 0.0).is_empty());
        // A tall narrow box just outside the circle: its roof may look in.
        let roofs = whole_words(&[at(0.0, 0.0), boxed(132.0, 0.0, 1.0, 100.0, 0.0)], 0.2).unwrap();
        assert_eq!(in_reach(&roofs, [0.0, 0.0, 2.0], 100.0, 0.0), [0]);
        assert_eq!(in_reach(&roofs, [0.0, 0.0, 2.0], 100.0, 101.0), [0, 1]);
    }

    #[test]
    fn every_point_inside_a_structure_finds_it_through_the_grid() {
        // Turned, touching and far-apart boxes, as a town's buildings, ruins and walls.
        let boxes = [
            Occluder {
                x: 975.0,
                y: 752.0,
                yaw: 0.0,
                hx: 15.0,
                hy: 12.0,
                base: 10.0,
                top: 18.0,
            },
            Occluder {
                x: 1047.0,
                y: 814.0,
                yaw: 0.6,
                hx: 18.0,
                hy: 9.0,
                base: 11.0,
                top: 21.0,
            },
            Occluder {
                x: 1000.0,
                y: 752.0,
                yaw: 0.0,
                hx: 10.0,
                hy: 12.0,
                base: 10.0,
                top: 12.0,
            },
            Occluder {
                x: 200.0,
                y: 1400.0,
                yaw: 2.2,
                hx: 0.25,
                hy: 20.0,
                base: 3.0,
                top: 5.0,
            },
        ];
        let margin = 0.2;
        let g = whole_words(&boxes, margin).unwrap();
        let lookup = |x: f64, y: f64| -> Vec<u32> {
            let cx = ((x - g.origin[0]) / g.cell_m).floor();
            let cy = ((y - g.origin[1]) / g.cell_m).floor();
            if cx < 0.0 || cy < 0.0 || cx >= g.nx as f64 || cy >= g.ny as f64 {
                return vec![];
            }
            let cell = g.words[cy as usize * g.nx as usize + cx as usize];
            let start = (g.items_base + (cell >> 8)) as usize;
            g.words[start..start + (cell & 255) as usize].to_vec()
        };
        for (i, b) in boxes.iter().enumerate() {
            let (c, s) = (libm::cos(b.yaw), libm::sin(b.yaw));
            for u in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                for v in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                    let (lu, lv) = (u * (b.hx + margin), v * (b.hy + margin));
                    let found = lookup(b.x + lu * c - lv * s, b.y + lu * s + lv * c);
                    assert!(found.contains(&(i as u32)), "box {i} at {u}, {v}");
                }
            }
        }
        // Seen flags start on a row of their own, one per box, unseen.
        let flags = g.flags_row as usize * WHOLE_TEXTURE_WIDTH;
        assert!(g.flag_rows as usize * WHOLE_TEXTURE_WIDTH >= boxes.len());
        assert!(g.words[flags..].iter().all(|&w| w == 0));
    }

    #[test]
    fn whole_fog_pairs_each_structure_only_with_eyes_that_reach_it() {
        let boxes = [
            at(40.0, 0.0),
            at(2040.0, 0.0),
            at(5000.0, 5000.0),
            boxed(132.0, 0.0, 1.0, 100.0, 0.0),
        ];
        let grid = whole_words(&boxes, 0.2).unwrap();
        let eyes = [([0.0, 0.0, 2.0], 100.0), ([2000.0, 0.0, 2.0], 100.0)];
        let (count, words) = whole_records(&grid, &eyes, 101.0);
        assert_eq!(count, 3);
        // Rows `[box, eye-list offset, eye count]`, then the eyes.
        assert_eq!(words, [0, 9, 1, 1, 10, 1, 3, 11, 1, 0, 1, 0]);
        assert_eq!(whole_records(&grid, &[], 101.0), (0, vec![]));
    }

    #[test]
    fn horizon_sectors_keep_every_box_a_ray_crosses_across_the_west_wrap() {
        let boxes = [
            at(-40.0, 0.0),
            at(40.0, 0.0),
            at(0.0, 0.0),
            boxed(15.0, 30.0, 12.0, 2.0, 0.8),
        ];
        let records = horizon_records(
            &whole_words(&boxes, 0.2).unwrap(),
            &[([0.0, 0.0, 2.0], 100.0)],
            &[0],
            0.0,
        );
        let sectors = records[2] as usize;
        let list = |sector: usize| {
            let row = records[1] as usize + sector * 2;
            let start = records[row] as usize;
            records[start..start + records[row + 1] as usize].to_vec()
        };
        assert_eq!(list(0), [0, 2]);
        assert_eq!(list(sectors - 1), [0, 2]);
        assert_eq!(list(sectors / 2), [1, 2]);
        for ai in 0..8192 {
            let theta = (ai as f64 + 0.5) / 8192.0 * std::f64::consts::TAU - std::f64::consts::PI;
            let candidates = list(((ai as f64 + 0.5) / 8192.0 * sectors as f64) as usize);
            for (i, b) in boxes.iter().enumerate() {
                let (c, s) = (libm::cos(b.yaw), libm::sin(b.yaw));
                let (ox, oy) = (-b.x * c - b.y * s, b.x * s - b.y * c);
                let (dx, dy) = (
                    libm::cos(theta) * c + libm::sin(theta) * s,
                    -libm::cos(theta) * s + libm::sin(theta) * c,
                );
                let (t0, t1) = ((-b.hx - ox) / dx, (b.hx - ox) / dx);
                let (u0, u1) = ((-b.hy - oy) / dy, (b.hy - oy) / dy);
                let near = t0.min(t1).max(u0.min(u1));
                let far = t0.max(t1).min(u0.max(u1));
                if near <= far && far > 0.0 && near <= 100.0 {
                    assert!(candidates.contains(&(i as u32)), "ray {ai} box {i}");
                }
            }
        }
    }

    #[test]
    fn a_fallen_map_prop_leaves_sight_and_its_known_remains_take_its_place() {
        let w = world(vec![at(975.0, 752.0), at(40.0, 0.0)], vec![7.0, 8.0]);
        let standing = FogSight::known_occluders(&w, &[]);
        assert_eq!(standing.len(), 2);
        let ruin = KnownBox {
            kind: "ruin",
            center: [975.0, 752.0],
            yaw: 0.0,
            half: [15.0, 12.0, 1.0],
            base_z: 0.0,
            authored: Some(7.0),
            destroyed: false,
        };
        let crate_box = KnownBox {
            kind: "crate",
            authored: None,
            ..known(&at(10.0, 10.0))
        };
        let gone = KnownBox {
            destroyed: true,
            ..known(&at(20.0, 20.0))
        };
        let after = FogSight::known_occluders(&w, &[ruin, crate_box, gone]);
        let boxes: Vec<_> = after.iter().map(|(_, b)| *b).collect();
        // The fallen map box is gone, its remains stand two metres tall; a
        // kind that hides nothing, and a destroyed one, never occlude.
        assert_eq!(
            boxes,
            [
                at(40.0, 0.0),
                Occluder {
                    x: 975.0,
                    y: 752.0,
                    yaw: 0.0,
                    hx: 15.0,
                    hy: 12.0,
                    base: 0.0,
                    top: 2.0
                }
            ]
        );
    }

    #[test]
    fn a_garrison_gives_an_eye_per_facade_keyed_by_unit_and_eye() {
        let sight = Sight {
            eyes: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            forward: 0.5,
            shape: SightShape {
                front: 1.0,
                side: 1.0,
                rear: 1.0,
            },
            range: 600.0,
        };
        let eyes = Eye::of_units([(7.0, &sight)]);
        let got: Vec<_> = eyes.iter().map(|e| (e.key.as_str(), e.position)).collect();
        assert_eq!(got, [("7:0", [1.0, 2.0, 3.0]), ("7:1", [4.0, 5.0, 6.0])]);
    }

    #[test]
    fn an_occluder_change_rebuilds_at_once_only_the_maps_it_touches() {
        let mut fog = FogSight::new(geometry()).unwrap();
        fog.set_world(world(vec![], vec![]), vec![]).unwrap();
        let camera = Camera::default();
        let frame = |fog: &mut FogSight| fog.prepare(Some(&camera), 1, 1, 0).unwrap();
        let eyes = [eye("0", 0.0), eye("1", 2000.0)];
        let boxes = [at(40.0, 0.0), at(2040.0, 0.0)];
        let low = |b: &Occluder| Occluder { top: 1.0, ..*b };
        let set = |fog: &mut FogSight, eyes: &[Eye], boxes: &[Occluder]| {
            let known: Vec<_> = boxes.iter().map(known).collect();
            fog.set_sight(eyes, Some(&known)).unwrap();
        };
        set(&mut fog, &eyes, &boxes);
        // New eyes build at once, past the budget of one.
        frame(&mut fog);
        assert_eq!(fog.rebuilt, 2);
        frame(&mut fog);
        assert_eq!(fog.rebuilt, 0);
        set(&mut fog, &eyes, &[boxes[0], low(&boxes[1])]);
        frame(&mut fog);
        assert_eq!((fog.rebuilt, fog.stats().pending), (1, 0));
        // Restoring the box's height rebuilds the eye it touches.
        set(&mut fog, &eyes, &[boxes[1], boxes[0]]);
        frame(&mut fog);
        assert_eq!(fog.rebuilt, 1);
        // The same boxes in another order change nothing.
        set(&mut fog, &eyes, &boxes);
        frame(&mut fog);
        assert_eq!(fog.rebuilt, 0);
        // A touched eye rebuilds at once, beside one moved eye in the budget.
        let moved: Vec<_> = eyes
            .iter()
            .map(|e| Eye {
                position: [e.position[0] + 1.0, 0.0, 2.0],
                ..e.clone()
            })
            .collect();
        set(&mut fog, &moved, &boxes);
        set(&mut fog, &moved, &[low(&boxes[0]), boxes[1]]);
        frame(&mut fog);
        assert_eq!((fog.rebuilt, fog.stats().pending), (2, 0));
        frame(&mut fog);
        assert_eq!(fog.rebuilt, 0, "no stale move repeats the rebuild");
    }

    #[test]
    fn a_full_block_past_eight_bits_of_foliage_is_refused() {
        let mut w = world(vec![], vec![]);
        w.sensors.foliage_full_block = FOLIAGE_MAX;
        let mut fog = FogSight::new(geometry()).unwrap();
        fog.set_world(w.clone(), vec![]).unwrap();
        w.sensors.foliage_full_block *= 2.0;
        assert!(fog
            .set_world(w, vec![])
            .unwrap_err()
            .contains("foliage_full_block"));
    }
}
