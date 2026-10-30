//! A map's buildings drawn from template art: each building's frame, bounds
//! and coarse chunk, and its template's rows placed in the world, in each
//! state a side can know it in. Both renderers draw these records; the
//! per-frame walk (which chunks are resident, at which tier) is theirs.
//!
//! A whole map's rows are never held: market-town places about 90 MB of
//! them over three states. A renderer asks for a set of buildings' rows in
//! one state ([`Buildings::rows`]) when a chunk comes near.
use crate::static_world::library::{Bounds, Library, ROW_FLOATS};
use crate::static_world::walk::Record;
use crate::view::TIERS;
use crate::world::BuildingStyle;
use std::ops::Range;
use std::sync::Arc;

/// Every building draws at the coarsest tier from afar.
const COARSEST: usize = TIERS - 1;

/// A map's buildings and the art they are drawn from, as `load_world` takes
/// them: the template library, every library module's mesh bounds (`None`:
/// its kit is not installed), the placed buildings, and what a side's
/// knowledge reads of them (`fallen.rs`): each part prop with its building,
/// and the prop kinds a gutted building's parts become.
pub struct PlacedBuildings {
    pub library: Library,
    pub bounds: Vec<Option<Bounds>>,
    pub placed: Vec<Placed>,
    pub parts: Vec<(u32, usize)>,
    pub shells: Vec<String>,
}

/// A building as the map places it: its owner prop, its template and the
/// frame its template's art stands at (x, y, z, yaw about +Z).
pub struct Placed {
    pub owner: u32,
    pub template: String,
    pub frame: [f64; 4],
}

/// Per building (64 B): its frame, its intact bounds in the world, its owner
/// prop, the coarse chunk it draws in (an index of the map's chunks with any
/// coarse row, by cell; -1 for none), and its row count per state (`STATES`
/// order; -1 where its template has none).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct BuildingRecord {
    pub frame: [f32; 4],
    pub min: [f32; 3],
    pub owner: u32,
    pub max: [f32; 3],
    pub chunk: i32,
    pub rows: [i32; 3],
}

/// A module instance: its renderer record (placed, and tinted linear times
/// its building's value), its kit module (an index of
/// [`Buildings::modules`]), its tier mask and its building.
#[derive(Clone)]
pub struct BuildingModule {
    pub record: Record,
    pub kit_module: usize,
    pub tiers: u32,
    pub building: usize,
}

pub struct Buildings {
    library: Arc<Library>,
    placed: Arc<Vec<Placed>>,
    bounds: Vec<Option<Bounds>>,
    style: BuildingStyle,
    /// Per building: its template's rows in each state.
    states: Vec<[Option<Range<usize>>; 3]>,
    /// Per building: its value, the factor its tint is drawn at.
    value: Vec<f32>,
    pub records: Vec<BuildingRecord>,
    /// The library modules the map's templates place in any state, in
    /// library order; a row's `kit_module` indexes these.
    pub modules: Vec<usize>,
    /// Per library module: its index in `modules`.
    compact: Vec<u32>,
    /// A byte of sRGB, linear.
    linear: [f32; 256],
}

/// `yaw`'s cosine and sine, `libm`'s on every client.
fn turn(yaw: f64) -> (f64, f64) {
    (libm::cos(yaw), libm::sin(yaw))
}

/// A template's intact rows' extent in its own frame: min x, y, z, max x, y, z.
fn template_extent(
    library: &Library,
    bounds: &[Option<Bounds>],
    rows: Range<usize>,
) -> Result<[f64; 6], String> {
    if rows.is_empty() {
        return Ok([0.0; 6]);
    }
    let mut e = [f64::INFINITY; 6];
    e[3..].fill(f64::NEG_INFINITY);
    for r in rows {
        let module = library.module[r] as usize;
        let b = bounds[module].ok_or_else(|| {
            format!(
                "kit.missing: kit \"{}\" is not installed",
                library.modules[module].kit
            )
        })?;
        let t = &library.transform[r * ROW_FLOATS..(r + 1) * ROW_FLOATS];
        let [x, y, z, yaw, sx, sy, sz] = std::array::from_fn(|i| t[i] as f64);
        let (cos, sin) = turn(yaw);
        for mx in [b[0] as f64 * sx, b[3] as f64 * sx] {
            for my in [b[1] as f64 * sy, b[4] as f64 * sy] {
                let (px, py) = (x + cos * mx - sin * my, y + sin * mx + cos * my);
                (e[0], e[3]) = (e[0].min(px), e[3].max(px));
                (e[1], e[4]) = (e[1].min(py), e[4].max(py));
            }
        }
        e[2] = e[2].min(z + b[2] as f64 * sz);
        e[5] = e[5].max(z + b[5] as f64 * sz);
    }
    Ok(e)
}

impl Buildings {
    /// The `placed` buildings drawn from `library`, every module's mesh
    /// `bounds` given by library module (`None`: its kit is not installed).
    /// A template without art, or without intact rows, or an intact row whose
    /// kit is not installed, is refused by name.
    pub fn new(
        library: Library,
        bounds: &[Option<Bounds>],
        placed: Vec<Placed>,
        style: &BuildingStyle,
    ) -> Result<Self, String> {
        Self::from_source(Arc::new(library), bounds, Arc::new(placed), style)
    }

    fn from_source(
        library: Arc<Library>,
        bounds: &[Option<Bounds>],
        placed: Arc<Vec<Placed>>,
        style: &BuildingStyle,
    ) -> Result<Self, String> {
        let states = placed
            .iter()
            .map(|p| library.states(&p.template).cloned())
            .collect::<Result<Vec<_>, _>>()?;
        let mut extents = std::collections::HashMap::new();
        let mut records = Vec::with_capacity(placed.len());
        let mut value = Vec::with_capacity(placed.len());
        let mut cells = Vec::with_capacity(placed.len());
        for (p, s) in placed.iter().zip(&states) {
            let intact = s[0].clone().ok_or_else(|| {
                format!(
                    "state.missing: template \"{}\" has no \"intact\" rows",
                    p.template
                )
            })?;
            if !extents.contains_key(&p.template) {
                extents.insert(
                    p.template.clone(),
                    template_extent(&library, bounds, intact.clone())?,
                );
            }
            let e = extents[&p.template];
            let [x, y, z, yaw] = p.frame;
            let (cos, sin) = turn(yaw);
            let mut b = [f64::INFINITY; 4];
            b[2..].fill(f64::NEG_INFINITY);
            for lx in [e[0], e[3]] {
                for ly in [e[1], e[4]] {
                    let (px, py) = (x + cos * lx - sin * ly, y + sin * lx + cos * ly);
                    (b[0], b[2]) = (b[0].min(px), b[2].max(px));
                    (b[1], b[3]) = (b[1].min(py), b[3].max(py));
                }
            }
            // The building's own value, from its owner: the same in every battle.
            value.push(
                (1.0 + (crate::seed::Mulberry32::new(p.owner).sample() * 2.0 - 1.0)
                    * style.tint_jitter) as f32,
            );
            let coarse = intact
                .clone()
                .any(|r| library.tiers[r] & (1 << COARSEST) != 0);
            // Bucketed where it was placed, as a 32-bit float.
            let cell = |v: f64| ((v as f32) as f64 / style.chunk_m).floor() as i64;
            cells.push(coarse.then(|| (cell(y) + 4096) * 8192 + cell(x) + 4096));
            records.push(BuildingRecord {
                frame: p.frame.map(|v| v as f32),
                min: [b[0], b[1], z + e[2]].map(|v| v as f32),
                owner: p.owner,
                max: [b[2], b[3], z + e[5]].map(|v| v as f32),
                chunk: -1,
                rows: std::array::from_fn(|k| s[k].as_ref().map_or(-1, |r| r.len() as i32)),
            });
        }
        // Chunks are numbered by cell, row-major.
        let mut ids: Vec<i64> = cells.iter().flatten().copied().collect();
        ids.sort_unstable();
        ids.dedup();
        for (record, cell) in records.iter_mut().zip(&cells) {
            if let Some(id) = cell {
                record.chunk = ids.binary_search(id).unwrap() as i32;
            }
        }
        let mut used = vec![false; library.modules.len()];
        for s in &states {
            for r in s.iter().flatten().flat_map(|r| r.clone()) {
                used[library.module[r] as usize] = true;
            }
        }
        let modules: Vec<usize> = (0..used.len()).filter(|&m| used[m]).collect();
        let mut compact = vec![u32::MAX; used.len()];
        for (i, &m) in modules.iter().enumerate() {
            compact[m] = i as u32;
        }
        let linear = std::array::from_fn(|v| super::srgb_to_linear(v as f64 / 255.0) as f32);
        Ok(Self {
            bounds: bounds.to_vec(),
            style: *style,
            library,
            placed,
            states,
            value,
            records,
            modules,
            compact,
            linear,
        })
    }

    /// Rebuilds authored extents while sharing immutable template placement.
    pub fn replace_bounds(
        &self,
        mut replacement: impl FnMut(&str, &str) -> Result<Option<Bounds>, String>,
    ) -> Result<Self, String> {
        let mut bounds = self.bounds.clone();
        for &i in &self.modules {
            let module = &self.library.modules[i];
            if let Some(next) = replacement(&module.kit, &module.name)? {
                bounds[i] = Some(next);
            }
        }
        Self::from_source(
            self.library.clone(),
            &bounds,
            self.placed.clone(),
            &self.style,
        )
    }

    /// The library's module `m` (kit appearance, module name).
    pub fn module(&self, m: usize) -> (&str, &str) {
        let module = &self.library.modules[self.modules[m]];
        (&module.kit, &module.name)
    }

    /// Appends building `b`'s rows in `state` (a `STATES` index) to `out`,
    /// those drawn at `tier` only when one is named, in template order.
    /// Nothing when its template has no rows for the state.
    pub fn rows(&self, b: usize, state: usize, tier: Option<usize>, out: &mut Vec<BuildingModule>) {
        let Some(rows) = self.states[b][state].clone() else {
            return;
        };
        let lib = &self.library;
        let [tx, ty, tz, yaw] = self.placed[b].frame;
        let (cos, sin) = turn(yaw);
        let value = self.value[b] as f64;
        for r in rows.filter(|&r| tier.is_none_or(|t| lib.tiers[r] & (1 << t) != 0)) {
            let t = &lib.transform[r * ROW_FLOATS..(r + 1) * ROW_FLOATS];
            let (x, y) = (t[0] as f64, t[1] as f64);
            let [px, py, pz] = [
                tx + cos * x - sin * y,
                ty + sin * x + cos * y,
                tz + t[2] as f64,
            ]
            .map(|v| v as f32);
            let [red, green, blue]: [f32; 3] = std::array::from_fn(|k| {
                (self.linear[lib.tint[r * 3 + k] as usize] as f64 * value) as f32
            });
            let turned = (yaw + t[3] as f64) as f32;
            let record = [
                px, py, pz, turned, 0.0, 0.0, 0.0, 0.0, red, green, blue, -1.0, t[4], t[5], t[6],
                0.0,
            ];
            out.push(BuildingModule {
                record,
                kit_module: self.compact[lib.module[r] as usize] as usize,
                tiers: lib.tiers[r] as u32,
                building: b,
            });
        }
    }

    /// Building `b`'s template id.
    pub fn template(&self, b: usize) -> &str {
        &self.placed[b].template
    }

    pub fn len(&self) -> usize {
        self.placed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }
}
