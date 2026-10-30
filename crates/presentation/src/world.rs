//! What a presentation is loaded from (`load_world`'s input): the light's sun, and a battle's `units` and
//! `effects` sections (resolved for both clients by `fielded`), each
//! skinned body's dense clip table, the overlay colours and the side
//! presented, and the static world's style. The template library and the
//! placed buildings come beside it as bytes. Static per battle.
use std::collections::BTreeMap;

use contract::catalog::{AltitudeLayer, FlightModel};
use serde::Deserialize;
use serde_json::Value;

use crate::effects::transit::TransitFeel;

pub type Vec3 = [f64; 3];
pub type Quat = [f64; 4];

/// `v`'s length.
pub fn length(v: Vec3) -> f64 {
    libm::hypot(libm::hypot(v[0], v[1]), v[2])
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorldInput {
    /// What a battle's units and effects draw from; none for a world of
    /// statics alone (a lab with no battle).
    #[serde(default)]
    pub battle: Option<BattleInput>,
    /// The sun's azimuth and elevation (rad): where shadows fall.
    pub sun_azimuth: f64,
    pub sun_elevation: f64,
    /// `presentation.buildings`, when buildings are loaded.
    #[serde(default)]
    pub buildings: Option<BuildingStyle>,
}

/// `presentation.buildings`: how buildings are tiered, chunked and pooled.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingStyle {
    /// Pixels a metre covers at a chunk's nearest point for tiers 0 to 2.
    pub lod_px_per_m: [f64; 3],
    pub chunk_m: f64,
    /// Rows the residency pool holds.
    pub pool_records: usize,
    pub tint_jitter: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BattleInput {
    pub units: UnitsSection,
    pub effects: EffectsSection,
    /// The presented side, `"blue"` or `"red"`.
    pub side: String,
    pub tick_hz: f64,
    /// A soldier's body, for the UI's hit tests (`physics`).
    pub soldier: SoldierBody,
    /// `presentation.overlay.xray`: the observing side's units, its
    /// selection, and its units an ally owns.
    pub xray: Xray,
    /// A purchase the simulation refused, drawn as a ghost (rgba).
    pub blocked: [f64; 4],
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SoldierBody {
    pub radius_m: f64,
    pub height_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Xray {
    pub own: [f64; 4],
    pub selected: [f64; 4],
    /// A unit of the side that another player on it owns (`Ui::player`).
    pub ally: [f64; 4],
}

/// A node's or joint's local transform.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Trs {
    pub t: Vec3,
    pub r: Quat,
    pub s: Vec3,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PoseRef {
    pub clip: String,
    pub phase: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitsSection {
    /// `presentation.pose`, as the pose driver reads it.
    pub pose: Value,
    /// `presentation.transit`, the visible approach and departure durations.
    pub transit: TransitFeel,
    pub lean_hold: f64,
    pub detail: Detail,
    pub tints: BTreeMap<String, Vec3>,
    pub types: BTreeMap<String, UnitType>,
    pub appearances: BTreeMap<String, Appearance>,
}

/// `presentation.models`: mesh tiers by projected height, and the card size.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Detail {
    pub lod_px: [f64; 3],
    pub impostor_px: f64,
    /// Minimum projected footprint for a visible model to cast a sun shadow;
    /// zero keeps every caster (the authored production control).
    #[serde(default)]
    pub shadow_caster_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitType {
    pub hull: bool,
    /// Its vehicle class (`vehicleClass`): the sound row its engine, running
    /// gear and turret take; none for a squad.
    #[serde(default)]
    pub vehicle_class: Option<String>,
    pub airborne: bool,
    /// The height band it occupies (its mobility's `layer`).
    pub layer: AltitudeLayer,
    /// How it flies (its mobility's `flight`); none on the ground.
    pub flight: Option<FlightModel>,
    pub roles: Vec<String>,
    pub mounts: Vec<TypeMount>,
    pub draws: BTreeMap<String, Draws>,
}

impl UnitType {
    /// It flies in the high-air layer: from its height it casts no sun
    /// shadow but shows one on the ground beneath it.
    pub fn high_air(&self) -> bool {
        self.layer == AltitudeLayer::HighAir
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeMount {
    pub weapons: Vec<MountWeapon>,
    pub on: Option<usize>,
    pub active_pose: Option<PoseRef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MountWeapon {
    pub kind: String,
    pub kick: crate::looks::Kick,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Draws {
    #[serde(rename = "halfTrack")]
    pub half_track: f64,
    pub hull: Vec<String>,
    pub slots: Vec<Vec<String>>,
    pub operated: Vec<Option<Operated>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Operated {
    pub active: Vec<String>,
    pub carried: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    pub name: String,
    pub parent: i32,
    pub bind: Trs,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub offset: usize,
    pub frames: usize,
    pub duration: f64,
    pub rate: f64,
    #[serde(rename = "loop")]
    pub looping: bool,
    pub stride: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Muzzle {
    pub joint: usize,
    pub offset: Trs,
}

/// How big a model is drawn, for its tier and culling (`view::model_detail`):
/// standing height and reach from its foot, metres.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Bounds {
    pub size: f64,
    pub radius: f64,
}

/// A box as a bundle's state gives it (`bounds`): its least and greatest
/// corners, metres.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Corners {
    pub min: Vec3,
    pub max: Vec3,
}

impl Corners {
    /// How big the box is drawn (`boundsSize`): its largest extent, and its
    /// reach from its foot.
    pub fn bounds(&self) -> Bounds {
        bounds_size([self.min, self.max])
    }
}

/// How big a box is drawn (`boundsSize`): its largest extent, and its reach
/// from its foot.
pub fn bounds_size([min, max]: [Vec3; 2]) -> Bounds {
    let reach = |a: f64, b: f64| a.abs().max(b.abs());
    let dz = max[2] - min[2];
    Bounds {
        size: dz.max(max[0] - min[0]).max(max[1] - min[1]),
        radius: length([reach(min[0], max[0]), reach(min[1], max[1]), dz]),
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Appearance {
    #[serde(rename_all = "camelCase")]
    Skinned {
        joints: Vec<Node>,
        /// Each joint's inverse bind, column-major.
        #[serde(default)]
        inverse_binds: Vec<[f64; 16]>,
        table: String,
        clips: BTreeMap<String, Clip>,
        far_pose: PoseRef,
        corpse_poses: Vec<PoseRef>,
        muzzle: Option<Muzzle>,
        #[serde(default)]
        tiers: u32,
        /// The far pose's bounds, then each way of lying's.
        #[serde(default)]
        bounds: Vec<Bounds>,
        /// It has an impostor card (far pose and lying).
        #[serde(default)]
        card: bool,
    },
    #[serde(rename_all = "camelCase")]
    Articulated {
        nodes: Vec<Node>,
        rig: Rig,
        #[serde(default)]
        tiers: u32,
        #[serde(default)]
        bounds: Vec<Bounds>,
        /// Its box seen from above at rest, in its own frame: least x,
        /// least y, greatest x, greatest y (m).
        plan: [f64; 4],
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rig {
    pub turret: i32,
    pub gun: i32,
    pub hmg: i32,
    pub hmg_gun: i32,
    pub pitch: RigPitch,
    pub stroke: Option<f64>,
    pub wheels: Vec<Wheel>,
    pub tracks: Vec<Track>,
    pub rotors: Vec<Rotor>,
    pub deploy: Vec<Deploy>,
    pub muzzles: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct RigPitch {
    pub gun: [f64; 2],
    pub hmg: [f64; 2],
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Wheel {
    pub node: usize,
    pub radius: f64,
    pub left: bool,
}

/// Which running-gear travel scrolls a pitch-qualified model node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(from = "bool", into = "bool")]
pub enum TrackSide {
    Left,
    Right,
}

impl From<bool> for TrackSide {
    fn from(left: bool) -> Self {
        if left {
            Self::Left
        } else {
            Self::Right
        }
    }
}

impl From<TrackSide> for bool {
    fn from(side: TrackSide) -> Self {
        side == TrackSide::Left
    }
}

impl Rig {
    pub fn track_side(&self, node: usize) -> Option<TrackSide> {
        self.tracks.iter().find(|t| t.node == node).map(|t| t.side)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub node: usize,
    #[serde(rename = "left")]
    pub side: TrackSide,
    pub link_pitch: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Rotor {
    pub node: usize,
    pub radius: f64,
    pub blades: f64,
    pub chord: f64,
    pub rest: f64,
    pub mast_below: f64,
    pub mast_above: f64,
    pub mast_radius: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Deploy {
    pub node: usize,
    pub start: f64,
    pub end: f64,
    #[serde(rename = "move")]
    pub shift: Vec3,
    pub turn: Vec3,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectsSection {
    /// `presentation.effects`, each weapon's rows resolved (`gameEffects`).
    pub presentation: Value,
    pub types: BTreeMap<String, EffectType>,
    /// Prop kinds that smoke once known, each with its look.
    pub smokes: BTreeMap<String, String>,
    pub wrecks: BTreeMap<String, Wreck>,
    /// By unit type: how its hull dies.
    #[serde(default)]
    pub wreck_looks: BTreeMap<String, WreckLook>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EffectType {
    pub half: Option<Vec3>,
    pub muzzles: Vec<Option<MountMuzzle>>,
}

/// A hull mount's muzzle model (`mountMuzzle.ts`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MountMuzzle {
    pub on: Option<usize>,
    pub pivot: Vec3,
    pub ring: Vec3,
    pub muzzle: Vec3,
}

/// How a unit type's hull dies, by unit type : its own wreck's appearance as the props fit it (`props.fit.wrecks`
/// and its `looks`), that wreck's footprint (half extents) and its whole
/// `default` state's resting bounds (min, max), and how hard the dead hull
/// brakes to a stop, m/s² (top speed over `movement.drive.wreck_stop_s`).
#[derive(Clone, Debug, Deserialize)]
pub struct WreckLook {
    pub appearance: String,
    pub footprint: Vec3,
    pub bounds: [Vec3; 2],
    pub braking: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wreck {
    pub lies: Option<Vec3>,
    pub underside: f64,
    pub debris_top: Option<f64>,
}

impl UnitsSection {
    /// The appearance drawing soldier `id` in slot `slot` of a `kind` unit on
    /// `side` (a hull's model whatever the id), or None where none is
    /// installed: `AppearanceCatalog.resolve` over the section's sets.
    pub fn resolve(
        &self,
        kind: &str,
        side: &str,
        id: f64,
        slot: usize,
        operator_mount: Option<usize>,
        active_mount: Option<usize>,
    ) -> Option<&str> {
        let draws = self.types.get(kind)?.draws.get(side)?;
        let mut names: &[String] = &draws.hull;
        if names.is_empty() {
            let operated = operator_mount
                .and_then(|m| draws.operated.get(m).and_then(Option::as_ref))
                .map(|e| {
                    if active_mount.is_some() && active_mount == operator_mount {
                        &e.active
                    } else {
                        &e.carried
                    }
                });
            names = match operated {
                Some(list) if !list.is_empty() => list,
                _ => draws.slots.get(slot).map_or(&[], Vec::as_slice),
            };
        }
        if names.is_empty() {
            return None;
        }
        let n = names.len() as f64;
        Some(&names[(id.rem_euclid(n)) as usize])
    }
}
