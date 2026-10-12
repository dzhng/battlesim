//! A browser parity run's scene, replayed through the presentation crate as
//! 18b's capture replay does: the battle's world loaded once, then for each
//! frame record in order, every publication up to its tick pushed and
//! `frame()` asked at its sim time and camera. Every record is framed, since
//! poses and effects carry state from frame to frame.
use std::fs;
use std::path::{Path, PathBuf};

use presentation::fog::FogPresentation;
use presentation::light::LightSettings;
use presentation::presentation::{Frame, Presentation, Ui};
use presentation::records::FogLook;
use presentation::static_world::files;
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

/// The manifest's `props.buildings`.
#[derive(Deserialize)]
struct Staged {
    parts: Vec<Vec<u32>>,
    shells: Vec<String>,
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
    fov_y: f64,
    near: f64,
}

pub struct Replay {
    pub dir: PathBuf,
    /// The saved map the scene was filmed on (`identity.map`).
    pub map: String,
    pub records: Vec<Record>,
    pub light: LightSettings,
    /// The fixture's selected unseen look (`presentation.fog`).
    pub fog_look: FogLook,
    /// The browser's posed-model counts by plate (`modelCounts`).
    pub model_counts: Vec<Value>,
    /// The browser's effect counts by plate (`effectCounts`).
    pub effect_counts: Vec<Value>,
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
    pub fn open(dir: &Path, game_path: &Path) -> Result<Self, String> {
        let capture: CaptureFile = read_json(&dir.join("capture.json"))?;
        if capture.schema != "battle-presentation-capture" {
            return Err(format!("capture schema {}", capture.schema));
        }
        let manifest: Value = read_json(&dir.join("manifest.json"))?;
        if manifest["schema"] != "native-visual-presentation" {
            return Err(format!("manifest schema {}", manifest["schema"]));
        }
        let game: Value = read_json(game_path)?;
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
            "cascade_far_m": manifest["light"]["cascades"]["max_far_m"],
            "buildings": game["presentation"]["buildings"],
        }))
        .map_err(|e| format!("world: {e}"))?;
        let fog: FogPresentation = serde_json::from_value(game["presentation"]["fog"].clone())
            .map_err(|e| format!("presentation.fog: {e}"))?;
        let fog_look = fog.selected()?.look()?;
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
        // The buildings, each one's part props and the kinds a gutted
        // building's parts become, as the page staged them.
        let map = manifest["identity"]["map"]
            .as_str()
            .ok_or("manifest: no identity.map")?
            .to_owned();
        let staged: Staged = serde_json::from_value(manifest["props"]["buildings"].clone())
            .map_err(|e| format!("props.buildings: {e}"))?;
        let root = game_path
            .parent()
            .and_then(Path::parent)
            .ok_or("no repository root")?;
        let (buildings, _) = files::placed_buildings(root, &map, staged.parts, staged.shells)?;
        presentation.load_world(input, tables, Some(buildings))?;
        Ok(Self {
            dir: dir.to_owned(),
            map,
            records: capture.frames,
            light,
            fog_look,
            model_counts: manifest["modelCounts"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            effect_counts: manifest["effectCounts"]
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

    pub fn presentation_mut(&mut self) -> &mut Presentation {
        &mut self.presentation
    }

    /// The camera `record` was drawn from, under the manifest's lens.
    pub fn camera(&self, record: &Record) -> Camera {
        Camera {
            eye: record.eye,
            target: record.target,
            up: record.up,
            fov_y: self.lens.fov_y,
            near: self.lens.near,
            size: record.canvas,
        }
    }

    /// The last frame asked for.
    pub fn last_frame(&self) -> &Frame {
        self.presentation.last_frame()
    }

    /// Frames `record`: pushes the publications up to its tick, `heard`
    /// after each, then asks for its frame. A frame that is `drawn` lists
    /// every static batch, as a renderer that drew the frames between would
    /// hold them.
    pub fn frame(
        &mut self,
        record: &Record,
        drawn: bool,
        mut heard: impl FnMut(&Presentation),
    ) -> Result<&Frame, String> {
        if drawn {
            self.presentation.resend_statics();
        }
        while let Some(s) = self
            .samples
            .get(self.pushed)
            .filter(|s| s.tick <= record.tick)
        {
            self.presentation.push_publication(&s.publication, 0.0)?;
            heard(&self.presentation);
            self.pushed += 1;
        }
        let camera = self.camera(record);
        self.presentation
            .frame(record.sim_time, &camera, &Ui::default())
    }
}
