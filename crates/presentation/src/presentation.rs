//! One side's publications into a frame of poses, effects, batches and palettes.
pub mod catalog;
pub mod lab;
pub use catalog::World;
mod ground;
mod sound;
mod static_frame;
use sound::emitters;
use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use sim::publication::reader::{FallenBody, KnownProp, Observation, Reader, Shared};

use crate::effects::cook_offs::{CookOffs, PlacedModel};
use crate::effects::exhaust::DrawnJet;
use crate::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use crate::effects::high_air_shadow::GroundShadow;
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
    Appearance as Row, Batch, Camera as CameraRecord, CastLight, Counts, Effect, Emitter,
    GrassParams, Instance, Joint, Pick, View as ViewRecord, GHOST_PASS, HIGH_PASS, NONE, VIEW_PASS,
};
use crate::scars::{LearnedGround, ScarCache, ScarStats, ScarUpload, MAX_AXIS};
use crate::shadow::{aircraft_shadow, ReceiverStats, ShadowFit, SunShadowFrame};
use crate::static_world::batches::StaticBatches;
use crate::static_world::buildings::{Buildings, PlacedBuildings};
use crate::static_world::fallen::{Knowledge, Known};
use crate::static_world::ground::biome::Biome;
use crate::static_world::ground::grass::{frame_params, metres_per_pixel, Window};
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
use crate::units::pose::{Articulation, ClipBlend, Kick};
use crate::units::skin::{self, Mat4};
use crate::view::{model_detail, model_fog, Camera, View, CULLED, IMPOSTOR};
use crate::world::{
    bounds_size, Appearance, BattleInput, Bounds, Trs, UnitsSection, Vec3, WorldInput,
};

mod aircraft;
mod batching;
use batching::Model;
mod cameras;
mod frame;
mod picks;
mod posing;
use posing::{pose_body, pose_vehicle, Scratch, UnitArt};
mod probe;
pub mod showcase;
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
    /// How much of its look's opacity it keeps, 0 to 1, as it fades out;
    /// absent, all of it.
    #[serde(default = "opaque")]
    pub opacity: f64,
}

fn opaque() -> f64 {
    1.0
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
    inverse_binds: HashMap<String, Arc<Vec<Mat4>>>,
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
    /// This frame's running jets, for their exhaust.
    jets: Vec<DrawnJet>,
    indoor: Vec<f64>,
    allied: Vec<f64>,
    sound: Option<(EffectPublication, Vec<Launch>)>,
    shadow_fall: [f64; 2],
    scratch: Scratch,
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
    catalog_state: catalog::CatalogState,
    lab: lab::Lab,
    showcase: Option<showcase::Showcase>,
    models: Vec<Model>,
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
/// Where a unit's shadow falls from it, per metre of height, under the sun.
fn shadow_fall(azimuth: f64, elevation: f64) -> [f64; 2] {
    let throw = 1.0 / libm::tan(elevation);
    [-libm::cos(azimuth) * throw, -libm::sin(azimuth) * throw]
}

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
            catalog_state: catalog::CatalogState::new(),
            lab: lab::Lab::default(),
            showcase: None,
            models: Vec::new(),
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
        if input.battle.is_some() && self.showcase.is_some() {
            return Err("a showcase is shown: a battle needs a presentation of its own".into());
        }
        if let Some(fit) = &mut self.shadow {
            fit.invalidate();
        }
        self.catalog_state = catalog::CatalogState::new();
        self.world = World::default();
        self.lab.frontier = None;
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
                let rows = catalog::building_rows(&mut self.world, &buildings);
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
        self.register_scenery_rows();
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
            self.register_shown_rows();
            return Ok(());
        };
        let units = Arc::new(input.units.clone());
        let tables = Arc::new(tables);
        let presentation = input.effects.presentation.as_ref().clone();
        let capacity = presentation.capacity;
        let side = input.side.clone();
        let inverse_binds = posing::inverse_binds(&units);
        for (name, a) in &units.appearances {
            let a = a.as_ref();
            posing::register_appearance(&mut self.world, name, a);
        }
        if let Some(stand_in) = input.stand_in.as_ref() {
            posing::register_stand_in(&mut self.world, &stand_in.appearance);
        }
        posing::register_wreck_rows(&mut self.world, &input.effects);
        let shadow_fall = shadow_fall(azimuth, elevation);
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
            jets: Vec::new(),
            indoor: Vec::new(),
            allied: Vec::new(),
            sound: None,
            shadow_fall,
            scratch: Scratch::default(),
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
        self.register_shown_rows();
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
    pub fn building_population(&self) -> Option<(usize, usize)> {
        self.buildings
            .as_ref()
            .map(|b| (b.walk.buildings.len(), b.walk.pool_capacity()))
    }

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
        self.catalog_state.renew();
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

    /// Fill the grass view and tile window from the last framed camera.
    pub fn grass_frame_params(&self, params: &mut GrassParams, window: &Window) {
        let Some(c) = &self.camera else { return };
        let metres_per_px = metres_per_pixel(c.fov_y, f64::from(c.size[1].max(1)));
        let view_proj: &[f32; 16] = self.frame.camera.view_proj.first_chunk().unwrap();
        frame_params(params, view_proj, c.eye, metres_per_px, window);
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
}

/// An appearance's drawn size: its far pose (`k` 0) or its `k − 1`th way of lying.
fn bounds(units: &UnitsSection, name: &str, k: usize) -> Bounds {
    match units.appearances.get(name).map(AsRef::as_ref) {
        Some(Appearance::Skinned { bounds, .. } | Appearance::Articulated { bounds, .. }) => {
            bounds.get(k).copied().unwrap_or_default()
        }
        None => Bounds::default(),
    }
}
