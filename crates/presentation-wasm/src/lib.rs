//! The browser's presentation module, 18b's WASM binding of
//! `crates/presentation`: one side's publications in; the game UI's
//! observation, and each drawn frame's records as views over this module's
//! memory, out. It links the publication reader and the presentation crate,
//! not the rules, and runs on the page's main thread.
use std::sync::Arc;

use presentation::presentation::{Ghost, Presentation as Inner, Ui};
use presentation::records::{FrameHeader, WorldHeader};
use presentation::static_world::buildings::{Placed, PlacedBuildings};
use presentation::static_world::library::{Bounds, Library};
use presentation::units::drawn::ClipTables;
use presentation::view::Camera;
use serde::Serialize;
use sim::publication::reader::{Corpse, FallenBody, Fog, KnownProp, Shared};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Presentation {
    inner: Inner,
    tables: ClipTables,
    buildings: Option<PlacedBuildings>,
    /// The static groups and the fog the page last took, kept to compare by pointer.
    taken: Option<(Shared<Corpse>, Shared<KnownProp>, Shared<FallenBody>)>,
    taken_fog: Option<Arc<Fog>>,
    frame_header: FrameHeader,
    world_header: WorldHeader,
}

fn error(e: String) -> JsError {
    JsError::new(&e)
}

/// A slice as (pointer, count) in this module's memory.
fn set<T>(items: &[T]) -> [u32; 2] {
    [items.as_ptr() as u32, items.len() as u32]
}

/// Appends `"name":<group>` to an object's JSON when `group` isn't the one
/// the page already holds.
fn splice<T: Serialize>(json: &mut String, name: &str, group: &Arc<T>, taken: Option<&Arc<T>>) {
    if taken.is_none_or(|t| !Arc::ptr_eq(t, group)) {
        json.pop();
        json.push_str(&format!(",\"{name}\":"));
        json.push_str(&serde_json::to_string(&**group).unwrap());
        json.push('}');
    }
}

#[wasm_bindgen]
impl Presentation {
    /// A presentation of the stream `layout` (the battle's
    /// `observation_layout`) names, at `tick_hz` (30 when not given); with
    /// no layout, of a world no battle publishes into, drawn from
    /// `load_world` alone.
    #[wasm_bindgen(constructor)]
    pub fn new(layout: Option<String>, tick_hz: Option<f64>) -> Result<Presentation, JsError> {
        Ok(Presentation {
            inner: Inner::new(layout.as_deref(), tick_hz.unwrap_or(30.0)).map_err(error)?,
            tables: ClipTables::new(),
            buildings: None,
            taken: None,
            taken_fog: None,
            frame_header: FrameHeader::default(),
            world_header: WorldHeader::default(),
        })
    }

    /// Reads the next publication's raw words, received at wall time
    /// `arrived_s`. False for a stale record; a malformed one throws.
    pub fn push_publication(&mut self, words: &[u32], arrived_s: f64) -> Result<bool, JsError> {
        self.inner.push_publication(words, arrived_s).map_err(error)
    }

    /// The latest publication's observation as JSON, keyed as `ObservationView`,
    /// without `fog.bits` and `groundPatch.runs`. A static group is present
    /// only when it changed since the previous call.
    pub fn observation(&mut self) -> String {
        let latest = self.inner.observation().expect("a publication was pushed");
        let taken = self.taken.as_ref();
        let mut json = serde_json::to_string(latest).unwrap();
        splice(&mut json, "corpses", &latest.corpses, taken.map(|t| &t.0));
        splice(
            &mut json,
            "knownProps",
            &latest.known_props,
            taken.map(|t| &t.1),
        );
        splice(
            &mut json,
            "fallenBodies",
            &latest.fallen_bodies,
            taken.map(|t| &t.2),
        );
        self.taken = Some((
            latest.corpses.clone(),
            latest.known_props.clone(),
            latest.fallen_bodies.clone(),
        ));
        json
    }

    /// The latest fog bits when they changed since the previous call.
    pub fn fog_bits(&mut self) -> Option<Vec<u32>> {
        let fog = &self.inner.observation()?.fog;
        let changed = self.taken_fog.as_ref().is_none_or(|t| !Arc::ptr_eq(t, fog));
        self.taken_fog = Some(fog.clone());
        changed.then(|| fog.bits.clone())
    }

    /// The latest publication's ground runs.
    pub fn ground_runs(&self) -> Vec<f32> {
        self.inner
            .observation()
            .map_or_else(Vec::new, |o| o.ground_patch.runs.clone())
    }

    /// Records still in flight are stale; the next must open a new epoch,
    /// for `side` when named.
    pub fn invalidate(&mut self, side: Option<String>) -> Result<(), JsError> {
        self.inner.invalidate(side.as_deref()).map_err(error)?;
        (self.taken, self.taken_fog) = (None, None);
        Ok(())
    }

    /// One skinned body's dense clip table, before `load_world`.
    pub fn add_clip_table(&mut self, name: String, floats: Vec<f32>) {
        self.tables.insert(name, floats);
    }

    /// The map's buildings, before `load_world`: the template library's
    /// file, inflated; per library module its mesh bounds (min, max), NaN
    /// where its kit is not installed; and per building its template (an
    /// index of `templates`, a JSON list of ids), frame (x, y, z, yaw) and
    /// owner prop. `load_world`'s `buildings` style places them.
    pub fn add_buildings(
        &mut self,
        library: &[u8],
        bounds: &[f32],
        templates: &str,
        template: &[u16],
        frames: &[f64],
        owners: &[u32],
    ) -> Result<(), JsError> {
        let ids: Vec<String> =
            serde_json::from_str(templates).map_err(|e| JsError::new(&e.to_string()))?;
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
        self.buildings = Some(PlacedBuildings {
            library: Library::decode(library).map_err(error)?,
            bounds,
            placed,
        });
        Ok(())
    }

    /// Loads what the world draws (18b's `load_world`, as `WorldInput`
    /// JSON) with the clip tables and buildings added; returns the
    /// `WorldHeader`'s address.
    pub fn load_world(&mut self, json: &str) -> Result<u32, JsError> {
        let input = serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner
            .load_world(
                input,
                std::mem::take(&mut self.tables),
                self.buildings.take(),
            )
            .map_err(error)?;
        let world = self.inner.world();
        self.world_header = WorldHeader {
            appearances: set(&world.appearances),
            strings: set(world.strings.as_bytes()),
        };
        Ok(&self.world_header as *const WorldHeader as u32)
    }

    /// What the side knows destroyed: buildings and their states (1 ruin, 2
    /// gutted). Returns those whose template has no rows for their state,
    /// and then changes nothing.
    pub fn set_fallen(&mut self, buildings: &[u32], states: &[u32]) -> Vec<u32> {
        let fallen: Vec<(usize, usize)> = buildings
            .iter()
            .zip(states)
            .map(|(&b, &s)| (b as usize, s as usize))
            .collect();
        self.inner
            .set_fallen(&fallen)
            .into_iter()
            .map(|b| b as u32)
            .collect()
    }

    /// The buildings' walk as the last frame left it: chunks, coarse rows,
    /// chunks resident, rows resident, rows expanded, chunks refused room,
    /// the fallen rows and those drawn per tier. Empty without buildings.
    pub fn building_stats(&self) -> Vec<u32> {
        self.inner
            .building_stats()
            .map_or_else(Vec::new, |s| s.iter().map(|&n| n as u32).collect())
    }

    /// The presentation time at wall time `wall_s`, sim seconds.
    pub fn clock(&self, wall_s: f64) -> Option<f64> {
        self.inner.clock(wall_s)
    }

    /// What to draw at presentation time `time` for `camera` (eye xyz,
    /// target xyz, up xyz, fov y, near, width, height): the `FrameHeader`'s
    /// address. Its sets are views valid until the next call.
    pub fn frame(
        &mut self,
        time: f64,
        camera: &[f64],
        selected: &[u32],
        xray: bool,
        ghosts: &str,
    ) -> Result<u32, JsError> {
        let c = camera;
        let camera = Camera {
            eye: [c[0], c[1], c[2]],
            target: [c[3], c[4], c[5]],
            up: [c[6] as f32, c[7] as f32, c[8] as f32],
            fov_y: c[9] as f32,
            near: c[10] as f32,
            size: [c[11] as u32, c[12] as u32],
        };
        let ghosts: Vec<Ghost> = if ghosts.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(ghosts).map_err(|e| JsError::new(&e.to_string()))?
        };
        let ui = Ui {
            selected,
            xray,
            ghosts: &ghosts,
        };
        let f = self.inner.frame(time, &camera, &ui);
        let header = FrameHeader {
            clock: f.clock as f32,
            tick: f.tick,
            index: f.index,
            light_wrap: f.light_wrap,
            batches: set(&f.batches),
            instances: set(&f.instances),
            palettes: set(&f.palettes),
            effects: [0; 2],
            lights: [0; 2],
            picks: set(&f.picks),
            emitters: set(&f.emitters),
            counts: set(std::slice::from_ref(&f.counts)),
        };
        self.frame_header = FrameHeader {
            effects: set(self.inner.effects()),
            lights: set(self.inner.lights()),
            ..header
        };
        Ok(&self.frame_header as *const FrameHeader as u32)
    }

    /// The last publication as sound reads it, JSON: its effect publication
    /// (with the hulls the side watched brew up) and the launches it shows.
    pub fn sound_publication(&self) -> String {
        self.inner.sound_json()
    }

    /// The last frame as the labs' scenes probe it, JSON.
    pub fn probe(&self) -> String {
        self.inner.probe_json()
    }

    /// Where the last frame draws the presented side's vehicle `unit`'s
    /// mount `mount`'s muzzle.
    pub fn drawn_muzzle(&mut self, unit: f64, mount: usize) -> Option<Vec<f64>> {
        self.inner.drawn_muzzle(unit, mount).map(|p| p.to_vec())
    }

    /// The known props whose wreck the presentation draws moving (a cook-off).
    pub fn moving_wrecks(&self) -> Vec<u32> {
        self.inner.moving_wrecks()
    }
}

/// Each of a scripted run's publications' launches, JSON: a lab's sound
/// without a battle, by the effects' own derivation.
#[wasm_bindgen]
pub fn launches(publications: &str) -> Result<String, JsError> {
    Inner::launches_json(publications).map_err(error)
}

/// The battle's light (`presentation.light`, JSON) resolved into its uniform
/// records' words in order: sky, aerial, sun, bloom, grade (`light.rs`). The
/// renderer needs them when it builds its frame, before any battle loads and
/// for scenes with none, so this needs no `Presentation`.
#[wasm_bindgen]
pub fn light_records(light: &str) -> Result<Vec<f32>, JsError> {
    let settings: presentation::light::LightSettings = serde_json::from_str(light)
        .map_err(|e| JsError::new(&format!("presentation.light: {e}")))?;
    Ok(settings.records().words())
}
