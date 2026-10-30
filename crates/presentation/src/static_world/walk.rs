//! The buildings' per-frame walk, the one both renderers draw from: which
//! chunks a view meets, the tier each takes, which near chunks the residency
//! pool holds, and the module instances each module and tier draws and casts.
//! Sim space, `f64` as the browser's walk was.
//!
//! - **Coarse.** Every chunk in view draws its standing buildings' coarsest
//!   rows, and casts them when its box meets a fitted cascade; an offscreen
//!   chunk meeting a cascade casts its coarse rows too.
//! - **Resident.** A chunk whose nearest point calls for a finer tier draws its
//!   rows at that tier instead, nearest first while the pool holds them, and
//!   casts them a tier coarser. The rest draw coarse.
//! - **Fallen.** A building the side knows destroyed leaves both and draws its
//!   template's rows for that state, chunked by where it stands, at the tier
//!   its chunk takes.
use crate::records::{CAST_PASS, FOG_FACES, VIEW_PASS};
use crate::shadow::{CasterVolumes, CASCADES};
use crate::static_world::batches::{List, Population};
use crate::static_world::buildings::{BuildingModule, Buildings};
use crate::static_world::index::BoundsIndex;
use crate::view::{building_tier, caster_tier, View, TIERS};
use std::collections::{BTreeMap, HashMap};

const COARSEST: usize = TIERS - 1;

/// A renderer's model record (64 B): x, y, z, yaw (sim), four zeros, the
/// linear tint and -1 (no card), the scale per axis and 0.
pub type Record = [f32; 16];

/// A box: min x, y, z, max x, y, z.
type Box6 = [f64; 6];

/// Buildings bucketed together: their box, them, and
/// the rows they draw (the coarse ones, or the fallen ones in their state).
#[derive(Default)]
struct Chunk {
    bounds: Box6,
    buildings: Vec<usize>,
    rows: Vec<BuildingModule>,
}

impl Chunk {
    fn add(&mut self, buildings: &Buildings, b: usize) {
        let r = &buildings.records[b];
        let bounds: Box6 =
            std::array::from_fn(|k| (if k < 3 { r.min[k] } else { r.max[k - 3] }) as f64);
        if self.buildings.is_empty() {
            self.bounds = bounds;
        }
        for k in 0..3 {
            self.bounds[k] = self.bounds[k].min(bounds[k]);
            self.bounds[k + 3] = self.bounds[k + 3].max(bounds[k + 3]);
        }
        self.buildings.push(b);
    }
}

/// Every module and tier's instances a walk draws and casts (`module · TIERS
/// + tier`); caster lists have one such block per cascade.
#[derive(Default)]
pub struct Lists {
    pub view: Vec<Vec<Record>>,
    pub cast: Vec<Vec<Record>>,
}

pub struct BuildingWalk {
    pub buildings: Buildings,
    /// The tier rule as authored, and as drawn (`set_lod_scale`).
    authored_lod: [f64; 3],
    lod: [f64; 3],
    pool: usize,
    chunk_m: f64,
    chunks: Vec<Chunk>,
    chunk_index: BoundsIndex,
    ruin_index: BoundsIndex,
    candidates: Vec<usize>,
    ruin_candidates: Vec<usize>,
    /// Per building: the state the side knows it fell in.
    fallen: Vec<Option<usize>>,
    ruins: Vec<Chunk>,
    /// Resident chunks' rows at their tier, kept while they stay resident.
    resident: HashMap<(usize, usize), Vec<BuildingModule>>,
    pub lists: Lists,
    scratch: Lists,
    pub counts: Vec<u32>,
    /// The chunks and their coarse rows, then the last walk's chunks
    /// resident, rows resident, rows expanded (not resident before) and near
    /// chunks drawn coarse for want of room, the fallen buildings' rows, and
    /// those drawn at each tier, then bounds visited by the last query.
    pub stats: [usize; 12],
}

impl BuildingWalk {
    /// A new art generation retains the player's detail and residency policy.
    pub fn replace_buildings(&self, buildings: Buildings) -> Self {
        let mut next = Self::new(buildings, self.authored_lod, self.chunk_m, self.pool);
        next.lod = self.lod;
        next
    }

    pub fn pool_capacity(&self) -> usize {
        self.pool
    }

    /// Draws the tier rule's thresholds `scale` times their authored values
    /// (a graphics quality: below 1 a building coarsens nearer the eye).
    pub fn set_lod_scale(&mut self, scale: f64) {
        self.lod = self.authored_lod.map(|v| v * scale);
    }

    /// `buildings` drawn under the tier rule `lod` (pixels a metre covers at
    /// a chunk's nearest point for tiers 0 to 2), in `chunk_m` chunks, with a
    /// pool of `pool` rows.
    pub fn new(buildings: Buildings, lod: [f64; 3], chunk_m: f64, pool: usize) -> Self {
        let chunk_count = buildings
            .records
            .iter()
            .map(|r| r.chunk + 1)
            .max()
            .unwrap_or(0) as usize;
        let mut chunks: Vec<Chunk> = (0..chunk_count).map(|_| Chunk::default()).collect();
        for b in 0..buildings.len() {
            if let Ok(c) = usize::try_from(buildings.records[b].chunk) {
                chunks[c].add(&buildings, b);
                buildings.rows(b, 0, Some(COARSEST), &mut chunks[c].rows);
            }
        }
        let keys = buildings.modules.len() * TIERS;
        let coarse_rows = chunks.iter().map(|c| c.rows.len()).sum();
        Self {
            fallen: vec![None; buildings.len()],
            buildings,
            authored_lod: lod,
            lod,
            pool,
            chunk_m,
            chunk_index: BoundsIndex::new(chunks.iter().map(|c| c.bounds)),
            chunks,
            ruin_index: BoundsIndex::default(),
            candidates: Vec::new(),
            ruin_candidates: Vec::new(),
            ruins: vec![],
            resident: HashMap::new(),
            lists: Lists {
                view: vec![vec![]; keys],
                cast: vec![vec![]; keys * CASCADES],
            },
            scratch: Lists {
                view: vec![vec![]; keys],
                cast: vec![vec![]; keys * CASCADES],
            },
            counts: vec![0; keys],
            stats: [chunk_count, coarse_rows, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    /// What the side knows destroyed: each building and its state (`STATES`
    /// index, 1 ruin or 2 gutted), replacing the last list. Returns those whose
    /// template has no rows for the state; they stay drawn as before.
    pub fn set_fallen(&mut self, fallen: &[(usize, usize)]) -> Vec<usize> {
        let refused: Vec<usize> = fallen
            .iter()
            .filter(|&&(b, s)| self.buildings.records[b].rows[s] < 0)
            .map(|&(b, _)| b)
            .collect();
        if !refused.is_empty() {
            return refused;
        }
        self.fallen = vec![None; self.buildings.len()];
        let mut cells: BTreeMap<i64, Chunk> = BTreeMap::new();
        for &(b, s) in fallen {
            self.fallen[b] = Some(s);
            let [x, y, ..] = self.buildings.records[b].frame;
            let cell = |v: f32| (v as f64 / self.chunk_m).floor() as i64;
            let chunk = cells
                .entry((cell(y) + 4096) * 8192 + cell(x) + 4096)
                .or_default();
            chunk.add(&self.buildings, b);
            self.buildings.rows(b, s, None, &mut chunk.rows);
        }
        self.ruins = cells.into_values().collect();
        self.ruin_index = BoundsIndex::new(self.ruins.iter().map(|c| c.bounds));
        self.stats[6] = self.ruins.iter().map(|c| c.rows.len()).sum();
        self.resident.clear();
        vec![]
    }

    /// Walks `view`; returns the keys (`module · TIERS + tier`) whose drawn
    /// or cast instances changed since the last walk.
    pub fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>) -> Vec<usize> {
        self.candidates.clear();
        self.ruin_candidates.clear();
        let intersects = |b: &[f64; 6]| view.sees_box(b) || shadow.is_some_and(|s| s.intersects(b));
        self.stats[11] = self
            .chunk_index
            .query(intersects, |i| self.candidates.push(i))
            + self
                .ruin_index
                .query(intersects, |i| self.ruin_candidates.push(i));
        self.candidates.sort_unstable();
        self.ruin_candidates.sort_unstable();
        let keys = self.counts.len();
        let mut lists = std::mem::take(&mut self.scratch);
        for lists in [&mut lists.view, &mut lists.cast] {
            for list in lists.iter_mut() {
                list.clear();
            }
        }
        let mut counts = std::mem::take(&mut self.counts);
        counts.fill(0);
        let cast = |lists: &mut Lists, r: &BuildingModule, tier: usize, mask: u32| {
            for cascade in 0..CASCADES {
                if mask & (1 << cascade) != 0 {
                    lists.cast[cascade * keys + r.kit_module * TIERS + tier].push(r.record);
                }
            }
        };
        let draw = |lists: &mut Lists,
                    counts: &mut [u32],
                    r: &BuildingModule,
                    tier: usize,
                    caster: Option<usize>,
                    mask: u32| {
            lists.view[r.kit_module * TIERS + tier].push(r.record);
            counts[r.kit_module * TIERS + tier] += 1;
            if let Some(t) = caster {
                cast(lists, r, t, mask);
            }
        };
        let (mut near, mut coarse) = (vec![], vec![]);
        for &c in &self.candidates {
            let chunk = &self.chunks[c];
            let mask = shadow.map_or(0, |s| s.mask(&chunk.bounds));
            if !view.sees_box(&chunk.bounds) {
                if mask != 0 {
                    for r in self.coarse_rows(c) {
                        cast(&mut lists, r, COARSEST, mask);
                    }
                }
                continue;
            }
            let d = view.distance_to_box(&chunk.bounds);
            match building_tier(view, d, &self.lod) {
                COARSEST => coarse.push((c, mask)),
                t => near.push((d, c, t)),
            }
        }
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut used, mut full, mut expanded, mut refused) = (0, false, 0, 0);
        let mut kept = HashMap::new();
        for (_, c, t) in near {
            let casts = shadow.map_or(0, |s| s.mask(&self.chunks[c].bounds));
            let rows = match self.resident.remove(&(c, t)) {
                Some(rows) => rows,
                None => {
                    let rows = self.expand(c, t);
                    expanded += rows.len();
                    rows
                }
            };
            full = full || used + rows.len() > self.pool;
            if full {
                refused += 1;
                coarse.push((c, casts));
                continue;
            }
            used += rows.len();
            for r in &rows {
                draw(&mut lists, &mut counts, r, t, Some(caster_tier(t)), casts);
            }
            kept.insert((c, t), rows);
        }
        self.stats[2..6].copy_from_slice(&[kept.len(), used, expanded, refused]);
        self.stats[7..11].fill(0);
        self.resident = kept;
        for (c, casts) in coarse {
            for r in self.coarse_rows(c) {
                draw(&mut lists, &mut counts, r, COARSEST, Some(COARSEST), casts);
            }
        }
        for &c in &self.ruin_candidates {
            let chunk = &self.ruins[c];
            let mask = shadow.map_or(0, |s| s.mask(&chunk.bounds));
            let has = |r: &BuildingModule, t: usize| r.tiers & (1 << t) != 0;
            if view.sees_box(&chunk.bounds) {
                let d = view.distance_to_box(&chunk.bounds);
                let t = building_tier(view, d, &self.lod);
                for r in chunk.rows.iter() {
                    if has(r, t) {
                        draw(&mut lists, &mut counts, r, t, None, 0);
                        self.stats[7 + t] += 1;
                    }
                    if mask != 0 && has(r, COARSEST) {
                        cast(&mut lists, r, COARSEST, mask);
                    }
                }
            } else if mask != 0 {
                for r in chunk.rows.iter().filter(|r| has(r, COARSEST)) {
                    cast(&mut lists, r, COARSEST, mask);
                }
            }
        }
        self.counts = counts;
        let changed = (0..keys)
            .filter(|&k| {
                lists.view[k] != self.lists.view[k]
                    || (0..CASCADES)
                        .any(|c| lists.cast[c * keys + k] != self.lists.cast[c * keys + k])
            })
            .collect();
        self.scratch = std::mem::replace(&mut self.lists, lists);
        changed
    }

    /// Chunk `c`'s standing buildings' coarse rows.
    fn coarse_rows(&self, c: usize) -> impl Iterator<Item = &BuildingModule> {
        self.chunks[c]
            .rows
            .iter()
            .filter(|r| self.fallen[r.building].is_none())
    }

    /// Chunk `c`'s standing buildings' rows at tier `t`.
    fn expand(&self, c: usize, t: usize) -> Vec<BuildingModule> {
        let mut rows = vec![];
        for &b in self.chunks[c]
            .buildings
            .iter()
            .filter(|&&b| self.fallen[b].is_none())
        {
            self.buildings.rows(b, 0, Some(t), &mut rows);
        }
        rows
    }
}

/// The buildings as a frame's static population: the walk, and each walked
/// module's `World.appearances` row (its kit, and the module as the state).
pub struct BuildingPopulation {
    pub walk: BuildingWalk,
    pub rows: Vec<u32>,
    /// Counts the walks that changed what it draws: its lists' print.
    pub changes: u64,
}

impl Population for BuildingPopulation {
    fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>) -> bool {
        let changed = !self.walk.select(view, shadow).is_empty();
        self.changes += u64::from(changed);
        changed
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        let lists = &self.walk.lists;
        for (pass, lists) in [(VIEW_PASS, &lists.view), (CAST_PASS, &lists.cast)] {
            for (key, list) in lists.iter().enumerate() {
                let cascade = key / self.walk.lists.view.len();
                let key = key % self.walk.lists.view.len();
                let tier = (key % TIERS) as u32;
                visit(List {
                    row: self.rows[key / TIERS],
                    tier,
                    pass,
                    caster: List::own_caster(pass, tier),
                    cascade_mask: if pass == CAST_PASS { 1 << cascade } else { 0 },
                    fog: FOG_FACES,
                    records: list,
                    print: self.changes,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
