//! What a battle's presentation is loaded from (18b's `load_world` input, as
//! far as slice 20 reads it): the `units` and `effects` sections both clients
//! already carry (`web/src/battle/benchmark/unitManifest.ts`,
//! `effectManifest.ts`), each skinned body's dense clip table, the light's
//! sun, the overlay colours and the side presented. Static per battle.
use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

pub type Vec3 = [f64; 3];
pub type Quat = [f64; 4];

#[derive(Clone, Debug, Deserialize)]
pub struct WorldInput {
    pub units: UnitsSection,
    pub effects: EffectsSection,
    /// The presented side, `"blue"` or `"red"`.
    pub side: String,
    pub tick_hz: f64,
    /// The sun's azimuth and elevation (rad): where an airframe's shadow falls.
    pub sun_azimuth: f64,
    pub sun_elevation: f64,
    /// A soldier's body, for the UI's hit tests (`physics`).
    pub soldier: SoldierBody,
    /// `presentation.overlay.xray`: the observing side's units and its selection.
    pub xray: Xray,
    /// A purchase the simulation refused, drawn as a ghost (rgba).
    pub blocked: [f64; 4],
    /// `air.cruise_agl_m`: where an aircraft's ghost flies.
    pub cruise_agl_m: f64,
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
    pub lean_hold: f64,
    pub detail: Detail,
    pub tints: BTreeMap<String, Vec3>,
    pub types: BTreeMap<String, UnitType>,
    pub appearances: BTreeMap<String, Appearance>,
}

/// `presentation.models`: mesh tiers by projected height, and the card size.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Detail {
    pub lod_px: [f64; 3],
    pub impostor_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitType {
    pub hull: Option<Hull>,
    pub airborne: bool,
    pub roles: Vec<String>,
    pub mounts: Vec<TypeMount>,
    pub draws: BTreeMap<String, Draws>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hull {
    pub half_track: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeMount {
    pub weapon: String,
    pub on: Option<usize>,
    pub active_pose: Option<PoseRef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Draws {
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

/// How big a model is drawn, for its tier and culling (`modelDetail`):
/// standing height and reach from its foot, metres.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Bounds {
    pub size: f64,
    pub radius: f64,
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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub node: usize,
    pub left: bool,
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
pub struct EffectsSection {
    /// `presentation.effects`, each weapon's rows resolved (`gameEffects`).
    pub presentation: Value,
    pub types: BTreeMap<String, EffectType>,
    /// Prop kinds that smoke once known, each with its look.
    pub smokes: BTreeMap<String, String>,
    pub wrecks: BTreeMap<String, Wreck>,
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
