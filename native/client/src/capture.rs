//! A browser parity run's scene, replayed through the presentation crate as
//! 18b's capture replay does: the battle's world loaded once, then for each
//! frame record in order, every publication up to its tick pushed and
//! `frame()` asked at its sim time and camera. Every record is framed, since
//! poses and effects carry state from frame to frame.
use std::fs;
use std::path::{Path, PathBuf};

use presentation::light::LightSettings;
use presentation::presentation::{Frame, Presentation, Ui};
use presentation::units::drawn::ClipTables;
use presentation::view::Camera;
use presentation::world::{Appearance, WorldInput};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureFile {
    schema: String,
    tick_hz: f64,
    side: String,
    layout: String,
    samples: Vec<Sample>,
    frames: Vec<Record>,
}

#[derive(Deserialize)]
struct Sample {
    tick: u64,
    publication: Vec<u32>,
}

/// One drawn browser frame.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub tick: u64,
    pub sim_time: f64,
    /// The reel's wall time when the browser drew it, milliseconds.
    pub reel_ms: f64,
    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub canvas: [u32; 2],
    /// The control plate written of this frame, if any.
    pub plate: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lens {
    fov_y: f32,
    near: f32,
}

pub struct Replay {
    pub dir: PathBuf,
    pub records: Vec<Record>,
    pub light: LightSettings,
    /// The browser's posed-model counts by plate (`modelCounts`).
    pub model_counts: Vec<Value>,
    samples: Vec<Sample>,
    pushed: usize,
    lens: Lens,
    presentation: Presentation,
}

fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

impl Replay {
    /// Loads scene `dir` of a parity run; `game` is the rules fixture, for
    /// the soldier's body and the aircraft's cruise height.
    pub fn open(dir: &Path, game: &Path) -> Result<Self, String> {
        let capture: CaptureFile = read_json(&dir.join("capture.json"))?;
        if capture.schema != "battle-presentation-capture" {
            return Err(format!("capture schema {}", capture.schema));
        }
        let manifest: Value = read_json(&dir.join("manifest.json"))?;
        if manifest["schema"] != "native-visual-presentation" {
            return Err(format!("manifest schema {}", manifest["schema"]));
        }
        let game: Value = read_json(game)?;
        let light: LightSettings =
            serde_json::from_value(manifest["light"].clone()).map_err(|e| format!("light: {e}"))?;
        let lens: Lens =
            serde_json::from_value(manifest["lens"].clone()).map_err(|e| format!("lens: {e}"))?;
        let physics = &game["physics"];
        let input: WorldInput = serde_json::from_value(json!({
            "battle": {
                "units": manifest["units"],
                "effects": manifest["effects"],
                "side": capture.side,
                "tick_hz": capture.tick_hz,
                "soldier": {
                    "radius_m": physics["soldier_radius_m"],
                    "height_m": physics["soldier_height_m"],
                },
                // The UI's colours: a replay selects nothing and shows no
                // x-ray or ghost, so no frame reads them.
                "xray": { "own": [0, 0, 0, 0], "selected": [0, 0, 0, 0] },
                "blocked": [0, 0, 0, 0],
                "cruise_agl_m": game["air"]["cruise_agl_m"],
            },
            "sun_azimuth": light.sun_azimuth,
            "sun_elevation": light.sun_elevation,
        }))
        .map_err(|e| format!("world: {e}"))?;
        let mut tables = ClipTables::new();
        let units = &input.battle.as_ref().ok_or("world: no battle")?.units;
        for a in units.appearances.values() {
            if let Appearance::Skinned { table, .. } = a {
                let path = dir.join(format!("units-{table}.u32"));
                let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let floats = bytes.as_chunks::<4>().0.iter();
                tables.insert(
                    table.clone(),
                    floats.map(|b| f32::from_le_bytes(*b)).collect(),
                );
            }
        }
        let mut presentation = Presentation::new(Some(&capture.layout), capture.tick_hz)?;
        // The static world (buildings, ground) arrives with the ground step.
        presentation.load_world(input, tables, None)?;
        Ok(Self {
            dir: dir.to_owned(),
            records: capture.frames,
            light,
            model_counts: manifest["modelCounts"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            samples: capture.samples,
            pushed: 0,
            lens,
            presentation,
        })
    }

    /// The presentation the replay drives: its world names what the models load.
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The last frame asked for.
    pub fn last_frame(&self) -> &Frame {
        self.presentation.last_frame()
    }

    /// Frames `record`: pushes the publications up to its tick, `heard`
    /// after each, then asks for its frame.
    pub fn frame(
        &mut self,
        record: &Record,
        mut heard: impl FnMut(&Presentation),
    ) -> Result<&Frame, String> {
        while let Some(s) = self
            .samples
            .get(self.pushed)
            .filter(|s| s.tick <= record.tick)
        {
            self.presentation.push_publication(&s.publication, 0.0)?;
            heard(&self.presentation);
            self.pushed += 1;
        }
        let camera = Camera {
            eye: record.eye,
            target: record.target,
            up: record.up.map(|v| v as f32),
            fov_y: self.lens.fov_y,
            near: self.lens.near,
            size: record.canvas,
        };
        self.presentation
            .frame(record.sim_time, &camera, &Ui::default())
    }
}
