//! The voice packs, as the browser's `VoiceBank` reads them: a set's
//! `pack.json` (`assets/runtime/audio/voices/<set>/`) names its voices and
//! each take's served file and length. The line arbitration
//! (`presentation::sound::voices::Chatter`) is told both as a pack loads; a
//! take is read and decoded when a line plays it, the last few played
//! staying decoded, and calibrated as every take is.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use presentation::sound::bank::source_normalization;
use presentation::sound::voices::{line_sound, Chatter, SetVoice};
use serde::Deserialize;

/// Takes kept decoded, the most recently played (the browser's `DECODED_TAKES`).
const DECODED_TAKES: usize = 32;

#[derive(Deserialize)]
struct PackTake {
    voice: String,
    trigger: String,
    intensity: String,
    take: u32,
    url: String,
    duration_s: f64,
}

/// `pack.json`: what a set can say.
#[derive(Deserialize)]
struct Pack {
    voices: Vec<SetVoice>,
    takes: Vec<PackTake>,
}

pub struct Packs {
    runtime: PathBuf,
    /// Each take's file, by its line sound and take.
    files: HashMap<(String, u32), PathBuf>,
    /// Takes decoded, least recently played first: the take and its calibration.
    decoded: Vec<((String, u32), StaticSoundData, f64)>,
}

impl Packs {
    pub fn new(runtime: &Path) -> Self {
        Self {
            runtime: runtime.to_owned(),
            files: HashMap::new(),
            decoded: Vec::new(),
        }
    }

    /// Reads `set`'s pack and tells `chatter` its voices and takes. A set
    /// without a readable pack stays silent; the error names it.
    pub fn load(&mut self, set: &str, chatter: &mut Chatter) -> Result<(), String> {
        let path = self.runtime.join(format!("audio/voices/{set}/pack.json"));
        let fail = |why: String| {
            format!(
                "{}: {why} (git lfs pull --include=\"assets/runtime/audio/voices/{set}/**\")",
                path.display()
            )
        };
        let text = std::fs::read_to_string(&path).map_err(|e| fail(e.to_string()))?;
        let pack: Pack = serde_json::from_str(&text).map_err(|e| fail(e.to_string()))?;
        chatter.set_voices(set, pack.voices);
        for t in pack.takes {
            let sound = line_sound(set, &t.voice, &t.trigger, &t.intensity);
            chatter.loaded(
                set,
                &t.voice,
                &t.trigger,
                &t.intensity,
                t.take,
                t.duration_s,
            );
            let file = self.runtime.join(t.url.trim_start_matches('/'));
            self.files.insert((sound, t.take), file);
        }
        Ok(())
    }

    /// Take `variant` of line `sound` (`set/voice/trigger/intensity`) and its
    /// calibration; none for a sound no pack names.
    pub fn take(
        &mut self,
        sound: &str,
        variant: u32,
    ) -> Option<Result<(StaticSoundData, f64), String>> {
        let key = (sound.to_owned(), variant);
        if let Some(i) = self.decoded.iter().position(|(k, ..)| *k == key) {
            // Most recently played last.
            let entry = self.decoded.remove(i);
            let found = (entry.1.clone(), entry.2);
            self.decoded.push(entry);
            return Some(Ok(found));
        }
        let path = self.files.get(&key)?;
        Some(decode(path).map(|(data, calibration)| {
            self.decoded.push((key, data.clone(), calibration));
            if self.decoded.len() > DECODED_TAKES {
                self.decoded.remove(0);
            }
            (data, calibration)
        }))
    }
}

/// A take's samples and its calibration (`source_normalization`, as the
/// browser's sink measures every take); refused by name when the file is
/// missing or not an MP3 (a pointer Git LFS has not fetched).
fn decode(path: &Path) -> Result<(StaticSoundData, f64), String> {
    let fail = |why: String| format!("{}: {why}", path.display());
    let bytes: Arc<[u8]> = std::fs::read(path).map_err(|e| fail(e.to_string()))?.into();
    let (rate, frames) = super::mp3::decode(bytes).map_err(fail)?;
    let mono = frames.iter().all(|f| f.left == f.right);
    let mut channels = vec![frames.iter().map(|f| f.left).collect::<Vec<_>>()];
    if !mono {
        channels.push(frames.iter().map(|f| f.right).collect());
    }
    let calibration = source_normalization(&channels, f64::from(rate), 1.0);
    let data = StaticSoundData {
        sample_rate: rate,
        frames: frames.into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    };
    Ok((data, calibration))
}
