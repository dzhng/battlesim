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
//!
//! A clip is framed on the reel's clock instead ([`ReelClock`]): a record's
//! sim time is the page's presentation clock, read off its own wall clock,
//! which jitters by up to a frame against the reel the camera is keyed to.
use std::fs;
use std::path::Path;

use presentation::fielded;
use presentation::light::LightSettings;
use presentation::presentation::{FogSetup, Frame, Presentation, Ui};
use presentation::records::FogLook;
use presentation::static_world::files;
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use renderer_core::scenery::place_scenery;
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::bundle::Library;
use crate::cards::{self, RecordedCards};
use crate::overlays::{self, Indicators, RecordedOverlays};

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
    /// The resolved encounter rules shared by fielding and sound.
    pub rules: contract::scenario::Rules,
    pub records: Vec<Record>,
    pub light: LightSettings,
    pub backdrop_reach_m: f64,
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
    /// The battle's time on the reel's clock; none without records.
    clock: Option<ReelClock>,
    /// The indicators the page drew over the battle, frame by frame; none
    /// when it drew none or playback leaves them out.
    pub indicators: Option<Indicators>,
    /// The unit info cards the page showed, frame by frame; none when it
    /// showed none or playback leaves them out.
    pub cards: Option<RecordedCards>,
}

/// Which of the layers a capture drew its playback draws. All default on:
/// a layer the capture never drew stays off regardless.
#[derive(Clone, Copy, Debug)]
pub struct Playback {
    /// The side's fog of war (the manifest's `fog`).
    pub fog: bool,
    /// The recorded indicators (the manifest's `overlays`).
    pub indicators: bool,
    /// The recorded unit info cards (the manifest's `cards`).
    pub cards: bool,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            fog: true,
            indicators: true,
            cards: true,
        }
    }
}

/// The battle's time on the reel's clock, for a clip: reel time plus the
/// run's typical lead (the median over its records of sim time less reel
/// time), so frames at even reel steps draw the battle at even sim steps,
/// as the camera moved. The median keeps the clip where the page drew the
/// battle on most frames, which is what its camera followed, and ignores a
/// late start or a stray frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReelClock {
    lead_s: f64,
}

impl ReelClock {
    /// The clock of `records`; none for a capture without any.
    pub fn of(records: &[Record]) -> Option<Self> {
        let mut leads: Vec<f64> = records
            .iter()
            .map(|r| r.sim_time - r.reel_ms / 1000.0)
            .collect();
        leads.sort_by(f64::total_cmp);
        let middle = leads.len() / 2;
        let lead_s = match leads.len() {
            0 => return None,
            n if n % 2 == 1 => leads[middle],
            _ => (leads[middle - 1] + leads[middle]) / 2.0,
        };
        Some(Self { lead_s })
    }

    /// The sim time at reel time `reel_ms`.
    pub fn sim_time(&self, reel_ms: f64) -> f64 {
        reel_ms / 1000.0 + self.lead_s
    }
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
        Self::open_played(dir, library, root, Playback::default())
    }

    /// Loads scene `dir` to play back as `playback` chooses: each layer the
    /// browser drew (`fog`, `overlays`, `cards`) is drawn unless it leaves
    /// it out. The manifest naming `overlays` requires the scene's recorded
    /// overlays ([`overlays::FILE`]), and naming `cards` its recorded cards
    /// ([`cards::FILE`]); a capture without the layer has none.
    pub fn open_played(
        dir: &Path,
        library: &Library,
        root: &Path,
        playback: Playback,
    ) -> Result<Self, String> {
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
        let drawn = |layer: &str| manifest.layers.iter().any(|l| l == layer);
        // The indicators, in lockstep with the frame records, where the
        // browser drew them: refused before the world loads.
        let recorded = if playback.indicators && drawn("overlays") {
            let path = dir.join(overlays::FILE);
            Some(RecordedOverlays::read(&path, capture.frames.len())?)
        } else {
            None
        };
        let recorded_cards = if playback.cards && drawn("cards") {
            Some(RecordedCards::read(
                &dir.join(cards::FILE),
                capture.frames.len(),
            )?)
        } else {
            None
        };
        let catalogue = sim::maps::Catalogue {
            maps: root.join("fixtures/maps"),
            libraries: root.join("fixtures"),
        };
        let scenario = catalogue
            .scenario(&map, encounter, files::rules(root, &map)?)
            .map_err(|e| e.to_string())?;
        let kinds = scenario
            .units
            .iter()
            .map(|u| u.kind.clone())
            .collect::<Vec<_>>();
        let rules = scenario.rules;
        let settings =
            presentation::settings::PresentationSettings::from_value(&read_json::<Value>(
                &root.join("fixtures/presentation.json"),
            )?)?;
        let light = settings.light;
        // The battle's units, effects and clip tables, as a page fielding
        // the encounter's units installs them.
        let fielded = fielded::fielded(root, &rules, &kinds)?;
        if capture.tick_hz != f64::from(rules.tick_hz) {
            return Err("capture tick_hz differs from the encounter rules".into());
        }
        let physics = &rules.physics;
        let battle = BattleInput {
            units: fielded.units,
            effects: fielded.effects,
            side: capture.side.clone(),
            tick_hz: capture.tick_hz,
            soldier: SoldierBody {
                radius_m: physics.soldier_radius_m,
                height_m: physics.soldier_height_m,
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
        let fog = settings.fog;
        let fog_look = fog.selected()?.look()?;
        let mut presentation = Presentation::new(Some(&capture.layout), capture.tick_hz)?;
        // The ground, and its scenery placed from each kind's finest mesh.
        let ground = files::ground(root, &map, &rules)?;
        let (buildings, style) = files::placed_buildings(root, &rules, &ground.world)?;
        presentation.set_ground(ground.source, ground.biome, ground.tuning)?;
        // Fog of war, where the browser drew it: the side's sight under the
        // game's sensor rules.
        let fog = playback.fog && drawn("fog");
        if fog {
            presentation.set_fog(Some(FogSetup {
                sensors: Some(ground.sensors),
                ..FogSetup::default()
            }))?;
        }
        place_scenery(&mut presentation, library)?;
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
        let indicators = recorded
            .map(|overlays| Indicators::new(overlays, &settings.overlay))
            .transpose()?;
        Ok(Self {
            indicators,
            cards: recorded_cards,
            map,
            rules,
            clock: ReelClock::of(&capture.frames),
            records: capture.frames,
            light,
            backdrop_reach_m: settings.ground.backdrop_reach_m,
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

    /// Frames `record` as the page drew it: pushes the publications up to its
    /// tick, `heard` after each, then draws it at its sim time (`draw`).
    pub fn frame(
        &mut self,
        record: &Record,
        generation: u32,
        mut heard: impl FnMut(&Presentation),
    ) -> Result<&Frame, String> {
        while self
            .samples
            .get(self.pushed)
            .is_some_and(|s| s.tick <= record.tick)
        {
            self.push_next(&mut heard)?;
        }
        self.draw(&self.camera(record), record.sim_time, generation)
    }

    /// The camera reel time `reel_ms` is seen through: the last the page
    /// drew by then. Before its first record (the reel starts a frame before
    /// the page records one) the page drew none, so its first camera is
    /// carried back along its first two records' motion to that time: a
    /// frame's camera is of the same moment as its battle. None without
    /// records.
    pub fn camera_at(&self, reel_ms: f64) -> Option<Camera> {
        let drawn = self.records.partition_point(|r| r.reel_ms <= reel_ms);
        if let (0, [first, second, ..]) = (drawn, &self.records[..]) {
            if second.reel_ms > first.reel_ms {
                let k = (reel_ms - first.reel_ms) / (second.reel_ms - first.reel_ms);
                let along =
                    |a: [f64; 3], b: [f64; 3]| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k);
                let up = along(first.up, second.up);
                let length = up.iter().map(|v| v * v).sum::<f64>().sqrt();
                return Some(Camera {
                    eye: along(first.eye, second.eye),
                    target: along(first.target, second.target),
                    up: up.map(|v| v / length),
                    ..self.camera(first)
                });
            }
        }
        Some(self.camera(self.records.get(drawn.saturating_sub(1))?))
    }

    /// The record the page was showing at reel time `reel_ms`: the last it
    /// drew by then (the first, before any), whose camera `camera_at` follows.
    pub fn record_at(&self, reel_ms: f64) -> usize {
        self.records
            .partition_point(|r| r.reel_ms <= reel_ms)
            .saturating_sub(1)
    }

    /// Frames reel time `reel_ms` of a clip through the camera at it
    /// (`camera_at`): the battle at the reel clock's time, with the
    /// publications that time blends toward pushed, `heard` after each. The
    /// time stops at the run's last publication; the presentation never
    /// draws past the newest it has, as the page's clock never does.
    pub fn frame_on_reel(
        &mut self,
        reel_ms: f64,
        generation: u32,
        mut heard: impl FnMut(&Presentation),
    ) -> Result<&Frame, String> {
        let (Some(clock), Some(camera)) = (self.clock, self.camera_at(reel_ms)) else {
            return Err("a capture without frames has no reel".into());
        };
        let time = clock.sim_time(reel_ms);
        let latest = |r: &Self| {
            r.pushed
                .checked_sub(1)
                .map(|i| r.samples[i].tick as f64 / r.tick_hz)
        };
        while latest(self).is_none_or(|t| t < time) && self.push_next(&mut heard)? {}
        let time = latest(self).map_or(time, |t| time.min(t));
        self.draw(&camera, time, generation)
    }

    /// Pushes the next publication, `heard` after it; false when the run
    /// has none left.
    fn push_next(&mut self, heard: &mut impl FnMut(&Presentation)) -> Result<bool, String> {
        let Some(s) = self.samples.get(self.pushed) else {
            return Ok(false);
        };
        self.presentation.push_publication(&s.publication, 0.0)?;
        heard(&self.presentation);
        self.pushed += 1;
        Ok(true)
    }

    /// Asks for the frame at sim time `time` through `camera`, for
    /// a renderer at `generation` (`Presentation::frame`): a new one is
    /// written every static batch, the same one only what changed. Then, as
    /// the page's simulation exports it, fog's foliage follows the ground the
    /// side has seen cleared, from the next frame on.
    fn draw(&mut self, camera: &Camera, time: f64, generation: u32) -> Result<&Frame, String> {
        self.presentation
            .frame(time, camera, &Ui::default(), generation)?;
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
