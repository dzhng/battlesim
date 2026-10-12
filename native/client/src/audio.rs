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
//!   through a limiter.
//!
//! The output is the default device, or a buffer rendered on demand
//! (`Output::Buffer`), which is how it is tested and recorded without
//! playing aloud.
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kira::backend::cpal::{CpalBackend, CpalBackendSettings};
use kira::backend::{Backend, Renderer};
use kira::effect::compressor::CompressorBuilder;
use kira::effect::filter::{FilterBuilder, FilterHandle, FilterMode};
use kira::effect::reverb::ReverbBuilder;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::PlaybackState;
use kira::track::{MainTrackBuilder, SendTrackBuilder, SendTrackHandle, TrackBuilder, TrackHandle};
use kira::{
    AudioManager, AudioManagerSettings, Capacities, Decibels, Frame, Mix, Panning, StartTime,
    Tween,
};
use presentation::presentation::Presentation;
use presentation::sound::bank::Bank;
use presentation::sound::frame::{Command, Listener, Params, Sound, Voice};
use presentation::sound::tables::Bus;
use presentation::world::Vec3;
use serde_json::{json, Value};

/// The rate every recording is prepared at, and the bank composes at.
pub const SAMPLE_RATE: u32 = 48_000;
/// Seconds a live loop's parameters glide to a new value over.
const GLIDE_S: f64 = 0.06;
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

/// Where the mix goes: the default device, or a buffer `render` fills.
pub enum Output {
    Device(CpalBackend),
    Buffer {
        renderer: Option<Renderer>,
        sample_rate: u32,
    },
}

/// Which output to open.
pub enum OutputSettings {
    Device,
    /// Rendered on demand at this rate.
    Buffer(u32),
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self::Device
    }
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
                    renderer: None,
                    sample_rate,
                },
                sample_rate,
            )),
        }
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), String> {
        match self {
            Self::Device(b) => b.start(renderer).map_err(|e| e.to_string()),
            Self::Buffer { renderer: r, .. } => {
                *r = Some(renderer);
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
    /// Voices sounding now, by the player's own count.
    pub live: usize,
}

pub struct Audio {
    manager: AudioManager<Output>,
    sound: Sound,
    /// Each prepared sound's alternatives, and each one's calibration.
    sounds: HashMap<String, Vec<(StaticSoundData, f64)>>,
    buses: Vec<TrackHandle>,
    reverbs: Vec<SendTrackHandle>,
    voices: HashMap<u32, Playing>,
    listener: Listener,
    played: Played,
    /// Seconds the buffer output has rendered.
    rendered: f64,
}

/// The game's sound setup, as the browser builds it (`soundFeed.ts`):
/// `presentation.audio`, the sound catalog, the weapon rows, the tick rate,
/// and the effects' smoke and cook-off.
pub fn setup(game: &Value, catalog: &Value) -> String {
    let fx = &game["presentation"]["effects"];
    json!({
        "audio": game["presentation"]["audio"],
        "catalog": catalog,
        "weapons": game["weapons"],
        "tick_hz": game["tick_hz"],
        "effects": { "smoke": fx["smoke"], "cook_off": fx["cook_off"] },
    })
    .to_string()
}

/// A recording's samples, from `assets/runtime` (`url` is where the
/// browser fetches it from); a pointer Git LFS has not fetched is named.
fn recording(runtime: &Path, url: &str) -> Result<Vec<Vec<f32>>, String> {
    let path = runtime.join(url.trim_start_matches('/'));
    let data = StaticSoundData::from_file(&path).map_err(|e| {
        format!(
            "{}: {e} (git lfs pull --include=\"assets/runtime/audio/clips/**\")",
            path.display()
        )
    })?;
    if data.sample_rate != SAMPLE_RATE {
        return Err(format!(
            "{}: {} Hz, not {SAMPLE_RATE}",
            path.display(),
            data.sample_rate
        ));
    }
    Ok(vec![data.frames.iter().map(|f| f.left).collect()])
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
        // The master, through a limiter (the browser's compressor).
        let limiter = CompressorBuilder::new()
            .threshold(-6.0)
            .ratio(20.0)
            .attack_duration(Duration::from_millis(2))
            .release_duration(Duration::from_millis(200));
        let mut manager = AudioManager::<Output>::new(AudioManagerSettings {
            capacities: Capacities {
                sub_track_capacity: 512,
                ..Default::default()
            },
            main_track_builder: MainTrackBuilder::new()
                .volume(decibels(audio.buses.master))
                .with_effect(limiter),
            backend_settings: output,
            ..Default::default()
        })
        .map_err(|e| format!("audio output: {e}"))?;
        let mut buses = Vec::new();
        let mut reverbs = Vec::new();
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
            buses.push(
                manager
                    .add_sub_track(
                        TrackBuilder::new()
                            .volume(decibels(gain))
                            .sub_track_capacity(256),
                    )
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(Self {
            manager,
            sound,
            sounds,
            buses,
            reverbs,
            voices: HashMap::new(),
            listener: Listener {
                position: [0.0; 3],
                forward: [0.0, 1.0, 0.0],
            },
            played: Played::default(),
            rendered: 0.0,
        })
    }

    pub fn played(&self) -> Played {
        Played {
            live: self.voices.len(),
            ..self.played
        }
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
    }

    fn run(&mut self, commands: &[Command], now: f64) {
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
            }
        }
        // A transient that has played out frees its voice.
        self.voices
            .retain(|_, v| v.sound.state() != PlaybackState::Stopped);
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
        let Some(alternatives) = self.sounds.get(&v.sound) else {
            return;
        };
        let (data, normalization) = &alternatives[v.variant as usize % alternatives.len()];
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
        let Some(v) = self.voices.get(&id) else { return };
        let (panning, level) = self.panning(p.position.as_ref(), None);
        let normalization = v.scale / if v.position.is_some() { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 };
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

    /// Renders the buffer output up to player time `now` (seconds),
    /// appending interleaved stereo samples to `out`; nothing for a device.
    pub fn render(&mut self, now: f64, out: &mut Vec<f32>) {
        let Output::Buffer {
            renderer: Some(renderer),
            sample_rate,
        } = self.manager.backend_mut()
        else {
            return;
        };
        let rate = f64::from(*sample_rate);
        let frames = ((now - self.rendered) * rate).floor().max(0.0) as usize;
        if frames == 0 {
            return;
        }
        let at = out.len();
        out.resize(at + frames * 2, 0.0);
        renderer.on_start_processing();
        renderer.process(&mut out[at..], 2);
        self.rendered += frames as f64 / rate;
    }
}
