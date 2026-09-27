//! What one side is allowed to know at a completed tick. Presentation, audio,
//! picking and controllers consume only this.
use crate::command::{Engagement, RoutePolicy, TargetRef};
use crate::ids::{Side, Tick, UnitId};
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
    /// The seen soldiers' ids, in `members` order: the raw `Soldier.id`, so
    /// they reveal roster size (accepted, F2) and survive reacquisition.
    pub member_ids: Vec<u32>,
    /// Every mount's pose, in the unit kind's mount order.
    pub weapon_poses: Vec<WeaponPose>,
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
    /// The authored prop this one stands in place of (a ruin's building).
    pub replaces: Option<u32>,
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
    /// The launcher is guiding a missile in flight; it cannot launch another.
    Guiding,
    /// A guided launcher needs its own identification of the target (P05).
    NoOwnSight,
    /// Garrisoned: every perimeter slot facing the target is taken.
    NoFacingSlot,
    /// Entering or leaving a building: weapons wait.
    ChangingPosition,
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
    /// The ammunition kind being reloaded, if any.
    pub reloading: Option<u8>,
    pub target: Option<TargetRef>,
    pub reason: ActionReason,
    /// This mount is guiding a missile in flight.
    pub guiding: bool,
}

/// One of this side's own guided missiles: where it is and the point it is
/// steering to; `supported` while its launcher still guides it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuidedMissile {
    /// Stable while it flies: this side's own round, so no disclosure.
    pub id: u64,
    pub position: [f64; 3],
    pub point: [f64; 3],
    pub supported: bool,
}

/// What a weapon mount is doing, for posing its model. The pose itself is
/// derived only in the renderer (Q7). Published for own mounts and for the
/// mounts of identified enemies.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeaponPose {
    /// Index into the unit kind's authored mount list.
    pub mount: u8,
    /// World bearing (radians, counter-clockwise from +X): a turret's heading,
    /// or a hand weapon's last aim.
    pub bearing: f64,
    /// Elevation above the horizontal of the mount's last launched round
    /// (radians); 0 until it first fires.
    pub elevation: f64,
    /// Rounds this mount has launched since the battle began, wrapping at
    /// 2³²; a squad volley counts one per soldier. A rise between two
    /// publications is a shot.
    pub shots: u32,
}

/// What ended a round's flight at a segment's `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentHit {
    /// Still flying, expired, or its end is not shown.
    None,
    Ground,
    /// A vehicle hull.
    Hull,
    /// A building, wall, wreck or other prop.
    Prop,
    Soldier,
}

/// A stretch of a round's flight this side may draw this tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisibleSegment {
    /// The flown path, at least two points: a polyline that bends where the
    /// round ricocheted.
    pub path: Vec<[f64; 3]>,
    /// Where along `path` the round glanced off a hull this tick.
    pub ricochets: Vec<SegmentRicochet>,
    /// Fired by this side.
    pub own: bool,
    /// The round kind: an index into the rules' weapon rows in name order.
    /// An enemy tracer reveals its shooter's class (F3).
    pub kind: usize,
    /// The soldier who fired it (`Soldier.id`); `None` for a vehicle's gun.
    pub shooter_member: Option<u32>,
    /// What the round struck at the path's end this tick (shown only when
    /// that point is seen).
    pub hit: SegmentHit,
    /// Outward surface normal at the impact, world frame; `None` without one.
    pub impact_normal: Option<[f64; 3]>,
}

/// A ricochet on a segment's path: the round glanced off a hull at
/// `path[point]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentRicochet {
    pub point: usize,
    /// Outward hull normal there, world frame.
    pub normal: [f64; 3],
}

/// A round's burst this tick: HE, grenades and missiles. Own blasts are all
/// published; enemy blasts only on ground this side sees.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blast {
    pub point: [f64; 3],
    pub radius: f64,
    /// The round kind, as on [`VisibleSegment::kind`].
    pub kind: usize,
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

/// Where a squad stands with a building (L08).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GarrisonPhase {
    /// Stationary beside it, entering.
    Entering,
    /// Entered, but the building has no room left for the whole squad.
    WaitingForRoom,
    /// At its perimeter slots.
    Inside,
    /// Stationary inside, leaving.
    Exiting,
}

/// A squad's garrison: which building, the phase and its timer progress.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarrisonState {
    pub building: u32,
    pub phase: GarrisonPhase,
    /// Entering or leaving progress in [0, 1]; 1 while inside.
    pub progress: f64,
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
    /// The friendly unit this one is waiting for (an enemy it waits for,
    /// Q14, is not named).
    pub blocker: Option<UnitId>,
    /// Remaining waypoints of the current route.
    pub route: Vec<[f64; 2]>,
    /// Destinations of the orders queued behind the current one.
    pub queue: Vec<[f64; 2]>,
    /// Living squad members' positions (infantry only).
    pub members: Vec<[f64; 3]>,
    /// Living squad members' ids (`Soldier.id`), in `members` order.
    pub member_ids: Vec<u32>,
    /// Enemies this unit's own sensors identify this tick (own sensor, not shared).
    pub sees: Vec<ObservedTargetId>,
    pub engagement: Engagement,
    pub mounts: Vec<MountReadiness>,
    /// Every mount's pose, in `mounts` order.
    pub weapon_poses: Vec<WeaponPose>,
    /// Units that set up in place (the supply vehicle); `None` for the rest.
    pub deployment: Option<DeploymentState>,
    /// Vehicle health (0 for infantry, whose health is per soldier).
    pub hp: f64,
    /// Health of each living soldier, in `members` order.
    pub member_hp: Vec<f64>,
    /// Infantry suppression in [0, 1] (P14).
    pub suppression: f64,
    /// A supply vehicle's remaining stock.
    pub stock: Option<u32>,
    pub service: ServiceStatus,
    /// The squad's building, while entering, inside or leaving it.
    pub garrison: Option<GarrisonState>,
    pub sight: UnitSight,
}

/// Where a unit's own sight reaches at the published tick: what spotting and
/// the fog sweep used this tick, for renderer fog. Never interpolated.
///
/// Reach toward world bearing `b` is `range * m`, where with
/// `c = cos(b - forward)`, `m = side * (1 - c²) + (c ≥ 0 ? front : rear) * c²`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitSight {
    /// Where sight starts: the unit's eye, or each garrison slot's eye.
    pub eyes: Vec<[f64; 3]>,
    /// World bearing the unit looks along (radians, counter-clockwise from +X):
    /// a tank's turret, otherwise the hull.
    pub forward: f64,
    pub shape: crate::scenario::SightShape,
    /// Ground range in the open, before shape, foliage and concealment.
    pub range: f64,
}

/// The fixture's local completion condition, as the referee sees it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterStatus {
    /// Seconds the attacker has held the zone uncontested, this spell.
    pub held_s: f64,
    pub result: EncounterResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterResult {
    Running,
    /// The attacker held the zone for the required time.
    Captured,
    /// The attacker lost every combat unit.
    Defeated,
    /// Past the assessment time with neither: play may continue.
    Inconclusive,
}

/// Why a unit is or is not being served by a supply vehicle (L03, L04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    /// No supply vehicle of its side is in range.
    OutOfRange,
    /// A supply vehicle is in range but not fully deployed.
    SourceNotDeployed,
    /// Recipients must be stationary.
    Moving,
    /// Recipients must not be firing this tick.
    Firing,
    /// Being replenished, repaired or reinforced.
    Serving,
    /// In range and deployed, but the stock cannot pay for the next item.
    NoStock,
    /// Nothing missing.
    Full,
    /// Inside a building: ammunition and repair only, no replacements.
    Garrisoned,
}

/// A fallen soldier: a permanent record that blocks nothing (M06).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Corpse {
    pub position: [f64; 3],
    pub own: bool,
    /// The fallen soldier's id (`Soldier.id`).
    pub soldier: u32,
    /// The kind of squad the soldier fought in.
    pub kind: UnitKind,
    /// The squad's heading when the soldier fell (radians).
    pub yaw: f64,
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
    /// Rounds that burst this tick (own everywhere, enemy over seen ground).
    pub blasts: Vec<Blast>,
    /// Own fallen, and enemy fallen this side has seen.
    pub corpses: Vec<Corpse>,
    /// This side's own guided missiles in flight.
    pub guided: Vec<GuidedMissile>,
    /// The fixture's completion condition, when it has one.
    pub encounter: Option<EncounterStatus>,
    pub ground_visibility: VisibilityField,
}

/// One side's learned ground cells, delivered as a patch beside its
/// observation (slice 08). The transport keeps a cursor per consumer: `epoch`
/// names one unbroken stream of patches, and a new epoch always starts with
/// a `full` snapshot (every learned cell, `base_revision` 0). Within an epoch
/// each patch carries exactly the cells whose learned marks changed after
/// `base_revision`, up to `revision`, so applying the stream in order rebuilds
/// the side's knowledge. The cursor is transport state, never battle state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundPatch {
    pub epoch: u32,
    pub side: Side,
    pub base_revision: u32,
    pub revision: u32,
    pub full: bool,
    pub cells: Vec<GroundCellPatch>,
}

/// A learned cell's marks, each in [0, 255]. `cell` is the ground grid's
/// row-major index (`j * cols + i`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundCellPatch {
    pub cell: u32,
    pub crater: u8,
    pub scorch: u8,
    pub tracks: u8,
    pub trampled: u8,
}
