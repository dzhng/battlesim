//! A browser parity run's scene, replayed through the presentation crate:
//! the world read from the repository (the saved map the run's manifest
//! names, its ground, scenery, props and buildings, and the units its
//! encounter fields), loaded once, then for each frame record in order,
//! every publication up to its tick pushed and `frame()` asked at its sim
//! time and camera. Every record is framed, since poses, effects and the
//! scenery's walk carry state from frame to frame.
//!
//! Both files are read as strictly as the browser's readers
//! (`presentationCapture.ts`, `visualPresentation.ts`): every key the format
//! has, and no other, so a stale file is refused rather than read in part or
//! defaulted. A value native has no use for is required but not read
//! (`IgnoredAny`).
use std::fs;
use std::path::Path;

use presentation::fielded;
use presentation::fog::FogPresentation;
use presentation::light::LightSettings;
use presentation::presentation::{FogSetup, Frame, Presentation, Ui};
use presentation::records::FogLook;
use presentation::static_world::files;
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::bundle::Library;
use crate::scenery::tier_mesh;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CaptureFile {
    schema: String,
    workload: Workload,
    tick_hz: f64,
    #[serde(rename = "warmTick")]
    _warm_tick: IgnoredAny,
    side: String,
    layout: String,
    samples: Vec<Sample>,
    frames: Vec<Record>,
}

/// What was filmed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Workload {
    #[serde(rename = "id")]
    _id: IgnoredAny,
    #[serde(rename = "fingerprint")]
    _fingerprint: IgnoredAny,
    scene: String,
    map: String,
    encounter: String,
    seed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    tick: u64,
    #[serde(rename = "digest")]
    _digest: IgnoredAny,
    publication: Vec<u32>,
}

/// One drawn browser frame.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Record {
    #[serde(rename = "index")]
    _index: IgnoredAny,
    pub tick: u64,
    pub sim_time: f64,
    /// The reel's wall time when the browser drew it, milliseconds.
    pub reel_ms: f64,
    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub canvas: [u32; 2],
    /// The control plate written of this frame, or null: the key is
    /// required, as the browser writes it.
    #[serde(deserialize_with = "Option::deserialize")]
    pub plate: Option<String>,
}

/// `native-visual-presentation`: what the browser drew of the scene.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Manifest {
    schema: String,
    identity: Identity,
    #[serde(rename = "viewport")]
    _viewport: IgnoredAny,
    lens: Lens,
    layers: Vec<String>,
    #[serde(rename = "grassCounts")]
    _grass_counts: IgnoredAny,
    instance_counts: Vec<Value>,
    model_counts: Vec<Value>,
    effect_counts: Vec<Value>,
    frame_counts: Vec<Value>,
}

/// The battle the control ran: the capture's workload, and both files' bytes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Identity {
    scene: String,
    map: String,
    encounter: String,
    seed: u64,
    map_hash: String,
    capture_hash: String,
}

/// The drawn lens, through the engine-wide depth convention.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Lens {
    fov_y: f64,
    near: f64,
    #[serde(rename = "depth")]
    _depth: ReverseZ,
    #[serde(rename = "far")]
    _far: Infinite,
}

#[derive(Deserialize)]
enum ReverseZ {
    #[serde(rename = "reverse-z")]
    ReverseZ,
}

#[derive(Deserialize)]
enum Infinite {
    #[serde(rename = "infinite")]
    Infinite,
}

pub struct Replay {
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
    /// The building modules, scenery and known props the browser drew by
    /// plate (`instanceCounts`).
    pub instance_counts: Vec<Value>,
    /// What the browser drew of each counted frame (`frameCounts`).
    pub frame_counts: Vec<Value>,
    /// The rate the page's presentation ran at.
    pub tick_hz: f64,
    /// The map's terrain triangles, as the simulation exports them:
    /// vertices (x, y, z), then indices.
    pub terrain: (Vec<f32>, Vec<u32>),
    /// The simulation's world, kept while fog is drawn: the side's foliage
    /// follows the ground it sees cleared.
    world: Option<files::World>,
    /// The cleared cells fog's foliage last followed.
    cleared: usize,
    samples: Vec<Sample>,
    pushed: usize,
    lens: Lens,
    presentation: Presentation,
}

fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The repository a replay reads its world and rules from.
pub const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

impl Replay {
    /// Loads scene `dir` of a parity run. The world is the repository's: the
    /// saved map the manifest names, the units its encounter fields and their
    /// art, the scenery placed from `library`'s bundles, under the game's
    /// rules (the soldier's body, the light).
    pub fn open(dir: &Path, library: &Library) -> Result<Self, String> {
        Self::open_with_root(dir, library, Path::new(ROOT))
    }

    /// Load a replay using an explicit repository root. Packaged Windows
    /// builds provide this because the build-time manifest path may be WSL's.
    pub fn open_with_root(dir: &Path, library: &Library, root: &Path) -> Result<Self, String> {
        let capture_path = dir.join("capture.json");
        let capture_bytes =
            fs::read(&capture_path).map_err(|e| format!("{}: {e}", capture_path.display()))?;
        let capture: CaptureFile = serde_json::from_slice(&capture_bytes)
            .map_err(|e| format!("{}: {e}", capture_path.display()))?;
        if capture.schema != "battle-presentation-capture" {
            return Err(format!("capture schema {}", capture.schema));
        }
        let manifest: Manifest = read_json(&dir.join("manifest.json"))?;
        if manifest.schema != "native-visual-presentation" {
            return Err(format!("manifest schema {}", manifest.schema));
        }
        // The battle the manifest names, and nothing else: the capture's
        // workload, and both files byte for byte.
        let identity = &manifest.identity;
        let workload = &capture.workload;
        for (key, filmed, named) in [
            ("scene", &workload.scene, &identity.scene),
            ("map", &workload.map, &identity.map),
            ("encounter", &workload.encounter, &identity.encounter),
            (
                "seed",
                &workload.seed.to_string(),
                &identity.seed.to_string(),
            ),
        ] {
            if filmed != named {
                return Err(format!(
                    "{}: the capture's workload {key} is {filmed}, the manifest's identity names {named}",
                    capture_path.display(),
                ));
            }
        }
        let map = workload.map.clone();
        let encounter = &workload.encounter;
        let map_path = root.join("fixtures/maps").join(&map).join("map.json");
        let map_bytes = fs::read(&map_path).map_err(|e| format!("{}: {e}", map_path.display()))?;
        for (path, bytes, key, named) in [
            (
                &capture_path,
                &capture_bytes,
                "captureHash",
                &identity.capture_hash,
            ),
            (&map_path, &map_bytes, "mapHash", &identity.map_hash),
        ] {
            let hash = format!("{:x}", Sha256::digest(bytes));
            if *named != hash {
                return Err(format!(
                    "{} is not the file the control ran on: sha256 {hash}, the manifest's identity.{key} is {named}",
                    path.display(),
                ));
            }
        }
        let game: Value = read_json(&root.join("fixtures/rules.json"))?;
        let settings: Value = read_json(&root.join("fixtures/presentation.json"))?;
        let light: LightSettings = serde_json::from_value(settings["light"].clone())
            .map_err(|e| format!("presentation.light: {e}"))?;
        // The battle's units, effects and clip tables, as a page fielding
        // the encounter's units installs them.
        let kinds = fielded::encounter_kinds(root, &map, encounter)?;
        let fielded = fielded::fielded(root, &game, &map, &kinds)?;
        let physics = &game["physics"];
        let number = |v: &Value, what: &str| v.as_f64().ok_or(format!("rules.json: no {what}"));
        let battle = BattleInput {
            units: fielded.units,
            effects: fielded.effects,
            side: capture.side.clone(),
            tick_hz: capture.tick_hz,
            soldier: SoldierBody {
                radius_m: number(&physics["soldier_radius_m"], "physics.soldier_radius_m")?,
                height_m: number(&physics["soldier_height_m"], "physics.soldier_height_m")?,
            },
            // The UI's colours: a replay selects nothing and shows no x-ray
            // or ghost, so no frame reads them.
            xray: Xray {
                own: [0.0; 4],
                selected: [0.0; 4],
                ally: [0.0; 4],
            },
            blocked: [0.0; 4],
        };
        let fog: FogPresentation = serde_json::from_value(settings["fog"].clone())
            .map_err(|e| format!("presentation.fog: {e}"))?;
        let fog_look = fog.selected()?.look()?;
        let mut presentation = Presentation::new(Some(&capture.layout), capture.tick_hz)?;
        // The ground, and its scenery placed from each kind's finest mesh.
        let ground = files::ground(root, &map)?;
        let (buildings, style) = files::placed_buildings(root, &map, &ground.world)?;
        presentation.set_ground(ground.source, ground.biome, ground.tuning)?;
        // Fog of war, where the browser drew it: the side's sight under the
        // game's sensor rules.
        let fog = manifest.layers.iter().any(|l| l == "fog");
        if fog {
            presentation.set_fog(Some(FogSetup {
                sensors: Some(ground.sensors),
                ..FogSetup::default()
            }))?;
        }
        let kinds = presentation
            .ground()
            .map_or_else(Vec::new, |g| g.scenery_kinds());
        let finest = kinds
            .iter()
            .map(|name| tier_mesh(name, library.bundle(name)?.as_ref(), 0))
            .collect::<Result<Vec<_>, String>>()?;
        let finest: Vec<&[f32]> = finest.iter().map(Vec::as_slice).collect();
        presentation.place_scenery(&finest)?;
        // The props, from the art a page running these units installs.
        let wearing: Vec<&str> = battle
            .units
            .appearances
            .keys()
            .map(String::as_str)
            .collect();
        let art = files::prop_art(root, ground.family.as_deref(), &wearing)?;
        let records = presentation
            .ground()
            .map_or_else(Vec::new, |g| g.source.props.clone());
        presentation.place_props(&records, &ground.layout, art)?;
        let input = WorldInput {
            battle: Some(battle),
            sun_azimuth: light.sun_azimuth,
            sun_elevation: light.sun_elevation,
            buildings: Some(style),
        };
        presentation.load_world(input, fielded.tables, Some(buildings))?;
        Ok(Self {
            map,
            records: capture.frames,
            light,
            fog_look,
            model_counts: manifest.model_counts,
            effect_counts: manifest.effect_counts,
            instance_counts: manifest.instance_counts,
            frame_counts: manifest.frame_counts,
            tick_hz: capture.tick_hz,
            terrain: (ground.positions, ground.indices),
            world: fog.then_some(ground.world),
            cleared: 0,
            samples: capture.samples,
            pushed: 0,
            lens: manifest.lens,
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
    fn camera(&self, record: &Record) -> Camera {
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
    /// after each, then asks for its frame for a renderer at `generation`
    /// (`Presentation::frame`): a new one is written every static batch, the
    /// same one only what changed. Then, as the page's simulation exports
    /// it, fog's foliage follows the ground the side has seen cleared, from
    /// the next frame on.
    pub fn frame(
        &mut self,
        record: &Record,
        generation: u32,
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
        let camera = self.camera(record);
        self.presentation
            .frame(record.sim_time, &camera, &Ui::default(), generation)?;
        // Fog's foliage is the side's: less the trees on ground it has seen
        // cleared, as the page re-exports it.
        if let (Some(world), Some(learned)) = (&self.world, self.presentation.learned()) {
            if learned.cleared_count() != self.cleared {
                self.cleared = learned.cleared_count();
                let foliage = files::side_foliage(world, learned);
                self.presentation.set_fog(Some(FogSetup {
                    foliage: Some(foliage),
                    ..FogSetup::default()
                }))?;
            }
        }
        Ok(self.presentation.last_frame())
    }
}
