//! What a presentation is loaded from (`load_world`'s input): the light's sun, and a battle's `units` and
//! `effects` sections (resolved for both clients by `fielded`), each
//! skinned body's dense clip table, the overlay colours and the side
//! presented, and the static world's style. The template library and the
//! placed buildings come beside it as bytes. Static per battle.
use std::collections::BTreeMap;

use contract::catalog::{AltitudeLayer, FlightModel};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::effects::transit::TransitFeel;

pub type Vec3 = [f64; 3];
pub type Quat = [f64; 4];

/// `v`'s length.
pub fn length(v: Vec3) -> f64 {
    libm::hypot(libm::hypot(v[0], v[1]), v[2])
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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
    /// The browser's canonical box appearance and its physical half extents
    /// for bodies whose art is not committed. Native input leaves this unset.
    pub stand_in: Option<StandIn>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandIn {
    pub appearance: String,
    pub half: Vec3,
    /// A display-encoded colour, supplied by the shell and converted to linear
    /// for the model material. One colour for every side: the block says only
    /// that art is on its way.
    pub tint: Vec3,
    /// Whether the placeholder is a solid sun-shadow caster.
    pub shadow: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
pub struct SoldierBody {
    pub radius_m: f64,
    pub height_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
pub struct Xray {
    pub own: [f64; 4],
    pub selected: [f64; 4],
    /// A unit of the side that another player on it owns (`Ui::player`).
    pub ally: [f64; 4],
}

/// A node's or joint's local transform.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
pub struct Trs {
    pub t: Vec3,
    pub r: Quat,
    pub s: Vec3,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct PoseRef {
    pub clip: String,
    pub phase: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitsSection {
    /// `presentation.pose`, as the pose driver reads it.
    #[serde(deserialize_with = "read_pose")]
    pub pose: Arc<crate::units::feel::PoseFeel>,
    /// `presentation.transit`, the visible approach and departure durations.
    pub transit: TransitFeel,
    pub lean_hold: f64,
    pub detail: Detail,
    pub tints: BTreeMap<String, Vec3>,
    /// Physical hull half extents for every unit type in the session catalog,
    /// including types whose art is not fielded yet.
    #[serde(rename = "bodyHalf")]
    pub body_half: BTreeMap<String, Option<Vec3>>,
    pub types: BTreeMap<String, Arc<UnitType>>,
    pub appearances: BTreeMap<String, Arc<Appearance>>,
}

/// `presentation.models`: mesh tiers by projected size, and the card's and
/// the far mesh's sizes.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Detail {
    pub lod_px: [f64; 3],
    pub impostor_px: f64,
    /// Below this a model whose bundle carries a far mesh draws it
    /// (`view::FAR`); under `lod_px[2]`.
    pub far_px: f64,
    /// Minimum conservative projected footprint, in pixels, for a prop to
    /// cast a sun shadow, whether the view sees it or not; zero keeps every
    /// caster. Scaled by the graphics tier (`Presentation::set_detail_scale`):
    /// authored 5, drawn 5 at High, 7 at Medium and 9 at Low, since at 9 a
    /// car's shadow already stops about 700 m from the camera. Buildings,
    /// trees and units do not read it.
    #[serde(default)]
    pub shadow_caster_px: f64,
}

impl Detail {
    /// These tiers with their mesh thresholds scaled by `scale`: the units'
    /// detail scale (`Presentation::set_detail_scale`). No unit carries a far
    /// mesh, so `far_px` is left as it is.
    pub fn scaled(self, scale: f64) -> Self {
        Self {
            lod_px: self.lod_px.map(|v| v * scale),
            ..self
        }
    }
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitType {
    pub hull: bool,
    /// Its vehicle class (`vehicleClass`): the sound row its engine, running
    /// gear and turret take; none for a squad.
    #[serde(default)]
    pub vehicle_class: Option<String>,
    /// Its mobility is flight, even when observed standing on the ground.
    pub flies: bool,
    /// The height band it occupies (its mobility's `layer`).
    pub layer: AltitudeLayer,
    /// How it flies (its mobility's `flight`); none on the ground.
    pub flight: Option<FlightModel>,
    /// The rate an aircraft turns at by choice, radians a second (its
    /// mobility's `turn_deg_s`, not its edge-guard `max_turn_deg_s`); none
    /// on the ground.
    #[serde(default)]
    pub turn_rad_s: Option<f64>,
    /// An aircraft's cruise speed, m/s (its mobility's `cruise_kmh`): its
    /// engines' throttle reads its drawn speed against it. None on the ground.
    #[serde(default)]
    pub cruise_mps: Option<f64>,
    /// Authored deployment timing; rotor presentation reads the same capability.
    #[serde(default)]
    pub deploy: Option<contract::catalog::Deploy>,
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

    /// Its flight model drives a rotor.
    pub fn has_rotor(&self) -> bool {
        self.flight == Some(FlightModel::Rotor)
    }

    /// This observed aircraft is off the ground now.
    pub fn in_the_air(&self, grounded: bool) -> bool {
        self.flies && !grounded
    }
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeMount {
    pub weapons: Vec<MountWeapon>,
    pub on: Option<usize>,
    pub active_pose: Option<PoseRef>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct MountWeapon {
    pub kind: String,
    pub kick: crate::looks::Kick,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct Draws {
    #[serde(rename = "halfTrack")]
    pub half_track: f64,
    pub hull: Vec<String>,
    pub slots: Vec<Vec<String>>,
    pub operated: Vec<Option<Operated>>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct Operated {
    pub active: Vec<String>,
    pub carried: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct Node {
    pub name: String,
    pub parent: i32,
    pub bind: Trs,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct Muzzle {
    pub joint: usize,
    pub offset: Trs,
}

/// How big a model is drawn, for its tier and culling (`view::model_detail`):
/// standing height and reach from its foot, metres.
#[derive(Clone, Copy, Debug, Default, Deserialize, serde::Serialize)]
pub struct Bounds {
    pub size: f64,
    pub radius: f64,
}

/// A box as a bundle's state gives it (`bounds`): its least and greatest
/// corners, metres.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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
    /// The top node of each landing-gear leg (`gear_*`): an aircraft in
    /// flight stows them (`Articulation::gear_up`).
    #[serde(default)]
    pub gear: Vec<usize>,
    pub muzzles: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
pub struct RigPitch {
    pub gun: [f64; 2],
    pub hmg: [f64; 2],
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
pub struct Deploy {
    pub node: usize,
    pub start: f64,
    pub end: f64,
    #[serde(rename = "move")]
    pub shift: Vec3,
    pub turn: Vec3,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectsSection {
    /// `presentation.effects`, each weapon's rows resolved (`gameEffects`).
    #[serde(deserialize_with = "read_effects")]
    pub presentation: Arc<crate::effects::tables::EffectPresentation>,
    pub types: BTreeMap<String, Arc<EffectType>>,
    /// Prop kinds that smoke once known, each with its look.
    pub smokes: Arc<BTreeMap<String, String>>,
    pub wrecks: BTreeMap<String, Wreck>,
    /// By unit type: how its hull dies.
    #[serde(default)]
    pub wreck_looks: BTreeMap<String, Arc<WreckLook>>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct EffectType {
    pub muzzles: Vec<Option<MountMuzzle>>,
}

/// A hull mount's muzzle model (`mountMuzzle.ts`).
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
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
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct WreckLook {
    pub appearance: String,
    pub footprint: Vec3,
    pub bounds: [Vec3; 2],
    pub braking: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
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

/// A body box prepared once for a frame and shared by drawing and picking.
pub struct BodyBox<'a> {
    units: &'a UnitsSection,
    soldier_half: Vec3,
}

impl<'a> BodyBox<'a> {
    pub fn new(units: &'a UnitsSection, soldier: SoldierBody) -> Self {
        Self {
            units,
            soldier_half: [soldier.radius_m, soldier.radius_m, soldier.height_m / 2.0],
        }
    }

    pub fn half(&self, kind: &str) -> Option<Vec3> {
        let unit = self.units.types.get(kind)?;
        if unit.hull {
            self.units.body_half.get(kind).copied().flatten()
        } else {
            Some(self.soldier_half)
        }
    }
}

// File inputs and retained fielding results have the same checked tuning.
fn read_pose<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Arc<crate::units::feel::PoseFeel>, D::Error> {
    let value = Value::deserialize(d)?;
    crate::units::feel::PoseFeel::from_value(&value)
        .map(Arc::new)
        .map_err(serde::de::Error::custom)
}
fn read_effects<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Arc<crate::effects::tables::EffectPresentation>, D::Error> {
    let value = Value::deserialize(d)?;
    crate::effects::tables::EffectPresentation::from_value(&value)
        .map(Arc::new)
        .map_err(serde::de::Error::custom)
}
