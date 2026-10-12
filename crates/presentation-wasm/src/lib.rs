//! The browser's presentation module, 18b's WASM binding of
//! `crates/presentation`: one side's publications in; the game UI's
//! observation, and each drawn frame's records as views over this module's
//! memory, out. It links the publication reader and the presentation crate,
//! not the rules, and runs on the page's main thread.
use std::sync::Arc;

use presentation::presentation::{Ghost, Presentation as Inner, Ui};
use presentation::records::{FrameHeader, WorldHeader};
use presentation::sound::frame::{self, Command, Listener, Motion, Scripted, Sound};
use presentation::sound::synth;
use presentation::static_world::buildings::{Placed, PlacedBuildings};
use presentation::static_world::fallen::Known;
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
    /// where its kit is not installed; per building its template (an index
    /// of `templates`, a JSON list of ids), frame (x, y, z, yaw) and owner
    /// prop; each part prop (`part_props`) and its building
    /// (`part_buildings`); and the prop kinds a gutted building's parts
    /// become (`shells`, a JSON list). `load_world`'s `buildings` style
    /// places them.
    #[allow(clippy::too_many_arguments)]
    pub fn add_buildings(
        &mut self,
        library: &[u8],
        bounds: &[f32],
        templates: &str,
        template: &[u16],
        frames: &[f64],
        owners: &[u32],
        part_props: &[u32],
        part_buildings: &[u32],
        shells: &str,
    ) -> Result<(), JsError> {
        let json = |e: serde_json::Error| JsError::new(&e.to_string());
        let ids: Vec<String> = serde_json::from_str(templates).map_err(json)?;
        let shells: Vec<String> = serde_json::from_str(shells).map_err(json)?;
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
            parts: part_props
                .iter()
                .zip(part_buildings)
                .map(|(&p, &b)| (p, b as usize))
                .collect(),
            shells,
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

    /// The buildings the side knows destroyed as the last frame drew them:
    /// building and state (1 ruin, 2 gutted), in pairs.
    pub fn fallen_buildings(&self) -> Vec<u32> {
        self.inner
            .fallen()
            .iter()
            .flat_map(|&(b, s)| [b as u32, s as u32])
            .collect()
    }

    /// Draws the units' tiers at `units` and the buildings' at `buildings`
    /// times their authored thresholds (a graphics quality; 1 as authored).
    pub fn set_detail_scale(&mut self, units: f64, buildings: f64) {
        self.inner.set_detail_scale(units, buildings);
    }

    /// The renderer holds no static batches (a new one): the next frame
    /// writes every one.
    pub fn resend_statics(&mut self) {
        self.inner.resend_statics();
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
    /// address. Its sets are views valid until the next call. `known` is a
    /// lab's stand-in for the publication's known props, JSON (empty: the
    /// publication's); a building known in a state its template lacks throws
    /// by name.
    pub fn frame(
        &mut self,
        time: f64,
        camera: &[f64],
        selected: &[u32],
        xray: bool,
        ghosts: &str,
        known: &str,
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
        let known: Option<Vec<Known>> = if known.is_empty() {
            None
        } else {
            Some(serde_json::from_str(known).map_err(|e| JsError::new(&e.to_string()))?)
        };
        let ui = Ui {
            selected,
            xray,
            ghosts: &ghosts,
            known: known.as_deref(),
        };
        let f = self.inner.frame(time, &camera, &ui).map_err(error)?;
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

/// One battle's sound (`presentation::sound::frame::Sound`): publications
/// and drawn frames in, the commands the page's mixer carries out out, as
/// JSON.
#[wasm_bindgen]
pub struct SoundFrame {
    inner: Sound,
}

fn commands(c: &[Command]) -> String {
    serde_json::to_string(c).expect("commands serialize")
}

#[wasm_bindgen]
impl SoundFrame {
    /// From its setup JSON: `presentation.audio`, the sound catalog, the
    /// game's weapon rows, the tick rate and `presentation.effects`' smoke
    /// and cook-off.
    #[wasm_bindgen(constructor)]
    pub fn new(setup: &str) -> Result<SoundFrame, JsError> {
        Ok(SoundFrame {
            inner: Sound::from_json(setup).map_err(error)?,
        })
    }

    /// The prepared sounds' lengths, JSON `{sound: [seconds per alternative]}`.
    pub fn durations(&mut self, durations: &str) -> Result<(), JsError> {
        let d = serde_json::from_str(durations).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner.set_durations(d).map_err(error)
    }

    /// Every sound a battle plays, JSON: what the page prepares.
    pub fn sounds(&self) -> String {
        serde_json::to_string(&self.inner.tables().battle_sounds()).expect("names serialize")
    }

    /// Takes `presentation`'s last publication, at mixer time `now`.
    pub fn note(&mut self, presentation: &Presentation, now: f64) -> String {
        match presentation.inner.heard() {
            Some(heard) => commands(self.inner.note(&heard, now)),
            None => "[]".into(),
        }
    }

    /// Takes a scripted publication (`SoundPublication` JSON).
    pub fn note_scripted(&mut self, publication: &str, now: f64) -> Result<String, JsError> {
        let scripted: Scripted =
            serde_json::from_str(publication).map_err(|e| JsError::new(&e.to_string()))?;
        let launches = scripted.launches();
        Ok(commands(self.inner.note(&scripted.heard(&launches), now)))
    }

    /// Hears `presentation`'s last frame at wall time `wall`, mixer time `now`.
    pub fn hear(&mut self, presentation: &Presentation, wall: f64, now: f64) -> String {
        let share = self.inner.tables().audio.listener_eye_share;
        let Some(listener) = presentation.inner.listener(share) else {
            return "[]".into();
        };
        let motion = presentation.inner.motion();
        let clock = presentation.inner.clock_of_frame();
        commands(self.inner.update(clock, wall, now, &motion, listener))
    }

    /// Hears scripted `motion` (`SoundMotion` JSON) at presentation time
    /// `clock` from `listener` (`Listener` JSON).
    pub fn update_scripted(
        &mut self,
        clock: f64,
        wall: f64,
        now: f64,
        motion: &str,
        listener: &str,
    ) -> Result<String, JsError> {
        let motion: Motion =
            serde_json::from_str(motion).map_err(|e| JsError::new(&e.to_string()))?;
        let listener: Listener =
            serde_json::from_str(listener).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(commands(
            self.inner.update(clock, wall, now, &motion, listener),
        ))
    }

    /// Silences and forgets everything, from mixer time `now`.
    pub fn reset(&mut self, now: f64) -> String {
        commands(self.inner.reset(now))
    }

    /// Voices, budget, holds and counts, JSON (`SoundStats`).
    pub fn stats(&self) -> String {
        serde_json::to_string(&self.inner.stats()).expect("stats serialize")
    }
}

/// The loops a vehicle of sound row `row` (`VehicleSound` JSON) sounds at
/// `load` and `traverse`, JSON.
#[wasm_bindgen]
pub fn vehicle_loops(
    row: &str,
    load: f64,
    traverse: f64,
    reversing: bool,
) -> Result<String, JsError> {
    let row = serde_json::from_str(row).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(
        serde_json::to_string(&frame::vehicle_loops(&row, load, traverse, reversing))
            .expect("loops serialize"),
    )
}

/// A hearing cue's stereo position for a listener looking along `forward`.
#[wasm_bindgen]
pub fn sector_pan(sector: f64, forward: &[f64]) -> f64 {
    frame::sector_pan(sector, [forward[0], forward[1], forward[2]])
}

/// The synthesized sound `name` at `sample_rate`: its channels one after
/// another, each `samples` long.
#[wasm_bindgen]
pub fn synthesize(name: &str, sample_rate: f64) -> Result<Vec<f32>, JsError> {
    let s = synth::synthesize(name, sample_rate).map_err(error)?;
    Ok(s.channels.concat())
}

/// Where sound listens for a camera at `eye` looking at `target` with
/// `up`, `share` of the way from the target to the eye: position, then facing.
#[wasm_bindgen]
pub fn camera_listener(eye: &[f64], target: &[f64], up: &[f64], share: f64) -> Vec<f64> {
    let v = |a: &[f64]| [a[0], a[1], a[2]];
    let l = Listener::of_camera(v(eye), v(target), v(up).map(|x| x as f32), share);
    [l.position, l.forward].concat()
}
