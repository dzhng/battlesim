//! The battle's sound on `kira`. What is heard, when, where and how loud is
//! the presentation crate's (`presentation::sound`), the browser's own
//! decisions; so are the prepared sounds (the recordings in
//! `assets/runtime/audio/clips`, the synthesized baselines and each recipe
//! composed and calibrated from them). This player only carries out the
//! commands, as the browser's `WebAudioSink` does:
//!
//! - a voice is a sound on its own track under its bus, at its level times
//!   its buffer's calibration, low-passed by the track's filter, placed left
//!   or right by an equal-power pan (by direction from the listener, or the
//!   cue's sector), and sending its share into its bus's reverb;
//! - buses (units, effects, ambience, voices) sum under the master,
//!   through the presentation's limiter and a hard clip at its ceiling
//!   (`audio.limiter`); the ambience bus, and its reverb's tail, dip under
//!   a loud sound through a duck of their own, so its level stays as set;
//! - voice lines are the line arbitration's (`presentation::sound::voices`):
//!   its answers and the barks the sound heard, each a take of a voice pack
//!   ([`voices`]) on the voices bus;
//! - the score is `presentation::sound::music`'s: its tracks stream on the
//!   music bus under the master, crossfading, and dip under voice lines
//!   ([`music`]).
//!
//! The output is the default device, or a buffer rendered on demand
//! (`Output::Buffer`), which is how it is tested and recorded without
//! playing aloud.
mod mp3;
mod music;
mod voices;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kira::backend::cpal::{CpalBackend, CpalBackendSettings};
use kira::backend::{Backend, Renderer};
use kira::effect::compressor::CompressorBuilder;
use kira::effect::distortion::{DistortionBuilder, DistortionKind};
use kira::effect::filter::{FilterBuilder, FilterHandle, FilterMode};
use kira::effect::reverb::ReverbBuilder;
use kira::effect::volume_control::VolumeControlBuilder;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::PlaybackState;
use kira::track::{MainTrackBuilder, SendTrackBuilder, SendTrackHandle, TrackBuilder, TrackHandle};
use kira::{
    AudioManager, AudioManagerSettings, Capacities, Decibels, Frame, Mix, Panning, StartTime, Tween,
};
use presentation::presentation::Presentation;
use presentation::sound::bank::Bank;
use presentation::sound::frame::{Command, Ducked, Listener, Params, Sound, Voice};
use presentation::sound::music::Score;
use presentation::sound::tables::Bus;
use presentation::sound::voices::{Chatter, Request, Speakers};
use presentation::world::Vec3;
use serde_json::{json, Value};

/// The rate every recording is prepared at, and the bank composes at.
pub const SAMPLE_RATE: u32 = 48_000;
/// Seconds a live loop's parameters glide to a new value over.
const GLIDE_S: f64 = 0.06;
/// The seeds a battle's voice takes and score shuffle are drawn from: the
/// browser's lines use 0; its score draws a seed per battle, which a replay
/// has no source for.
const CHATTER_SEED: f64 = 0.0;
const SCORE_SEED: u32 = 0;
const BUSES: [Bus; 4] = [Bus::Units, Bus::Effects, Bus::Ambience, Bus::Voices];

fn tween(seconds: f64) -> Tween {
    Tween {
        duration: Duration::from_secs_f64(seconds.max(0.0)),
        ..Default::default()
    }
}

fn decibels(gain: f64) -> Decibels {
    Decibels((20.0 * gain.max(1e-9).log10()) as f32)
}

/// Where the mix goes: the default device, or a buffer filled up to each
/// player time the mix is told of, so what it schedules starts on time.
pub enum Output {
    Device(CpalBackend),
    Buffer {
        renderer: Box<Option<Renderer>>,
        sample_rate: u32,
        /// Interleaved stereo.
        samples: Vec<f32>,
    },
}

/// Which output to open.
#[derive(Default)]
pub enum OutputSettings {
    #[default]
    Device,
    /// Rendered on demand at this rate.
    Buffer(u32),
}

impl Backend for Output {
    type Settings = OutputSettings;
    type Error = String;

    fn setup(settings: Self::Settings, buffer: usize) -> Result<(Self, u32), String> {
        match settings {
            OutputSettings::Device => {
                let (b, rate) = CpalBackend::setup(CpalBackendSettings::default(), buffer)
                    .map_err(|e| e.to_string())?;
                Ok((Self::Device(b), rate))
            }
            OutputSettings::Buffer(sample_rate) => Ok((
                Self::Buffer {
                    renderer: Box::new(None),
                    sample_rate,
                    samples: Vec::new(),
                },
                sample_rate,
            )),
        }
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), String> {
        match self {
            Self::Device(b) => b.start(renderer).map_err(|e| e.to_string()),
            Self::Buffer { renderer: r, .. } => {
                **r = Some(renderer);
                Ok(())
            }
        }
    }
}

struct Playing {
    /// Dropping the track frees it once its sound has finished.
    _track: TrackHandle,
    sound: StaticSoundHandle,
    filter: FilterHandle,
    bus: usize,
    /// Its buffer's calibration, and the equal-power pan's level.
    scale: f64,
    position: Option<Vec3>,
}

/// Counts of what the player has done, for its checks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Played {
    pub started: usize,
    pub stopped: usize,
    /// Voices the mixer had no room for (none expected: the sound's budget
    /// is far inside its capacity).
    pub refused: usize,
    /// Voices sounding now, by the player's own count.
    pub live: usize,
    /// Voice lines started, and music tracks.
    pub lines: usize,
    pub tracks: usize,
}

pub struct Audio {
    manager: AudioManager<Output>,
    sound: Sound,
    /// Each prepared sound's alternatives, and each one's calibration.
    sounds: HashMap<String, Vec<(StaticSoundData, f64)>>,
    buses: Vec<TrackHandle>,
    reverbs: Vec<SendTrackHandle>,
    /// The ambience bus's parent, which dips under a loud sound, and the
    /// bus's level, which its reverb's tail dips from.
    ambience_duck: TrackHandle,
    ambience_level: f64,
    voices: HashMap<u32, Playing>,
    listener: Listener,
    played: Played,
    chatter: Chatter,
    score: Score,
    packs: voices::Packs,
    music: music::Music,
    /// What could not be played, by name (a missing pack, take or track).
    errors: Vec<String>,
}

/// The game's sound setup, as the browser builds it (`soundFeed.ts`):
/// `audio.json`, the sound catalog, the weapon rows, the tick rate,
/// and the effects' smoke and cook-off.
pub fn setup(
    rules: &Value,
    presentation: &Value,
    audio: &Value,
    catalog: &Value,
) -> Result<String, String> {
    let settings = presentation::settings::PresentationSettings::from_value(presentation)?;
    let fx = settings.effects;
    Ok(json!({
        "audio": audio,
        "catalog": catalog,
        "weapons": rules["weapons"],
        "tick_hz": rules["tick_hz"],
        "effects": { "smoke": fx.smoke, "cook_off": fx.cook_off },
    })
    .to_string())
}

/// A recording's samples, from `assets/runtime` (`url` is where the browser
/// fetches it from): the asset tool's mono 16-bit PCM WAV at
/// [`SAMPLE_RATE`]; anything else, or a pointer Git LFS has not fetched, is
/// refused by name.
fn recording(runtime: &Path, url: &str) -> Result<Vec<Vec<f32>>, String> {
    let path = runtime.join(url.trim_start_matches('/'));
    let fail = |why: &str| format!("{}: {why}", path.display());
    let bytes = std::fs::read(&path).map_err(|e| fail(&e.to_string()))?;
    if !bytes.starts_with(b"RIFF") {
        return Err(fail(
            "not a WAV (git lfs pull --include=\"assets/runtime/audio/clips/**\")",
        ));
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let mut at = 12;
    let mut format = None;
    while at + 8 <= bytes.len() {
        let (id, size) = (&bytes[at..at + 4], u32_at(at + 4) as usize);
        let body = at + 8;
        if id == b"fmt " {
            format = Some((
                u16_at(body),
                u16_at(body + 2),
                u32_at(body + 4),
                u16_at(body + 14),
            ));
        } else if id == b"data" {
            if format != Some((1, 1, SAMPLE_RATE, 16)) {
                return Err(fail(&format!(
                    "format {format:?}, not mono 16-bit PCM at {SAMPLE_RATE} Hz"
                )));
            }
            let data = bytes
                .get(body..body + size)
                .ok_or_else(|| fail("truncated"))?;
            let samples = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| f32::from(i16::from_le_bytes(*b)) / 32768.0)
                .collect();
            return Ok(vec![samples]);
        }
        at = body + size + size % 2;
    }
    Err(fail("no data chunk"))
}

impl Audio {
    /// The battle's sound from its `setup` (see [`setup`]), its recordings
    /// read from `runtime` (`assets/runtime`), on `output`.
    pub fn new(setup: &str, runtime: &Path, output: OutputSettings) -> Result<Self, String> {
        let mut sound = Sound::from_json(setup)?;
        let tables = sound.tables().clone();
        let mut bank = Bank::new(f64::from(SAMPLE_RATE));
        bank.prepare(&tables, |id| {
            let clip = &tables.catalog.clips[id];
            recording(runtime, &clip.url)
        })?;
        sound.set_durations(bank.durations())?;
        let sounds = bank
            .sounds
            .into_iter()
            .map(|(name, buffers)| {
                let alternatives = buffers
                    .into_iter()
                    .map(|b| {
                        let (l, r) = (&b.channels[0], b.channels.last().expect("a channel"));
                        let frames: Arc<[Frame]> =
                            l.iter().zip(r).map(|(l, r)| Frame::new(*l, *r)).collect();
                        let data = StaticSoundData {
                            sample_rate: SAMPLE_RATE,
                            frames,
                            settings: StaticSoundSettings::default(),
                            slice: None,
                        };
                        (data, b.normalization)
                    })
                    .collect();
                (name, alternatives)
            })
            .collect();

        let audio = &tables.audio;
        // The master's level, then the presentation's limiter (kira's
        // compressor is hard-knee, as the browser's is set) and a hard clip at
        // its ceiling for what the limiter's attack lets over, in the
        // browser's order: kira scales a track by its volume after its
        // effects, so the master's level is an effect ahead of them. Kira's
        // hard clip clamps at 1 after its drive, so a drive of -clip_db
        // clamps at clip_db.
        let l = audio.limiter;
        let limiter = CompressorBuilder::new()
            .threshold(l.threshold_db)
            .ratio(l.ratio)
            .attack_duration(Duration::from_secs_f64(l.attack_s))
            .release_duration(Duration::from_secs_f64(l.release_s));
        let clip = DistortionBuilder::new()
            .kind(DistortionKind::HardClip)
            .drive(Decibels(-l.clip_db as f32))
            .mix(Mix::WET);
        let mut manager = AudioManager::<Output>::new(AudioManagerSettings {
            capacities: Capacities {
                sub_track_capacity: 512,
                ..Default::default()
            },
            main_track_builder: MainTrackBuilder::new()
                .with_effect(VolumeControlBuilder::new(decibels(audio.buses.master)))
                .with_effect(limiter)
                .with_effect(clip),
            backend_settings: output,
            ..Default::default()
        })
        .map_err(|e| format!("audio output: {e}"))?;
        let mut buses = Vec::new();
        let mut reverbs = Vec::new();
        let mut ambience_duck = manager
            .add_sub_track(TrackBuilder::new())
            .map_err(|e| e.to_string())?;
        for bus in BUSES {
            let gain = match bus {
                Bus::Units => audio.buses.units,
                Bus::Effects => audio.buses.effects,
                Bus::Ambience => audio.buses.ambience,
                Bus::Voices => audio.buses.voices,
            };
            // The distance reverb's tail joins its bus, so it takes the
            // bus's level; send tracks sum straight into the master.
            reverbs.push(
                manager
                    .add_send_track(
                        SendTrackBuilder::new().volume(decibels(gain)).with_effect(
                            ReverbBuilder::new()
                                .feedback(0.85)
                                .damping(0.6)
                                .mix(Mix::WET),
                        ),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let track = TrackBuilder::new()
                .volume(decibels(gain))
                .sub_track_capacity(256);
            buses.push(
                if bus == Bus::Ambience {
                    ambience_duck.add_sub_track(track)
                } else {
                    manager.add_sub_track(track)
                }
                .map_err(|e| e.to_string())?,
            );
        }
        let catalog: Value = serde_json::from_str(setup).map_err(|e| format!("sound: {e}"))?;
        let music = music::Music::new(
            &mut manager,
            audio.buses.music,
            &catalog["catalog"],
            runtime,
        )?;
        Ok(Self {
            manager,
            chatter: Chatter::new(audio.voices, CHATTER_SEED),
            score: Score::new(
                audio.music.clone(),
                tables.catalog.music.moods.battle(),
                SCORE_SEED,
            ),
            packs: voices::Packs::new(runtime),
            music,
            sound,
            sounds,
            buses,
            reverbs,
            ambience_duck,
            ambience_level: audio.buses.ambience,
            voices: HashMap::new(),
            listener: Listener {
                position: [0.0; 3],
                forward: [0.0, 1.0, 0.0],
            },
            played: Played::default(),
            errors: Vec::new(),
        })
    }

    pub fn played(&self) -> Played {
        Played {
            live: self.voices.len(),
            tracks: self.music.started,
            ..self.played
        }
    }

    /// What could not be played, by name.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Which voice set each unit kind speaks with, for the side's own
    /// faction and its enemy's: every set named loads its pack, and a shout
    /// heard in the open is a bark in its kind's set.
    pub fn speakers(&mut self, speakers: Speakers) {
        let sets: BTreeSet<&String> = speakers
            .own
            .values()
            .chain(speakers.enemy.values())
            .collect();
        for set in sets {
            if let Err(e) = self.packs.load(set, &mut self.chatter) {
                self.errors.push(e);
            }
        }
        self.sound.set_speakers(speakers);
    }

    /// A unit's answer to its owner, asked at player time `now`; the next
    /// [`Audio::speak`] decides whether it plays.
    pub fn respond(&mut self, request: Request, now: f64) {
        self.chatter.request(request, now);
    }

    /// The sound's own counts (`presentation::sound::frame::Stats`).
    pub fn stats(&self) -> presentation::sound::frame::Stats {
        self.sound.stats()
    }

    /// Hears `presentation`'s last publication, at player time `now` (seconds).
    pub fn note(&mut self, presentation: &Presentation, now: f64) {
        if let Some(heard) = presentation.heard() {
            let commands = self.sound.note(&heard, now).to_vec();
            self.run(&commands, now);
        }
    }

    /// Hears `presentation`'s last frame, from its camera, at wall time
    /// `wall` and player time `now` (seconds).
    pub fn hear(&mut self, presentation: &Presentation, wall: f64, now: f64) {
        let share = self.sound.tables().audio.listener_eye_share;
        let Some(listener) = presentation.listener(share) else {
            return;
        };
        let motion = presentation.motion();
        let clock = presentation.clock_of_frame();
        let commands = self
            .sound
            .update(clock, wall, now, &motion, listener)
            .to_vec();
        self.run(&commands, now);
        self.speak(now);
    }

    /// The voice lines and the score at player time `now`, once the sound
    /// has heard the frame: the lines arbitrate the answers asked and the
    /// barks the sound heard, and the score follows those barks and dips
    /// under any line.
    pub fn speak(&mut self, now: f64) {
        let lines = self.chatter.hear(&mut self.sound, now);
        let (commands, heard) = (lines.commands.clone(), lines.heard.clone());
        self.run(&commands, now);
        let speaking = self.chatter.speaking(now);
        let music = self.score.hear(now, &heard, speaking).to_vec();
        self.run(&music, now);
    }

    /// The sound the player carries out, for a script to drive.
    pub fn sound_mut(&mut self) -> &mut Sound {
        &mut self.sound
    }

    /// Carries out `commands` at player time `now`.
    pub fn run(&mut self, commands: &[Command], now: f64) {
        self.render(now);
        for c in commands {
            match c {
                Command::Start { id, voice } => self.start(*id, voice, now),
                Command::Set { id, params, .. } => self.set(*id, params),
                Command::Stop { id, fade, .. } => {
                    if let Some(mut v) = self.voices.remove(id) {
                        v.sound.stop(tween(*fade));
                        self.played.stopped += 1;
                    }
                }
                Command::Listen { listener, .. } => self.listen(*listener),
                Command::Music { track, fade, .. } => {
                    if let Err(e) = self.music.play(track, *fade) {
                        self.errors.push(e);
                    }
                }
                Command::MusicStop { fade, .. } => self.music.stop(*fade),
                Command::Duck {
                    bus: Ducked::Music,
                    depth,
                    glide,
                    ..
                } => self.music.duck(*depth, *glide),
                Command::Duck {
                    bus: Ducked::Ambience,
                    depth,
                    glide,
                    ..
                } => self.duck_ambience(*depth, *glide),
            }
        }
        // A transient that has played out frees its voice.
        self.voices
            .retain(|_, v| v.sound.state() != PlaybackState::Stopped);
    }

    /// Dips the ambience bus and its reverb's tail by `depth` (0 restores
    /// them), gliding over `glide` seconds, as the music's duck does.
    fn duck_ambience(&mut self, depth: f64, glide: f64) {
        let level = 1.0 - depth.clamp(0.0, 1.0);
        self.ambience_duck.set_volume(decibels(level), tween(glide));
        let bus = BUSES
            .iter()
            .position(|b| *b == Bus::Ambience)
            .expect("a bus");
        self.reverbs[bus].set_volume(decibels(self.ambience_level * level), tween(glide));
    }

    /// The equal-power pan of a sound at `position` (the Web Audio panner's,
    /// by direction only), or of a cue at `pan`: kira's pan, and the level
    /// that matches the browser's (kira's law peaks at √2 a side).
    fn panning(&self, position: Option<&Vec3>, pan: Option<f64>) -> (Panning, f64) {
        let x = match (position, pan) {
            (Some(p), _) => {
                let l = &self.listener;
                let d = [p[0] - l.position[0], p[1] - l.position[1]];
                let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
                let right = [l.forward[1], -l.forward[0]];
                let side = if len > 0.0 {
                    ((d[0] * right[0] + d[1] * right[1]) / len).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                // Azimuth in [-90°, 90°] (behind folds forward), as 0..1.
                0.5 + side.asin() / std::f64::consts::PI
            }
            (None, Some(pan)) => (pan + 1.0) / 2.0,
            (None, None) => return (Panning::CENTER, 1.0),
        };
        // The browser's gains are cos and sin of x·90°; kira's are √2·√(1−m)
        // and √2·√m, so m = sin²(x·90°) at a level of 1/√2.
        let m = (x * std::f64::consts::FRAC_PI_2).sin().powi(2);
        (
            Panning((2.0 * m - 1.0) as f32),
            std::f64::consts::FRAC_1_SQRT_2,
        )
    }

    fn start(&mut self, id: u32, v: &Voice, now: f64) {
        // A prepared sound, or else a voice line's take.
        let (data, normalization) = match self.sounds.get(&v.sound) {
            Some(alternatives) => alternatives[v.variant as usize % alternatives.len()].clone(),
            None => match self.packs.take(&v.sound, v.variant) {
                Some(Ok(take)) => {
                    self.played.lines += 1;
                    take
                }
                Some(Err(e)) => return self.errors.push(e),
                None => return self.errors.push(format!("sound: no {}", v.sound)),
            },
        };
        let data = &data;
        let bus = BUSES.iter().position(|b| *b == v.bus).expect("a bus");
        let (panning, level) = self.panning(v.position.as_ref(), v.pan);
        let (filter, filter_handle) = {
            let mut b = TrackBuilder::new()
                .with_send(self.reverbs[bus].id(), decibels(v.wet))
                .persist_until_sounds_finish(true);
            let h = b.add_effect(
                FilterBuilder::new()
                    .mode(FilterMode::LowPass)
                    .cutoff(v.lowpass)
                    .resonance(0.0),
            );
            (b, h)
        };
        let Ok(mut track) = self.buses[bus].add_sub_track(filter) else {
            self.played.refused += 1;
            return;
        };
        let scale = normalization * level;
        let mut settings = StaticSoundSettings::new()
            .volume(decibels(v.gain * scale))
            .playback_rate(v.rate)
            .panning(panning)
            .start_time(StartTime::Delayed(Duration::from_secs_f64(
                (v.at - now).max(0.0),
            )));
        if v.attack > 0.0 {
            settings = settings.fade_in_tween(tween(v.attack));
        }
        if v.looping {
            // Repeated loops start at staggered phases, as the browser's do.
            let phase = (f64::from(id) * 0.618_033_988_7) % 1.0;
            settings = settings
                .loop_region(..)
                .start_position(phase * data.duration().as_secs_f64());
        }
        let Ok(sound) = track.play(data.with_settings(settings)) else {
            self.played.refused += 1;
            return;
        };
        self.played.started += 1;
        self.voices.insert(
            id,
            Playing {
                _track: track,
                sound,
                filter: filter_handle,
                bus,
                scale: normalization * level,
                position: v.position,
            },
        );
    }

    fn set(&mut self, id: u32, p: &Params) {
        let Some(v) = self.voices.get(&id) else {
            return;
        };
        let (panning, level) = self.panning(p.position.as_ref(), None);
        let normalization = v.scale
            / if v.position.is_some() {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.0
            };
        let reverb = self.reverbs[v.bus].id();
        let v = self.voices.get_mut(&id).expect("looked up");
        let t = tween(GLIDE_S);
        v.scale = normalization * level;
        v.position = p.position;
        v.sound.set_volume(decibels(p.gain * v.scale), t);
        v.sound.set_playback_rate(p.rate, t);
        v.sound.set_panning(panning, t);
        v.filter.set_cutoff(p.lowpass, t);
        let _ = v._track.set_send(reverb, decibels(p.wet), t);
    }

    /// Places every positional voice for a listener that moved.
    fn listen(&mut self, listener: Listener) {
        if listener == self.listener {
            return;
        }
        self.listener = listener;
        let pans: Vec<(u32, Panning)> = self
            .voices
            .iter()
            .filter_map(|(id, v)| Some((*id, self.panning(Some(v.position.as_ref()?), None).0)))
            .collect();
        for (id, panning) in pans {
            if let Some(v) = self.voices.get_mut(&id) {
                v.sound.set_panning(panning, tween(GLIDE_S));
            }
        }
    }

    /// Fills the buffer output up to player time `now` (seconds); a device
    /// plays on its own clock.
    pub fn render(&mut self, now: f64) {
        let Output::Buffer {
            renderer,
            sample_rate,
            samples,
        } = self.manager.backend_mut()
        else {
            return;
        };
        let Some(renderer) = renderer.as_mut() else {
            return;
        };
        let rate = f64::from(*sample_rate);
        let frames = (now * rate).floor() as usize;
        let at = samples.len();
        if frames * 2 <= at {
            return;
        }
        samples.resize(frames * 2, 0.0);
        renderer.on_start_processing();
        renderer.process(&mut samples[at..], 2);
    }

    /// What the buffer output has rendered: interleaved stereo at
    /// [`SAMPLE_RATE`]; none for a device.
    pub fn samples(&mut self) -> &[f32] {
        match self.manager.backend_mut() {
            Output::Buffer { samples, .. } => samples,
            Output::Device(_) => &[],
        }
    }
}
