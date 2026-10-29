//! Presentation and sound bindings linked into the browser renderer's module.
//! Side publications become canonical Rust frames and UI observation views;
//! rendering borrows those records directly in the same memory.
mod fielded;
mod fog;
mod ground;
mod lab;
mod shadow;
mod world;

#[cfg(test)]
mod header_tests;

use std::sync::Arc;

use presentation::presentation::{Ghost, Presentation as Inner, Ui};
use presentation::records::{FrameHeader, WorldHeader};
use presentation::sound::bank::{self, Bank, Core};
use presentation::sound::catalog::{Cadence, Recipe};
use presentation::sound::frame::{
    self, Command, Listener, Motion, Scripted, Sound, SoundSoldier, SoundVehicle,
};
use presentation::sound::voices::{self, Request};
use presentation::sound::{music, synth};
use presentation::static_world::buildings::PlacedBuildings;
use presentation::static_world::fallen::Known;
use presentation::units::drawn::ClipTables;
use presentation::view::Camera;
use serde::Serialize;
use sim::publication::reader::{Corpse, FallenBody, Fog, KnownProp, Shared};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Presentation {
    inner: Inner,
    tables: ClipTables,
    lab_assets: std::collections::BTreeMap<String, presentation::presentation::lab::Asset>,
    buildings: Option<PlacedBuildings>,
    /// The static groups and the fog the page last took, kept to compare by pointer.
    taken: Option<(Shared<Corpse>, Shared<KnownProp>, Shared<FallenBody>)>,
    taken_fog: Option<Arc<Fog>>,
    frame_header: FrameHeader,
    world_header: WorldHeader,
    /// The biome's own grass rules, while a lab grows the ground's by others.
    grass: Option<presentation::static_world::ground::biome::GrassRules>,
}

impl Presentation {
    /// The canonical presentation borrowed by the browser's Rust renderer.
    pub fn core(&self) -> &Inner {
        &self.inner
    }

    /// Publish current canonical record pointers after preparation may append
    /// batches or instances without invoking the JavaScript frame entrypoint.
    pub fn refresh_frame_header(&mut self) -> u32 {
        let f = self.inner.last_frame();
        let shadow = f.shadow.as_ref();
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
            view: set(std::slice::from_ref(&f.view)),
            camera: set(std::slice::from_ref(&f.camera)),
            sun_shadow: set(shadow.map_or(&[][..], |s| std::slice::from_ref(&s.receiver))),
            casters: set(shadow.map_or(&[][..], |s| &s.casters)),
        };
        self.frame_header = FrameHeader {
            effects: set(self.inner.effects()),
            lights: set(self.inner.lights()),
            ..header
        };
        &self.frame_header as *const FrameHeader as u32
    }

    /// Mutable preparation never carries a borrow across an asynchronous call.
    pub fn core_mut(&mut self) -> &mut Inner {
        &mut self.inner
    }
}

fn error(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// A camera as the page sends it: eye, target, up, the vertical field of
/// view (rad), the near plane (m), and the drawing size in device pixels.
fn camera_of(c: &[f64]) -> Camera {
    Camera {
        eye: [c[0], c[1], c[2]],
        target: [c[3], c[4], c[5]],
        up: [c[6], c[7], c[8]],
        fov_y: c[9],
        near: c[10],
        size: [c[11] as u32, c[12] as u32],
    }
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
            lab_assets: Default::default(),
            buildings: None,
            taken: None,
            taken_fog: None,
            frame_header: FrameHeader::default(),
            world_header: WorldHeader::default(),
            grass: None,
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

    /// The scar cache's `ScarStats`, as JSON.
    pub fn scar_stats(&self) -> String {
        serde_json::to_string(&self.inner.scar_stats()).unwrap()
    }

    /// The learned ground's epoch, revision and cleared cells (0 without a layout).
    pub fn ground_epoch(&self) -> f64 {
        self.inner.learned().map_or(0.0, |g| g.epoch)
    }
    pub fn ground_revision(&self) -> f64 {
        self.inner.learned().map_or(0.0, |g| g.revision)
    }
    pub fn ground_cleared_count(&self) -> u32 {
        self.inner.learned().map_or(0, |g| g.cleared_count() as u32)
    }
    /// The side whose ground it learned, once a snapshot came.
    pub fn ground_side(&self) -> Option<String> {
        self.inner.learned().and_then(|g| g.side.clone())
    }

    /// Whether the side has seen cell (i, j) cleared.
    pub fn ground_cleared(&self, i: u32, j: u32) -> bool {
        self.inner
            .learned()
            .is_some_and(|g| g.is_cleared(i as usize, j as usize))
    }

    /// The cleared spans in tile order: (tile, start + length × 256) pairs.
    pub fn ground_cleared_runs(&self) -> Vec<u32> {
        self.inner
            .learned()
            .map_or_else(Vec::new, |g| g.cleared_runs())
    }

    /// Every marked or cleared cell in row-major order: (cell index, marks
    /// as drawn: crater, scorch, tracks, trampled a byte each) pairs.
    pub fn ground_marked(&self) -> Vec<u32> {
        self.inner
            .learned()
            .map_or_else(Vec::new, |g| g.marked_cells())
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
    /// address, with the camera's view, its uniform and the sun's cascades.
    /// Its sets are views valid until the next call. `known` is a lab's
    /// stand-in for the publication's known props, JSON (empty: the
    /// publication's); a building known in a state its template lacks throws
    /// by name. `generation` is the renderer's, bumped whenever it holds no
    /// static batches: a new one is written every one. `player` is the
    /// index of the player the UI commands as (`Ui::player`), absent where
    /// units have no owner.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        time: f64,
        camera: &[f64],
        selected: &[u32],
        xray: bool,
        player: Option<u32>,
        ghosts: &str,
        known: &str,
        generation: u32,
    ) -> Result<u32, JsError> {
        let camera = camera_of(camera);
        let ghosts: Vec<Ghost> = if ghosts.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(ghosts).map_err(error)?
        };
        let known: Option<Vec<Known>> = if known.is_empty() {
            None
        } else {
            Some(serde_json::from_str(known).map_err(error)?)
        };
        let ui = Ui {
            selected,
            xray,
            player,
            ghosts: &ghosts,
            known: known.as_deref(),
        };
        self.inner
            .frame(time, &camera, &ui, generation)
            .map_err(error)?;
        Ok(self.refresh_frame_header())
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
}

/// Each of a scripted run's publications' launches, JSON: a lab's sound
/// without a battle, by the effects' own derivation.
#[wasm_bindgen]
pub fn launches(publications: &str) -> Result<String, JsError> {
    Inner::launches_json(publications).map_err(error)
}

/// The tier each of a lab's own models draws at (`view::model_detail`:
/// `-1` culled, a mesh tier 0–3, or `4` its card) for the frame's `View`
/// record (its words) and `presentation.models` (JSON). Each model is six
/// numbers: x, y, z, its size and reach (`Bounds`), and 1 when it has a
/// card, else 0.
#[wasm_bindgen]
pub fn model_tiers(view: &[f32], detail: &str, models: &[f64]) -> Result<Vec<i32>, JsError> {
    use presentation::view::{model_detail, View};
    let record = bytemuck::pod_read_unaligned::<presentation::records::View>(
        &bytemuck::cast_slice(view)[..std::mem::size_of::<presentation::records::View>()],
    );
    let view = View::from_record(&record);
    let detail: presentation::world::Detail = serde_json::from_str(detail)?;
    let models = models.as_chunks::<6>().0;
    Ok(models
        .iter()
        .map(|&[x, y, z, size, radius, card]| {
            model_detail(&detail, &view, [x, y, z], size, radius, card != 0.0, None)
        })
        .collect())
}

/// The battle's light (`presentation.light`, JSON) resolved into its uniform
/// records' words in order: sky, aerial, sun, bloom, grade (`light.rs`). The
/// renderer needs them when it builds its frame, before any battle loads and
/// for scenes with none, so this needs no `Presentation`.
#[wasm_bindgen]
pub fn light_records(light: &str) -> Result<Vec<f32>, JsError> {
    let settings = presentation::light::LightSettings::from_value(
        &serde_json::from_str(light).map_err(error)?,
    )
    .map_err(error)?;
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
        let d = serde_json::from_str(durations).map_err(error)?;
        self.inner.set_durations(d).map_err(error)
    }

    /// The enemy faction's shout pool as far as it has loaded, JSON
    /// `[{sound, duration}]`: the takes an unseen shout is drawn from.
    pub fn shouts(&mut self, takes: &str) -> Result<(), JsError> {
        let takes = serde_json::from_str(takes).map_err(error)?;
        self.inner.set_shouts(takes);
        Ok(())
    }

    /// Which voice set each unit kind speaks with, JSON `{own: {kind: set},
    /// enemy: {kind: set}}`: the side's own faction's and its enemy's.
    pub fn speakers(&mut self, speakers: &str) -> Result<(), JsError> {
        let speakers =
            serde_json::from_str(speakers).map_err(|e| JsError::new(&format!("speakers: {e}")))?;
        self.inner.set_speakers(speakers);
        Ok(())
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
        let scripted: Scripted = serde_json::from_str(publication).map_err(error)?;
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

    /// Hears scripted motion at presentation time `clock` from `listener`
    /// (its position, then forward): `soldiers` four numbers each (id, x, y,
    /// z), `vehicles` eight each (key, x, y, z, left and right travel,
    /// turret, 1 reversing), and `classes` the vehicles' sound classes,
    /// space-separated, in their order.
    #[allow(clippy::too_many_arguments)]
    pub fn update_scripted(
        &mut self,
        clock: f64,
        wall: f64,
        now: f64,
        soldiers: &[f64],
        vehicles: &[f64],
        classes: &str,
        listener: &[f64],
    ) -> String {
        let soldiers = soldiers
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[id, x, y, z]| SoundSoldier {
                id,
                position: [x, y, z],
            })
            .collect();
        let vehicles = vehicles
            .as_chunks::<8>()
            .0
            .iter()
            .zip(classes.split(' '))
            .map(
                |(&[key, x, y, z, travel_l, travel_r, turret, reverse], class)| SoundVehicle {
                    key,
                    vehicle_class: class.to_owned(),
                    position: [x, y, z],
                    travel_l,
                    travel_r,
                    turret,
                    reverse: reverse != 0.0,
                },
            )
            .collect();
        let listener = Listener {
            position: [listener[0], listener[1], listener[2]],
            forward: [listener[3], listener[4], listener[5]],
        };
        let motion = Motion { vehicles, soldiers };
        commands(self.inner.update(clock, wall, now, &motion, listener))
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

/// Which voice lines play (`presentation::sound::voices::Chatter`): the
/// takes the page has loaded and its units' requests in, the start commands
/// its voices bus carries out, and the lines started and dropped, out, as JSON.
#[wasm_bindgen]
pub struct Chatter {
    inner: voices::Chatter,
}

#[wasm_bindgen]
impl Chatter {
    /// By `rules` (`presentation.audio.voices`, JSON), takes chosen by `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(rules: &str, seed: f64) -> Result<Chatter, JsError> {
        let rules: voices::ChatterRules = serde_json::from_str(rules)
            .map_err(|e| JsError::new(&format!("presentation.audio.voices: {e}")))?;
        rules.check().map_err(error)?;
        Ok(Chatter {
            inner: voices::Chatter::new(rules, seed),
        })
    }

    /// Set `set`'s pack arrived with these voices (`[{id, speaks}]` JSON).
    pub fn voices(&mut self, set: &str, voices: &str) -> Result<(), JsError> {
        let voices = serde_json::from_str(voices).map_err(error)?;
        self.inner.set_voices(set, voices);
        Ok(())
    }

    /// A take is loaded and lasts `duration` seconds.
    pub fn loaded(
        &mut self,
        set: &str,
        voice: &str,
        trigger: &str,
        intensity: &str,
        take: u32,
        duration: f64,
    ) {
        self.inner
            .loaded(set, voice, trigger, intensity, take, duration);
    }

    /// Asks for a line (`VoiceRequest` JSON) at mixer time `now`.
    pub fn request(&mut self, request: &str, now: f64) -> Result<(), JsError> {
        let request: Request = serde_json::from_str(request).map_err(error)?;
        self.inner.request(request, now);
        Ok(())
    }

    /// Arbitrates at mixer time `now`, nearest `listener` first.
    pub fn update(&mut self, now: f64, listener: &[f64]) -> String {
        let listener = [listener[0], listener[1], listener[2]];
        serde_json::to_string(self.inner.update(now, listener)).expect("lines serialize")
    }

    /// A battle's arbitration at mixer time `now`: the barks `sound` heard,
    /// heard from where it last heard, each placed where it was shouted.
    pub fn hear(&mut self, sound: &mut SoundFrame, now: f64) -> String {
        serde_json::to_string(self.inner.hear(&mut sound.inner, now)).expect("lines serialize")
    }

    /// A line is sounding at mixer time `now`.
    pub fn speaking(&self, now: f64) -> bool {
        self.inner.speaking(now)
    }

    /// Forgets every line waiting and sounding (a new battle).
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// The battle's score (`presentation::sound::music::Score`): the barks the
/// voice lines heard in, the music bus's commands out, as JSON.
#[wasm_bindgen]
pub struct Score {
    inner: music::Score,
}

#[wasm_bindgen]
impl Score {
    /// By `rules` (`presentation.audio.music`, JSON) over `moods` (the sound
    /// catalog's `music.moods`, JSON; its battle moods), shuffled by `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(rules: &str, moods: &str, seed: u32) -> Result<Score, JsError> {
        let rules = serde_json::from_str(rules)
            .map_err(|e| JsError::new(&format!("presentation.audio.music: {e}")))?;
        let moods =
            serde_json::from_str(moods).map_err(|e| JsError::new(&format!("music.moods: {e}")))?;
        Ok(Score {
            inner: music::Score::new(rules, moods, seed),
        })
    }

    /// At mixer time `now`: the barks heard (`VoiceRequest[]` JSON) and
    /// whether a line sounds. The music bus's commands, JSON.
    pub fn hear(&mut self, now: f64, barks: &str, speaking: bool) -> Result<String, JsError> {
        let barks: Vec<Request> = serde_json::from_str(barks).map_err(error)?;
        Ok(commands(self.inner.hear(now, &barks, speaking)))
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
    let row = serde_json::from_str(row).map_err(error)?;
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

/// The synthesized sound `name` at `sample_rate`: its channel count, then
/// its channels one after another.
#[wasm_bindgen]
pub fn synthesize(name: &str, sample_rate: f64) -> Result<Vec<f32>, JsError> {
    let s = synth::synthesize(name, sample_rate).map_err(error)?;
    Ok([vec![s.channels.len() as f32], s.channels.concat()].concat())
}

/// `samples` split into channels by `layout` (channels, then frames, per entry).
fn split(layout: &[u32], samples: &[f32]) -> Vec<Vec<Vec<f32>>> {
    let mut at = 0;
    layout
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[channels, frames]| {
            let (channels, frames) = (channels as usize, frames as usize);
            (0..channels)
                .map(|_| {
                    at += frames;
                    samples[at - frames..at].to_vec()
                })
                .collect()
        })
        .collect()
}

/// The loudest K-weighted 100 ms window of one sound (`layout`: its
/// channels and frames), LUFS.
#[wasm_bindgen]
pub fn max_momentary_loudness(layout: &[u32], samples: &[f32], sample_rate: f64) -> f64 {
    bank::max_momentary_loudness(&split(layout, samples)[0], sample_rate)
}

/// The gain bringing one sound to the library's reference, times `authored`.
#[wasm_bindgen]
pub fn source_normalization(
    layout: &[u32],
    samples: &[f32],
    sample_rate: f64,
    authored: f64,
) -> f64 {
    bank::source_normalization(&split(layout, samples)[0], sample_rate, authored)
}

/// The page's recipe composer at one sample rate (`sound::bank::Bank`),
/// keeping the synthesized sounds it has made.
#[wasm_bindgen]
pub struct RecipeBank {
    bank: Bank,
    normalizations: Vec<f64>,
}

#[wasm_bindgen]
impl RecipeBank {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f64) -> RecipeBank {
        RecipeBank {
            bank: Bank::new(sample_rate),
            normalizations: Vec::new(),
        }
    }

    /// `recipe`'s (JSON) alternatives from its recordings: each one's
    /// channels and frames in `layout`, its samples in `samples`, its burst
    /// (shots, interval) in `cadences`. Out: per alternative, its channel
    /// count and frames, then its channels; `normalizations` holds each
    /// one's calibration.
    pub fn compose(
        &mut self,
        recipe: &str,
        cadences: &[f64],
        layout: &[u32],
        samples: &[f32],
    ) -> Result<Vec<f32>, JsError> {
        let recipe: Recipe =
            serde_json::from_str(recipe).map_err(|e| JsError::new(&format!("recipe: {e}")))?;
        let clips = split(layout, samples);
        let cores: Vec<Core> = clips
            .iter()
            .zip(cadences.as_chunks::<2>().0)
            .map(|(c, &[shots, interval_s])| {
                let cadence = Cadence {
                    shots: shots as usize,
                    interval_s,
                };
                (&c[..], cadence)
            })
            .collect();
        let buffers = self.bank.recipe(&recipe, &cores).map_err(error)?;
        self.normalizations = buffers.iter().map(|b| b.normalization).collect();
        let mut out = Vec::new();
        for b in buffers {
            out.extend([b.channels.len() as f32, b.channels[0].len() as f32]);
            out.extend(b.channels.concat());
        }
        Ok(out)
    }

    /// The last composed alternatives' calibrations.
    pub fn normalizations(&self) -> Vec<f64> {
        self.normalizations.clone()
    }
}

/// Where sound listens for a camera at `eye` looking at `target` with
/// `up`, `share` of the way from the target to the eye: position, then facing.
#[wasm_bindgen]
pub fn camera_listener(eye: &[f64], target: &[f64], up: &[f64], share: f64) -> Vec<f64> {
    let v = |a: &[f64]| [a[0], a[1], a[2]];
    let l = Listener::of_camera(v(eye), v(target), v(up).map(|x| x as f32), share);
    [l.position, l.forward].concat()
}

/// One unseen look (a `presentation.fog` style, JSON) resolved into the
/// `FogLook` uniform's words (`fog.rs`); errs naming a field out of range.
/// The fog mask passes need it when the frame is built and whenever a lab
/// tunes the look, so this needs no `Presentation`.
#[wasm_bindgen]
pub fn fog_look(style: &str) -> Result<Vec<f32>, JsError> {
    let style: presentation::fog::FogStyle =
        serde_json::from_str(style).map_err(|e| JsError::new(&format!("fog style: {e}")))?;
    Ok(style.look().map_err(error)?.words())
}

/// Admit all authored presentation sections and select the fog style once.
#[wasm_bindgen]
pub fn check_presentation(text: &str) -> Result<String, JsError> {
    let value = serde_json::from_str(text).map_err(error)?;
    let settings =
        presentation::settings::PresentationSettings::from_value(&value).map_err(error)?;
    let [r, g, b] = settings.hud.accent;
    serde_json::to_string(&serde_json::json!({
        "fog_style": settings.fog.selected().map_err(error)?,
        "ally_xray": settings.ally_xray(),
        "ruler_reach": [r,g,b,1.0],
    }))
    .map_err(error)
}

/// Shared library admission, including workbench-only metadata and routing.
#[wasm_bindgen]
pub fn check_sound_catalog(text: &str) -> Result<(), JsError> {
    presentation::sound::catalog::SoundCatalog::from_value(
        &serde_json::from_str(text).map_err(error)?,
    )
    .map_err(error)?;
    Ok(())
}

#[wasm_bindgen]
pub fn check_audio_mix(text: &str, catalog: Option<String>) -> Result<(), JsError> {
    let mix = serde_json::from_str(text).map_err(error)?;
    match catalog {
        Some(catalog) => {
            presentation::sound::tables::SoundTables::from_parts(
                mix,
                serde_json::from_str(&catalog).map_err(error)?,
                &serde_json::json!({}),
            )
            .map_err(error)?;
        }
        None => {
            presentation::sound::tables::AudioMix::from_value(&mix).map_err(error)?;
        }
    }
    Ok(())
}
