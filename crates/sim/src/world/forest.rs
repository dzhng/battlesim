//! Forests as bodies (Q14, Q16, Q21). A forest rect and its density are only
//! authoring input: they generate trunks, and at runtime a forest is those
//! trunk bodies plus the ground a heavy vehicle has cleared.
//!
//! - **Foliage** is precomputed per fog cell (`sensors.fog_cell_m`) from the concealing bodies
//!   whose crown covers the cell's centre: strength `1 − Π(1 − conceals)`,
//!   scaling the densest covering forest's concealment and attenuation.
//!   Removing a trunk refreshes the cells its crown reached.
//! - **Forest ground** is the ground the forests were authored over, less
//!   cleared ground: forest speed applies there, and the forest floor is
//!   drawn there. A knocked tree takes its foliage but not the ground; only
//!   a cleared lane is open ground again.
//! - **Cleared** ground is a mask of ground cells (`ground.cell_m`): where a vehicle knocked its way
//!   through, the ground is open, whatever foliage its cell holds. Every
//!   forest query ([`WorldGeometry::foliage_at`], [`WorldGeometry::forest_ground`])
//!   reads it.
use super::{in_rect, WorldGeometry};
use crate::math::{v2, Obb2, V2, V3};
use contract::map::Forest;
use contract::scenario::ForestDensity;

/// Sight lines are sampled this often through foliage.
const SAMPLE_M: f64 = 1.0;

/// A tree's crown: which forest it stands in, and how tall its canopy is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canopy {
    pub height_m: f64,
    pub density: ForestDensity,
}

/// What a point's foliage does to sight: open ground is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Foliage {
    /// The canopy top above the ground; 0 where nothing stands.
    pub canopy_m: f64,
    /// Foliage depth per metre a sight line crosses below the canopy.
    pub depth_per_m: f64,
    /// Detection-range multipliers for a target standing here, by class.
    pub infantry: f64,
    pub vehicle: f64,
}

impl Foliage {
    pub fn open() -> Self {
        Foliage {
            canopy_m: 0.0,
            depth_per_m: 0.0,
            infantry: 1.0,
            vehicle: 1.0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.canopy_m <= 0.0
    }

    /// The detection-range multiplier for a target of this class here: the
    /// per-class strength is the game rule (Q21).
    pub fn concealment(&self, infantry: bool) -> f64 {
        if infantry {
            self.infantry
        } else {
            self.vehicle
        }
    }
}

pub(super) struct ForestState {
    /// The foliage grid's cell: the fog's (`sensors.fog_cell_m`, Q21).
    foliage_m: f64,
    /// The cleared mask's cell: the ground layer's (`ground.cell_m`).
    cleared_m: f64,
    /// Each forest's reach: its rect grown by its crowns, for clipping rays.
    bounds: Vec<[f64; 4]>,
    max_crown_m: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Foliage>,
    cleared_nx: usize,
    cleared_ny: usize,
    cleared: Vec<u64>,
}

impl ForestState {
    pub(super) fn new(width: f64, depth: f64, foliage_m: f64, cleared_m: f64) -> Self {
        let nx = (width / foliage_m).ceil().max(1.0) as usize;
        let ny = (depth / foliage_m).ceil().max(1.0) as usize;
        let cleared_nx = (width / cleared_m).ceil().max(1.0) as usize;
        let cleared_ny = (depth / cleared_m).ceil().max(1.0) as usize;
        ForestState {
            foliage_m,
            cleared_m,
            bounds: Vec::new(),
            max_crown_m: 0.0,
            nx,
            ny,
            cells: vec![Foliage::open(); nx * ny],
            cleared_nx,
            cleared_ny,
            cleared: vec![0; (cleared_nx * cleared_ny).div_ceil(64)],
        }
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

/// A forest's seed: its index and rect, so the world, the wasm view and the
/// renderer all place the same trunks.
fn forest_seed(index: usize, forest: &Forest) -> u64 {
    forest
        .rect
        .iter()
        .fold(0x9e37_79b9_7f4a_7c15 ^ index as u64, |h, v| {
            (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3)
        })
}

impl WorldGeometry {
    /// Where `forest`'s trunks stand (Q16): a grid at its density's spacing,
    /// each trunk jittered off its cell's centre, none near a road or
    /// another body, none off the map.
    pub(super) fn trunk_positions(
        &self,
        index: usize,
        forest: &Forest,
        d: &ForestDensity,
    ) -> Vec<V2> {
        let [x0, y0, w, h] = forest.rect;
        let step = d.trunk_spacing_m;
        let mut rng = crate::rng::Rng::new(forest_seed(index, forest));
        let mut out = Vec::new();
        let mut y = y0 + step / 2.0;
        while y <= y0 + h {
            let mut x = x0 + step / 2.0;
            while x <= x0 + w {
                let jx = (rng.unit() * 2.0 - 1.0) * d.trunk_jitter * step;
                let jy = (rng.unit() * 2.0 - 1.0) * d.trunk_jitter * step;
                let p = v2(x + jx, y + jy);
                let near_road = self.roads.iter().any(|(pts, width)| {
                    super::distance_to_polyline(pts, p) <= width / 2.0 + forest.trunk_clearance_m
                });
                let near_prop = self.props().any(|prop| {
                    prop.canopy.is_none() && prop.footprint().contains(p, forest.trunk_clearance_m)
                });
                if in_rect(forest.rect, p.x, p.y)
                    && !near_road
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

    /// Record a forest's reach once its trunks stand.
    pub(super) fn note_forest(&mut self, forest: &Forest, d: &ForestDensity) {
        let r = d.canopy_radius_m;
        let [x, y, w, h] = forest.rect;
        self.forest
            .bounds
            .push([x - r, y - r, w + 2.0 * r, h + 2.0 * r]);
        self.forest.max_crown_m = self.forest.max_crown_m.max(r);
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
                self.forest.cells[j * self.forest.nx + i] = self.foliage_cell(mid, |_| false);
            }
        }
    }

    /// The foliage of the cell centred on `mid` (Q21), from the standing
    /// concealing bodies whose crown covers it, less those `gone` names.
    fn foliage_cell(&self, mid: V2, gone: impl Fn(&super::Prop) -> bool) -> Foliage {
        let mut transmit = 1.0;
        let mut canopy_m: f64 = 0.0;
        let mut densest: Option<ForestDensity> = None;
        for prop in self.props_near(mid, self.forest.max_crown_m) {
            let Some(crown) = prop.canopy else { continue };
            if prop.body.conceals <= 0.0
                || (prop.center - mid).length() > crown.density.canopy_radius_m
                || gone(prop)
            {
                continue;
            }
            transmit *= 1.0 - prop.body.conceals;
            canopy_m = canopy_m.max(crown.height_m);
            if densest.is_none_or(|d| crown.density.attenuation_per_m > d.attenuation_per_m) {
                densest = Some(crown.density);
            }
        }
        match densest {
            None => Foliage::open(),
            Some(d) => {
                let s = 1.0 - transmit;
                Foliage {
                    canopy_m,
                    depth_per_m: d.attenuation_per_m * s,
                    infantry: 1.0 + (d.concealment_infantry - 1.0) * s,
                    vehicle: 1.0 + (d.concealment_vehicle - 1.0) * s,
                }
            }
        }
    }

    /// Whether the ground at (x, y) was cleared by a vehicle knocking its
    /// way through the trees.
    pub fn cleared(&self, x: f64, y: f64) -> bool {
        self.forest
            .cleared_index(x, y)
            .is_some_and(|k| self.forest.cleared[k / 64] >> (k % 64) & 1 == 1)
    }

    /// Whether (x, y) is forest ground (forest speed): inside an authored
    /// forest, and not cleared since.
    pub fn forest_ground(&self, x: f64, y: f64) -> bool {
        self.forests.iter().any(|f| in_rect(f.rect, x, y)) && !self.cleared(x, y)
    }

    /// The foliage over (x, y): its fog cell's, or open ground where the
    /// ground is cleared.
    pub fn foliage_at(&self, x: f64, y: f64) -> Foliage {
        match self.forest.cell_index(x, y) {
            Some(k) if !self.forest.cells[k].is_open() && !self.cleared(x, y) => {
                self.forest.cells[k]
            }
            _ => Foliage::open(),
        }
    }

    /// The foliage depth of the segment `a`→`b` (Q21): each metre below the
    /// canopy of uncleared foliage adds its cell's depth per metre.
    pub fn foliage_depth(&self, a: V3, b: V3) -> f64 {
        let d = b - a;
        let len = d.length();
        // The spans of the segment within some forest's reach, merged.
        let mut spans: Vec<(f64, f64)> = Vec::new();
        for &[x0, y0, w, h] in &self.forest.bounds {
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
        if let Some(crown) = prop.canopy {
            let reach = crown.density.canopy_radius_m + self.forest.foliage_m;
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
                self.forest.cleared[k / 64] |= 1 << (k % 64);
                out.push(mid);
            }
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
            .filter(|p| p.canopy.is_some())
            .map(|p| p.center)
            .collect();
        let mut out = Vec::new();
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
                self.forest.cleared[k / 64] |= 1 << (k % 64);
                out.push(mid);
            }
        }
        out
    }

    /// Whether any forest ground lies within `r` of `center` (a cheap test
    /// before clearing).
    pub fn forest_near(&self, center: V2, r: f64) -> bool {
        self.forests.iter().any(|f| {
            let [x, y, w, h] = f.rect;
            center.x + r >= x && center.x - r <= x + w && center.y + r >= y && center.y - r <= y + h
        })
    }

    /// The cleared mask's cell edge (`ground.cell_m`).
    pub fn cleared_cell_m(&self) -> f64 {
        self.forest.cleared_m
    }

    /// Cells cleared so far.
    pub fn cleared_cells(&self) -> u32 {
        self.forest.cleared.iter().map(|w| w.count_ones()).sum()
    }

    /// The cleared mask, word by word (the digest's): each word that holds
    /// a cleared cell, with its index.
    pub fn digest_cleared(&self, d: &mut crate::digest::Digest) {
        for (k, &w) in self.forest.cleared.iter().enumerate() {
            if w != 0 {
                d.u64(k as u64).u64(w);
            }
        }
    }

    /// The foliage grid for presentation: `[nx, ny, cell_m]`, then per cell
    /// row-major `canopy_m, depth_per_m`.
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
            .filter(|p| p.canopy.is_some() && cleared(p.center.x, p.center.y))
            .map(|p| p.id)
            .collect();
        let mut cells = f.cells.clone();
        // Only the cells a fallen crown reached change.
        let reach = f.max_crown_m + c;
        let mut touched = vec![false; cells.len()];
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
                    touched[j * f.nx + i] = true;
                }
            }
        }
        for (k, cell) in cells.iter_mut().enumerate() {
            let mid = v2((k % f.nx) as f64 + 0.5, (k / f.nx) as f64 + 0.5) * c;
            if cell.is_open() {
                continue;
            }
            if cleared(mid.x, mid.y) {
                *cell = Foliage::open();
            } else if touched[k] {
                *cell = self.foliage_cell(mid, |p| fallen.contains(&p.id));
            }
        }
        let mut out = vec![f.nx as f32, f.ny as f32, c as f32];
        for cell in &cells {
            out.push(cell.canopy_m as f32);
            out.push(cell.depth_per_m as f32);
        }
        out
    }
}
