//! The music bus, as the browser's `MusicPlayer` builds it: each track a
//! stream decoded as it plays, at its catalog gain, fading in and out in
//! amplitude; every track sums into the bus (the music level) and then a
//! duck that dips under voice lines, so neither overwrites the other.
//! Which track plays and when is the score's (`presentation::sound::music`);
//! this resolves a track id to its served file.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use kira::info::Info;
use kira::sound::{Sound, SoundData};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, Frame};
use serde_json::Value;

use super::mp3::Mp3;
use super::{decibels, tween, Output};

/// What the bus tells a sounding stream: the gain to fade to and over how
/// long (two `f32`s in one word, so it is read whole), and whether it failed.
#[derive(Default)]
struct Control {
    fade: AtomicU64,
    failed: AtomicBool,
}

impl Control {
    fn fade(&self, gain: f64, seconds: f64) {
        let word =
            (u64::from((gain as f32).to_bits()) << 32) | u64::from((seconds as f32).to_bits());
        self.fade.store(word, Ordering::Release);
    }
}

/// A track looping from its MP3 bytes, resampled to the output rate.
struct Stream {
    mp3: Mp3,
    control: Arc<Control>,
    seen: u64,
    /// The decoded packet, and the next frame of it.
    packet: Vec<Frame>,
    at: usize,
    /// The two frames the output lies between, and how far along.
    prev: Frame,
    next: Frame,
    frac: f64,
    gain: f32,
    target: f32,
    step: f32,
    ramp: u64,
    /// Told to fade to silence: freed once there.
    stopping: bool,
    failed: bool,
}

impl Stream {
    fn pull(&mut self) -> Frame {
        while self.at >= self.packet.len() && !self.failed {
            self.packet.clear();
            self.at = 0;
            match self.mp3.next(&mut self.packet) {
                Ok(true) => {}
                // At the end it loops.
                Ok(false) => self.failed = self.mp3.rewind().is_err(),
                Err(_) => self.failed = true,
            }
        }
        if self.failed {
            self.control.failed.store(true, Ordering::Release);
            return Frame::ZERO;
        }
        self.at += 1;
        self.packet[self.at - 1]
    }
}

impl Sound for Stream {
    fn process(&mut self, out: &mut [Frame], dt: f64, _info: &Info) {
        let word = self.control.fade.load(Ordering::Acquire);
        if word != self.seen {
            self.seen = word;
            self.target = f32::from_bits((word >> 32) as u32);
            self.stopping = self.target == 0.0;
            let frames = (f64::from(f32::from_bits(word as u32)) / dt).round() as u64;
            self.ramp = frames.max(1);
            self.step = (self.target - self.gain) / self.ramp as f32;
        }
        let advance = f64::from(self.mp3.rate) * dt;
        for frame in out {
            *frame = (self.prev + (self.next - self.prev) * self.frac as f32) * self.gain;
            self.frac += advance;
            while self.frac >= 1.0 {
                self.frac -= 1.0;
                self.prev = self.next;
                self.next = self.pull();
            }
            if self.ramp > 0 {
                self.ramp -= 1;
                self.gain = if self.ramp == 0 {
                    self.target
                } else {
                    self.gain + self.step
                };
            }
        }
    }

    fn finished(&self) -> bool {
        self.failed || (self.stopping && self.ramp == 0)
    }
}

impl SoundData for Stream {
    type Error = ();
    type Handle = ();

    fn into_sound(self) -> Result<(Box<dyn Sound>, ()), ()> {
        Ok((Box::new(self), ()))
    }
}

pub struct Music {
    _bus: TrackHandle,
    duck: TrackHandle,
    /// Each track's served file and catalog gain.
    tracks: HashMap<String, (PathBuf, f64)>,
    playing: Vec<Arc<Control>>,
    /// Streams started.
    pub started: usize,
}

impl Music {
    /// The bus at `level` under the master, playing the catalog's tracks
    /// (`music.tracks`) from `runtime/audio/music/<id>.<sha8>.mp3`.
    pub fn new(
        manager: &mut AudioManager<Output>,
        level: f64,
        catalog: &Value,
        runtime: &Path,
    ) -> Result<Self, String> {
        let mut bus = manager
            .add_sub_track(TrackBuilder::new().volume(decibels(level)))
            .map_err(|e| e.to_string())?;
        let duck = bus
            .add_sub_track(TrackBuilder::new())
            .map_err(|e| e.to_string())?;
        let tracks = catalog["music"]["tracks"]
            .as_object()
            .ok_or("sound catalog: no music.tracks")?
            .iter()
            .map(|(id, t)| {
                let sha = t["sha256"].as_str().unwrap_or_default();
                let file = format!("audio/music/{id}.{}.mp3", sha.get(..8).unwrap_or(sha));
                (
                    id.clone(),
                    (runtime.join(file), t["gain"].as_f64().unwrap_or(0.0)),
                )
            })
            .collect();
        Ok(Self {
            _bus: bus,
            duck,
            tracks,
            playing: Vec::new(),
            started: 0,
        })
    }

    /// Streams `track`, fading in over `fade` seconds; refused by name when
    /// its file is missing or not an MP3 (a pointer Git LFS has not fetched).
    pub fn play(&mut self, track: &str, fade: f64) -> Result<(), String> {
        let (path, gain) = self
            .tracks
            .get(track)
            .ok_or_else(|| format!("music: no track {track}"))?;
        let fail = |why: String| {
            format!(
                "{}: {why} (git lfs pull --include=\"assets/runtime/audio/music/**\")",
                path.display()
            )
        };
        let bytes: Arc<[u8]> = std::fs::read(path).map_err(|e| fail(e.to_string()))?.into();
        let mp3 = Mp3::open(bytes).map_err(fail)?;
        let control = Arc::new(Control::default());
        control.fade(*gain, fade);
        let stream = Stream {
            mp3,
            control: control.clone(),
            seen: 0,
            packet: Vec::new(),
            at: 0,
            prev: Frame::ZERO,
            next: Frame::ZERO,
            frac: 0.0,
            gain: 0.0,
            target: 0.0,
            step: 0.0,
            ramp: 0,
            stopping: false,
            failed: false,
        };
        self.duck
            .play(stream)
            .map_err(|e| format!("music: {e:?}"))?;
        self.playing.retain(|c| !c.failed.load(Ordering::Acquire));
        self.playing.push(control);
        self.started += 1;
        Ok(())
    }

    /// Fades every sounding track out over `fade` seconds, then frees it.
    pub fn stop(&mut self, fade: f64) {
        for control in self.playing.drain(..) {
            control.fade(0.0, fade);
        }
    }

    /// Dips the bus by `depth` (0 restores it), gliding over `glide` seconds.
    pub fn duck(&mut self, depth: f64, glide: f64) {
        let level = 1.0 - depth.clamp(0.0, 1.0);
        self.duck.set_volume(decibels(level), tween(glide));
    }
}
