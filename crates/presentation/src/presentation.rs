//! One side's publications into a frame of poses, effects, batches and palettes.
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sim::publication::reader::{FallenBody, KnownProp, Observation, Reader, Shared};

use crate::effects::cook_offs::{CookOffs, PlacedModel};
use crate::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use crate::effects::high_air_shadow::{high_air_ground_shadow, GroundShadow};
use crate::effects::launches::Launch;
use crate::effects::lights::CastLights;
use crate::effects::publication::{EffectFeed, EffectPublication};
use crate::effects::tables::EffectPresentation;
use crate::effects::transit::TransitHistory;
use crate::effects::MuzzleSource;
use crate::fog::sight::{
    Eye, FogSight, FogStats, FogUpload, KnownBox, Occluder, Sensors, SightWorld,
};
use crate::light::LightSettings;
use crate::records::{
    Appearance as Row, Batch, Camera as CameraRecord, CastLight, Counts, Effect, Emitter, Instance,
    Joint, Pick, View as ViewRecord, GHOST_PASS, HIGH_PASS, NONE, VIEW_PASS,
};
use crate::scars::{LearnedGround, ScarCache, ScarStats, ScarUpload, MAX_AXIS};
use crate::shadow::{aircraft_shadow, ReceiverStats, ShadowFit, SunShadowFrame};
use crate::sound::frame::{Cue, Heard, Listener, Motion, Shout};
use crate::static_world::batches::{Population, StaticBatches};
use crate::static_world::buildings::{Buildings, PlacedBuildings};
use crate::static_world::fallen::{Knowledge, Known};
use crate::static_world::ground::biome::Biome;
use crate::static_world::ground::grass::{frame_params, metres_per_pixel, Window};
use crate::static_world::ground::kinds;
use crate::static_world::ground::partition::TerrainPartition;
use crate::static_world::ground::walk::{
    Cleared, Draws, FelledTree, Quality as SceneryQuality, SceneryPopulation, SceneryWalk, DRAWS,
};
use crate::static_world::ground::{Ground, Source, Tuning};
use crate::static_world::library::STATES;
use crate::static_world::props::{KnownBody, PropArt, PropInput, PropLayout, PropPopulation};
use crate::static_world::walk::{BuildingPopulation, BuildingWalk};
use crate::units::articulation::{articulate, rest_locals, track_scroll};
use crate::units::drawn::{ClipTables, DrawnModels};
use crate::units::driver::PoseDriver;
use crate::units::feed::ObservationFeed;
use crate::units::interpolate::{FrameSample, TickInterpolator};
use crate::units::pose::{Articulation, ClipBlend, Kick, PoseFrame};
use crate::units::skin::{self, Mat4};
use crate::view::{batch_caster, model_detail, model_fog, Camera, View, CULLED, IMPOSTOR};
use crate::world::{
    bounds_size, Appearance, BattleInput, Bounds, Trs, UnitsSection, Vec3, WorldInput,
};

mod cameras;
mod picks;
mod probe;
mod side_fog;
mod taken;
use side_fog::Fog;
pub use side_fog::FogSetup;

/// Publications kept for the effects while the world loads: two minutes at 30 Hz.
const PENDING_MAX: usize = 3600;

/// What the UI decides of a frame.
#[derive(Default)]
pub struct Ui<'a> {
    /// Selected own units, x-rayed in the selection's colour.
    pub selected: &'a [u32],
    /// Own units hidden by the world show through it.
    pub xray: bool,
    /// The player the UI commands as, by its index in the scenario's
    /// players: a side's unit another player owns is its ally's, shown
    /// through the world in the ally's colour. None where units have no
    /// owner (a lab).
    pub player: Option<u32>,
    pub ghosts: &'a [Ghost],
    /// A lab's stand-in for the publication's known props, where no battle
    /// publishes (a switch that shows a building destroyed); none reads the
    /// latest publication's.
    pub known: Option<&'a [Known]>,
}

/// One body of a unit the UI shows where it will stand.
#[derive(Clone, Debug, Deserialize)]
pub struct Ghost {
    pub kind: String,
    pub slot: usize,
    pub id: f64,
    pub x: f64,
    pub y: f64,
    /// Final world altitude, shared with the ghost's floating card.
    pub z: f64,
    pub yaw: f64,
    /// 0 the side's own colour, 1 the selection's, 2 a refused purchase.
    pub look: u32,
}

/// One drawn frame's records; the effects and lights are
/// [`Presentation::effects`] and [`Presentation::lights`].
#[derive(Default)]
pub struct Frame {
    pub clock: f64,
    pub tick: u32,
    pub index: u32,
    pub light_wrap: f32,
    pub view: ViewRecord,
    /// The camera uniform, at the frame's clock.
    pub camera: CameraRecord,
    /// The sun's cascades fitted to the camera; none without a light
    /// ([`Presentation::set_shadow`]).
    pub shadow: Option<SunShadowFrame>,
    pub batches: Vec<Batch>,
    pub instances: Vec<Instance>,
    pub palettes: Vec<Joint>,
    pub picks: Vec<Pick>,
    pub emitters: Vec<Emitter>,
    pub counts: Counts,
}

/// The world's records, read once per load.
#[derive(Default)]
pub struct World {
    pub appearances: Vec<Row>,
    pub strings: String,
    rows: HashMap<(String, String, u32, u32), u32>,
}

impl World {
    fn row(&mut self, name: &str, state: &str, lies: u32, skin: u32, placed: bool) -> u32 {
        self.drawn_row(name, state, lies, skin, placed, 0)
    }

    fn drawn_row(
        &mut self,
        name: &str,
        state: &str,
        lies: u32,
        skin: u32,
        placed: bool,
        draws: u32,
    ) -> u32 {
        let key = (name.to_owned(), state.to_owned(), lies, draws);
        if let Some(&row) = self.rows.get(&key) {
            return row;
        }
        let mut text = |s: &str| {
            let at = self.strings.len() as u32;
            self.strings.push_str(s);
            (at, s.len() as u32)
        };
        let (name_at, name_len) = text(name);
        let (state_at, state_len) = text(state);
        let row = self.appearances.len() as u32;
        self.appearances.push(Row {
            name: name_at,
            name_len,
            state: state_at,
            state_len,
            lies,
            skin,
            placed: u32::from(placed),
            draws,
        });
        self.rows.insert(key, row);
        row
    }
}

/// One model this frame may draw, before batching.
struct Model {
    row: u32,
    tier: u32,
    pass: u32,
    at: Vec3,
    yaw: f64,
    tint: [f32; 3],
    scale: [f32; 3],
    colour: [f64; 4],
    /// First palette joint, or 0 (identity).
    palette: u32,
    scroll: [f64; 2],
    /// How fog takes it (`Batch.fog`).
    fog: u32,
}

/// What the buildings' known states were last derived from.
#[derive(Default)]
enum KnownFrom {
    #[default]
    Unread,
    Lab(Vec<Known>),
    Published(Option<Shared<KnownProp>>),
}

/// What a battle draws: its sections, and the per-frame chain's state.
struct Loaded {
    input: BattleInput,
    units: Arc<UnitsSection>,
    tables: Arc<ClipTables>,
    inverse_binds: HashMap<String, Vec<Mat4>>,
    feed: ObservationFeed,
    driver: PoseDriver,
    drawn: DrawnModels,
    effect_feed: EffectFeed,
    effects: EffectFrame,
    batch: EffectBatch,
    lights: CastLights,
    cook_offs: CookOffs,
    transits: TransitHistory,
    pieces: Vec<PlacedModel>,
    shadows: Vec<GroundShadow>,
    indoor: Vec<f64>,
    allied: Vec<f64>,
    sound: Option<(EffectPublication, Vec<Launch>)>,
    shadow_fall: [f64; 2],
    locals: Vec<Trs>,
    worlds: Vec<Mat4>,
    models: Vec<Model>,
}

pub struct Presentation {
    /// The side's stream; none for a world no battle publishes into.
    reader: Option<Reader>,
    tick_hz: f64,
    interpolator: TickInterpolator,
    latest: Option<Arc<Observation>>,
    /// Publications taken before the world loaded, noted when it does.
    pending: Vec<Arc<Observation>>,
    loaded: Option<Loaded>,
    buildings: Option<BuildingPopulation>,
    /// How the side's knowledge stands the buildings, what it stands them
    /// in (building, `STATES` index), and what that was derived from.
    knowledge: Knowledge,
    fallen: Vec<(usize, usize)>,
    known_from: KnownFrom,
    statics: StaticBatches,
    /// The map's ground (`set_ground`), and its scenery's walk once placed.
    ground: Option<Ground>,
    scenery: Option<SceneryWalk>,
    scenery_quality: SceneryQuality,
    /// The learned ground's epoch and cleared cells the scenery last
    /// followed: within an epoch cells are only ever cleared, so the count
    /// names the set; a new epoch is a new ground.
    cleared_from: (u64, usize),
    /// The map's props and what the side knows of them (`place_props`).
    props: Option<PropPopulation>,
    /// The fallen bodies the scenery last read.
    felled_from: Option<Shared<FallenBody>>,
    /// The units' and the buildings' tier thresholds over their authored
    /// values (`set_detail_scale`).
    detail_scale: [f64; 2],
    world: World,
    frame: Frame,
    /// The camera the last frame was asked for.
    camera: Option<Camera>,
    /// Immutable receiver inputs and cached fit.
    shadow: Option<ShadowFit>,
    /// The side's learned ground (none without a layout), and its scars as
    /// the renderer holds them.
    learned: Option<LearnedGround>,
    scars: ScarCache,
    /// Fog of war's sight over the ground (`set_fog`); none draws no fog.
    fog: Option<Fog>,
}

/// The layout's ground grid, learned blank.
fn ground_of(layout: &str) -> Result<LearnedGround, String> {
    let layout: serde_json::Value = serde_json::from_str(layout).map_err(|e| e.to_string())?;
    let g = &layout["ground"];
    let (cols, rows) = (g["cols"].as_u64(), g["rows"].as_u64());
    let (Some(cell_m), Some(cols), Some(rows)) = (g["cellM"].as_f64(), cols, rows) else {
        return Err("the layout's ground has no grid".into());
    };
    let (cols, rows) = (cols as usize, rows as usize);
    if cols > MAX_AXIS || rows > MAX_AXIS {
        return Err("the ground grid exceeds the scars' exact Q8 coordinates".into());
    }
    Ok(LearnedGround::new(cell_m, cols, rows))
}

/// Rows of the x-ray or ghost colour (`packXray`): rgb as one integer
/// under 2²⁴, exact in a float.
fn pack_colour(c: [f64; 4]) -> f32 {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round();
    (byte(c[0]) * 65536.0 + byte(c[1]) * 256.0 + byte(c[2])) as f32
}

const IDENTITY: Joint = Joint {
    x: [1.0, 0.0, 0.0, 0.0],
    y: [0.0, 1.0, 0.0, 0.0],
    z: [0.0, 0.0, 1.0, 0.0],
};

fn joint(m: &[f32; 12]) -> Joint {
    Joint {
        x: [m[0], m[3], m[6], m[9]],
        y: [m[1], m[4], m[7], m[10]],
        z: [m[2], m[5], m[8], m[11]],
    }
}

impl Presentation {
    /// A presentation of the stream `layout` (the battle's
    /// `observation_layout`) names, at `tick_hz`; with no layout, of a world
    /// no battle publishes into (a lab's), drawn from `load_world` alone.
    pub fn new(layout: Option<&str>, tick_hz: f64) -> Result<Self, String> {
        let learned = layout.map(ground_of).transpose()?;
        Ok(Self {
            scars: ScarCache::new(learned.as_ref().map(|g| (g.cols, g.rows, g.cell_m))),
            learned,
            reader: layout.map(Reader::new).transpose()?,
            tick_hz,
            interpolator: TickInterpolator::new(tick_hz),
            latest: None,
            pending: Vec::new(),
            loaded: None,
            buildings: None,
            knowledge: Knowledge::default(),
            fallen: Vec::new(),
            known_from: KnownFrom::Unread,
            statics: StaticBatches::default(),
            ground: None,
            scenery: None,
            scenery_quality: SceneryQuality::default(),
            cleared_from: (0, 0),
            props: None,
            felled_from: None,
            detail_scale: [1.0, 1.0],
            world: World::default(),
            frame: Frame::default(),
            camera: None,
            shadow: None,
            fog: None,
        })
    }

    fn reader(&mut self) -> Result<&mut Reader, String> {
        self.reader
            .as_mut()
            .ok_or_else(|| "this presentation has no publication layout".to_owned())
    }

    /// Records still in flight are stale; the next opens a new epoch, for
    /// `side` when named. The learned ground is blank until it does.
    pub fn invalidate(&mut self, side: Option<&str>) -> Result<(), String> {
        self.reader()?.invalidate(side)?;
        if let Some(learned) = &mut self.learned {
            learned.forget();
        }
        self.latest = None;
        self.interpolator = TickInterpolator::new(self.tick_hz);
        if self.props.as_mut().is_some_and(|p| p.forget_shoved()) {
            self.statics.rewalk();
        }
        self.pending.clear();
        if let Some(loaded) = &mut self.loaded {
            loaded.forget();
        }
        Ok(())
    }

    /// Reads the next publication's words, received at wall time
    /// `arrived_s`. False for a stale record; a malformed one errs.
    pub fn push_publication(&mut self, words: &[u32], arrived_s: f64) -> Result<bool, String> {
        let Some(observation) = self.reader()?.decode(words)? else {
            return Ok(false);
        };
        if let Some(learned) = &mut self.learned {
            learned.apply(&observation.ground_patch)?;
        }
        let observation = Arc::new(observation);
        self.interpolator.push(observation.clone(), arrived_s);
        self.latest = Some(observation.clone());
        match &mut self.loaded {
            Some(loaded) => loaded.note(&observation, self.ground.as_ref()),
            None => {
                // Older than any effect lives: a battle's appearances load in seconds.
                if self.pending.len() == PENDING_MAX {
                    self.pending.remove(0);
                }
                self.pending.push(observation);
            }
        }
        Ok(true)
    }

    /// The latest publication's observation, for the game UI.
    pub fn observation(&self) -> Option<&Observation> {
        self.latest.as_deref()
    }

    /// The side's learned ground, as its publications taught it.
    pub fn learned(&self) -> Option<&LearnedGround> {
        self.learned.as_ref()
    }

    /// Whether the scars changed since the last call: what grows on them
    /// grows again.
    pub fn scars_changed(&mut self) -> bool {
        self.scars.changed(self.learned.as_mut())
    }

    /// The regions (metres, `[x0, y0, x1, y1]`) the terrain draws one by one,
    /// each after its own `prepare_scars(Some(region), true)`; none when
    /// every mark fits at once and one `prepare_scars(None, false)` serves.
    pub fn scar_regions(&self) -> Vec<[f64; 4]> {
        self.scars.regions()
    }

    /// What the renderer uploads before drawing the scars over `region`
    /// (none: the whole view), clipped to it when `clipped`.
    pub fn prepare_scars(&mut self, region: Option<[f64; 4]>, clipped: bool) -> ScarUpload {
        self.scars.prepare(self.learned.as_mut(), region, clipped)
    }

    /// The renderer holds no scars (a new one, or scars shown again): the
    /// next upload writes them all.
    pub fn resend_scars(&mut self) {
        self.scars.resend();
    }

    pub fn scar_stats(&self) -> ScarStats {
        self.scars.stats()
    }

    /// Loads what the world draws: `buildings` placed from their template
    /// art, and the battle's units and effects, `tables` being the skinned
    /// bodies' dense clip tables by name. Publications already taken are
    /// noted now.
    pub fn load_world(
        &mut self,
        input: WorldInput,
        tables: ClipTables,
        buildings: Option<PlacedBuildings>,
    ) -> Result<(), String> {
        if let Some(fit) = &mut self.shadow {
            fit.invalidate();
        }
        self.world = World::default();
        self.fallen.clear();
        self.known_from = KnownFrom::Unread;
        self.knowledge = Knowledge::default();
        self.buildings = match buildings {
            None => None,
            Some(placed) => {
                self.knowledge = Knowledge::new(placed.parts, placed.shells);
                let style = input
                    .buildings
                    .ok_or("buildings are drawn under presentation.buildings")?;
                let buildings =
                    Buildings::new(placed.library, &placed.bounds, placed.placed, &style)?;
                let rows = (0..buildings.modules.len())
                    .map(|m| {
                        let (kit, module) = buildings.module(m);
                        self.world.row(kit, module, NONE, 0, true)
                    })
                    .collect();
                let mut walk = BuildingWalk::new(
                    buildings,
                    style.lod_px_per_m,
                    style.chunk_m,
                    style.pool_records,
                );
                walk.set_lod_scale(self.detail_scale[1]);
                Some(BuildingPopulation {
                    walk,
                    rows,
                    changes: 0,
                })
            }
        };
        if let Some(walk) = &mut self.scenery {
            let world = &mut self.world;
            // Scenery draws its appearances' summer state (winter will be
            // the next biome's).
            walk.rows = DRAWS
                .iter()
                .map(|&d| {
                    let names = walk.scenery.kinds.iter();
                    names
                        .map(|name| world.drawn_row(name, "summer", NONE, 0, true, d as u32))
                        .collect()
                })
                .collect();
        }
        if let Some(props) = &mut self.props {
            let world = &mut self.world;
            props.rows = props
                .looks
                .list
                .iter()
                .map(|(name, state, draws)| world.drawn_row(name, state, NONE, 0, true, *draws))
                .collect();
        }
        self.statics.invalidate();
        self.loaded = None;
        let (azimuth, elevation) = (input.sun_azimuth, input.sun_elevation);
        let Some(input) = input.battle else {
            return Ok(());
        };
        let units = Arc::new(input.units.clone());
        let tables = Arc::new(tables);
        let presentation = EffectPresentation::from_value(&input.effects.presentation)?;
        let capacity = presentation.capacity;
        let side = input.side.clone();
        let mut inverse_binds = HashMap::new();
        for (name, a) in &units.appearances {
            if let Appearance::Skinned {
                joints,
                inverse_binds: given,
                ..
            } = a
            {
                inverse_binds.insert(name.clone(), skin::inverse_binds(joints, given));
            }
        }
        for (name, a) in &units.appearances {
            match a {
                Appearance::Skinned {
                    joints,
                    corpse_poses,
                    ..
                } => {
                    self.world.row(name, "", NONE, joints.len() as u32, false);
                    for lies in 0..corpse_poses.len() {
                        self.world.row(name, "", lies as u32, 0, false);
                    }
                }
                Appearance::Articulated { nodes, .. } => {
                    self.world.row(name, "", NONE, nodes.len() as u32, false);
                }
            }
        }
        // A cook-off draws its wreck's pieces: whole, hull, turret, debris.
        for look in input.effects.wreck_looks.values() {
            for state in ["default", "hull", "turret", "debris"] {
                self.world.row(&look.appearance, state, NONE, 0, false);
            }
        }
        let throw = 1.0 / libm::tan(elevation);
        let shadow_fall = [-libm::cos(azimuth) * throw, -libm::sin(azimuth) * throw];
        let mut loaded = Loaded {
            feed: ObservationFeed::new(&side),
            driver: PoseDriver::new(&units)?,
            drawn: DrawnModels::new(units.clone(), tables.clone(), &side),
            effect_feed: EffectFeed::new(&side),
            effects: EffectFrame::new(self.tick_hz, presentation),
            batch: EffectBatch::new(capacity),
            lights: CastLights::default(),
            cook_offs: CookOffs::new(self.tick_hz),
            transits: TransitHistory::default(),
            pieces: Vec::new(),
            shadows: Vec::new(),
            indoor: Vec::new(),
            allied: Vec::new(),
            sound: None,
            shadow_fall,
            locals: Vec::new(),
            worlds: Vec::new(),
            models: Vec::new(),
            inverse_binds,
            units,
            tables,
            input,
        };
        // Publications taken before the first load wait in `pending`. A
        // world loaded again (for another side, or another fielded kind)
        // starts from the latest, so what the side knows (the smoke over
        // what it saw destroyed) is drawn without waiting for the next.
        if self.pending.is_empty() {
            if let Some(o) = &self.latest {
                loaded.note(o, self.ground.as_ref());
            }
        }
        for o in self.pending.drain(..) {
            loaded.note(&o, self.ground.as_ref());
        }
        self.loaded = Some(loaded);
        Ok(())
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// The buildings the side knows destroyed, each with its state (`STATES`
    /// index, 1 ruin or 2 gutted), as the last frame drew them.
    pub fn fallen(&self) -> &[(usize, usize)] {
        &self.fallen
    }

    /// What the side knows (`fallen.rs`, `props.rs`), from `lab` or else
    /// the latest publication's known props, taken again only when that
    /// changed: the props it knows, and the state it knows each building in.
    /// A building known in a state its template has no rows for is refused
    /// by name, and the buildings stand as they were.
    fn know(&mut self, lab: Option<&[Known]>) -> Result<(), String> {
        if self.buildings.is_none() && self.props.is_none() {
            return Ok(());
        }
        let known: Vec<KnownBody> = match lab {
            Some(known) => {
                if matches!(&self.known_from, KnownFrom::Lab(last) if last == known) {
                    return Ok(());
                }
                self.known_from = KnownFrom::Lab(known.to_vec());
                known.iter().map(KnownBody::lab).collect()
            }
            None => {
                let group = self.latest.as_ref().map(|o| o.known_props.clone());
                let ptr = |g: &Option<Shared<KnownProp>>| g.as_ref().map(Arc::as_ptr);
                if matches!(&self.known_from, KnownFrom::Published(last) if ptr(last) == ptr(&group))
                {
                    return Ok(());
                }
                self.known_from = KnownFrom::Published(group.clone());
                group.map_or_else(Vec::new, |g| g.iter().map(KnownBody::published).collect())
            }
        };
        if let Some(props) = &mut self.props {
            // A building's part is its building's state to draw.
            let knowledge = &self.knowledge;
            let standing = known
                .iter()
                .filter(|k| !k.authored.is_some_and(|a| knowledge.is_part(a)))
                .cloned()
                .collect();
            if props.set_known(standing) {
                self.statics.rewalk();
            }
        }
        let Some(buildings) = &mut self.buildings else {
            return Ok(());
        };
        let fallen = self.knowledge.fallen(
            known
                .iter()
                .map(|k| (k.authored, k.kind.as_str(), k.destroyed)),
        );
        if fallen == self.fallen {
            return Ok(());
        }
        if let Some(&b) = buildings.walk.set_fallen(&fallen).first() {
            let state = fallen.iter().find(|f| f.0 == b).map_or(0, |f| f.1);
            return Err(format!(
                "state.missing: template \"{}\" has no \"{}\" rows",
                buildings.walk.buildings.template(b),
                STATES[state]
            ));
        }
        self.fallen = fallen;
        self.statics.invalidate();
        Ok(())
    }

    /// The buildings' walk as the last frame left it (`BuildingWalk::stats`);
    /// none without buildings.
    pub fn building_stats(&self) -> Option<&[usize]> {
        self.buildings.as_ref().map(|b| &b.walk.stats[..])
    }

    /// Draws the units' tiers at `units` and the buildings' at `buildings`
    /// times their authored thresholds (a graphics quality; 1 as authored).
    pub fn set_detail_scale(&mut self, units: f64, buildings: f64) {
        self.detail_scale = [units, buildings];
        if let Some(b) = &mut self.buildings {
            b.walk.set_lod_scale(buildings);
        }
        self.statics.invalidate();
    }

    /// A skinned row's body as it holds still (`World.appearances`): a
    /// corpse row as it lies (its `corpse_poses` entry), a body row in its
    /// far pose (what its impostor card shows). Each joint's bind-to-model
    /// motion: a renderer skins the body's mesh by it once, when it loads
    /// the row (`posedMesh`), and draws that mesh where the frame's
    /// instance takes the identity. None for a rigid row, or before a world
    /// loads.
    pub fn rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        let loaded = self.loaded.as_ref()?;
        let r = self.world.appearances.get(row as usize)?;
        let name = self
            .world
            .strings
            .get(r.name as usize..(r.name + r.name_len) as usize)?;
        let Some(Appearance::Skinned {
            joints,
            clips,
            table,
            far_pose,
            corpse_poses,
            ..
        }) = loaded.units.appearances.get(name)
        else {
            return None;
        };
        let pose = match r.lies {
            NONE => far_pose,
            lies => corpse_poses.get(lies as usize)?,
        };
        let mut worlds = Vec::new();
        let table = loaded.tables.get(table)?;
        skin::joint_worlds(
            joints,
            clips,
            table,
            &pose.clip,
            pose.phase,
            None,
            &mut worlds,
        );
        let ibs = &loaded.inverse_binds[name];
        Some(
            worlds
                .iter()
                .zip(ibs)
                .map(|(w, ib)| joint(&skin::affine(&skin::mul(w, ib))))
                .collect(),
        )
    }

    /// The presentation time at wall time `wall_s` (live only).
    pub fn clock(&self, wall_s: f64) -> Option<f64> {
        self.interpolator.time(wall_s)
    }

    /// This frame's effect instances, in draw order.
    pub fn effects(&self) -> &[Effect] {
        self.loaded.as_ref().map_or(&[], |l| {
            bytemuck::cast_slice(&l.batch.data[..l.batch.count * EFFECT_FLOATS])
        })
    }

    /// This frame's cast lights, chosen for the view.
    pub fn lights(&self) -> &[CastLight] {
        self.loaded.as_ref().map_or(&[], |l| {
            let count = l.lights.packed[0] as usize;
            bytemuck::cast_slice(&l.lights.packed[4..4 + count * 8])
        })
    }

    /// Each of a scripted run's publications' launches, JSON (`[[Shot]]`):
    /// what a lab's sound hears without a battle, by the effects' own
    /// derivation.
    pub fn launches_json(publications: &str) -> Result<String, String> {
        let publications: Vec<EffectPublication> =
            serde_json::from_str(publications).map_err(|e| e.to_string())?;
        let mut tracker = crate::effects::launches::LaunchTracker::default();
        let shots: Vec<Vec<Shot>> = publications
            .iter()
            .map(|p| tracker.note(p, false, None).iter().map(Shot::of).collect())
            .collect();
        serde_json::to_string(&shots).map_err(|e| e.to_string())
    }

    /// The last publication as sound hears it: what the effects took of it,
    /// the launches it shows, the side's hearing cues and the shouts it
    /// hears in the open; none before one.
    pub fn heard(&self) -> Option<Heard<'_>> {
        let (effects, launches) = self.loaded.as_ref()?.sound.as_ref()?;
        let latest = self.latest.as_ref()?;
        Some(Heard {
            effects,
            launches,
            audible: latest.audible.iter().map(Cue::from).collect(),
            shouts: latest.shouts.iter().map(Shout::from).collect(),
        })
    }

    /// What the last frame draws moving, as sound hears it: its emitters,
    /// each vehicle as its type's vehicle class.
    pub fn motion(&self) -> Motion {
        let types = self.loaded.as_ref().map(|l| &l.units.types);
        Motion::of(&self.frame.emitters, |class| {
            types?
                .values()
                .nth(class as usize)?
                .vehicle_class
                .as_deref()
        })
    }

    /// Where sound listens for the last frame's camera, `share` of the way
    /// from what it looks at to its eye; none before a frame.
    pub fn listener(&self, share: f64) -> Option<Listener> {
        let c = self.camera.as_ref()?;
        Some(Listener::of_camera(
            c.eye,
            c.target,
            c.up.map(|v| v as f32),
            share,
        ))
    }

    /// The last frame's presentation time.
    pub fn clock_of_frame(&self) -> f64 {
        self.frame.clock
    }

    /// The last frame asked for.
    pub fn last_frame(&self) -> &Frame {
        &self.frame
    }

    /// Where the last frame draws the muzzle of the presented side's vehicle
    /// `unit`'s mount `mount`, where its flash goes; None when no rig draws it.
    pub fn drawn_muzzle(&mut self, unit: f64, mount: usize) -> Option<Vec3> {
        let loaded = self.loaded.as_mut()?;
        loaded
            .drawn
            .muzzle(crate::seed::side_key(unit, true), mount, None)
    }

    /// The map's ground, resolved from `source` under `biome` (its region's
    /// finish) and the fixture's tuning; its scenery is placed by
    /// `place_scenery` and drawn from the next `load_world`.
    pub fn set_ground(
        &mut self,
        source: Source,
        biome: Biome,
        tuning: Tuning,
    ) -> Result<(), String> {
        let ground = Ground::new(source, biome, tuning)?;
        // A region's scars load as far past it as a crater's bowl moves them.
        self.scars.set_reach(ground.scar_reach_m());
        // Sight is cut by the new ground.
        if let Some(fog) = &mut self.fog {
            fog.sight = FogSight::new(ground.tuning.fog_geometry.clone())?;
            fog.sight.set_world(
                SightWorld::of(&ground.source, fog.sensors),
                ground.source.foliage.clone(),
            )?;
            fog.from = None;
            fog.known_from = None;
            fog.dirty = true;
        }
        self.ground = Some(ground);
        self.scenery = None;
        self.props = None;
        self.statics.invalidate();
        Ok(())
    }

    pub fn ground(&self) -> Option<&Ground> {
        self.ground.as_ref()
    }

    /// The ground's mutably (a lab growing grass by other rules).
    pub fn ground_mut(&mut self) -> Option<&mut Ground> {
        self.ground.as_mut()
    }

    /// Places the ground's scenery, given each of its kinds' finest mesh
    /// (`Ground::scenery_kinds` order) in the renderer's world vertex
    /// (`ground::mesh::VERTEX_FLOATS`, 1 in the last float for foliage):
    /// their sizes, a tree's trunk and crown, and its stump (`kinds`) are
    /// read off it. Drawn from the next `load_world`.
    pub fn place_scenery(&mut self, finest: &[&[f32]]) -> Result<(), String> {
        let ground = self
            .ground
            .as_ref()
            .ok_or("scenery stands on a ground: set_ground first")?;
        let names = ground.scenery_kinds();
        if finest.len() != names.len() {
            return Err(format!(
                "scenery: {} kinds' meshes for {} kinds",
                finest.len(),
                names.len()
            ));
        }
        let trees = &ground.biome.trees;
        let kinds = names
            .iter()
            .zip(finest)
            .map(|(name, tier)| {
                let tree = trees.species.iter().any(|s| &s.appearance == name);
                kinds::kind(tier, tree.then_some(&trees.felled)).map_err(|e| format!("{name}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sizes: Vec<_> = kinds.iter().map(|k| k.size).collect();
        let trunks = kinds.iter().map(|k| k.trunk).collect();
        let mut walk = SceneryWalk::new(ground.scenery(&sizes), sizes, trunks);
        walk.stump_meshes = kinds.into_iter().map(|k| k.stump).collect();
        walk.set_quality(self.scenery_quality);
        self.scenery = Some(walk);
        self.felled_from = None;
        self.cleared_from = (0, 0);
        Ok(())
    }

    /// Places the map's props (the world's exported `records` under its
    /// `layout`) drawn from `input`'s art; drawn from the next `load_world`.
    pub fn place_props(
        &mut self,
        records: &[f32],
        layout: &str,
        input: PropInput,
    ) -> Result<(), String> {
        let layout = PropLayout::parse(layout)?;
        let props = crate::static_world::props::map_props(records, &layout);
        self.props = Some(PropPopulation::new(PropArt::new(layout, input)?, props));
        self.known_from = KnownFrom::Unread;
        Ok(())
    }

    /// The props' bodies and models (`PropPopulation::stats`); none without props.
    pub fn prop_stats(&self) -> Option<[usize; 6]> {
        self.props.as_ref().map(PropPopulation::stats)
    }

    /// The props placed (none without them).
    pub fn props(&self) -> Option<&PropPopulation> {
        self.props.as_ref()
    }

    /// The mesh `World.appearances` row `row` draws, when it is a felled
    /// tree's stump (`Appearance.draws` 6): cut from its kind's trunk, in
    /// the renderer's world vertex. None for any other row.
    pub fn stump_mesh(&self, row: u32) -> Option<&[f32]> {
        let walk = self.scenery.as_ref()?;
        let stumps = walk.rows.get(Draws::Stump as usize - 1)?;
        let kind = stumps.iter().position(|&r| r == row)?;
        walk.stump_meshes.get(kind)?.as_deref()
    }

    /// The player's scenery settings (`walk::Quality`).
    pub fn set_scenery_quality(&mut self, quality: SceneryQuality) {
        self.scenery_quality = quality;
        if let Some(walk) = &mut self.scenery {
            walk.set_quality(quality);
            self.statics.rewalk();
        }
    }

    /// The scenery follows the ground the side has learned cleared: its
    /// shrubs and dressing there leave.
    fn follow_cleared(&mut self) {
        let (Some(walk), Some(learned)) = (&mut self.scenery, &self.learned) else {
            return;
        };
        let from = (learned.epoch.to_bits(), learned.cleared_count());
        if from == self.cleared_from {
            return;
        }
        self.cleared_from = from;
        let runs = learned.cleared_runs();
        walk.set_cleared(Cleared::from_runs(
            &runs,
            learned.cols,
            learned.rows,
            learned.cell_m,
        ));
        self.statics.rewalk();
    }

    /// The window of tiles the grass grows over for the last frame's camera,
    /// and the metres a pixel spans per metre of distance there; none before
    /// a frame or without a ground.
    pub fn grass_window(&self) -> Option<(Window, f64)> {
        let (ground, c) = (self.ground.as_ref()?, self.camera.as_ref()?);
        let metres_per_px = metres_per_pixel(c.fov_y, f64::from(c.size[1].max(1)));
        Some((ground.grass_window(c.eye, metres_per_px), metres_per_px))
    }

    /// Writes the last frame's fields into `GrassParams`' `words` for
    /// `window` (`grass_window`'s): its view's side planes, its eye and
    /// pixel span, and the window.
    pub fn grass_frame_params(&self, words: &mut [u32], window: &Window) {
        let Some(c) = &self.camera else { return };
        let metres_per_px = metres_per_pixel(c.fov_y, f64::from(c.size[1].max(1)));
        let view_proj: &[f32; 16] = self.frame.camera.view_proj.first_chunk().unwrap();
        frame_params(words, view_proj, c.eye, metres_per_px, window);
    }

    pub fn shadow_stats(&self) -> ReceiverStats {
        self.shadow.as_ref().map_or_default(|fit| fit.stats)
    }

    /// Install local receiver terrain and light; none disables shadows.
    pub fn set_shadow(&mut self, shadow: Option<(LightSettings, Arc<TerrainPartition>)>) {
        self.shadow = shadow.map(ShadowFit::new);
        self.statics.rewalk();
    }

    /// Wanted dressing cells are still to be laid: frame again before
    /// trusting a picture.
    pub fn scenery_pending(&self) -> bool {
        self.scenery.as_ref().is_some_and(|w| w.pending)
    }

    /// A felled tree still falls.
    pub fn scenery_falling(&self) -> bool {
        self.scenery.as_ref().is_some_and(|w| w.falling)
    }

    /// Scenery populations, query visits and known-fall work.
    pub fn scenery_stats(&self) -> Option<[usize; 8]> {
        self.scenery.as_ref().map(SceneryWalk::stats)
    }

    /// The trees the latest publication says fell, falling on `time`.
    fn follow_felled(&mut self, time: f64) {
        let Some(walk) = &mut self.scenery else {
            return;
        };
        let fallen = self.latest.as_ref().map(|o| o.fallen_bodies.clone());
        let same = match (&fallen, &self.felled_from) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            let felled = fallen.iter().flat_map(|f| f.iter()).filter_map(|f| {
                Some(FelledTree {
                    prop: f.prop? as u32,
                    toward: f.toward,
                    fell_at: (f.tick - 1.0) / self.tick_hz,
                })
            });
            if walk.set_felled(felled.collect()) {
                self.statics.rewalk();
            }
            self.felled_from = fallen;
        }
        if walk.set_clock(time) || walk.pending {
            self.statics.rewalk();
        }
    }

    /// What to draw at presentation time `time` for `camera`, for a renderer
    /// at `generation`: a number it bumps whenever it holds no static batches
    /// (a new GPU frame, a new world, a jump), so a new one is written every
    /// static batch. The camera's view, its uniform and the sun's cascades
    /// come with it, and the scenery has followed the ground the side has
    /// learned cleared by the publications pushed so far. Errs when the side
    /// knows a building in a state its template lacks.
    pub fn frame(
        &mut self,
        time: f64,
        camera: &Camera,
        ui: &Ui,
        generation: u32,
    ) -> Result<&Frame, String> {
        self.know(ui.known)?;
        self.camera = Some(*camera);
        let f = &mut self.frame;
        f.index += 1;
        f.clock = time;
        f.batches.clear();
        f.instances.clear();
        f.palettes.clear();
        f.palettes.push(IDENTITY);
        f.picks.clear();
        f.emitters.clear();
        f.counts = Counts::default();
        f.light_wrap = 0.0;
        let view = View::new(camera);
        f.view = view.record(camera);
        cameras::cameras(f, &view, &mut self.shadow, &self.ground, camera, time);
        self.follow_felled(time);
        self.follow_cleared();
        // A wreck the cook-off draws moving is left out of the props until
        // it lies still.
        if let (Some(props), Some(loaded)) = (&mut self.props, &self.loaded) {
            if props.set_moving(loaded.cook_offs.moving_props().collect()) {
                self.statics.rewalk();
            }
        }
        let sample = self.interpolator.frame(time);
        // A body the side watches shoved moves every frame, as units do.
        if let Some(props) = &mut self.props {
            if props.set_shoved(sample.map_or(&[], |s| &s.shoved)) {
                self.statics.rewalk();
            }
        }
        let mut scenery = match (&mut self.scenery, &self.ground) {
            (Some(walk), Some(ground)) if !walk.rows.is_empty() => {
                Some(SceneryPopulation { walk, ground })
            }
            _ => None,
        };
        let mut populations: Vec<&mut dyn Population> = Vec::new();
        if let Some(b) = &mut self.buildings {
            populations.push(b);
        }
        if let Some(s) = &mut scenery {
            populations.push(s);
        }
        if let Some(p) = self.props.as_mut().filter(|p| !p.rows.is_empty()) {
            populations.push(p);
        }
        let f = &mut self.frame;
        self.statics
            .write(&mut populations, camera, &view, generation, f);
        let (Some(loaded), Some(sample)) = (self.loaded.as_mut(), sample) else {
            return Ok(&self.frame);
        };
        f.tick = sample.observation.tick as u32;
        loaded.frame(
            sample,
            &view,
            ui,
            &self.world,
            self.ground.as_ref(),
            self.detail_scale[0],
            f,
        );
        Ok(&self.frame)
    }
}

impl Loaded {
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        sample: &FrameSample,
        view: &View,
        ui: &Ui,
        world: &World,
        ground: Option<&Ground>,
        unit_scale: f64,
        f: &mut Frame,
    ) {
        let Loaded {
            input,
            units,
            tables,
            inverse_binds,
            feed,
            driver,
            drawn,
            effects,
            batch,
            lights,
            cook_offs,
            transits,
            pieces,
            shadows,
            indoor,
            allied,
            shadow_fall,
            worlds,
            locals,
            models,
            ..
        } = self;
        let time = sample.time;
        let poses = driver.update(feed.frame(sample, units, &input.effects), units, ground);
        drawn.update(poses);
        let rotors = drawn.rotors();
        let side: &str = &input.side;
        let height = |x: f64, y: f64| ground.map_or(0.0, |g| g.source.grid.height(x, y));
        // Edge transits are own-side aircraft records reconstructed from the
        // publication, drawn as a live aircraft of their kind is.
        let transit_poses = transits.tracks(time * input.tick_hz, input.tick_hz, &units.transit);
        let flying = poses
            .vehicles
            .iter()
            .filter(|v| v.airborne)
            .map(|v| (&*v.kind, &*v.side, v.unit, v.position, v.yaw))
            .chain(
                transit_poses
                    .iter()
                    .map(|t| (&*t.kind, side, 0.0, t.position, t.heading)),
            );
        let style = effects.presentation().high_air_shadow;
        shadows.clear();
        shadows.extend(
            flying
                .filter(|(kind, ..)| units.types.get(*kind).is_some_and(|t| t.high_air()))
                .filter_map(|(kind, on, unit, at, yaw)| {
                    let name = units.resolve(kind, on, unit, 0, None, None)?;
                    let Some(Appearance::Articulated { plan, .. }) = units.appearances.get(name)
                    else {
                        return None;
                    };
                    Some(high_air_ground_shadow(at, yaw, *plan, height, &style))
                }),
        );
        let wind = ground.map(|g| &g.biome.wind);
        effects.build(time, batch, Some(drawn), &rotors, shadows, wind);
        let counts = effects.counts(time, batch);
        f.light_wrap = batch.lights.wrap as f32;
        lights.pack(&batch.lights, view);
        f.counts.running = counts.running.map(|n| n as u32);
        f.counts.lights = counts.lights as u32;
        f.counts.dropped = batch.dropped as u32;

        let mut detail = units.detail;
        detail.lod_px = detail.lod_px.map(|v| v * unit_scale);
        let xray = input.xray;
        let o = &sample.observation;
        indoor.clear();
        indoor.extend(
            o.own
                .iter()
                .filter(|u| u.hold.as_ref().is_some_and(|h| &*h.phase != "entering"))
                .map(|u| u.id),
        );
        allied.clear();
        if let Some(player) = ui.player {
            allied.extend(
                o.own
                    .iter()
                    .filter(|u| u.owner.is_some_and(|owner| owner != f64::from(player)))
                    .map(|u| u.id),
            );
        }
        let colour_of = |unit_side: &str, unit: f64| -> [f64; 4] {
            if !ui.xray || unit_side != side || indoor.contains(&unit) {
                [0.0; 4]
            } else if allied.contains(&unit) {
                xray.ally
            } else if ui.selected.contains(&(unit as u32)) {
                xray.selected
            } else {
                xray.own
            }
        };
        let tint = |s: &str| units.tints.get(s).map_or([1.0; 3], |t| t.map(|v| v as f32));
        let row = |name: &str, state: &str, lies: u32| {
            world.rows[&(name.to_owned(), state.to_owned(), lies, 0)]
        };
        models.clear();
        // Soldiers off screen, as the gates count bodies drawn, carded or culled.
        let mut culled = 0;
        let model = |row: u32,
                     tier: i32,
                     pass: u32,
                     at: Vec3,
                     yaw: f64,
                     tint: [f32; 3],
                     colour: [f64; 4],
                     palette: u32,
                     scroll: [f64; 2]| Model {
            row,
            tier: tier as u32,
            pass,
            at,
            yaw,
            tint,
            scale: [1.0; 3],
            colour,
            palette,
            scroll,
            fog: model_fog(false, false, false),
        };

        // Soldiers, living and dying: a body's far bounds tier him.
        for s in &poses.soldiers {
            let Some(name) = units.resolve(
                &s.kind,
                &s.side,
                s.soldier,
                s.slot,
                s.operator_mount,
                s.active_mount,
            ) else {
                continue;
            };
            f.counts.soldiers += 1;
            let far = bounds(units, name, 0);
            let tier = model_detail(&detail, view, s.position, far.size, far.radius, true, None);
            if tier == CULLED {
                culled += 1;
                continue;
            }
            let palette = if tier == IMPOSTOR {
                0
            } else {
                pose_body(
                    units,
                    tables,
                    inverse_binds,
                    worlds,
                    &mut f.palettes,
                    name,
                    &s.clip,
                    s.phase,
                    s.blend.as_ref(),
                    s.kick,
                )
            };
            models.push(model(
                row(name, "", NONE),
                tier,
                0,
                s.position,
                s.facing,
                tint(&s.side),
                colour_of(&s.side, s.unit),
                palette,
                [0.0; 2],
            ));
        }
        // Where an aircraft's shadow lies, and the pass that draws it: a
        // low-air airframe's shadow lands its height along the sun's fall; a
        // high-air one, drawn in its own pass, casts none, and its shadow
        // lies on the ground beneath it.
        let aircraft_shadow_at = |kind: &str, p: Vec3| {
            let high = units.types.get(kind).is_some_and(|t| t.high_air());
            let ground_z = height(p[0], p[1]);
            if high {
                ([p[0], p[1], ground_z], HIGH_PASS)
            } else {
                (aircraft_shadow(p, ground_z, *shadow_fall), VIEW_PASS)
            }
        };
        // Vehicles and aircraft; an aircraft draws while it or its shadow
        // is in view.
        for v in &poses.vehicles {
            let Some(name) = units.resolve(&v.kind, &v.side, v.unit, 0, None, None) else {
                continue;
            };
            if v.airborne {
                f.counts.aircraft += 1;
            } else {
                f.counts.vehicles += 1;
            }
            let far = bounds(units, name, 0);
            let (shadow, pass) = if v.airborne {
                let (at, pass) = aircraft_shadow_at(&v.kind, v.position);
                (Some(at), pass)
            } else {
                (None, VIEW_PASS)
            };
            let tier = model_detail(
                &detail, view, v.position, far.size, far.radius, false, shadow,
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = pose_vehicle(
                units,
                locals,
                worlds,
                &mut f.palettes,
                name,
                &v.articulation,
                v.motion.as_ref(),
            );
            models.push(model(
                row(name, "", NONE),
                tier,
                pass,
                v.position,
                v.yaw,
                tint(&v.side),
                colour_of(&v.side, v.unit),
                palette,
                scroll,
            ));
        }
        // Edge transits use the same appearance, articulation and shadow as a
        // live aircraft, but have no unit identity for selection or cards.
        for t in &transit_poses {
            let Some(name) = units.resolve(&t.kind, side, 0.0, 0, None, None) else {
                continue;
            };
            f.counts.aircraft += 1;
            let far = bounds(units, name, 0);
            let (shadow, pass) = aircraft_shadow_at(&t.kind, t.position);
            let tier = model_detail(
                &detail,
                view,
                t.position,
                far.size,
                far.radius,
                false,
                Some(shadow),
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = pose_vehicle(
                units,
                locals,
                worlds,
                &mut f.palettes,
                name,
                &Articulation::default(),
                None,
            );
            models.push(model(
                row(name, "", NONE),
                tier,
                pass,
                t.position,
                t.heading,
                tint(side),
                [0.0; 4],
                palette,
                scroll,
            ));
        }
        // Corpses lying, and those the cap pushed out sinking away.
        let lying = poses
            .corpses
            .iter()
            .map(|c| (c, 0.0, false))
            .chain(poses.fading.iter().map(|f| (&f.corpse, f.sink, true)));
        for (c, sink, fading) in lying {
            let Some(name) =
                units.resolve(&c.kind, &c.side, c.soldier, c.slot, c.operator_mount, None)
            else {
                continue;
            };
            if fading {
                f.counts.fading += 1;
            } else {
                f.counts.corpses += 1;
            }
            let at = [c.position[0], c.position[1], c.position[2] - sink];
            let size = bounds(units, name, 1 + c.lies);
            let tier = model_detail(&detail, view, at, size.size, size.radius, true, None);
            if tier == CULLED {
                continue;
            }
            // Remains are seen or unseen whole, as the ground under them.
            models.push(Model {
                fog: model_fog(true, false, false),
                ..model(
                    row(name, "", c.lies as u32),
                    tier,
                    0,
                    at,
                    c.yaw,
                    tint(&c.side),
                    [0.0; 4],
                    0,
                    [0.0; 2],
                )
            });
        }
        // A cook-off's moving wreck and its thrown debris; a piece in
        // motion is never a card, and reaches as far as it has moved.
        let looks = &input.effects.wreck_looks;
        pieces.clear();
        cook_offs.models(
            &poses.vehicles,
            time,
            units,
            looks,
            &effects.presentation().cook_off,
            pieces,
        );
        for piece in pieces.iter() {
            let size = match piece.state {
                Some(_) => looks
                    .values()
                    .find(|l| *l.appearance == *piece.appearance)
                    .map_or_else(Bounds::default, |l| bounds_size(l.bounds)),
                None => bounds(units, &piece.appearance, 0),
            };
            let grow = piece.scale[0].max(piece.scale[1]).max(piece.scale[2]);
            let moved = piece.motion.map_or(0.0, |m| {
                grow * (m[12] * m[12] + m[13] * m[13] + m[14] * m[14]).sqrt()
            });
            let tier = model_detail(
                &detail,
                view,
                piece.position,
                size.size * grow,
                size.radius * grow + moved,
                false,
                None,
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = match (&piece.articulation, &piece.motion) {
                (Some(a), motion) => pose_vehicle(
                    units,
                    locals,
                    worlds,
                    &mut f.palettes,
                    &piece.appearance,
                    a,
                    motion.as_ref(),
                ),
                (None, Some(m)) => {
                    f.palettes.push(joint(&skin::affine(m)));
                    (f.palettes.len() as u32 - 1, [0.0; 2])
                }
                (None, None) => (0, [0.0; 2]),
            };
            let tint = piece.tint.map_or([1.0; 3], |t| t.map(|v| v as f32));
            let mut m = model(
                row(&piece.appearance, piece.state.unwrap_or(""), NONE),
                tier,
                0,
                piece.position,
                piece.yaw,
                tint,
                [0.0; 4],
                palette,
                scroll,
            );
            m.scale = piece.scale.map(|v| v as f32);
            // A thrown piece is the world's; the whole wreck still moving is
            // posed as a unit is.
            m.fog = model_fog(false, piece.state.is_some(), false);
            models.push(m);
        }
        // Ghosts: where the UI's units will stand, at rest, in their colour.
        for g in ui.ghosts {
            let Some(_) = units.types.get(&g.kind) else {
                continue;
            };
            let Some(name) = units.resolve(&g.kind, side, g.id, g.slot, None, None) else {
                continue;
            };
            let at = [g.x, g.y, g.z];
            let far = bounds(units, name, 0);
            let tier = model_detail(&detail, view, at, far.size, far.radius, false, None);
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = match units.appearances.get(name) {
                Some(Appearance::Skinned { far_pose, .. }) => (
                    pose_body(
                        units,
                        tables,
                        inverse_binds,
                        worlds,
                        &mut f.palettes,
                        name,
                        &far_pose.clip,
                        far_pose.phase,
                        None,
                        Kick::default(),
                    ),
                    [0.0; 2],
                ),
                _ => pose_vehicle(
                    units,
                    locals,
                    worlds,
                    &mut f.palettes,
                    name,
                    &Articulation::default(),
                    None,
                ),
            };
            let colour = match g.look {
                1 => xray.selected,
                2 => input.blocked,
                _ => xray.own,
            };
            models.push(model(
                row(name, "", NONE),
                tier,
                GHOST_PASS,
                at,
                g.yaw,
                [1.0; 3],
                colour,
                palette,
                scroll,
            ));
        }
        f.counts.culled = culled;
        batches(models, f);
        picks::picks(sample, units, input, f);
        emitters(o, poses, side, units, f);
    }
}

/// The models as batches: one per drawable, tier and pass, in that order.
/// A model in view casts a tier coarser; a card, a ghost and a high-air
/// model cast nothing (`batch_caster`).
fn batches(models: &mut [Model], f: &mut Frame) {
    let run_of = |m: &Model| (m.row, m.tier, m.pass);
    models.sort_by_key(run_of);
    for run in models.chunk_by(|a, b| run_of(a) == run_of(b)) {
        let m = &run[0];
        let caster = batch_caster(m.tier, m.pass);
        let cascade_mask = if caster == NONE {
            0
        } else {
            crate::shadow::ALL_CASCADES
        };
        f.batches.push(Batch {
            key: crate::view::batch_key(m.row, m.tier, m.pass, cascade_mask),
            appearance: m.row,
            tier: m.tier,
            pass: m.pass,
            caster,
            cascade_mask,
            fog: m.fog,
            version: f.index,
            first: f.instances.len() as u32,
            count: run.len() as u32,
            pad: [0; 2],
        });
        let fract = |v: f64| (v - v.floor()) as f32;
        f.instances.extend(run.iter().map(|m| Instance {
            placement: [m.at[0] as f32, m.at[1] as f32, m.at[2] as f32, m.yaw as f32],
            data: [
                m.palette as f32,
                fract(m.scroll[0]),
                fract(m.scroll[1]),
                pack_colour(m.colour),
            ],
            tint: [m.tint[0], m.tint[1], m.tint[2], -1.0],
            scale: [m.scale[0], m.scale[1], m.scale[2], m.colour[3] as f32],
        }));
    }
}

/// A body in a clip at a phase, crossfaded and kicked: its palette's first joint.
#[allow(clippy::too_many_arguments)]
fn pose_body(
    units: &UnitsSection,
    tables: &ClipTables,
    inverse_binds: &HashMap<String, Vec<Mat4>>,
    worlds: &mut Vec<Mat4>,
    palettes: &mut Vec<Joint>,
    name: &str,
    clip: &str,
    phase: f64,
    blend: Option<&ClipBlend>,
    kick: Kick,
) -> u32 {
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        ..
    }) = units.appearances.get(name)
    else {
        return 0;
    };
    let Some(table) = tables.get(table) else {
        return 0;
    };
    skin::drawn_worlds(joints, clips, table, clip, phase, blend, kick, worlds);
    let first = palettes.len() as u32;
    let ibs = &inverse_binds[name];
    palettes.extend(
        worlds
            .iter()
            .zip(ibs)
            .map(|(w, ib)| joint(&skin::affine(&skin::mul(w, ib)))),
    );
    first
}

/// A vehicle articulated, its whole model moved by `motion` (an aircraft's
/// lean, a hull's kick): its palette's first joint and its track scrolls.
fn pose_vehicle(
    units: &UnitsSection,
    locals: &mut Vec<Trs>,
    worlds: &mut Vec<Mat4>,
    palettes: &mut Vec<Joint>,
    name: &str,
    articulation: &Articulation,
    motion: Option<&Mat4>,
) -> (u32, [f64; 2]) {
    let Some(Appearance::Articulated { nodes, rig, .. }) = units.appearances.get(name) else {
        return (0, [0.0; 2]);
    };
    *locals = rest_locals(nodes);
    articulate(locals, nodes, rig, articulation);
    skin::node_worlds(nodes, locals, motion, worlds);
    let first = palettes.len() as u32;
    palettes.extend(worlds.iter().map(|w| joint(&skin::affine(w))));
    (first, track_scroll(rig, articulation))
}

/// What audio hears move this frame: each drawn vehicle's tracks and
/// turret, and each drawn soldier.
fn emitters(o: &Observation, poses: &PoseFrame, side: &str, units: &UnitsSection, f: &mut Frame) {
    let reversing = |unit: f64, own: bool| {
        if own {
            o.own.iter().any(|u| u.id == unit && u.reversing)
        } else {
            o.identified.iter().any(|e| e.id == unit && e.reversing)
        }
    };
    for v in &poses.vehicles {
        let own = &*v.side == side;
        let a = v.articulation;
        f.emitters.push(Emitter {
            unit: v.unit as u32,
            member: NONE,
            class: units
                .types
                .keys()
                .position(|k| **k == *v.kind)
                .map_or(NONE, |i| i as u32),
            flags: u32::from(!own) | if reversing(v.unit, own) { 2 } else { 0 },
            position: [
                v.position[0] as f32,
                v.position[1] as f32,
                v.position[2] as f32,
                0.0,
            ],
            motion: [
                a.travel_l as f32,
                a.travel_r as f32,
                a.turret_yaw as f32,
                0.0,
            ],
        });
    }
    for s in &poses.soldiers {
        f.emitters.push(Emitter {
            unit: s.unit as u32,
            member: s.soldier as u32,
            class: NONE,
            flags: u32::from(&*s.side != side),
            position: [
                s.position[0] as f32,
                s.position[1] as f32,
                s.position[2] as f32,
                0.0,
            ],
            motion: [0.0; 4],
        });
    }
}

/// A launch as sound reads it.
#[derive(Serialize)]
struct Shot {
    shooter: f64,
    mount: usize,
    soldier: Option<f64>,
    x: f64,
    y: f64,
    z: f64,
    kind: Arc<str>,
}

impl Shot {
    fn of(l: &Launch) -> Self {
        Self {
            shooter: l.shooter,
            mount: l.mount,
            soldier: l.soldier,
            x: l.at[0],
            y: l.at[1],
            z: l.at[2],
            kind: l.kind.clone(),
        }
    }
}

/// An appearance's drawn size: its far pose (`k` 0) or its `k − 1`th way of lying.
fn bounds(units: &UnitsSection, name: &str, k: usize) -> Bounds {
    match units.appearances.get(name) {
        Some(Appearance::Skinned { bounds, .. } | Appearance::Articulated { bounds, .. }) => {
            bounds.get(k).copied().unwrap_or_default()
        }
        None => Bounds::default(),
    }
}
