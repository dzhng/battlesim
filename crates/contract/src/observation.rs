//! What one side is allowed to know at a completed tick. Presentation, audio,
//! picking and controllers consume only this.
use crate::command::{Engagement, RoutePolicy, TargetRef};
use crate::ids::{Tick, UnitId};
use crate::scenario::UnitKind;
use serde::{Deserialize, Serialize};

/// A side-scoped handle for an identified enemy. It is not the enemy's unit
/// id, and it is retired when identification lapses past its grace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservedTargetId(pub u32);

/// What a side knows of an identified enemy: its class and value and what its
/// sensors saw this tick. Never health, ammunition, orders or timers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdentifiedUnit {
    pub id: ObservedTargetId,
    pub kind: UnitKind,
    pub cost: u32,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Observed ground velocity (from successive sightings; zero when first seen).
    pub velocity: [f64; 2],
    /// Positions of the squad members actually seen this tick (infantry only).
    pub members: Vec<[f64; 3]>,
}

/// A side-scoped handle for an approximate contact, unrelated to any enemy id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContactId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactSource {
    /// Something fired from somewhere in this area.
    Firing,
    /// An identified enemy was last seen here.
    LastSeen,
}

/// Uncertain evidence: a ground area where something is or was. It carries no
/// class, velocity, cost or exact position, and never moves by itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApproximateContact {
    pub id: ContactId,
    pub source: ContactSource,
    pub center: [f64; 2],
    pub radius: f64,
    pub evidence_tick: Tick,
    pub expires_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundCategory {
    Infantry,
    Vehicle,
    Shot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundBand {
    Near,
    Far,
}

/// A friendly listener heard an unseen enemy: broad character, one of eight
/// directions and a near/far band. Never a position; never a visual contact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundCue {
    pub listener: UnitId,
    pub category: SoundCategory,
    /// 0 = east, counter-clockwise in 45° steps.
    pub sector: u8,
    pub band: SoundBand,
    /// The source was moving (engine working, footsteps) rather than idle.
    pub moving: bool,
}

/// A solid obstacle added after the battle began that this side knows about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownProp {
    pub kind: crate::map::PropKind,
    pub center: [f64; 2],
    pub yaw: f64,
    pub half_extents: [f64; 3],
    pub base_z: f64,
}

/// Which ground this side can currently see: row-major cells of `cell_m`,
/// one bit per cell, set when visible.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VisibilityField {
    pub cell_m: f64,
    pub nx: u32,
    pub ny: u32,
    pub bits: Vec<u32>,
}

impl VisibilityField {
    pub fn visible(&self, x: f64, y: f64) -> bool {
        let (i, j) = ((x / self.cell_m).floor(), (y / self.cell_m).floor());
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return false;
        }
        let k = j as usize * self.nx as usize + i as usize;
        self.bits[k / 32] & (1 << (k % 32)) != 0
    }
}

/// Why a weapon is not firing right now (or that it is).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionReason {
    Firing,
    NoCompatibleTarget,
    HoldingFire,
    OutOfRange,
    BlockedTrajectory,
    FriendlyInLine,
    Aiming,
    Reloading,
    TurretTraversing,
    MovingStationaryWeapon,
    OutOfAmmo,
    /// Identification lapsed: aiming at the last sighting through the grace.
    TrackingLastSighting,
}

/// One weapon mount's readiness: what the rings and panel show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MountReadiness {
    /// Index into the unit kind's authored mount list.
    pub mount: u8,
    /// Index into the mount's ammunition kinds.
    pub loaded: Option<u8>,
    /// Rounds left per kind (including a loaded one); `None` is unlimited.
    pub ammo: Vec<Option<u32>>,
    /// Aim progress in [0, 1]; 1 once acquired.
    pub aim: f64,
    /// Reload progress in [0, 1]; 0 when loaded or idle.
    pub reload: f64,
    pub target: Option<TargetRef>,
    pub reason: ActionReason,
}

/// A stretch of a round's flight this side may draw this tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisibleSegment {
    pub from: [f64; 3],
    pub to: [f64; 3],
    /// Fired by this side.
    pub own: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveState {
    Idle,
    Moving,
    /// Yielding to friendly traffic; `blocker` says who.
    Waiting,
    /// No known route; the destination is kept and retried on relevant change.
    RouteBlocked,
    /// An attack-move holding its advance while a weapon engages (W16).
    Halted,
    /// Has somewhere to go but must finish packing before it may move (L01).
    Packing,
}

/// The end state a deploying unit's progress is heading to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posture {
    Packed,
    Deployed,
}

/// A deploying unit's one progress value and where it is heading (L01).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeploymentState {
    /// In [0, 1]: 0 packed and free to move, 1 fully deployed.
    pub progress: f64,
    pub target: Posture,
}

/// A unit of the observing side: its own state is complete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnUnit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Destination of the current movement order, if any.
    pub goal: Option<[f64; 2]>,
    pub policy: Option<RoutePolicy>,
    pub state: MoveState,
    /// The friendly unit this one is waiting for.
    pub blocker: Option<UnitId>,
    /// Remaining waypoints of the current route.
    pub route: Vec<[f64; 2]>,
    /// Destinations of the orders queued behind the current one.
    pub queue: Vec<[f64; 2]>,
    /// Living squad members' positions (infantry only).
    pub members: Vec<[f64; 3]>,
    /// Enemies this unit's own sensors identify this tick (own sensor, not shared).
    pub sees: Vec<ObservedTargetId>,
    pub engagement: Engagement,
    pub mounts: Vec<MountReadiness>,
    /// Units that set up in place (the supply vehicle); `None` for the rest.
    pub deployment: Option<DeploymentState>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub tick: Tick,
    pub own: Vec<OwnUnit>,
    /// Enemies identified by any friendly sensor (team-shared).
    pub identified: Vec<IdentifiedUnit>,
    /// Uncertain evidence not explained by a current identification.
    pub contacts: Vec<ApproximateContact>,
    /// Sounds heard this bucket (empty between buckets).
    pub audible: Vec<SoundCue>,
    /// Obstacles added after setup that this side has seen or run into.
    pub known_props: Vec<KnownProp>,
    /// Round flight this side may draw this tick (own rounds whole, enemy
    /// rounds only over ground it sees).
    pub projectiles: Vec<VisibleSegment>,
    pub ground_visibility: VisibilityField,
}
