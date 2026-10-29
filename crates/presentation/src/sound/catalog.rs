//! The sound library's authored schema, including source receipts and crops.
//! Admission happens in Rust before any client prepares or auditions a sound.
mod check;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundCatalog {
    pub sources: HashMap<String, Source>,
    #[serde(default)]
    pub provenance: HashMap<String, Provenance>,
    pub clips: HashMap<String, Clip>,
    pub sounds: BTreeMap<String, Recipe>,
    pub defaults: HashMap<String, ShotChoice>,
    pub impacts: HashMap<String, HashMap<String, String>>,
    pub effects: HashMap<String, String>,
    pub music: MusicCatalog,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub label: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub url: Option<String>,
    pub path: String,
    pub sha256: String,
    pub notes: Option<String>,
    pub kind: Option<String>,
    pub generator: Option<String>,
    pub model: Option<String>,
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub voice_id: Option<String>,
    pub voice_description: Option<String>,
    pub date: Option<String>,
    pub plan: Option<String>,
    pub length_s: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub label: String,
    pub author: String,
    pub license: String,
    pub url: String,
    pub sha256: Option<String>,
    pub source_frames: [u64; 2],
    pub processing: String,
    pub download: Download,
    pub path: Option<String>,
    pub notes: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub tool: String,
    pub version: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Burst {
    pub shots: Vec<[u64; 2]>,
    pub interval_s: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub source: String,
    pub source_rate: u32,
    pub source_frames: [u64; 2],
    pub at_s: f64,
    pub gain: f64,
    pub semitones: f64,
    pub lowpass_hz: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub label: String,
    pub category: String,
    pub notes: Option<String>,
    pub role: String,
    pub frames: u64,
    pub sample_rate: u32,
    pub sha256: String,
    pub url: String,
    #[serde(rename = "loop")]
    pub looping: bool,
    pub processing: String,
    pub source: Option<String>,
    pub source_rate: Option<u32>,
    pub source_frames: Option<[u64; 2]>,
    pub layers: Option<Vec<Layer>>,
    pub burst: Option<Burst>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub label: String,
    pub clips: Vec<String>,
    pub synth: Option<String>,
    pub synth_gain: f64,
    pub gain: f64,
    #[serde(rename = "loop")]
    pub looping: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShotChoice {
    pub near: String,
    pub far: String,
    pub gain: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicCatalog {
    pub moods: LibraryMoods,
    pub tracks: HashMap<String, Track>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryMoods {
    pub menu: Vec<String>,
    pub tension: Vec<String>,
    pub combat: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub source: String,
    pub sha256: String,
    pub gain: f64,
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
