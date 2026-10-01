//! Forests as bodies (Q14, Q16, Q21). A forest's shape is only authoring
//! input: the one `forests.rule` generates its trunks, and at runtime a forest
//! is those trunk bodies plus the ground a heavy vehicle has cleared. Logs
//! and boulders are ordinary cover bodies, placed after all trunks; they
//! neither supply foliage nor clear forest ground when shoved.
//!
//! - **Foliage** is precomputed per fog cell (`map.fog_cell_m`) from the concealing bodies
//!   whose crown covers the cell's centre: strength `1 − Π(1 − conceals)`,
//!   scaling sight-line attenuation.
//!   Removing a trunk refreshes the cells its crown reached.
//! - **Forest ground** is the ground the forests were authored over, less
//!   cleared ground: forest speed and concealment apply there, and the forest floor is
//!   drawn there. A knocked tree takes its foliage but not the ground; only
//!   a cleared lane is open ground again.
//! - **Cleared** ground is a mask of ground cells (`ground.cell_m`): where a vehicle knocked its way
//!   through, the ground is open, whatever foliage its cell holds. Every
//!   forest query ([`WorldGeometry::foliage_at`], [`WorldGeometry::forest_ground`])
//!   reads it.
use std::collections::BTreeSet;

use super::WorldGeometry;
use crate::cell_page::Page;
use crate::math::{v2, Obb2, V2, V3};
use contract::map::Forest;
use contract::scenario::ForestRule;

/// Sight lines are sampled this often through foliage.
const SAMPLE_M: f64 = 1.0;
/// Forest ground is looked up in buckets this wide.
const GROUND_BUCKET_M: f64 = 64.0;

/// What a point's foliage does to sight: open ground is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Foliage {
    /// The canopy top above the ground; 0 where nothing stands.
    pub canopy_m: f64,
    /// Foliage depth per metre a sight line crosses below the canopy.
    pub depth_per_m: f64,
}

impl Foliage {
    pub fn open() -> Self {
        Foliage {
            canopy_m: 0.0,
            depth_per_m: 0.0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.canopy_m <= 0.0
    }
}

pub(super) struct ForestState {
    /// The foliage grid's cell: the fog's (`map.fog_cell_m`, Q21).
    foliage_m: f64,
    /// The cleared mask's cell: the ground layer's (`ground.cell_m`).
    cleared_m: f64,
    /// Each forest's conservative reach, grown by its crowns, for clipping rays.
    bounds: Vec<[f64; 4]>,
    /// Cached physical bounds and exact-rectangle flags; shape data stays authored.
    ground_regions: Vec<([f64; 4], bool)>,
    /// Per bucket of the map, row by row, the forests whose bounds reach
    /// it: a point on the map asks only about those.
    ground_buckets: Vec<Vec<u32>>,
    ground_nx: usize,
    /// Immutable source associations; removals never recycle the original IDs.
    trunk_ranges: Vec<[u32; 2]>,
    /// The one rule every forest's trees follow.
    pub(super) rule: ForestRule,
    nx: usize,
    ny: usize,
    cells: Vec<Option<Page<Foliage>>>,
    cleared_nx: usize,
    cleared_ny: usize,
    cleared: Vec<Option<Page<u64>>>,
    /// Every cleared cell, in the order they were cleared: what a reader
    /// that keeps its place sees as new.
    cleared_order: Vec<u32>,
}

impl ForestState {
    pub(super) fn new(
        width: f64,
        depth: f64,
        foliage_m: f64,
        cleared_m: f64,
        forests: &[Forest],
        rule: ForestRule,
    ) -> Self {
        let nx = (width / foliage_m).ceil().max(1.0) as usize;
        let ny = (depth / foliage_m).ceil().max(1.0) as usize;
        let cleared_nx = (width / cleared_m).ceil().max(1.0) as usize;
        let cleared_ny = (depth / cleared_m).ceil().max(1.0) as usize;
        let ground_regions: Vec<([f64; 4], bool)> = forests
            .iter()
            .map(|f| (f.shape.limits(), f.shape.exact_rectangle().is_some()))
            .collect();
        let ground_nx = (width / GROUND_BUCKET_M).ceil().max(1.0) as usize;
        let ground_ny = (depth / GROUND_BUCKET_M).ceil().max(1.0) as usize;
        let mut ground_buckets = vec![Vec::new(); ground_nx * ground_ny];
        for (id, forest) in forests.iter().enumerate() {
            let [x, y, w, h] = forest.shape.bounds();
            let r = rule.canopy_radius_m;
            let [lo_x, lo_y, hi_x, hi_y] = ground_regions[id].0;
            // Ground membership and foliage spans share these buckets. Include
            // the exact ground limits and the canopy's recomposed span bounds.
            let cell =
                |v: f64, n: usize| ((v / GROUND_BUCKET_M).floor().max(0.0) as usize).min(n - 1);
            let x0 = lo_x.min(x - r);
            let y0 = lo_y.min(y - r);
            let x1 = hi_x.max((x - r) + (w + 2.0 * r));
            let y1 = hi_y.max((y - r) + (h + 2.0 * r));
            for j in cell(y0, ground_ny)..=cell(y1, ground_ny) {
                for i in cell(x0, ground_nx)..=cell(x1, ground_nx) {
                    ground_buckets[j * ground_nx + i].push(id as u32);
                }
            }
        }
        ForestState {
            foliage_m,
            cleared_m,
            bounds: Vec::new(),
            ground_regions,
            ground_buckets,
            ground_nx,
            trunk_ranges: Vec::with_capacity(forests.len()),
            rule,
            nx,
            ny,
            cells: vec![None; (nx * ny).div_ceil(256)],
            cleared_nx,
            cleared_ny,
            cleared: vec![None; (cleared_nx * cleared_ny).div_ceil(64 * 256)],
            cleared_order: Vec::new(),
        }
    }

    pub(super) fn bounds(&self) -> &[[f64; 4]] {
        &self.bounds
    }

    pub(super) fn trunk_ranges(&self) -> &[[u32; 2]] {
        &self.trunk_ranges
    }

    fn cleared_index(&self, x: f64, y: f64) -> Option<usize> {
        let (i, j) = ((x / self.cleared_m).floor(), (y / self.cleared_m).floor());
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.cleared_nx && (j as usize) < self.cleared_ny)
            .then(|| j as usize * self.cleared_nx + i as usize)
    }

    fn cell_index(&self, x: f64, y: f64) -> Option<usize> {
        let (i, j) = ((x / self.foliage_m).floor(), (y / self.foliage_m).floor());
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then(|| j as usize * self.nx + i as usize)
    }
}

/// A forest's jitter seed, from its index and its own geometry, so the same
/// forest always stands the same trunks. A rectangle hashes its extent, which
/// keeps the trunks of the maps authored as rectangles; polygons and strokes
/// hash their vertices under their own salt.
fn forest_seed(index: usize, forest: &Forest) -> u64 {
    let initial = 0x9e37_79b9_7f4a_7c15 ^ index as u64;
    let hash = |h: u64, v: &f64| (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3);
    if let Some(rect) = forest.shape.exact_rectangle() {
        return rect.iter().fold(initial, hash);
    }
    match &forest.shape {
        contract::ground::GroundShape::Polygon { ring } => {
            ring.iter().flatten().fold(initial ^ 1, hash)
        }
        contract::ground::GroundShape::Stroke {
            centerline,
            width_m,
        } => centerline
            .control_points()
            .iter()
            .flatten()
            .fold(hash(initial ^ 2, width_m), hash),
    }
}

pub(super) struct FloorBody<'a> {
    pub kind: &'a str,
    pub density: f64,
    pub half: [f64; 3],
    pub salt: u64,
}

impl WorldGeometry {
    /// Where `forest`'s trunks stand (Q16): a grid at the rule's spacing,
    /// each trunk jittered off its cell's centre, none near a road or
    /// another body, none off the map.
    pub(super) fn trunk_positions(&self, index: usize, forest: &Forest) -> Vec<V2> {
        let d = &self.forest.rule;
        let [x0, y0, max_x, max_y] = forest.shape.limits();
        let step = d.trunk_spacing_m;
        assert!(
            step.is_finite() && step > 0.0,
            "forest spacing must be finite and positive"
        );
        assert!(
            [x0, y0, max_x, max_y]
                .iter()
                .all(|v| v.is_finite() && (v + step).is_finite() && v + step > *v),
            "forest lattice spacing must advance at every coordinate bound"
        );
        let mut rng = crate::rng::Rng::new(forest_seed(index, forest));
        let mut out = Vec::new();
        let mut y = y0 + step / 2.0;
        while y <= max_y {
            let mut x = x0 + step / 2.0;
            while x <= max_x {
                let jx = (rng.unit() * 2.0 - 1.0) * d.trunk_jitter * step;
                let jy = (rng.unit() * 2.0 - 1.0) * d.trunk_jitter * step;
                let p = v2(x + jx, y + jy);
                let near_open = self.surfaces.road_near(p, d.trunk_clearance_m)
                    || self.surfaces.water_near(p, d.trunk_clearance_m);
                // The prop index, not every prop: a full-size map stands tens
                // of thousands of trunks, each already a prop.
                let near_prop = self.props_near(p, d.trunk_clearance_m).iter().any(|prop| {
                    !prop.forest_tree && prop.footprint().contains(p, d.trunk_clearance_m)
                });
                if forest.shape.contains([p.x, p.y], 0.0)
                    && !near_open
                    && !near_prop
                    && self.field.contains(p.x, p.y)
                {
                    out.push(p);
                }
                x += step;
            }
            y += step;
        }
        out
    }

    /// Independent jittered lattices place sparse floor bodies in gaps. A
    /// rejected candidate stays rejected: no retries, and no trunks move.
    pub(super) fn place_forest_bodies(
        &mut self,
        index: usize,
        forest: &Forest,
        placement: FloorBody<'_>,
        nav: &mut crate::navigation::NavGrid,
        movers: &[crate::navigation::Mobility],
    ) {
        let FloorBody {
            kind,
            density,
            half,
            salt,
        } = placement;
        let step = (10_000.0 / density).sqrt();
        let [x0, y0, max_x, max_y] = forest.shape.limits();
        assert!(
            [x0, y0, max_x, max_y]
                .iter()
                .all(|v| (v + step).is_finite() && v + step > *v),
            "forest floor lattice spacing must advance at every bound"
        );
        let radius = half[0].hypot(half[1]);
        let clearance = self.forest.rule.trunk_clearance_m;
        let mut rng = crate::rng::Rng::new(forest_seed(index, forest) ^ salt);
        let mut y = y0 + step / 2.0;
        while y <= max_y {
            let mut x = x0 + step / 2.0;
            while x <= max_x {
                let p = v2(
                    x + (rng.unit() * 2.0 - 1.0) * self.forest.rule.trunk_jitter * step,
                    y + (rng.unit() * 2.0 - 1.0) * self.forest.rule.trunk_jitter * step,
                );
                let yaw = rng.unit() * std::f64::consts::TAU;
                let inside = match &forest.shape {
                    contract::ground::GroundShape::Polygon { ring } => {
                        forest.shape.contains([p.x, p.y], 0.0)
                            && contract::ground::edges(ring).all(|(a, b)| {
                                contract::ground::segment_distance(*a, *b, [p.x, p.y]) >= radius
                            })
                    }
                    contract::ground::GroundShape::Stroke { .. } => {
                        forest.shape.contains([p.x, p.y], -radius)
                    }
                };
                let clear = inside
                    && p.x >= radius
                    && p.y >= radius
                    && p.x + radius <= self.field.width()
                    && p.y + radius <= self.field.depth()
                    && !self.surfaces.road_near(p, radius + clearance)
                    && !self.surfaces.water_near(p, radius + clearance)
                    && self
                        .props_near(p, radius + clearance)
                        .iter()
                        .all(|q| q.footprint().distance(p) >= radius + clearance);
                if clear {
                    let def = contract::map::PropDefinition {
                        kind: kind.into(),
                        center: [p.x, p.y],
                        yaw,
                        half_extents: half,
                        base_z: None,
                    };
                    if nav.admit_floor_body(self, self.placed_prop(&def), movers) {
                        self.add_prop(&def);
                    }
                }
                x += step;
            }
            y += step;
        }
    }

    /// Record a forest's reach once its trunks stand.
    pub(super) fn note_forest(&mut self, forest: &Forest, range: [u32; 2]) {
        self.forest.trunk_ranges.push(range);
        let r = self.forest.rule.canopy_radius_m;
        let [x, y, w, h] = forest.shape.bounds();
        self.forest
            .bounds
            .push([x - r, y - r, w + 2.0 * r, h + 2.0 * r]);
        self.refresh_foliage(v2(x + w / 2.0, y + h / 2.0), w.hypot(h) / 2.0 + r);
    }

    /// Recompute every foliage cell whose centre lies within `radius` of `center`.
    fn refresh_foliage(&mut self, center: V2, radius: f64) {
        let c = self.forest.foliage_m;
        let f = &self.forest;
        let i0 = ((center.x - radius) / c).floor().max(0.0) as usize;
        let j0 = ((center.y - radius) / c).floor().max(0.0) as usize;
        let i1 = (((center.x + radius) / c).floor().max(0.0) as usize).min(f.nx - 1);
        let j1 = (((center.y + radius) / c).floor().max(0.0) as usize).min(f.ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let mid = v2((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
                let cell = self.foliage_cell(mid, |_| false);
                let key = j * self.forest.nx + i;
                let page = &mut self.forest.cells[key / 256];
                if !cell.is_open() || page.is_some() {
                    page.get_or_insert_with(|| Page::Uniform(Foliage::open()))
                        .set(key % 256, cell);
                }
            }
        }
    }

    /// The foliage of the cell centred on `mid` (Q21), from the standing
    /// concealing bodies whose crown covers it, less those `gone` names.
    fn foliage_cell(&self, mid: V2, gone: impl Fn(&super::Prop) -> bool) -> Foliage {
        let d = self.forest.rule;
        let mut transmit = 1.0;
        let mut crowned = false;
        for prop in self.props_near(mid, d.canopy_radius_m) {
            if !prop.forest_tree
                || prop.body.conceals <= 0.0
                || (prop.center - mid).length() > d.canopy_radius_m
                || gone(prop)
            {
                continue;
            }
            transmit *= 1.0 - prop.body.conceals;
            crowned = true;
        }
        if !crowned {
            return Foliage::open();
        }
        let s = 1.0 - transmit;
        Foliage {
            canopy_m: d.canopy_height_m,
            depth_per_m: d.attenuation_per_m * s,
        }
    }

    /// Whether the ground at (x, y) was cleared by a vehicle knocking its
    /// way through the trees.
    pub fn cleared(&self, x: f64, y: f64) -> bool {
        self.forest.cleared_index(x, y).is_some_and(|k| {
            self.forest
                .cleared
                .get(k / (64 * 256))
                .and_then(Option::as_ref)
                .is_some_and(|page| page.get((k / 64) % 256) >> (k % 64) & 1 == 1)
        })
    }

    /// Whether (x, y) is forest ground (forest speed): inside an authored
    /// forest, and not cleared since.
    pub fn forest_ground(&self, x: f64, y: f64) -> bool {
        let f = &self.forest;
        let within = |id: usize| {
            let (bounds, rectangle) = &f.ground_regions[id];
            x >= bounds[0]
                && x <= bounds[2]
                && y >= bounds[1]
                && y <= bounds[3]
                && (*rectangle || self.forests[id].shape.contains([x, y], 0.0))
        };
        let (i, j) = ((x / GROUND_BUCKET_M).floor(), (y / GROUND_BUCKET_M).floor());
        let bucket = (i >= 0.0 && j >= 0.0 && (i as usize) < f.ground_nx)
            .then(|| f.ground_buckets.get(j as usize * f.ground_nx + i as usize))
            .flatten();
        let forested = match bucket {
            Some(near) => near.iter().any(|&id| within(id as usize)),
            // Off the bucketed map: ask every forest.
            None => (0..f.ground_regions.len()).any(within),
        };
        forested && !self.cleared(x, y)
    }

    /// Forest concealment is a binary ground property, independent of crown coverage.
    pub fn forest_concealment(&self, infantry: bool, x: f64, y: f64) -> f64 {
        if !self.forest_ground(x, y) || self.surfaces.water_at(v2(x, y)) {
            return 1.0;
        }
        let rule = self.forest.rule;
        if infantry {
            rule.concealment_infantry
        } else {
            rule.concealment_vehicle
        }
    }

    /// The foliage over (x, y): its fog cell's, or open ground where the
    /// ground is cleared.
    pub fn foliage_at(&self, x: f64, y: f64) -> Foliage {
        let foliage = self
            .forest
            .cell_index(x, y)
            .and_then(|k| self.forest.cells[k / 256].as_ref().map(|p| p.get(k % 256)))
            .unwrap_or_else(Foliage::open);
        // Clearing can only remove foliage, so open cells need no mask query.
        if foliage.is_open() || self.cleared(x, y) {
            Foliage::open()
        } else {
            foliage
        }
    }

    /// The foliage depth of the segment `a`→`b` (Q21): each metre below the
    /// canopy of uncleared foliage adds its cell's depth per metre.
    pub fn foliage_depth(&self, a: V3, b: V3) -> f64 {
        if self.forest.bounds.is_empty() {
            return 0.0;
        }
        let d = b - a;
        let len = d.length();
        // The spans of the segment within some forest's reach, merged.
        let mut spans: Vec<(f64, f64)> = Vec::new();
        let f = &self.forest;
        let ny = f.ground_buckets.len() / f.ground_nx;
        let cell = |v: f64, n: usize| ((v / GROUND_BUCKET_M).floor().max(0.0) as usize).min(n - 1);
        let mut candidates = Vec::new();
        for j in cell(a.y.min(b.y), ny)..=cell(a.y.max(b.y), ny) {
            for i in cell(a.x.min(b.x), f.ground_nx)..=cell(a.x.max(b.x), f.ground_nx) {
                candidates.extend_from_slice(&f.ground_buckets[j * f.ground_nx + i]);
            }
        }
        // Preserve authored forest order before the exact span integration.
        candidates.sort_unstable();
        candidates.dedup();
        for id in candidates {
            let [x0, y0, w, h] = f.bounds[id as usize];
            let (mut t0, mut t1) = (0.0f64, 1.0f64);
            for (o, dv, lo, hi) in [(a.x, d.x, x0, x0 + w), (a.y, d.y, y0, y0 + h)] {
                if dv.abs() < 1e-12 {
                    if o < lo || o > hi {
                        t1 = -1.0;
                    }
                } else {
                    let (p, q) = ((lo - o) / dv, (hi - o) / dv);
                    t0 = t0.max(p.min(q));
                    t1 = t1.min(p.max(q));
                }
            }
            if t0 < t1 {
                spans.push((t0, t1));
            }
        }
        spans.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut total = 0.0;
        let mut done = 0.0f64;
        for (t0, t1) in spans {
            let t0 = t0.max(done);
            if t0 >= t1 {
                continue;
            }
            done = t1;
            let span = (t1 - t0) * len;
            let n = (span / SAMPLE_M).ceil().max(1.0) as usize;
            let piece = span / n as f64;
            for k in 0..n {
                let p = a + d * (t0 + (t1 - t0) * ((k as f64 + 0.5) / n as f64));
                let f = self.foliage_at(p.x, p.y);
                if f.is_open() {
                    continue;
                }
                if let Some(ground) = self.height_at(p.x, p.y) {
                    if p.z < ground + f.canopy_m {
                        total += f.depth_per_m * piece;
                    }
                }
            }
        }
        total
    }

    /// Knock a tree down (Q16): its body goes, the foliage its crown gave
    /// goes with it, and the obstacle revision bumps.
    pub fn knock_down(&mut self, id: super::PropId) -> Option<super::Prop> {
        let prop = self.remove_prop(id)?;
        if prop.forest_tree {
            let reach = self.forest.rule.canopy_radius_m + self.forest.foliage_m;
            self.refresh_foliage(prop.center, reach);
        }
        Some(prop)
    }

    /// Clear the ground under `area`, outside `keep`, where it is forest
    /// ground: the lane a vehicle knocks through the trees becomes open
    /// ground. Returns the centres of the cells newly cleared.
    pub fn clear(&mut self, area: &Obb2, keep: &Obb2) -> Vec<V2> {
        let r = area.half.length();
        let c = self.forest.cleared_m;
        let mut out = Vec::new();
        let mut touched = BTreeSet::new();
        let i0 = ((area.center.x - r) / c).floor().max(0.0) as usize;
        let j0 = ((area.center.y - r) / c).floor().max(0.0) as usize;
        let i1 = ((area.center.x + r) / c).floor().max(0.0) as usize;
        let j1 = ((area.center.y + r) / c).floor().max(0.0) as usize;
        for j in j0..=j1.min(self.forest.cleared_ny - 1) {
            for i in i0..=i1.min(self.forest.cleared_nx - 1) {
                let mid = v2((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
                if !area.contains(mid, 0.0)
                    || keep.contains(mid, 0.0)
                    || !self.forest_ground(mid.x, mid.y)
                {
                    continue;
                }
                let k = j * self.forest.cleared_nx + i;
                let page = self.forest.cleared[k / (64 * 256)].get_or_insert_with(Page::default);
                let word = (k / 64) % 256;
                page.set(word, page.get(word) | (1 << (k % 64)));
                touched.insert(k / (64 * 256));
                self.forest.cleared_order.push(k as u32);
                out.push(mid);
            }
        }
        for key in touched {
            self.forest.cleared[key]
                .as_mut()
                .expect("cleared page exists")
                .compress();
        }
        out
    }

    /// Clear a felled tree's spot (Q17): the forest ground within `reach`
    /// of where it stood that lies nearer it than any standing trunk, so a
    /// patch whose trees all fell is open ground from trunk to trunk. Call
    /// once the tree is down. Returns the centres of the cells newly cleared.
    pub fn clear_spot(&mut self, center: V2, reach: f64) -> Vec<V2> {
        let c = self.forest.cleared_m;
        let i0 = ((center.x - reach) / c).floor().max(0.0) as usize;
        let j0 = ((center.y - reach) / c).floor().max(0.0) as usize;
        let i1 =
            (((center.x + reach) / c).floor().max(0.0) as usize).min(self.forest.cleared_nx - 1);
        let j1 =
            (((center.y + reach) / c).floor().max(0.0) as usize).min(self.forest.cleared_ny - 1);
        let standing: Vec<V2> = self
            .props_near(center, 2.0 * reach)
            .into_iter()
            .filter(|p| p.forest_tree)
            .map(|p| p.center)
            .collect();
        let mut out = Vec::new();
        let mut touched = BTreeSet::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let mid = v2((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
                let d = (mid - center).length();
                if d > reach
                    || !self.forest_ground(mid.x, mid.y)
                    || standing.iter().any(|t| (*t - mid).length() < d)
                {
                    continue;
                }
                let k = j * self.forest.cleared_nx + i;
                let page = self.forest.cleared[k / (64 * 256)].get_or_insert_with(Page::default);
                let word = (k / 64) % 256;
                page.set(word, page.get(word) | (1 << (k % 64)));
                touched.insert(k / (64 * 256));
                self.forest.cleared_order.push(k as u32);
                out.push(mid);
            }
        }
        for key in touched {
            self.forest.cleared[key]
                .as_mut()
                .expect("cleared page exists")
                .compress();
        }
        out
    }

    /// Whether any forest ground lies within `r` of `center` (a cheap test
    /// before clearing).
    pub fn forest_near(&self, center: V2, r: f64) -> bool {
        self.forest.ground_regions.iter().any(|(bounds, _)| {
            let [x, y, max_x, max_y] = *bounds;
            center.x + r >= x && center.x - r <= max_x && center.y + r >= y && center.y - r <= max_y
        })
    }

    /// The one rule every forest's trees follow.
    pub fn forest_rule(&self) -> &ForestRule {
        &self.forest.rule
    }

    /// The cleared mask's cell edge (`ground.cell_m`).
    pub fn cleared_cell_m(&self) -> f64 {
        self.forest.cleared_m
    }

    /// Cells cleared so far.
    pub fn cleared_cells(&self) -> u32 {
        self.forest.cleared_order.len() as u32
    }

    /// The middle of every cell cleared after the first `taken`, in the
    /// order they were cleared.
    pub fn cleared_since(&self, taken: usize) -> impl Iterator<Item = V2> + '_ {
        let (c, nx) = (self.forest.cleared_m, self.forest.cleared_nx);
        self.forest.cleared_order[taken..].iter().map(move |&k| {
            let k = k as usize;
            v2((k % nx) as f64 + 0.5, (k / nx) as f64 + 0.5) * c
        })
    }

    /// The cleared mask, word by word (the digest's): each word that holds
    /// a cleared cell, with its index.
    pub fn digest_cleared(&self, d: &mut crate::digest::Digest) {
        let words = self
            .forest
            .cleared
            .iter()
            .enumerate()
            .filter_map(|(key, page)| page.as_ref().map(|p| (key, p)))
            .flat_map(|(key, page)| {
                (0..256).filter_map(move |i| {
                    let word = page.get(i);
                    (word != 0).then_some((key * 256 + i, word))
                })
            });
        // The count frames the pairs: nothing after them reads as one.
        d.u64(words.clone().count() as u64);
        for (k, w) in words {
            d.u64(k as u64).u64(w);
        }
    }

    /// Sparse foliage: `[nx, ny, cell_m]`, then sorted non-open records
    /// `[column, row, canopy_m, depth_per_m]`. Missing cells are open.
    pub fn export_foliage(&self) -> Vec<f32> {
        self.export_foliage_cleared(|_, _| false)
    }

    /// The foliage grid as a side that has seen the ground `cleared` says
    /// cleared knows it (34c): the trees standing on such ground are gone,
    /// with the foliage their crowns gave, and a cell whose centre stands on
    /// it is open (the rule `foliage_at` reads at that point). Drawn fog
    /// follows a lane knocked, or a patch shelled, during a battle.
    pub fn export_foliage_cleared(&self, cleared: impl Fn(f64, f64) -> bool) -> Vec<f32> {
        let f = &self.forest;
        let c = f.foliage_m;
        let fallen: std::collections::BTreeSet<super::PropId> = self
            .props()
            .filter(|p| p.forest_tree && cleared(p.center.x, p.center.y))
            .map(|p| p.id)
            .collect();
        // Only the cells a fallen crown reached change.
        let reach = f.rule.canopy_radius_m + c;
        let mut touched = BTreeSet::new();
        for id in &fallen {
            let p = self
                .prop(*id)
                .expect("a fallen tree stands in the static world");
            let i0 = ((p.center.x - reach) / c).floor().max(0.0) as usize;
            let j0 = ((p.center.y - reach) / c).floor().max(0.0) as usize;
            let i1 = (((p.center.x + reach) / c).floor().max(0.0) as usize).min(f.nx - 1);
            let j1 = (((p.center.y + reach) / c).floor().max(0.0) as usize).min(f.ny - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    touched.insert(j * f.nx + i);
                }
            }
        }
        let mut out = vec![f.nx as f32, f.ny as f32, c as f32];
        for (k, original) in f
            .cells
            .iter()
            .enumerate()
            .filter_map(|(id, page)| page.as_ref().map(|page| (id, page)))
            .flat_map(|(id, page)| (0..256).map(move |cell| (id * 256 + cell, page.get(cell))))
            .filter(|(_, cell)| !cell.is_open())
        {
            let mid = v2((k % f.nx) as f64 + 0.5, (k / f.nx) as f64 + 0.5) * c;
            let cell = if cleared(mid.x, mid.y) {
                Foliage::open()
            } else if touched.contains(&k) {
                self.foliage_cell(mid, |p| fallen.contains(&p.id))
            } else {
                original
            };
            if !cell.is_open() {
                // Axis coordinates stay exact in f32 even when global indices exceed 2^24.
                out.extend([
                    (k % f.nx) as f32,
                    (k / f.nx) as f32,
                    cell.canopy_m as f32,
                    cell.depth_per_m as f32,
                ]);
            }
        }
        out
    }
}
