//! A pose frame: one pose per living or dying
//! soldier, one articulation per vehicle, the static corpses, and the
//! corpses sinking away. The driver rewrites it in place each frame.
use std::sync::Arc;

use crate::world::Vec3;

/// How a vehicle's articulated parts stand (`articulation.ts`).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Articulation {
    /// Turret heading relative to the hull, radians about +Z.
    pub turret_yaw: f64,
    /// Gun elevation, radians; positive raises the muzzle.
    pub gun_pitch: f64,
    /// Metres the gun has run back after a shot.
    pub recoil: f64,
    pub hmg_yaw: f64,
    pub hmg_pitch: f64,
    /// Metres the left and right running gear have rolled forward.
    pub travel_l: f64,
    pub travel_r: f64,
    /// Deployment progress: 0 packed, 1 deployed.
    pub deploy: f64,
    /// Metres every rotor's blade tips have swept.
    pub rotor: f64,
    /// 1: rotors drawn by their blur; 0: by their geometry.
    pub rotor_blur: f64,
}

/// A clip being faded out under the main one.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipBlend {
    pub clip: Arc<str>,
    pub phase: f64,
    /// Weight of this clip over the main clip, 0..1.
    pub weight: f64,
}

/// A shot's kick as drawn: the upper body turned back `pitch` radians about
/// [`KICK_JOINT`](super::skin::KICK_JOINT) (forward, +X, climbing), and
/// thrown `back` metres along −X. Both 0 at rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Kick {
    pub pitch: f64,
    pub back: f64,
}

#[derive(Clone, Debug)]
pub struct SoldierPose {
    pub soldier: f64,
    /// His unit's id; -1 once he has fallen.
    pub unit: f64,
    pub kind: Arc<str>,
    pub slot: usize,
    /// Operated weapon's appearance override, or None for his ordinary kit.
    pub operator_mount: Option<usize>,
    pub active_mount: Option<usize>,
    pub side: Arc<str>,
    pub position: Vec3,
    /// Heading of the body's +X, radians.
    pub facing: f64,
    pub clip: Arc<str>,
    pub phase: f64,
    pub blend: Option<ClipBlend>,
    /// His last shot's kick as drawn now (`PoseFeel::soldier_kick`).
    pub kick: Kick,
}

#[derive(Clone, Debug)]
pub struct VehiclePose {
    pub unit: f64,
    pub kind: Arc<str>,
    pub side: Arc<str>,
    pub position: Vec3,
    pub yaw: f64,
    /// It flies: its rotors turn, and nothing it has rolls.
    pub airborne: bool,
    pub articulation: Articulation,
    /// Its rigid motion in its own frame about its foot, column-major: an
    /// aircraft's lean (`FeedUnit::attitude`, or into its flight), or a hull
    /// rocking on its springs from a shot's kick (`PoseFeel::hull_kick`);
    /// None at rest.
    pub motion: Option<[f64; 16]>,
}

/// A fallen soldier whose death has played out: drawn lying, never posed.
#[derive(Clone, Debug)]
pub struct CorpsePose {
    /// Which way he lies (`INFANTRY_FALLS`).
    pub lies: usize,
    pub operator_mount: Option<usize>,
    pub soldier: f64,
    pub kind: Arc<str>,
    pub slot: usize,
    pub side: Arc<str>,
    pub position: Vec3,
    pub yaw: f64,
}

/// A corpse pushed out by the cap, sinking until its fade has run.
#[derive(Clone, Debug)]
pub struct FadingCorpse {
    pub corpse: CorpsePose,
    /// How far below where he lay he is drawn, metres.
    pub sink: f64,
    /// When he left the static list, simulation seconds.
    pub since: f64,
}

#[derive(Clone, Debug, Default)]
pub struct PoseFrame {
    pub soldiers: Vec<SoldierPose>,
    pub vehicles: Vec<VehiclePose>,
    /// The static corpses, oldest first.
    pub corpses: Vec<CorpsePose>,
    /// Rises whenever `corpses` changed.
    pub corpses_version: u64,
    pub fading: Vec<FadingCorpse>,
}
