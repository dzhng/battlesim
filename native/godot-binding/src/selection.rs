//! The static world's per-frame selection, as native's buildings, scenery and
//! props layers port it from the browser (`world/building_layer.gd`,
//! `scenery_layer.gd`, `prop_layer.gd` own the rules and their tests): which
//! chunks a view meets, the tier each chunk or model takes, and the instances
//! each appearance and tier draws and casts. GDScript hands the browser's
//! word tables over once per map and the camera every frame; this walks them.
//!
//! Godot space, `f32` geometry as Godot's `Vector3`, `f64` pixel ratios as
//! GDScript's `float`, so the selection is the GDScript's to the bit.

use godot::prelude::*;

/// Floats an instance takes: `MultiMesh.TRANSFORM_3D`'s 3×4 rows, then its tint.
pub const FLOATS: usize = 16;
pub const TIERS: usize = 4;
const COARSEST: usize = TIERS - 1;
/// A model's sun shadow reaches this far past it (`modelDetail.ts`).
const SHADOW_MARGIN_M: f32 = 3.0;
/// Casters are batched by square regions this big, so each cascade draws
/// only the regions it covers (Godot culls a batch by its bounds).
const REGION_M: f32 = 512.0;

type V3 = [f32; 3];

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Aabb {
    pub lo: V3,
    pub hi: V3,
}

impl Aabb {
    /// Godot's `AABB(position, size)`: its end is `position + size`.
    fn new(position: V3, size: V3) -> Self {
        Self { lo: position, hi: add(position, size) }
    }
    /// A sim box `lo`..`hi` in Godot space (`detail_view.gd` `sim_box`).
    pub fn sim(lo: V3, hi: V3) -> Self {
        let (a, b) = (to_godot(lo), to_godot(hi));
        let position = [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])];
        let size = [(a[0] - b[0]).abs(), (a[1] - b[1]).abs(), (a[2] - b[2]).abs()];
        Self::new(position, size)
    }
    /// Godot's `AABB.merge`.
    fn merge(&self, o: &Aabb) -> Aabb {
        let lo = [self.lo[0].min(o.lo[0]), self.lo[1].min(o.lo[1]), self.lo[2].min(o.lo[2])];
        let hi = [self.hi[0].max(o.hi[0]), self.hi[1].max(o.hi[1]), self.hi[2].max(o.hi[2])];
        Aabb::new(lo, sub(hi, lo))
    }
    fn size(&self) -> V3 {
        sub(self.hi, self.lo)
    }
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn length(a: V3) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
/// Sim `(x, y, z)` as Godot `(x, z, −y)` (`core/coords.gd`).
pub fn to_godot(p: V3) -> V3 {
    [p[0], p[2], -p[1]]
}

/// What a frame's camera means for detail (`detail_view.gd`).
pub struct View {
    pub eye: V3,
    pub pixels_per_metre: f64,
    /// Side planes facing out: normal and offset (`Plane.d`).
    pub sides: [[f32; 4]; 4],
    pub fall: V3,
    pub reach: f64,
}

impl View {
    fn plane_distance(plane: &[f32; 4], p: V3) -> f32 {
        plane[0] * p[0] + plane[1] * p[1] + plane[2] * p[2] - plane[3]
    }
    pub fn box_meets(&self, b: &Aabb) -> bool {
        self.sides.iter().all(|pl| {
            let near = [
                if pl[0] > 0.0 { b.lo[0] } else { b.hi[0] },
                if pl[1] > 0.0 { b.lo[1] } else { b.hi[1] },
                if pl[2] > 0.0 { b.lo[2] } else { b.hi[2] },
            ];
            Self::plane_distance(pl, near) <= 0.0
        })
    }
    pub fn box_within(&self, b: &Aabb) -> bool {
        self.sides.iter().all(|pl| {
            let far = [
                if pl[0] > 0.0 { b.hi[0] } else { b.lo[0] },
                if pl[1] > 0.0 { b.hi[1] } else { b.lo[1] },
                if pl[2] > 0.0 { b.hi[2] } else { b.lo[2] },
            ];
            Self::plane_distance(pl, far) <= 0.0
        })
    }
    pub fn sphere_meets(&self, c: V3, r: f32) -> bool {
        self.sides.iter().all(|pl| Self::plane_distance(pl, c) <= r)
    }
    pub fn distance_to(&self, b: &Aabb) -> f32 {
        let e = self.eye;
        let c = [e[0].clamp(b.lo[0], b.hi[0]), e[1].clamp(b.lo[1], b.hi[1]), e[2].clamp(b.lo[2], b.hi[2])];
        length(sub(c, e))
    }
    pub fn farthest(&self, b: &Aabb) -> f32 {
        let e = self.eye;
        let pick = |i: usize| if e[i] - b.lo[i] > b.hi[i] - e[i] { b.lo[i] } else { b.hi[i] };
        length(sub([pick(0), pick(1), pick(2)], e))
    }
    pub fn distance(&self, p: V3) -> f32 {
        length(sub(p, self.eye))
    }
    pub fn casts_into_view(&self, b: &Aabb, height: f32) -> bool {
        let shifted = Aabb::new(add(b.lo, [self.fall[0] * height, self.fall[1] * height, self.fall[2] * height]), b.size());
        let swept = b.merge(&shifted);
        (self.distance_to(&swept) as f64) <= self.reach && self.box_meets(&swept)
    }
}

/// The tier `px` projected pixels take under `lod` (else the coarsest).
fn tier(px: f64, lod: &[f64; 3]) -> usize {
    if px > lod[0] {
        0
    } else if px > lod[1] {
        1
    } else if px > lod[2] {
        2
    } else {
        COARSEST
    }
}

/// One placed instance: its packed draw record, the centre and size its tier
/// is chosen by, its reach (models' cull), its kind (appearance in the
/// layer's batches) and its tier mask (building rows).
#[derive(Clone)]
pub struct Instance {
    pub pack: [f32; FLOATS],
    pub centre: V3,
    pub size: f32,
    pub radius: f32,
    pub kind: u32,
    pub mask: u8,
    /// What hides it (a map prop, a building), or -1.
    pub key: i64,
    /// All it can draw, where its chunk's box follows what stands (scenery).
    pub bounds: Aabb,
}

/// A row-major 3×4 transform of yaw about Godot's +y and a per-axis sim
/// scale, at sim `position`, tinted (`model_batches.gd` `pack`).
pub fn pack(position: V3, yaw: f32, scale: V3, tint: V3) -> [f32; FLOATS] {
    let (s, c) = yaw.sin_cos();
    let o = to_godot(position);
    // Columns: x = (c, 0, −s)·sx, y = (0, 1, 0)·sz, z = (s, 0, c)·sy.
    [
        c * scale[0], 0.0, s * scale[1], o[0],
        0.0, scale[2], 0.0, o[1],
        -s * scale[0], 0.0, c * scale[1], o[2],
        tint[0], tint[1], tint[2], 1.0,
    ]
}

/// The rule a population's chunks follow.
#[derive(Clone, Copy)]
pub enum Rule {
    /// `buildingPlacements.ts`: a chunk takes the tier a metre's pixels give
    /// its nearest point; finer than coarse it is resident while the pool
    /// holds its rows, nearest first.
    Buildings { pool: usize },
    /// The fallen buildings: each chunk at its tier, rows by mask; the coarse
    /// rows cast.
    Ruins,
    /// `lod.ts`: a chunk whose tallest takes the coarsest tier draws whole at
    /// it, else each instance by its own height.
    Scenery { casts: bool },
    /// `modelDetail.ts`: each model culled by its sphere and tiered by its
    /// size; nothing out of view casts.
    Models,
}

pub struct Chunk {
    pub bounds: Aabb,
    /// The caster region it stands in.
    pub region: u32,
    pub height: f32,
    /// Its instances (indices), all of them and those standing.
    pub all: Vec<u32>,
    pub standing: Vec<u32>,
    /// Its standing instances' least and most size.
    pub least: f32,
    pub most: f32,
}

pub struct Population {
    pub rule: Rule,
    pub lod: [f64; 3],
    pub instances: Vec<Instance>,
    pub chunks: Vec<Chunk>,
    pub hidden: Vec<bool>,
    /// Per kind and tier (`kind · 4 + tier`): what the last selection drew.
    pub counts: Vec<i32>,
    /// The most rows the pool held (buildings).
    pub most_resident: usize,
}

impl Population {
    pub fn new(rule: Rule, lod: [f64; 3], kinds: usize) -> Self {
        Self { rule, lod, instances: vec![], chunks: vec![], hidden: vec![], counts: vec![0; kinds * TIERS], most_resident: 0 }
    }

    /// Adds a chunk of `members` with `bounds` and the `height` its shadow is
    /// swept by.
    pub fn add_chunk(&mut self, bounds: Aabb, height: f32, members: Vec<u32>) {
        let centre = [(bounds.lo[0] + bounds.hi[0]) / 2.0, (bounds.lo[2] + bounds.hi[2]) / 2.0];
        let cell = |v: f32| ((v / REGION_M).floor() as i32 + 32768) as u32 & 0xffff;
        let region = cell(centre[0]) << 16 | cell(centre[1]);
        let mut chunk = Chunk { bounds, region, height, all: members, standing: vec![], least: 0.0, most: 0.0 };
        self.stand(&mut chunk);
        self.chunks.push(chunk);
    }

    fn stand(&self, chunk: &mut Chunk) {
        chunk.standing = chunk.all.iter().copied().filter(|&i| !self.hidden.get(i as usize).copied().unwrap_or(false)).collect();
        chunk.least = f32::INFINITY;
        chunk.most = 0.0;
        for &i in &chunk.standing {
            let size = self.instances[i as usize].size;
            chunk.least = chunk.least.min(size);
            chunk.most = chunk.most.max(size);
        }
        // A scenery chunk's box is what stands in it (`scenery_layer.gd` `_rechunk`).
        if let Rule::Scenery { .. } = self.rule {
            let mut bounds: Option<Aabb> = None;
            for &i in &chunk.standing {
                let b = self.instances[i as usize].bounds;
                bounds = Some(bounds.map_or(b, |a| a.merge(&b)));
            }
            chunk.bounds = bounds.unwrap_or_default();
        }
    }

    /// Hides exactly the instances whose key `keys` holds.
    pub fn hide_keys(&mut self, keys: &std::collections::HashSet<i64>) {
        let hidden = self.instances.iter().map(|inst| inst.key >= 0 && keys.contains(&inst.key)).collect();
        self.set_hidden(hidden);
    }

    /// Hides exactly the instances `hidden` marks.
    pub fn set_hidden(&mut self, hidden: Vec<bool>) {
        self.hidden = hidden;
        let mut chunks = std::mem::take(&mut self.chunks);
        for chunk in &mut chunks {
            self.stand(chunk);
        }
        self.chunks = chunks;
    }
}

/// Every appearance and tier's instances a selection draws, and casts by
/// region.
pub struct Lists {
    pub view: Vec<Vec<f32>>,
    pub cast: Vec<std::collections::BTreeMap<u32, Vec<f32>>>,
}

impl Lists {
    pub fn new(kinds: usize) -> Self {
        Self { view: vec![vec![]; kinds * TIERS], cast: vec![Default::default(); kinds * TIERS] }
    }
    fn clear(&mut self) {
        self.view.iter_mut().for_each(Vec::clear);
        self.cast.iter_mut().for_each(|regions| regions.values_mut().for_each(Vec::clear));
    }
    fn cast(&mut self, key: usize, region: u32, pack: &[f32]) {
        self.cast[key].entry(region).or_default().extend_from_slice(pack);
    }
}

/// Adds `pop`'s instance `i` at `tier` to the view (counted), casting it a
/// tier coarser when `casts`.
fn draw(pop_counts: &mut [i32], lists: &mut Lists, inst: &Instance, tier: usize, casts: bool, region: u32) {
    let key = inst.kind as usize * TIERS + tier;
    lists.view[key].extend_from_slice(&inst.pack);
    pop_counts[key] += 1;
    if casts {
        lists.cast(key - tier + (tier + 1).min(COARSEST), region, &inst.pack);
    }
}

fn cast_at(lists: &mut Lists, inst: &Instance, tier: usize, region: u32) {
    lists.cast(inst.kind as usize * TIERS + tier, region, &inst.pack);
}

/// Adds what `view` draws of `pop` to `lists`, counting it.
pub fn select(pop: &mut Population, view: &View, lists: &mut Lists) {
    pop.counts.iter_mut().for_each(|n| *n = 0);
    let ppm = view.pixels_per_metre;
    let lod = pop.lod;
    match pop.rule {
        Rule::Buildings { pool } => {
            let mut near: Vec<(f32, usize, usize)> = vec![];
            let mut coarse: Vec<(usize, bool)> = vec![];
            for (c, chunk) in pop.chunks.iter().enumerate() {
                if chunk.all.is_empty() {
                    continue;
                }
                if view.box_meets(&chunk.bounds) {
                    let distance = view.distance_to(&chunk.bounds);
                    let wanted = tier(ppm / (distance as f64).max(1e-3), &lod);
                    if wanted < COARSEST {
                        near.push((distance, c, wanted));
                    } else {
                        coarse.push((c, distance as f64 <= view.reach));
                    }
                } else if view.casts_into_view(&chunk.bounds, chunk.height) {
                    for &i in &pop.chunks[c].standing {
                        let inst = &pop.instances[i as usize];
                        if inst.mask & (1 << COARSEST) != 0 {
                            cast_at(lists, inst, COARSEST, chunk.region);
                        }
                    }
                }
            }
            near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut used = 0;
            let mut full = false;
            for (distance, c, wanted) in near {
                let rows = pop.chunks[c].standing.iter().filter(|&&i| pop.instances[i as usize].mask & (1 << wanted) != 0).count();
                if !full && used + rows <= pool {
                    used += rows;
                    for &i in &pop.chunks[c].standing {
                        let inst = &pop.instances[i as usize];
                        if inst.mask & (1 << wanted) != 0 {
                            draw(&mut pop.counts, lists, inst, wanted, true, pop.chunks[c].region);
                        }
                    }
                } else {
                    full = true;
                    coarse.push((c, distance as f64 <= view.reach));
                }
            }
            pop.most_resident = pop.most_resident.max(used);
            for (c, casts) in coarse {
                for &i in &pop.chunks[c].standing {
                    let inst = &pop.instances[i as usize];
                    if inst.mask & (1 << COARSEST) != 0 {
                        let key = inst.kind as usize * TIERS + COARSEST;
                        lists.view[key].extend_from_slice(&inst.pack);
                        pop.counts[key] += 1;
                        if casts {
                            cast_at(lists, inst, COARSEST, pop.chunks[c].region);
                        }
                    }
                }
            }
        }
        Rule::Ruins => {
            for chunk in &pop.chunks {
                if chunk.standing.is_empty() {
                    continue;
                }
                let (level, casts) = if view.box_meets(&chunk.bounds) {
                    let distance = view.distance_to(&chunk.bounds);
                    (Some(tier(ppm / (distance as f64).max(1e-3), &lod)), distance as f64 <= view.reach)
                } else {
                    (None, view.casts_into_view(&chunk.bounds, chunk.height))
                };
                for &i in &chunk.standing {
                    let inst = &pop.instances[i as usize];
                    if let Some(level) = level {
                        if inst.mask & (1 << level) != 0 {
                            let key = inst.kind as usize * TIERS + level;
                            lists.view[key].extend_from_slice(&inst.pack);
                            pop.counts[key] += 1;
                        }
                    }
                    if casts && inst.mask & (1 << COARSEST) != 0 {
                        cast_at(lists, inst, COARSEST, chunk.region);
                    }
                }
            }
        }
        Rule::Scenery { casts } => {
            for chunk in &pop.chunks {
                if chunk.standing.is_empty() {
                    continue;
                }
                if !view.box_meets(&chunk.bounds) {
                    if casts && view.casts_into_view(&chunk.bounds, chunk.most) {
                        for &i in &chunk.standing {
                            cast_at(lists, &pop.instances[i as usize], COARSEST, chunk.region);
                        }
                    }
                    continue;
                }
                let distance = view.distance_to(&chunk.bounds);
                let near = tier(chunk.most as f64 * ppm / (distance as f64).max(1e-3), &lod);
                if near == COARSEST {
                    let far_casts = casts && distance as f64 <= view.reach;
                    for &i in &chunk.standing {
                        let inst = &pop.instances[i as usize];
                        let key = inst.kind as usize * TIERS + COARSEST;
                        lists.view[key].extend_from_slice(&inst.pack);
                        pop.counts[key] += 1;
                        if far_casts {
                            cast_at(lists, inst, COARSEST, chunk.region);
                        }
                    }
                    continue;
                }
                let uniform = near == tier(chunk.least as f64 * ppm / (view.farthest(&chunk.bounds) as f64).max(1e-3), &lod);
                for &i in &chunk.standing {
                    let inst = &pop.instances[i as usize];
                    let t = if uniform { near } else { tier(inst.size as f64 * ppm / (view.distance(inst.centre) as f64).max(1e-3), &lod) };
                    draw(&mut pop.counts, lists, inst, t, casts, chunk.region);
                }
            }
        }
        Rule::Models => {
            for chunk in &pop.chunks {
                if chunk.standing.is_empty() || !view.box_meets(&chunk.bounds) {
                    continue;
                }
                let mut uniform = None;
                if view.box_within(&chunk.bounds) {
                    let near = tier(chunk.most as f64 * ppm / (view.distance_to(&chunk.bounds) as f64).max(1e-3), &lod);
                    if near == tier(chunk.least as f64 * ppm / (view.farthest(&chunk.bounds) as f64).max(1e-3), &lod) {
                        uniform = Some(near);
                    }
                }
                for &i in &chunk.standing {
                    let inst = &pop.instances[i as usize];
                    let t = match uniform {
                        Some(t) => t,
                        None => {
                            if !view.sphere_meets(inst.centre, inst.radius + SHADOW_MARGIN_M) {
                                continue;
                            }
                            tier(inst.size as f64 * ppm / (view.distance(inst.centre) as f64).max(1e-3), &lod)
                        }
                    };
                    draw(&mut pop.counts, lists, inst, t, true, chunk.region);
                }
            }
        }
    }
}

/// A word table's word `w` as `f32`, `i32`.
pub struct Words<'a>(pub &'a [u8]);
impl Words<'_> {
    pub fn f(&self, w: usize) -> f32 {
        f32::from_le_bytes(self.0[w * 4..w * 4 + 4].try_into().unwrap())
    }
    pub fn i(&self, w: usize) -> i32 {
        i32::from_le_bytes(self.0[w * 4..w * 4 + 4].try_into().unwrap())
    }
    pub fn v3(&self, w: usize) -> V3 {
        [self.f(w), self.f(w + 1), self.f(w + 2)]
    }
}

/// The selection of one layer: its populations, the lists they draw into
/// and those of the last frame, so only what changed is handed back.
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct StaticSelection {
    kinds: usize,
    pops: Vec<Population>,
    lists: Option<Lists>,
    last: Option<Lists>,
}

/// A layout's `{name: word}` offsets, as GDScript reads them (`WordTables.records`).
fn word(at: &VarDictionary, name: &str) -> usize {
    at.get(name).map(|v| v.to::<i64>() as usize).unwrap_or_else(|| panic!("no field {name}"))
}

#[godot_api]
impl StaticSelection {
    /// The appearances (kinds) this layer's batches hold.
    #[func]
    fn set_kinds(&mut self, kinds: i64) {
        self.kinds = kinds as usize;
        self.lists = Some(Lists::new(self.kinds));
        self.last = Some(Lists::new(self.kinds));
    }

    /// A population of scenery records (`SceneryInstance`), chunked by
    /// `chunk_m` where they stand; returns its id.
    #[func]
    fn add_scenery(&mut self, words: PackedByteArray, stride: i64, at: VarDictionary, chunk_m: f64, lod: PackedFloat64Array, casts: bool) -> i64 {
        let w = Words(words.as_slice());
        let stride = stride as usize;
        let n = words.len() / 4 / stride.max(1);
        let mut pop = Population::new(Rule::Scenery { casts }, lod3(&lod), self.kinds);
        let (pose, shape, tint, kind, height, reach, id) = (word(&at, "pose"), word(&at, "shape"), word(&at, "tint"), word(&at, "kind"), word(&at, "height"), word(&at, "reach"), word(&at, "id"));
        let mut cells: std::collections::HashMap<(i64, i64), (Aabb, Vec<u32>)> = Default::default();
        let mut order: Vec<(i64, i64)> = vec![];
        for r in 0..n {
            let o = r * stride;
            let (x, y, z) = (w.f(o + pose), w.f(o + pose + 1), w.f(o + pose + 2));
            let h = w.f(o + height);
            let reach_m = w.f(o + reach);
            pop.instances.push(Instance {
                pack: pack([x, y, z], w.f(o + pose + 3), [w.f(o + shape), w.f(o + shape + 1), w.f(o + shape + 2)], w.v3(o + tint)),
                centre: to_godot([x, y, z + h / 2.0]),
                size: h,
                radius: reach_m,
                kind: w.i(o + kind) as u32,
                mask: 0,
                // A forest tree is hidden by its trunk's body; no body is u32::MAX.
                key: if w.i(o + id) as u32 == u32::MAX { -1 } else { w.i(o + id) as u32 as i64 },
                bounds: Aabb::sim([x - reach_m, y - reach_m, z], [x + reach_m, y + reach_m, z + h]),
            });
            let bounds = pop.instances[r].bounds;
            let cell = ((x as f64 / chunk_m).floor() as i64, (y as f64 / chunk_m).floor() as i64);
            let entry = cells.entry(cell).or_insert_with(|| {
                order.push(cell);
                (bounds, vec![])
            });
            entry.0 = if entry.1.is_empty() { bounds } else { entry.0.merge(&bounds) };
            entry.1.push(r as u32);
        }
        for cell in order {
            let (bounds, members) = cells.remove(&cell).unwrap();
            pop.add_chunk(bounds, 0.0, members);
        }
        self.push(pop)
    }

    /// The map's fitted props (`props-map.u32`), each look `sizes`/`radii`
    /// big a unit scale, chunked by `chunk_m`; returns its id. The props a
    /// key in `keys` names (a map prop id per model, -1 none) can be hidden.
    #[func]
    fn add_props(&mut self, words: PackedByteArray, stride: i64, at: VarDictionary, sizes: PackedFloat32Array, radii: PackedFloat32Array, chunk_m: f64, lod: PackedFloat64Array) -> i64 {
        let w = Words(words.as_slice());
        let stride = stride as usize;
        let n = words.len() / 4 / stride.max(1);
        let mut pop = Population::new(Rule::Models, lod3(&lod), self.kinds);
        let (position, yaw, scale, look, tint, prop) = (word(&at, "position"), word(&at, "yaw"), word(&at, "scale"), word(&at, "look"), word(&at, "tint"), word(&at, "prop"));
        let mut cells: std::collections::HashMap<(i64, i64), (Aabb, Vec<u32>)> = Default::default();
        let mut order: Vec<(i64, i64)> = vec![];
        for r in 0..n {
            let o = r * stride;
            let p = w.v3(o + position);
            let s = w.v3(o + scale);
            let l = w.i(o + look) as usize;
            if sizes.as_slice()[l] <= 0.0 {
                // A look that is not staged: placed but never drawn.
                pop.instances.push(Instance { pack: [0.0; FLOATS], centre: [0.0; 3], size: 0.0, radius: 0.0, kind: l as u32, mask: 0, key: -1, bounds: Aabb::default() });
                continue;
            }
            let grow = s[0].max(s[1].max(s[2]));
            let packed = pack(p, w.f(o + yaw), s, w.v3(o + tint));
            let size = sizes.as_slice()[l] * grow;
            let radius = radii.as_slice()[l] * grow;
            let centre = add([packed[3], packed[7], packed[11]], [0.0, size / 2.0, 0.0]);
            let key = w.i(o + prop) as u32;
            pop.instances.push(Instance { pack: packed, centre, size, radius, kind: l as u32, mask: 0, key: if key == u32::MAX { -1 } else { key as i64 }, bounds: Aabb::default() });
            let r3 = radius + SHADOW_MARGIN_M;
            let bounds = Aabb::new(sub(centre, [r3, r3, r3]), [r3 * 2.0, r3 * 2.0, r3 * 2.0]);
            let cell = ((p[0] as f64 / chunk_m).floor() as i64, (p[1] as f64 / chunk_m).floor() as i64);
            let entry = cells.entry(cell).or_insert_with(|| {
                order.push(cell);
                (bounds, vec![])
            });
            entry.0 = if entry.1.is_empty() { bounds } else { entry.0.merge(&bounds) };
            entry.1.push(r as u32);
        }
        for cell in order {
            let (bounds, members) = cells.remove(&cell).unwrap();
            pop.add_chunk(bounds, 0.0, members);
        }
        self.push(pop)
    }

    /// A population of models placed per frame (known props); returns its id.
    #[func]
    fn add_models(&mut self, lod: PackedFloat64Array) -> i64 {
        self.push(Population::new(Rule::Models, lod3(&lod), self.kinds))
    }

    /// Replaces population `pop`'s models: `packs` (16 floats each), their
    /// kinds, sizes and reaches; centres half their size above their origin.
    #[func]
    fn set_models(&mut self, pop: i64, packs: PackedFloat32Array, kinds: PackedInt32Array, sizes: PackedFloat32Array, radii: PackedFloat32Array) {
        let p = &mut self.pops[pop as usize];
        let packs = packs.as_slice();
        p.instances.clear();
        p.chunks.clear();
        p.hidden.clear();
        let mut bounds: Option<Aabb> = None;
        for (i, &kind) in kinds.as_slice().iter().enumerate() {
            let mut pk = [0.0; FLOATS];
            pk.copy_from_slice(&packs[i * FLOATS..(i + 1) * FLOATS]);
            let (size, radius) = (sizes.as_slice()[i], radii.as_slice()[i]);
            let centre = add([pk[3], pk[7], pk[11]], [0.0, size / 2.0, 0.0]);
            let r3 = radius + SHADOW_MARGIN_M;
            let b = Aabb::new(sub(centre, [r3, r3, r3]), [r3 * 2.0, r3 * 2.0, r3 * 2.0]);
            bounds = Some(bounds.map_or(b, |a| a.merge(&b)));
            p.instances.push(Instance { pack: pk, centre, size, radius, kind: kind as u32, mask: 0, key: -1, bounds: Aabb::default() });
        }
        if let Some(b) = bounds {
            p.add_chunk(b, 0.0, (0..p.instances.len() as u32).collect());
        }
    }

    /// The buildings: their intact rows (`BuildingModule`) chunked by the
    /// exported chunk of their building (`buildings` table, `BuildingRecord`),
    /// each chunk bounded by its buildings; returns its id. Its fallen are a
    /// second population (`set_fallen`).
    #[func]
    fn add_buildings(&mut self, placed: PackedByteArray, placed_stride: i64, placed_at: VarDictionary, rows: PackedByteArray, rows_stride: i64, rows_at: VarDictionary, lod: PackedFloat64Array, pool: i64) -> i64 {
        let pw = Words(placed.as_slice());
        let ps = placed_stride as usize;
        let count = placed.len() / 4 / ps.max(1);
        let (min, max, chunk_at, rows_field) = (word(&placed_at, "min"), word(&placed_at, "max"), word(&placed_at, "chunk"), word(&placed_at, "rows"));
        let mut pop = Population::new(Rule::Buildings { pool: pool as usize }, lod3(&lod), self.kinds);
        load_rows(&mut pop, &rows, rows_stride as usize, &rows_at);
        let chunks = (0..count).map(|b| pw.i(b * ps + chunk_at)).max().unwrap_or(-1) + 1;
        let mut members: Vec<Vec<u32>> = vec![vec![]; chunks as usize];
        let mut bounds: Vec<Option<Aabb>> = vec![None; chunks as usize];
        let mut heights = vec![0.0f32; chunks as usize];
        let mut first = 0usize;
        for b in 0..count {
            let o = b * ps;
            let n = pw.i(o + rows_field).max(0) as usize;
            let c = pw.i(o + chunk_at);
            if c >= 0 {
                let c = c as usize;
                let (lo, hi) = (pw.v3(o + min), pw.v3(o + max));
                let bb = Aabb::sim(lo, hi);
                bounds[c] = Some(bounds[c].map_or(bb, |a| a.merge(&bb)));
                heights[c] = heights[c].max(hi[2] - lo[2]);
                members[c].extend((first..first + n).map(|r| r as u32));
            }
            first += n;
        }
        for c in 0..chunks as usize {
            pop.add_chunk(bounds[c].unwrap_or_default(), heights[c], std::mem::take(&mut members[c]));
        }
        self.push(pop)
    }

    /// The fallen buildings' population (`pop`, or a new one when -1): the
    /// rows `rows` holds (each fallen building's ruin or gutted rows), chunk
    /// by chunk; replaces the last. Returns its id.
    #[func]
    fn set_ruins(&mut self, pop: i64, rows: PackedByteArray, rows_stride: i64, rows_at: VarDictionary, boxes: PackedFloat32Array, heights: PackedFloat32Array, firsts: PackedInt32Array, counts: PackedInt32Array, lod: PackedFloat64Array) -> i64 {
        // Each chunk: its cell's rows `firsts[c]..firsts[c] + counts[c]` of
        // `rows`, its box (6 floats: position, end) and height.
        let mut p = Population::new(Rule::Ruins, lod3(&lod), self.kinds);
        load_rows(&mut p, &rows, rows_stride as usize, &rows_at);
        let b = boxes.as_slice();
        for c in 0..firsts.len() {
            let bounds = Aabb { lo: [b[c * 6], b[c * 6 + 1], b[c * 6 + 2]], hi: [b[c * 6 + 3], b[c * 6 + 4], b[c * 6 + 5]] };
            let first = firsts.as_slice()[c] as u32;
            p.add_chunk(bounds, heights.as_slice()[c], (first..first + counts.as_slice()[c] as u32).collect());
        }
        if pop < 0 {
            self.push(p)
        } else {
            self.pops[pop as usize] = p;
            pop
        }
    }

    /// Hides population `pop`'s instances `hidden` marks (one byte each).
    #[func]
    fn set_hidden(&mut self, pop: i64, hidden: PackedByteArray) {
        self.pops[pop as usize].set_hidden(hidden.as_slice().iter().map(|&b| b != 0).collect());
    }

    /// Hides exactly population `pop`'s instances whose key `keys` holds (a
    /// building's rows by its index, a map prop's models by its id).
    #[func]
    fn hide_keys(&mut self, pop: i64, keys: PackedInt64Array) {
        self.pops[pop as usize].hide_keys(&keys.as_slice().iter().copied().collect());
    }

    /// Selects what the view draws and casts; returns the keys
    /// (`kind · 4 + tier`) whose view or cast instances changed.
    #[func]
    fn select(&mut self, eye: Vector3, sides: PackedFloat32Array, pixels_per_metre: f64, reach: f64, fall: Vector3) -> PackedInt32Array {
        let s = sides.as_slice();
        let view = View {
            eye: [eye.x, eye.y, eye.z],
            pixels_per_metre,
            sides: [0, 1, 2, 3].map(|k| [s[k * 4], s[k * 4 + 1], s[k * 4 + 2], s[k * 4 + 3]]),
            fall: [fall.x, fall.y, fall.z],
            reach,
        };
        let mut lists = self.lists.take().unwrap_or_else(|| Lists::new(self.kinds));
        lists.clear();
        for pop in &mut self.pops {
            select(pop, &view, &mut lists);
        }
        let last = self.last.as_ref().unwrap();
        let changed: Vec<i32> = (0..self.kinds * TIERS).filter(|&k| lists.view[k] != last.view[k] || !same_regions(&lists.cast[k], &last.cast[k])).map(|k| k as i32).collect();
        // Keep this frame's as the last; reuse the last's storage next frame.
        self.lists = self.last.replace(lists);
        PackedInt32Array::from(changed.as_slice())
    }

    /// The instances the last selection draws at key `key`.
    #[func]
    fn view(&self, key: i64) -> PackedFloat32Array {
        PackedFloat32Array::from(self.last.as_ref().unwrap().view[key as usize].as_slice())
    }

    /// The instances the last selection casts at key `key`, by caster region.
    #[func]
    fn cast(&self, key: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        for (region, pack) in &self.last.as_ref().unwrap().cast[key as usize] {
            if !pack.is_empty() {
                out.set(*region as i64, &PackedFloat32Array::from(pack.as_slice()));
            }
        }
        out
    }

    /// What population `pop` drew last, per kind and tier.
    #[func]
    fn counts(&self, pop: i64) -> PackedInt32Array {
        PackedInt32Array::from(self.pops[pop as usize].counts.as_slice())
    }

    /// The most rows population `pop`'s pool held in one selection.
    #[func]
    fn most_resident(&self, pop: i64) -> i64 {
        self.pops[pop as usize].most_resident as i64
    }
}

impl StaticSelection {
    fn push(&mut self, pop: Population) -> i64 {
        self.pops.push(pop);
        self.pops.len() as i64 - 1
    }
}

/// Whether two keys' casts hold the same instances in every region.
fn same_regions(a: &std::collections::BTreeMap<u32, Vec<f32>>, b: &std::collections::BTreeMap<u32, Vec<f32>>) -> bool {
    a.iter().filter(|(_, v)| !v.is_empty()).eq(b.iter().filter(|(_, v)| !v.is_empty()))
}

fn lod3(lod: &PackedFloat64Array) -> [f64; 3] {
    let s = lod.as_slice();
    [s[0], s[1], s[2]]
}

/// Building rows (`BuildingModule`) as instances: kind is the module, mask
/// the tiers it draws at.
fn load_rows(pop: &mut Population, rows: &PackedByteArray, stride: usize, at: &VarDictionary) {
    let w = Words(rows.as_slice());
    let n = rows.len() / 4 / stride.max(1);
    let (position, yaw, scale, module, tint, tiers, building) = (word(at, "position"), word(at, "yaw"), word(at, "scale"), word(at, "kitModule"), word(at, "tint"), word(at, "tiers"), word(at, "building"));
    pop.instances.reserve(n);
    for r in 0..n {
        let o = r * stride;
        pop.instances.push(Instance {
            pack: pack(w.v3(o + position), w.f(o + yaw), w.v3(o + scale), w.v3(o + tint)),
            centre: [0.0; 3],
            size: 0.0,
            radius: 0.0,
            kind: w.i(o + module) as u32,
            mask: w.i(o + tiers) as u8,
            key: w.i(o + building) as i64,
            bounds: Aabb::default(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera at the origin looking down Godot's −z, 90° across, 1000 px
    /// a metre at a metre away; the sun's shadow falls along +x.
    fn view() -> View {
        let k = std::f32::consts::FRAC_1_SQRT_2;
        View {
            eye: [0.0; 3],
            pixels_per_metre: 1000.0,
            sides: [[k, 0.0, k, 0.0], [-k, 0.0, k, 0.0], [0.0, k, k, 0.0], [0.0, -k, k, 0.0]],
            fall: [1.0, 0.0, 0.0],
            reach: 200.0,
        }
    }

    /// A 2 m instance of `kind` standing at Godot `(x, 0, z)`, hidden by `key`.
    fn at(kind: u32, x: f32, z: f32, key: i64) -> Instance {
        let bounds = Aabb::new([x - 1.0, -1.0, z - 1.0], [2.0; 3]);
        Instance { pack: pack([x, -z, 0.0], 0.0, [1.0; 3], [1.0; 3]), centre: [x, 0.0, z], size: 2.0, radius: 1.0, kind, mask: 0b1111, key, bounds }
    }

    /// One chunk per instance, its box the instance's.
    fn population(rule: Rule, instances: Vec<Instance>) -> Population {
        let mut pop = Population::new(rule, [40.0, 20.0, 10.0], 3);
        pop.hidden = vec![false; instances.len()];
        pop.instances = instances;
        for i in 0..pop.instances.len() {
            let bounds = pop.instances[i].bounds;
            pop.add_chunk(bounds, 2.0, vec![i as u32]);
        }
        pop
    }

    fn drawn(pop: &Population, kind: usize) -> [i32; TIERS] {
        pop.counts[kind * TIERS..kind * TIERS + TIERS].try_into().unwrap()
    }

    #[test]
    fn the_pool_takes_the_nearest_buildings_and_the_rest_draw_coarse() {
        // 9 m wants tier 0, 31 m tier 1, 500 m the coarsest; the pool holds one row.
        let mut pop = population(Rule::Buildings { pool: 1 }, vec![at(0, 0.0, -10.0, 0), at(1, 0.0, -32.0, 1), at(2, 0.0, -501.0, 2)]);
        let mut lists = Lists::new(3);
        select(&mut pop, &view(), &mut lists);
        assert_eq!([drawn(&pop, 0), drawn(&pop, 1), drawn(&pop, 2)], [[1, 0, 0, 0], [0, 0, 0, 1], [0, 0, 0, 1]]);
        assert_eq!(pop.most_resident, 1);
        // The resident casts a tier coarser; the far one is past the shadows' reach.
        assert_eq!(lists.cast[1].values().map(Vec::len).sum::<usize>(), FLOATS);
        assert!(lists.cast[2 * TIERS + COARSEST].values().all(Vec::is_empty));

        // Known fallen, the nearest leaves the pool to the next.
        pop.hide_keys(&[0].into());
        let mut lists = Lists::new(3);
        select(&mut pop, &view(), &mut lists);
        assert_eq!([drawn(&pop, 0), drawn(&pop, 1)], [[0; TIERS], [0, 1, 0, 0]]);
    }

    #[test]
    fn scenery_out_of_view_only_casts_when_its_shadow_lands_in_view() {
        // Behind the camera, and just off its left side with the shadow falling right.
        let mut pop = population(Rule::Scenery { casts: true }, vec![at(0, 0.0, 20.0, -1), at(1, -14.0, -10.0, -1)]);
        let mut lists = Lists::new(3);
        select(&mut pop, &view(), &mut lists);
        assert_eq!([drawn(&pop, 0), drawn(&pop, 1)], [[0; TIERS], [0; TIERS]]);
        assert!(lists.cast[COARSEST].values().all(Vec::is_empty));
        assert_eq!(lists.cast[TIERS + COARSEST].values().map(Vec::len).sum::<usize>(), FLOATS);
    }

    #[test]
    fn casters_batch_by_the_region_they_stand_in() {
        let mut pop = population(Rule::Models, vec![at(0, 0.0, -10.0, -1), at(0, 600.0, -700.0, -1)]);
        let mut lists = Lists::new(3);
        select(&mut pop, &view(), &mut lists);
        let regions: Vec<usize> = lists.cast.iter().flat_map(|r| r.iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| *k as usize)).collect();
        assert_eq!(regions.len(), 2);
        assert_ne!(regions[0], regions[1]);
    }
}
