//! The static world's layers (buildings, scenery, props), the first module of
//! the native core: the browser's scenery and props word tables are handed
//! over once per map, what the side knows when it changes, and the view every
//! frame; a layer gets back the kinds and tiers whose drawn or cast instances
//! changed. The rules are `world/*_layer.gd`'s ports of the browser's; this
//! walks them. The buildings walk in the shared crate (`building_walk.rs`).

mod walk;

use godot::prelude::*;
use std::collections::HashMap;
pub use walk::{pack, region};
use walk::{Aabb, *};

/// A word table (`{words, layout}`, `WordTables`): its records' fields by name.
struct Table {
    words: PackedByteArray,
    stride: usize,
    at: HashMap<String, usize>,
}

impl Table {
    fn of(table: &VarDictionary) -> Self {
        let layout: VarDictionary = table.at("layout").to();
        let mut at = HashMap::new();
        for field in layout.at("fields").to::<VarArray>().iter_shared() {
            let field: VarDictionary = field.to();
            at.insert(
                field.at("name").to::<GString>().to_string(),
                num(&field.at("word")) as usize,
            );
        }
        Self {
            words: table.at("words").to(),
            stride: num(&layout.at("stride")) as usize,
            at,
        }
    }
    fn len(&self) -> usize {
        self.words.len() / 4 / self.stride.max(1)
    }
    /// Record `r`'s field `name`, `n` words of it.
    fn f<const N: usize>(&self, r: usize, name: &str) -> [f32; N] {
        let w = r * self.stride + self.at[name];
        std::array::from_fn(|k| f32::from_le_bytes(self.word(w + k)))
    }
    fn i(&self, r: usize, name: &str) -> i32 {
        let w = r * self.stride + self.at[name];
        i32::from_le_bytes(self.word(w))
    }
    fn word(&self, w: usize) -> [u8; 4] {
        self.words.as_slice()[w * 4..w * 4 + 4].try_into().unwrap()
    }
}

/// An id that is all ones names nothing.
fn key(id: i32) -> i64 {
    if id == -1 {
        -1
    } else {
        id as u32 as i64
    }
}

/// A number, as JSON's floats or GDScript's ints.
fn num(v: &Variant) -> f64 {
    v.try_to::<f64>().unwrap_or_else(|_| v.to::<i64>() as f64)
}

/// A packed or plain array's numbers.
fn numbers(v: &Variant) -> Vec<f64> {
    match v.try_to::<PackedFloat64Array>() {
        Ok(p) => p.to_vec(),
        Err(_) => v.to::<VarArray>().iter_shared().map(|n| num(&n)).collect(),
    }
}

/// A tier rule's three thresholds.
fn lod(v: &Variant) -> [f64; 3] {
    numbers(v)[..3].try_into().unwrap()
}

/// Chunks `members` (instance, sim foot) by the `chunk_m` cell each stands
/// in, in the order cells are first met, each bounded by its members' `bounds`.
fn chunk_by_cell(pop: &mut Population, members: Vec<(usize, [f32; 2])>, chunk_m: f64) {
    let (mut cells, mut order) = (HashMap::<(i64, i64), (Aabb, Vec<u32>)>::new(), vec![]);
    for (r, [x, y]) in members {
        let cell = (
            (x as f64 / chunk_m).floor() as i64,
            (y as f64 / chunk_m).floor() as i64,
        );
        let b = pop.instances[r].bounds;
        let entry = cells
            .entry(cell)
            .or_insert_with(|| (order.push(cell), (b, vec![])).1);
        entry.0 = if entry.1.is_empty() {
            b
        } else {
            entry.0.merge(&b)
        };
        entry.1.push(r as u32);
    }
    for cell in order {
        let (bounds, all) = cells.remove(&cell).unwrap();
        pop.add_chunk(bounds, all);
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct StaticLayer {
    kinds: usize,
    pops: Vec<Population>,
    /// Per key (`kind · 4 + tier`): instances drawn as given (stumps).
    placed: HashMap<usize, Vec<f32>>,
    last: Lists,
}

#[godot_api]
impl StaticLayer {
    /// The kinds its populations draw; `slots` the kinds it hands back
    /// (theirs, then any placed as given).
    #[func]
    fn set_kinds(&mut self, kinds: i64, slots: i64) {
        self.kinds = kinds as usize;
        self.last = Lists::new(slots as usize * TIERS);
    }

    /// Scenery records (`SceneryInstance`) chunked by `chunk_m`; a forest tree
    /// is hidden by its trunk's id.
    #[func]
    fn add_scenery(
        &mut self,
        table: VarDictionary,
        chunk_m: f64,
        lods: Variant,
        casts: bool,
    ) -> i64 {
        let t = Table::of(&table);
        let mut pop = Population::new(Rule::Scenery { casts }, lod(&lods), self.kinds);
        for r in 0..t.len() {
            let ([x, y, z, yaw], h, reach) = (
                t.f(r, "pose"),
                t.f::<1>(r, "height")[0],
                t.f::<1>(r, "reach")[0],
            );
            pop.instances.push(Instance {
                pack: pack([x, y, z], yaw, t.f(r, "shape"), t.f(r, "tint")),
                centre: to_godot([x, y, z + h / 2.0]),
                size: h,
                radius: reach,
                kind: t.i(r, "kind") as u32,
                key: key(t.i(r, "id")),
                bounds: Aabb::sim([x - reach, y - reach, z], [x + reach, y + reach, z + h]),
            });
        }
        chunk_by_cell(
            &mut pop,
            (0..t.len()).map(|r| (r, t.f(r, "pose"))).collect(),
            chunk_m,
        );
        self.push(pop)
    }

    /// The map's fitted props (`MapProp`), each look `sizes` big and `radii`
    /// in reach at unit scale (0: not staged), chunked by `chunk_m`; a prop is
    /// hidden by its map prop id.
    #[func]
    fn add_props(
        &mut self,
        table: VarDictionary,
        sizes: PackedFloat32Array,
        radii: PackedFloat32Array,
        chunk_m: f64,
        lods: Variant,
    ) -> i64 {
        let t = Table::of(&table);
        let mut pop = Population::new(Rule::Models, lod(&lods), self.kinds);
        let mut staged = vec![];
        for r in 0..t.len() {
            let (p, s, look) = (
                t.f(r, "position"),
                t.f::<3>(r, "scale"),
                t.i(r, "look") as usize,
            );
            let grow = s[0].max(s[1].max(s[2]));
            let packed = pack(p, t.f::<1>(r, "yaw")[0], s, t.f(r, "tint"));
            // A look not staged is placed but never drawn.
            if sizes.as_slice()[look] > 0.0 {
                staged.push((r, [p[0], p[1]]));
            }
            let (size, radius) = (sizes.as_slice()[look] * grow, radii.as_slice()[look] * grow);
            pop.instances.push(model(
                packed,
                look as u32,
                size,
                radius,
                key(t.i(r, "prop")),
            ));
        }
        chunk_by_cell(&mut pop, staged, chunk_m);
        self.push(pop)
    }

    /// Models placed per frame (known props); `set_models` places them.
    #[func]
    fn add_models(&mut self, lods: Variant) -> i64 {
        self.push(Population::new(Rule::Models, lod(&lods), self.kinds))
    }

    /// Replaces population `pop`'s models, as one chunk: each its pack (16
    /// floats), then its kind, size and reach.
    #[func]
    fn set_models(&mut self, pop: i64, models: PackedFloat32Array) {
        let p = &mut self.pops[pop as usize];
        let each = models.as_slice().as_chunks::<{ FLOATS + 3 }>().0.iter();
        p.instances = each
            .map(|m| model(m[..FLOATS].to_vec(), m[FLOATS] as u32, m[17], m[18], -1))
            .collect();
        p.chunks.clear();
        let mut boxes = p.instances.iter().map(|i| i.bounds);
        if let Some(first) = boxes.next() {
            let all = (0..p.instances.len() as u32).collect();
            p.add_chunk(boxes.fold(first, |a, b| a.merge(&b)), all);
        }
    }

    /// Hides exactly population `pop`'s instances `hidden` marks (a byte each).
    #[func]
    fn set_hidden(&mut self, pop: i64, hidden: PackedByteArray) {
        self.pops[pop as usize].set_hidden(hidden.as_slice().iter().map(|&b| b != 0).collect());
    }

    /// Hides exactly population `pop`'s instances whose key `keys` holds.
    #[func]
    fn hide_keys(&mut self, pop: i64, keys: PackedInt64Array) {
        self.pops[pop as usize].hide_keys(&keys.as_slice().iter().copied().collect());
    }

    /// Draws and casts `view` at `key` as given (16 floats each), until placed again.
    #[func]
    fn place(&mut self, key: i64, view: PackedFloat32Array) {
        self.placed.insert(key as usize, view.to_vec());
    }

    /// Walks `view` (`detail_view.gd` `packed`); returns the keys (`kind · 4 +
    /// tier`) whose drawn or cast instances changed.
    #[func]
    fn select(&mut self, view: PackedFloat64Array) -> PackedInt32Array {
        let v: Vec<f32> = view.as_slice().iter().map(|&x| x as f32).collect();
        let sides = std::array::from_fn(|k| v[8 + k * 4..12 + k * 4].try_into().unwrap());
        let (eye, fall) = ([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
        let view = View {
            eye,
            fall,
            pixels_per_metre: view[6],
            reach: view[7],
            sides,
        };
        let mut lists = Lists::new(self.last.view.len());
        for pop in &mut self.pops {
            walk(pop, &view, &mut lists);
        }
        for (&key, packs) in self.placed.iter().filter(|(_, p)| !p.is_empty()) {
            lists.view[key].extend_from_slice(packs);
            lists.cast(key, 0, packs);
        }
        let changed = (0..lists.view.len())
            .filter(|&k| lists.view[k] != self.last.view[k] || lists.cast[k] != self.last.cast[k]);
        let changed: PackedInt32Array = changed.map(|k| k as i32).collect();
        self.last = lists;
        changed
    }

    /// What the last walk draws at `key`.
    #[func]
    fn view(&self, key: i64) -> PackedFloat32Array {
        PackedFloat32Array::from(self.last.view[key as usize].as_slice())
    }

    /// What the last walk casts at `key`, by caster region.
    #[func]
    fn cast(&self, key: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        for (region, pack) in &self.last.cast[key as usize] {
            out.set(*region as i64, &PackedFloat32Array::from(pack.as_slice()));
        }
        out
    }

    /// What population `pop` (-1: all) drew last, per kind and tier.
    #[func]
    fn counts(&self, pop: i64) -> PackedInt32Array {
        let mut sum = vec![0; self.kinds * TIERS];
        for (_, p) in self
            .pops
            .iter()
            .enumerate()
            .filter(|&(p, _)| pop < 0 || p as i64 == pop)
        {
            sum.iter_mut().zip(&p.counts).for_each(|(a, b)| *a += b);
        }
        sum.into_iter().collect()
    }
}

impl StaticLayer {
    fn push(&mut self, pop: Population) -> i64 {
        self.pops.push(pop);
        self.pops.len() as i64 - 1
    }
}

/// A model at `pack`: culled by the sphere `radius` round its centre, half
/// its `size` above its origin.
fn model(pack: Vec<f32>, kind: u32, size: f32, radius: f32, key: i64) -> Instance {
    let centre = [pack[3], pack[7] + size / 2.0, pack[11]];
    let bounds = Aabb::around(centre, radius + SHADOW_MARGIN_M);
    Instance {
        pack,
        centre,
        size,
        radius,
        kind,
        key,
        bounds,
    }
}
