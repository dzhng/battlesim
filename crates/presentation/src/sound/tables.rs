//! What the battle's sound reads: `presentation.audio` (every level, curve,
//! budget and per-kind sound table, each table with a `default` row) and the
//! sound catalog (`fixtures/sounds.json`: the recipes, the clips' burst
//! cadences, the firing and contact choices and the effect replacements),
//! with each weapon row heard as its nearest styled ancestor where it has no
//! row of its own. Sound names are recipes; a synthesized baseline is a
//! recipe of the same name as its synth sound.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Deserialize;
use serde_json::Value;

use super::music::{Moods, MusicRules};
use super::synth;
use super::voices::ChatterRules;

/// The mix buses under the master.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Bus {
    Units,
    Effects,
    Ambience,
    Voices,
}

/// A gunshot or blast: a near and a far sound, switched at `far_m`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NearFar {
    pub near: String,
    pub far: String,
    pub gain: f64,
    pub far_m: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneShot {
    pub sound: String,
    pub gain: f64,
}

/// A vehicle class's loops: engine (idle to load), running gear (by speed),
/// turret traverse and reverse beeper. A null sound is none.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleSound {
    pub engine: String,
    pub idle_rate: f64,
    pub load_rate: f64,
    pub idle_gain: f64,
    pub load_gain: f64,
    pub running: Option<String>,
    pub running_gain: f64,
    /// The speed at which load and running gear are full, m/s.
    pub full_speed_mps: f64,
    pub turret: Option<String>,
    pub turret_gain: f64,
    /// The traverse rate at which the turret's whine is full, rad/s.
    pub full_traverse_rps: f64,
    pub reverse: Option<String>,
    pub reverse_gain: f64,
}

/// A burning source's loop: full while it burns, `smoulder_gain` after.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireSound {
    pub sound: String,
    pub gain: f64,
    pub smoulder_gain: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buses {
    pub master: f64,
    pub units: f64,
    pub effects: f64,
    pub ambience: f64,
    /// The music bus's level: streamed tracks, outside the voice budget.
    pub music: f64,
    pub voices: f64,
}

/// Full within `ref_m`, then inverse (`rolloff`) down toward `floor`, a
/// share never dropped below within `max_m`, and silent past it. A voice
/// quieter than `cull` at the listener is not started.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distance {
    pub ref_m: f64,
    pub rolloff: f64,
    pub max_m: f64,
    pub floor: f64,
    pub cull: f64,
}

/// The colour of distance, from nothing at the listener to full at
/// `far_m`: a low-pass from `near_hz` to `far_hz` (log-spaced), a reverb
/// send up to `wet` (a tail `reverb_s` long) and onsets ramped over up to
/// `attack_s`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Air {
    pub near_hz: f64,
    pub far_hz: f64,
    pub far_m: f64,
    pub wet: f64,
    pub reverb_s: f64,
    pub attack_s: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub voices: usize,
    pub loops: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CookOffLanding {
    pub hit: String,
    #[serde(rename = "as")]
    pub round: String,
    pub gain: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CookOffSound {
    pub blast: NearFar,
    pub landing: CookOffLanding,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Footsteps {
    pub sound: String,
    pub gain: f64,
    pub stride_m: f64,
    pub max_step_m: f64,
}

/// Unseen enemies, from hearing cues: a sound by `category` or
/// `category_moving` (a category with neither is silent), a shout from the
/// enemy's pool, its gain by band, the far band muffled, and the same cue not
/// repeated within `repeat_s`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cues {
    pub sounds: HashMap<String, String>,
    pub shout: ShoutCue,
    pub near_gain: f64,
    pub far_gain: f64,
    pub far_lowpass_hz: f64,
    pub repeat_s: f64,
}

/// An unseen enemy's shout: a stressed bark from the generic pool of the
/// listener's enemy faction (never the speaker's own voice), on the voices
/// bus at `gain` times its band's, through a `lowpass_hz` low-pass and a
/// `wet` reverb send in either band, so the words can't be made out. The
/// player loads the pool: up to `takes_per_voice` stressed barks of each
/// shouting voice of every set the enemy fields.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShoutCue {
    pub gain: f64,
    pub lowpass_hz: f64,
    pub wet: f64,
    pub takes_per_voice: usize,
}

/// `presentation.audio`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioPresentation {
    pub buses: Buses,
    /// The listener stands this share of the way from the camera's target
    /// to its eye.
    pub listener_eye_share: f64,
    pub distance: Distance,
    pub air: Air,
    pub budget: Budget,
    /// The clock standing still this long (wall seconds) is a hold.
    pub hold_s: f64,
    /// A transient more than this late (a clock jump) is dropped.
    pub late_s: f64,
    pub shots: HashMap<String, NearFar>,
    pub impacts: HashMap<String, OneShot>,
    pub impact_scale: HashMap<String, f64>,
    pub ricochet: OneShot,
    pub blasts: HashMap<String, NearFar>,
    pub cook_off: CookOffSound,
    /// A flying round's motor by kind; a kind without a row has none.
    pub motors: HashMap<String, OneShot>,
    pub vehicles: HashMap<String, VehicleSound>,
    pub footsteps: Footsteps,
    pub fires: HashMap<String, FireSound>,
    /// Battle music's mood direction and ducking ([`super::music`]).
    pub music: MusicRules,
    /// How unit voice lines are chosen ([`super::voices`]).
    pub voices: ChatterRules,
    pub ambience: OneShot,
    /// The one alarm raised when a side's first fixed-wing aircraft enters.
    pub air_alarm: OneShot,
    /// A positional warning for a visible missile steering onto an own unit.
    pub missile_warning: OneShot,
    /// A positional burst when an aircraft releases a flare.
    pub flare_deploy: OneShot,
    pub cues: Cues,
}

/// A recorded burst: these source shots, re-laid `interval_s` apart.
#[derive(Clone, Debug, Deserialize)]
pub struct Burst {
    pub shots: Vec<[u64; 2]>,
    pub interval_s: f64,
}

/// A prepared clip, as far as the sound reads it: where it is served
/// (`/audio/clips/<id>.wav`), and its burst. The catalog's own validator
/// (`catalog.ts`) owns the rest.
#[derive(Clone, Debug, Deserialize)]
pub struct Clip {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub burst: Option<Burst>,
}

/// A recipe: alternative recordings, each with optional synthesized support.
#[derive(Clone, Debug, Deserialize)]
pub struct Recipe {
    pub clips: Vec<String>,
    pub synth: Option<String>,
    pub synth_gain: f64,
    pub gain: f64,
    #[serde(rename = "loop")]
    pub looping: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShotChoice {
    pub near: String,
    pub far: String,
    pub gain: f64,
}

/// `fixtures/sounds.json`, as far as a battle plays it (provenance is the
/// workbench's).
#[derive(Clone, Debug, Deserialize)]
pub struct SoundCatalog {
    pub clips: HashMap<String, Clip>,
    pub sounds: BTreeMap<String, Recipe>,
    /// Firing choices by weapon row; `default` for a row without one.
    pub defaults: HashMap<String, ShotChoice>,
    /// Contact sounds by contact (a hit material or a glance), then round kind.
    pub impacts: HashMap<String, HashMap<String, String>>,
    /// Replacements for baseline effect and loop slots.
    pub effects: HashMap<String, String>,
    /// The score's tracks by mood; which file a track is, is the player's.
    pub music: MusicCatalog,
}

/// The catalog's `music` section, as far as a battle decides it: each battle
/// mood's track ids (the menu's is the app's, and each track's file the
/// player's).
#[derive(Clone, Debug, Deserialize)]
pub struct MusicCatalog {
    pub moods: Moods,
}

/// The rounds one firing voice covers, `interval_s` apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cadence {
    pub shots: usize,
    pub interval_s: f64,
}

impl Clip {
    /// Its burst's shots, or one.
    pub fn cadence(&self) -> Cadence {
        self.burst.as_ref().map_or(
            Cadence {
                shots: 1,
                interval_s: 0.0,
            },
            |b| Cadence {
                shots: b.shots.len(),
                interval_s: b.interval_s,
            },
        )
    }
}

/// `table`'s row for `key`, or its `default`.
pub fn pick<'a, T>(table: &'a HashMap<String, T>, key: &str) -> &'a T {
    table
        .get(key)
        .or_else(|| table.get("default"))
        .expect("tables are checked for a default")
}

/// `table` with a row for every weapon of `weapons` that has none: its
/// nearest ancestor's (`extends`), so a derived weapon sounds as its parent.
pub(crate) fn inherit<T: Clone>(
    table: &mut HashMap<String, T>,
    weapons: &BTreeMap<String, Option<String>>,
) {
    let mut added = Vec::new();
    for kind in weapons.keys() {
        if table.contains_key(kind) {
            continue;
        }
        let mut at = Some(kind.as_str());
        let mut seen = BTreeSet::new();
        while let Some(a) = at.filter(|a| !table.contains_key(*a) && seen.insert(*a)) {
            at = weapons.get(a).and_then(|p| p.as_deref());
        }
        if let Some(row) = at.and_then(|a| table.get(a)) {
            added.push((kind.clone(), row.clone()));
        }
    }
    table.extend(added);
}

/// Everything the battle's sound reads.
#[derive(Clone, Debug)]
pub struct SoundTables {
    pub audio: AudioPresentation,
    pub catalog: SoundCatalog,
}

impl SoundTables {
    /// From `presentation.audio`, the sound catalog and the game's weapon
    /// rows (for `extends`); errs on a table without its `default`, an
    /// unknown sound, or a budget that leaves no transients.
    pub fn from_parts(audio: Value, catalog: Value, weapons: &Value) -> Result<Self, String> {
        let audio: AudioPresentation =
            serde_json::from_value(audio).map_err(|e| format!("presentation.audio: {e}"))?;
        let catalog: SoundCatalog =
            serde_json::from_value(catalog).map_err(|e| format!("sound catalog: {e}"))?;
        let weapons: BTreeMap<String, Option<String>> = weapons
            .as_object()
            .ok_or("weapons: expected an object")?
            .iter()
            .map(|(k, w)| (k.clone(), w["extends"].as_str().map(str::to_owned)))
            .collect();
        let (mut audio, mut catalog) = (audio, catalog);
        inherit(&mut audio.shots, &weapons);
        inherit(&mut audio.impact_scale, &weapons);
        inherit(&mut audio.blasts, &weapons);
        inherit(&mut audio.motors, &weapons);
        inherit(&mut catalog.defaults, &weapons);
        for row in catalog.impacts.values_mut() {
            inherit(row, &weapons);
        }
        let tables = Self { audio, catalog };
        tables.check()?;
        Ok(tables)
    }

    fn check(&self) -> Result<(), String> {
        let a = &self.audio;
        let defaults = [
            ("shots", a.shots.contains_key("default")),
            ("impacts", a.impacts.contains_key("default")),
            ("impact_scale", a.impact_scale.contains_key("default")),
            ("blasts", a.blasts.contains_key("default")),
            ("vehicles", a.vehicles.contains_key("default")),
            ("fires", a.fires.contains_key("default")),
        ];
        if let Some((t, _)) = defaults.iter().find(|(_, ok)| !ok) {
            return Err(format!("presentation.audio.{t} needs a default"));
        }
        if !(a.budget.voices > a.budget.loops) {
            return Err("presentation.audio.budget: voices must exceed loops".into());
        }
        a.voices.check()?;
        let d = a.distance;
        if !((0.0..=1.0).contains(&d.floor) && d.cull >= 0.0) {
            return Err(
                "presentation.audio.distance: floor must be 0 to 1, cull at least 0".into(),
            );
        }
        let shout = &a.cues.shout;
        if !(shout.gain >= 0.0 && shout.lowpass_hz > 0.0 && (0.0..=1.0).contains(&shout.wet)) {
            return Err(
                "presentation.audio.cues.shout: gain at least 0, lowpass_hz > 0, wet 0 to 1".into(),
            );
        }
        let air = a.air;
        if !(air.near_hz >= air.far_hz && air.far_hz > 0.0 && air.far_m > 0.0) {
            return Err("presentation.audio.air: near_hz ≥ far_hz > 0 and far_m > 0".into());
        }
        if !((0.0..=1.0).contains(&air.wet) && air.reverb_s > 0.0 && air.attack_s >= 0.0) {
            return Err("presentation.audio.air: wet 0 to 1, reverb_s > 0, attack_s ≥ 0".into());
        }
        for (name, r) in &self.catalog.sounds {
            if let Some(c) = r
                .clips
                .iter()
                .find(|c| !self.catalog.clips.contains_key(*c))
            {
                return Err(format!("sound {name}: unknown clip {c}"));
            }
            if let Some(s) = r.synth.as_deref().filter(|s| !synth::exists(s)) {
                return Err(format!("sound {name}: unknown synth {s}"));
            }
        }
        Ok(())
    }

    /// Every recipe a battle can play: the synthesized baselines, and each
    /// assigned or replacing recipe. The rest of the library is the
    /// workbench's, prepared only when auditioned.
    pub fn battle_sounds(&self) -> Vec<String> {
        let c = &self.catalog;
        let mut names: BTreeSet<&str> = c
            .sounds
            .keys()
            .map(String::as_str)
            .filter(|n| synth::exists(n))
            .collect();
        for choice in c.defaults.values() {
            names.extend([choice.near.as_str(), choice.far.as_str()]);
        }
        for row in c.impacts.values() {
            names.extend(row.values().map(String::as_str));
        }
        names.extend(c.effects.values().map(String::as_str));
        names.extend(self.audio.vehicles.values().map(|row| row.engine.as_str()));
        names.extend(
            self.audio
                .vehicles
                .values()
                .filter_map(|row| row.running.as_deref()),
        );
        names.extend(
            self.audio
                .vehicles
                .values()
                .filter_map(|row| row.turret.as_deref()),
        );
        names.extend(
            self.audio
                .vehicles
                .values()
                .filter_map(|row| row.reverse.as_deref()),
        );
        names.insert(self.audio.ambience.sound.as_str());
        names.insert(self.audio.air_alarm.sound.as_str());
        names.insert(self.audio.missile_warning.sound.as_str());
        names.insert(self.audio.flare_deploy.sound.as_str());
        names.into_iter().map(str::to_owned).collect()
    }

    /// An implicit baseline slot's sound: its replacement, else itself.
    pub fn effect<'a>(&'a self, baseline: &'a str) -> &'a str {
        self.catalog
            .effects
            .get(baseline)
            .map_or(baseline, String::as_str)
    }

    /// How a round of weapon row `kind` fires: its chosen recording, else the
    /// `default` choice, else the synthesized `base` (with replacements).
    pub fn shot(&self, kind: &str) -> NearFar {
        let base = pick(&self.audio.shots, kind);
        let choice = self
            .catalog
            .defaults
            .get(kind)
            .or_else(|| self.catalog.defaults.get("default"));
        match choice {
            Some(c) => NearFar {
                near: c.near.clone(),
                far: c.far.clone(),
                gain: base.gain * c.gain,
                far_m: base.far_m,
            },
            None => NearFar {
                near: self.effect(&base.near).to_owned(),
                far: self.effect(&base.far).to_owned(),
                ..base.clone()
            },
        }
    }

    /// A round's sound on a contact (a hit material, or a glance): the
    /// round's choice, the contact's default, or the baseline's replacement.
    pub fn contact<'a>(&'a self, contact: &str, kind: &str, baseline: &'a str) -> &'a str {
        let row = self.catalog.impacts.get(contact);
        row.and_then(|r| r.get(kind).or_else(|| r.get("default")))
            .map_or_else(|| self.effect(baseline), String::as_str)
    }

    /// A firing sound's cadence: its first recording's burst, or one round.
    pub fn cadence(&self, sound: &str) -> Cadence {
        self.catalog
            .sounds
            .get(sound)
            .and_then(|r| r.clips.first())
            .and_then(|c| self.catalog.clips.get(c))
            .map_or(
                Cadence {
                    shots: 1,
                    interval_s: 0.0,
                },
                Clip::cadence,
            )
    }
}
