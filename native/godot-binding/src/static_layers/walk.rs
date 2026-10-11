//! The static layers' per-frame walk: which chunks a view meets, the tier each
//! chunk or model takes, and the instances each kind and tier draws and casts.
//! Godot space; `f32` geometry as Godot's `Vector3` and `f64` ratios as
//! GDScript's `float`, so it chooses what the GDScript did, to the bit.

use std::collections::{BTreeMap, HashSet};

/// Floats an instance takes: `MultiMesh.TRANSFORM_3D`'s 3×4 rows, then its tint.
pub const FLOATS: usize = 16;
pub const TIERS: usize = 4;
const COARSEST: usize = TIERS - 1;
/// A model's sun shadow reaches this far past it (`modelDetail.ts`).
pub const SHADOW_MARGIN_M: f32 = 3.0;
/// Casters batch by square regions this big, so a cascade draws only those it meets.
const REGION_M: f32 = 512.0;

pub type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length(a: V3) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// Sim `(x, y, z)` as Godot `(x, z, −y)`.
pub fn to_godot(p: V3) -> V3 {
    [p[0], p[2], -p[1]]
}

/// Godot's `AABB`: its end is always `position + size`.
#[derive(Clone, Copy, Default)]
pub struct Aabb {
    pub lo: V3,
    pub hi: V3,
}

impl Aabb {
    pub fn new(lo: V3, size: V3) -> Self {
        let hi = [0, 1, 2].map(|k| lo[k] + size[k]);
        Self { lo, hi }
    }
    /// A sim box in Godot space (`detail_view.gd` `sim_box`).
    pub fn sim(lo: V3, hi: V3) -> Self {
        let (a, b) = (to_godot(lo), to_godot(hi));
        Self::new(
            [0, 1, 2].map(|k| a[k].min(b[k])),
            [0, 1, 2].map(|k| (a[k] - b[k]).abs()),
        )
    }
    pub fn merge(&self, o: &Aabb) -> Aabb {
        let lo = [0, 1, 2].map(|k| self.lo[k].min(o.lo[k]));
        Aabb::new(lo, sub([0, 1, 2].map(|k| self.hi[k].max(o.hi[k])), lo))
    }
    pub fn around(centre: V3, r: f32) -> Aabb {
        Aabb::new(sub(centre, [r; 3]), [r * 2.0; 3])
    }
    /// Its `lo` corner on each axis `lo` holds, else its `hi`.
    fn corner(&self, lo: [bool; 3]) -> V3 {
        [0, 1, 2].map(|k| if lo[k] { self.lo[k] } else { self.hi[k] })
    }
}

/// What a frame's camera means for detail (`detail_view.gd`): its side
/// planes facing out (normal, `d`), pixels per metre, shadows' fall and reach.
pub struct View {
    pub eye: V3,
    pub pixels_per_metre: f64,
    pub sides: [[f32; 4]; 4],
    pub fall: V3,
    pub reach: f64,
}

impl View {
    /// Whether every side has `b`'s nearest corner (or, `all`, its farthest) inside.
    fn holds(&self, b: &Aabb, all: bool) -> bool {
        self.sides.iter().all(|p| {
            let c = b.corner([0, 1, 2].map(|k| (p[k] > 0.0) != all));
            p[0] * c[0] + p[1] * c[1] + p[2] * c[2] - p[3] <= 0.0
        })
    }
    fn sphere_meets(&self, c: V3, r: f32) -> bool {
        let out = |p: &[f32; 4]| p[0] * c[0] + p[1] * c[1] + p[2] * c[2] - p[3];
        self.sides.iter().all(|p| out(p) <= r)
    }
    fn distance_to(&self, b: &Aabb) -> f32 {
        length(sub(
            [0, 1, 2].map(|k| self.eye[k].clamp(b.lo[k], b.hi[k])),
            self.eye,
        ))
    }
    fn farthest(&self, b: &Aabb) -> f32 {
        let e = self.eye;
        length(sub(
            b.corner([0, 1, 2].map(|k| e[k] - b.lo[k] > b.hi[k] - e[k])),
            e,
        ))
    }
    fn casts_into_view(&self, b: &Aabb, height: f32) -> bool {
        let shifted = [0, 1, 2].map(|k| b.lo[k] + self.fall[k] * height);
        let swept = b.merge(&Aabb::new(shifted, sub(b.hi, b.lo)));
        self.distance_to(&swept) as f64 <= self.reach && self.holds(&swept, false)
    }
    /// The tier `size` metres take seen from `distance`.
    fn tier(&self, size: f32, distance: f32, lod: &[f64; 3]) -> usize {
        let px = size as f64 * self.pixels_per_metre / (distance as f64).max(1e-3);
        lod.iter().position(|&t| px > t).unwrap_or(COARSEST)
    }
}

/// One placed instance: its packed record, the centre and size its tier is
/// chosen by, its reach, its kind, its tier mask (building rows), what hides
/// it (or -1) and all it can draw (scenery chunks follow what stands).
#[derive(Clone, Default)]
pub struct Instance {
    pub pack: Vec<f32>,
    pub centre: V3,
    pub size: f32,
    pub radius: f32,
    pub kind: u32,
    pub mask: u8,
    pub key: i64,
    pub bounds: Aabb,
}

/// A row-major 3×4 transform of `yaw` about Godot's +y and a per-axis sim
/// `scale`, at sim `position`, then `tint` (`model_batches.gd` `pack`).
#[rustfmt::skip]
pub fn pack(position: V3, yaw: f32, [sx, sy, sz]: V3, [r, g, b]: V3) -> Vec<f32> {
    let ((s, c), o) = (yaw.sin_cos(), to_godot(position));
    vec![c * sx, 0.0, s * sy, o[0], 0.0, sz, 0.0, o[1], -s * sx, 0.0, c * sy, o[2], r, g, b, 1.0]
}

/// The rule a population's chunks follow.
#[derive(Clone, Copy, Default)]
pub enum Rule {
    /// `buildingPlacements.ts`: a chunk takes the tier a metre's pixels give
    /// its nearest point; finer than coarse, it is resident while the pool
    /// holds its rows, nearest first.
    Buildings { pool: usize },
    /// The fallen buildings: each chunk at its tier, its rows by mask.
    Ruins,
    /// `lod.ts`: a chunk whose tallest takes the coarsest tier draws whole at
    /// it, else each instance by its own height.
    Scenery { casts: bool },
    /// `modelDetail.ts`: each model culled by its sphere and tiered by its size.
    #[default]
    Models,
}

#[derive(Default)]
pub struct Chunk {
    pub bounds: Aabb,
    region: u32,
    height: f32,
    pub all: Vec<u32>,
    standing: Vec<u32>,
    least: f32,
    most: f32,
}

#[derive(Default)]
pub struct Population {
    pub rule: Rule,
    pub lod: [f64; 3],
    pub instances: Vec<Instance>,
    pub chunks: Vec<Chunk>,
    hidden: Vec<bool>,
    /// Per kind and tier (`kind · 4 + tier`): what the last walk drew.
    pub counts: Vec<i32>,
    /// The most rows the pool held (buildings).
    pub most_resident: usize,
}

impl Population {
    pub fn new(rule: Rule, lod: [f64; 3], kinds: usize) -> Self {
        Self {
            rule,
            lod,
            counts: vec![0; kinds * TIERS],
            ..Default::default()
        }
    }

    /// Adds a chunk of `all` within `bounds`, its shadow swept by `height`.
    pub fn add_chunk(&mut self, bounds: Aabb, height: f32, all: Vec<u32>) {
        let cell = |v: f32| ((v / REGION_M).floor() as i32 + 32768) as u32 & 0xffff;
        let (x, z) = (
            (bounds.lo[0] + bounds.hi[0]) / 2.0,
            (bounds.lo[2] + bounds.hi[2]) / 2.0,
        );
        let region = cell(x) << 16 | cell(z);
        let mut chunk = Chunk {
            bounds,
            region,
            height,
            all,
            ..Default::default()
        };
        self.stand(&mut chunk);
        self.chunks.push(chunk);
    }

    /// Stands `chunk`'s instances not hidden.
    fn stand(&self, chunk: &mut Chunk) {
        let hidden = |i: &u32| self.hidden.get(*i as usize).copied().unwrap_or(false);
        chunk.standing = chunk.all.iter().copied().filter(|i| !hidden(i)).collect();
        let sizes = chunk
            .standing
            .iter()
            .map(|&i| self.instances[i as usize].size);
        chunk.least = sizes.clone().fold(f32::INFINITY, f32::min);
        chunk.most = sizes.fold(0.0, f32::max);
        if let Rule::Scenery { .. } = self.rule {
            let mut boxes = chunk
                .standing
                .iter()
                .map(|&i| self.instances[i as usize].bounds);
            let first = boxes.next().unwrap_or_default();
            chunk.bounds = boxes.fold(first, |a, b| a.merge(&b));
        }
    }

    /// Hides exactly the instances `hidden` marks.
    pub fn set_hidden(&mut self, hidden: Vec<bool>) {
        self.hidden = hidden;
        let mut chunks = std::mem::take(&mut self.chunks);
        chunks.iter_mut().for_each(|c| self.stand(c));
        self.chunks = chunks;
    }

    /// Hides exactly the instances whose key `keys` holds.
    pub fn hide_keys(&mut self, keys: &HashSet<i64>) {
        let hidden = self
            .instances
            .iter()
            .map(|i| i.key >= 0 && keys.contains(&i.key));
        self.set_hidden(hidden.collect());
    }
}

/// Every kind and tier's instances a walk draws, and casts by region.
#[derive(Default)]
pub struct Lists {
    pub view: Vec<Vec<f32>>,
    pub cast: Vec<BTreeMap<u32, Vec<f32>>>,
}

impl Lists {
    pub fn new(keys: usize) -> Self {
        Self {
            view: vec![vec![]; keys],
            cast: vec![BTreeMap::new(); keys],
        }
    }
    pub fn cast(&mut self, key: usize, region: u32, pack: &[f32]) {
        self.cast[key]
            .entry(region)
            .or_default()
            .extend_from_slice(pack);
    }
    /// Casts `inst` at `tier`.
    fn cast_at(&mut self, inst: &Instance, tier: usize, region: u32) {
        self.cast(inst.kind as usize * TIERS + tier, region, &inst.pack);
    }
    /// Draws `inst` at `tier`, counted into `counts`; casts it at `cast` if any.
    fn draw(
        &mut self,
        counts: &mut [i32],
        inst: &Instance,
        tier: usize,
        cast: Option<usize>,
        region: u32,
    ) {
        let key = inst.kind as usize * TIERS + tier;
        self.view[key].extend_from_slice(&inst.pack);
        counts[key] += 1;
        cast.inspect(|&t| self.cast_at(inst, t, region));
    }
}

fn coarser(tier: usize) -> Option<usize> {
    Some((tier + 1).min(COARSEST))
}

/// Adds what `view` draws of `pop` to `lists`, counting it.
pub fn walk(pop: &mut Population, view: &View, lists: &mut Lists) {
    pop.counts.fill(0);
    let (lod, instances, chunks, counts) = (&pop.lod, &pop.instances, &pop.chunks, &mut pop.counts);
    let at = |i: &u32| &instances[*i as usize];
    let has = |i: &u32, t: usize| at(i).mask & (1 << t) != 0;
    let standing = chunks.iter().filter(|c| !c.standing.is_empty());
    match pop.rule {
        Rule::Buildings { pool } => {
            let (mut near, mut coarse) = (vec![], vec![]);
            for (c, chunk) in chunks.iter().enumerate().filter(|(_, c)| !c.all.is_empty()) {
                if view.holds(&chunk.bounds, false) {
                    let d = view.distance_to(&chunk.bounds);
                    match view.tier(1.0, d, lod) {
                        COARSEST => coarse.push((c, d as f64 <= view.reach)),
                        t => near.push((d, c, t)),
                    }
                } else if view.casts_into_view(&chunk.bounds, chunk.height) {
                    for i in chunk.standing.iter().filter(|i| has(i, COARSEST)) {
                        lists.cast_at(at(i), COARSEST, chunk.region);
                    }
                }
            }
            near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let (mut used, mut full) = (0, false);
            for (d, c, t) in near {
                let rows = chunks[c].standing.iter().filter(|i| has(i, t));
                full = full || used + rows.clone().count() > pool;
                if full {
                    coarse.push((c, d as f64 <= view.reach));
                    continue;
                }
                used += rows.clone().count();
                rows.for_each(|i| lists.draw(counts, at(i), t, coarser(t), chunks[c].region));
            }
            pop.most_resident = pop.most_resident.max(used);
            for (c, casts) in coarse {
                let cast = casts.then_some(COARSEST);
                for i in chunks[c].standing.iter().filter(|i| has(i, COARSEST)) {
                    lists.draw(counts, at(i), COARSEST, cast, chunks[c].region);
                }
            }
        }
        Rule::Ruins => {
            for chunk in standing {
                let (level, casts) = if view.holds(&chunk.bounds, false) {
                    let d = view.distance_to(&chunk.bounds);
                    (Some(view.tier(1.0, d, lod)), d as f64 <= view.reach)
                } else {
                    (None, view.casts_into_view(&chunk.bounds, chunk.height))
                };
                for i in &chunk.standing {
                    if let Some(t) = level.filter(|&t| has(i, t)) {
                        lists.draw(counts, at(i), t, None, chunk.region);
                    }
                    if casts && has(i, COARSEST) {
                        lists.cast_at(at(i), COARSEST, chunk.region);
                    }
                }
            }
        }
        Rule::Scenery { casts } => {
            for chunk in standing {
                if !view.holds(&chunk.bounds, false) {
                    if casts && view.casts_into_view(&chunk.bounds, chunk.most) {
                        chunk
                            .standing
                            .iter()
                            .for_each(|i| lists.cast_at(at(i), COARSEST, chunk.region));
                    }
                    continue;
                }
                let d = view.distance_to(&chunk.bounds);
                let near = view.tier(chunk.most, d, lod);
                let whole = near == COARSEST;
                let uniform =
                    whole || near == view.tier(chunk.least, view.farthest(&chunk.bounds), lod);
                for i in &chunk.standing {
                    let t = if uniform {
                        near
                    } else {
                        view.tier(at(i).size, length(sub(at(i).centre, view.eye)), lod)
                    };
                    let cast = match (casts, whole) {
                        (false, _) => None,
                        (true, true) => (d as f64 <= view.reach).then_some(COARSEST),
                        (true, false) => coarser(t),
                    };
                    lists.draw(counts, at(i), t, cast, chunk.region);
                }
            }
        }
        Rule::Models => {
            for chunk in standing.filter(|c| view.holds(&c.bounds, false)) {
                let near = view.tier(chunk.most, view.distance_to(&chunk.bounds), lod);
                let far = view.tier(chunk.least, view.farthest(&chunk.bounds), lod);
                let uniform = view.holds(&chunk.bounds, true) && near == far;
                for inst in chunk.standing.iter().map(at) {
                    if !uniform && !view.sphere_meets(inst.centre, inst.radius + SHADOW_MARGIN_M) {
                        continue;
                    }
                    let t = if uniform {
                        near
                    } else {
                        view.tier(inst.size, length(sub(inst.centre, view.eye)), lod)
                    };
                    lists.draw(counts, inst, t, coarser(t), chunk.region);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera at the origin looking down −z, 90° across, 1000 px a metre
    /// at a metre; the shadow falls along +x. Two 2 m scenery instances (kinds
    /// 0, 1) at Godot `(x, 0, z)`, a chunk each.
    fn walked(rule: Rule, at: [(f32, f32); 2]) -> (Population, Lists) {
        let k = std::f32::consts::FRAC_1_SQRT_2;
        let sides = [
            [k, 0.0, k, 0.0],
            [-k, 0.0, k, 0.0],
            [0.0, k, k, 0.0],
            [0.0, -k, k, 0.0],
        ];
        let view = View {
            eye: [0.0; 3],
            pixels_per_metre: 1000.0,
            sides,
            fall: [1.0, 0.0, 0.0],
            reach: 200.0,
        };
        let mut pop = Population::new(rule, [40.0, 20.0, 10.0], 2);
        for (kind, (x, z)) in at.into_iter().enumerate() {
            let (bounds, pack) = (
                Aabb::new([x - 1.0, -1.0, z - 1.0], [2.0; 3]),
                pack([x, -z, 0.0], 0.0, [1.0; 3], [1.0; 3]),
            );
            let inst = Instance {
                pack,
                centre: [x, 0.0, z],
                size: 2.0,
                radius: 1.0,
                kind: kind as u32,
                bounds,
                ..Default::default()
            };
            pop.instances.push(inst);
            pop.add_chunk(bounds, 2.0, vec![kind as u32]);
        }
        let mut lists = Lists::new(2 * TIERS);
        walk(&mut pop, &view, &mut lists);
        (pop, lists)
    }

    fn cast_at(lists: &Lists, key: usize) -> usize {
        lists.cast[key].values().map(Vec::len).sum::<usize>() / FLOATS
    }

    #[test]
    fn scenery_out_of_view_only_casts_when_its_shadow_lands_in_view() {
        // Behind the camera, and just off its left side with the shadow falling right.
        let (pop, lists) = walked(Rule::Scenery { casts: true }, [(0.0, 20.0), (-14.0, -10.0)]);
        assert!(pop.counts.iter().all(|&n| n == 0));
        assert_eq!(
            (cast_at(&lists, COARSEST), cast_at(&lists, TIERS + COARSEST)),
            (0, 1)
        );
    }

    #[test]
    fn casters_batch_by_the_region_they_stand_in() {
        let (_, lists) = walked(Rule::Models, [(0.0, -10.0), (600.0, -700.0)]);
        let regions: HashSet<u32> = lists.cast.iter().flat_map(|r| r.keys().copied()).collect();
        assert_eq!(regions.len(), 2);
    }
}
