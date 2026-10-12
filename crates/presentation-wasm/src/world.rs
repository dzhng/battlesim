//! The static world for the page (`crates/static-world`): a map's buildings
//! resolved against the template art library, and the walk that chooses what
//! each view draws of them, handed out as flat records.
use presentation::static_world::buildings::{Buildings, Placed, Style, TIERS};
use presentation::static_world::library::{Bounds, Library};
use presentation::static_world::walk::{BuildingWalk, Shadow};
use presentation::view::{Plane, View};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct StaticWorld {
    walk: BuildingWalk,
}

#[wasm_bindgen]
impl StaticWorld {
    /// The map's buildings: per building its template (an index of
    /// `templates`, a JSON list of ids), frame (x, y, z, yaw) and owner prop;
    /// the library file's bytes, inflated; per library module its mesh bounds
    /// (min, max), NaN where its kit is not installed; and
    /// `presentation.buildings` (JSON).
    #[wasm_bindgen(constructor)]
    pub fn new(
        library: &[u8],
        bounds: &[f32],
        templates: &str,
        template: &[u16],
        frames: &[f64],
        owners: &[u32],
        style: &str,
    ) -> Result<StaticWorld, JsError> {
        let error = |e: String| JsError::new(&e);
        let library = Library::decode(library).map_err(error)?;
        let ids: Vec<String> = serde_json::from_str(templates).map_err(|e| error(e.to_string()))?;
        let style: serde_json::Value =
            serde_json::from_str(style).map_err(|e| error(e.to_string()))?;
        let number = |key: &str| style[key].as_f64().unwrap_or(f64::NAN);
        let lod = |t: usize| style["lod_px_per_m"][t].as_f64().unwrap_or(f64::NAN);
        let bounds: Vec<Option<Bounds>> = bounds
            .as_chunks::<6>()
            .0
            .iter()
            .map(|b| (!b[0].is_nan()).then_some(*b))
            .collect();
        let placed = (0..owners.len())
            .map(|b| Placed {
                owner: owners[b],
                template: ids[template[b] as usize].clone(),
                frame: frames[b * 4..b * 4 + 4].try_into().unwrap(),
            })
            .collect();
        let resolved = Style {
            chunk_m: number("chunk_m"),
            tint_jitter: number("tint_jitter"),
        };
        let buildings = Buildings::new(library, &bounds, placed, &resolved).map_err(error)?;
        let pool = number("pool_records") as usize;
        Ok(StaticWorld {
            walk: BuildingWalk::new(buildings, [lod(0), lod(1), lod(2)], resolved.chunk_m, pool),
        })
    }

    /// Per module the walk draws (`module · 4 + tier` in its keys), its library module.
    pub fn modules(&self) -> Vec<u32> {
        self.walk
            .buildings
            .modules
            .iter()
            .map(|&m| m as u32)
            .collect()
    }

    /// What the side knows destroyed: buildings and their states (1 ruin, 2
    /// gutted). Returns those whose template has no rows for their state, and
    /// then changes nothing.
    pub fn set_fallen(&mut self, buildings: &[u32], states: &[u32]) -> Vec<u32> {
        let fallen: Vec<(usize, usize)> = buildings
            .iter()
            .zip(states)
            .map(|(&b, &s)| (b as usize, s as usize))
            .collect();
        self.walk
            .set_fallen(&fallen)
            .into_iter()
            .map(|b| b as u32)
            .collect()
    }

    /// Walks `view` (`[eye; 3, pixels per metre, shadow fall; 2, reach,
    /// sides; 16]`, sim space); true when what it draws or casts changed.
    pub fn select(&mut self, view: &[f64]) -> bool {
        let side = |i: usize| Plane { normal: [view[7 + i * 4], view[8 + i * 4], view[9 + i * 4]], constant: view[10 + i * 4] };
        let detail = View::for_detail([view[0], view[1], view[2]], view[3], std::array::from_fn(side));
        let shadow = (view[6] >= 0.0).then_some(Shadow { fall: [view[4], view[5]], reach: view[6] });
        !self.walk.select(&detail, shadow.as_ref()).is_empty()
    }

    /// The last walk's records drawn into the view (`pass` 0) or cast (1),
    /// one list after another.
    pub fn records(&self, pass: u32) -> Vec<f32> {
        self.lists(pass)
            .iter()
            .flatten()
            .flatten()
            .copied()
            .collect()
    }

    /// Its draws for the same pass: library module, tier, first record, count.
    pub fn draws(&self, pass: u32) -> Vec<u32> {
        let (mut out, mut first) = (vec![], 0);
        for (key, list) in self
            .lists(pass)
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_empty())
        {
            let module = self.walk.buildings.modules[key / TIERS] as u32;
            out.extend([module, (key % TIERS) as u32, first, list.len() as u32]);
            first += list.len() as u32;
        }
        out
    }

    /// The last walk's drawn instances per module and tier (`module · 4 + tier`).
    pub fn counts(&self) -> Vec<u32> {
        self.walk.counts.clone()
    }

    /// The chunks, their coarse rows, then the last walk's chunks resident,
    /// rows resident, rows expanded, chunks refused room, the fallen rows and
    /// those drawn per tier.
    pub fn stats(&self) -> Vec<u32> {
        self.walk.stats.map(|n| n as u32).to_vec()
    }
}

impl StaticWorld {
    fn lists(&self, pass: u32) -> &Vec<Vec<[f32; 16]>> {
        if pass == 0 {
            &self.walk.lists.view
        } else {
            &self.walk.lists.cast
        }
    }
}
