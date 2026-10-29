//! What one side is allowed to know at a completed tick. Presentation, audio,
//! picking and controllers consume only this.
use crate::catalog::{AltitudeLayer, PropKind, TypeIndex};
use crate::command::{Engagement, RoutePolicy, TargetRef};
use crate::ids::{Tick, UnitId};
use serde::{Deserialize, Serialize};

mod sound;
pub use sound::{Intensity, Shout, ShoutSpeaker, ShoutTrigger, SoundBand, SoundCategory, SoundCue};

/// A side-scoped handle for an identified enemy. It is not the enemy's unit
/// id, and it is retired when identification lapses past its grace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservedTargetId(pub u32);

/// What a side knows of an identified enemy: its class and value and what its
/// sensors saw this tick. Never health, ammunition, orders or timers. The one
/// audible exception: a seen enemy's shouts (`ObservationFrame::shouts`)
/// carry its intensity and bark, so a stressed take or a pinned call reveals
/// that it is hurt or suppressed, as hearing it would (decision D5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdentifiedUnit {
    pub id: ObservedTargetId,
    /// Its unit type: its rank in the catalog, the layout's `unitKinds`.
    pub kind: TypeIndex,
    pub cost: u32,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Observed ground velocity (from successive sightings; zero when first seen).
    pub velocity: [f64; 2],
    /// Body poses of the squad members actually seen this tick (infantry only).
    /// Garrison bodies stand indoors behind their abstract window exposure points.
    pub members: Vec<[f64; 3]>,
    /// The seen soldiers' ids, in `members` order: the raw `Soldier.id`, so
    /// they reveal roster size (accepted, F2) and survive reacquisition.
    pub member_ids: Vec<u32>,
    /// Each seen soldier's slot in his squad type, in `members` order: which
    /// soldier kind he is, so he is drawn as one.
    pub member_slots: Vec<u8>,
    /// Each published soldier's selected mount, in `members` order; absent
    /// when he is using no weapon. Visibility follows the member identity.
    pub member_active_mounts: Vec<Option<u8>>,
    /// Every mount's pose, in the unit type's mount order.
    pub weapon_poses: Vec<WeaponPose>,
    /// Driving backwards this tick, seen as plainly as its position (the
    /// reverse beeper's cue for a seen enemy vehicle).
    pub reversing: bool,
    /// It trails smoke (D34): a coarse sign of damage, never its health.
    #[serde(default)]
    pub smoking: bool,
    /// Each seen soldier's lean, in `members` order: out past his cover's
    /// edge, where rounds meet him, while he fires. His `members`
    /// position stays where he tucks in.
    pub member_leans: Vec<Option<MemberLean>>,
}

/// Which way a soldier leans out round his cover, facing the threat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeanSide {
    Left,
    Right,
}

/// A soldier out on his lean this tick: the side, and the point his
/// body stands at while he fires from past his cover's edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemberLean {
    pub side: LeanSide,
    pub at: [f64; 2],
}

/// A squad's area: a disc round its anchor, which only an order moves.
/// Its soldiers take cover and fire inside it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SquadArea {
    pub anchor: [f64; 2],
    pub radius: f64,
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
    /// A radar-only report from a radar receiver. It is deliberately distinct
    /// from firing and last-seen evidence so presentation can label it
    /// "RADAR CONTACT" without exposing an identity or ground visibility.
    Radar,
}

/// Uncertain evidence: an area where something is or was. It carries no
/// velocity, cost, strength or exact position, and never moves by itself. It
/// carries only what the side learned when the evidence came: a last
/// sighting, the type it identified; a firing report, the weapons it heard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApproximateContact {
    pub id: ContactId,
    pub source: ContactSource,
    /// The area's centre; `z` is the cause's height when the evidence came
    /// (a helicopter's in the air, a ground unit's on the ground).
    pub center: [f64; 3],
    /// The height band the cause occupied: an air contact has no ground
    /// point to aim at, so area fire refuses it (D22).
    pub layer: AltitudeLayer,
    /// The area hangs in the air: its cause was in the air band and more
    /// than the low hover over the ground under it (D33). One heard from a
    /// helicopter sitting at the low hover lies on the ground like any other.
    pub aloft: bool,
    pub radius: f64,
    pub evidence_tick: Tick,
    pub expires_tick: Tick,
    /// The unit type previously identified by this side, retained through
    /// later firing evidence; `None` when the cause has never been identified.
    pub kind: Option<TypeIndex>,
    /// A firing report's weapons as heard, over the report's whole episode:
    /// a bit per weapon row (the arsenal's rows in name order, the layout's
    /// `roundKinds`), set for every row of each mount heard firing, since a
    /// gun's report doesn't say which round it loaded (a cannon's AP and HE
    /// sound alike). 0 for a last sighting.
    pub heard: u64,
    /// Quantized bearing from the receiver that found it, in degrees in
    /// [0, 360), counter-clockwise from +X like the map's yaw. Radar reports
    /// carry this separately from the uncertain area centre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearing_deg: Option<f64>,
    /// Quantized range from the receiver that found it, in metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_m: Option<f64>,
}

/// A solid obstacle added after the battle began that this side knows about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownProp {
    /// Its prop type: its rank in the catalog, the layout's `propKinds`.
    pub kind: PropKind,
    pub center: [f64; 2],
    pub yaw: f64,
    pub half_extents: [f64; 3],
    pub base_z: f64,
    /// The authored prop this one stands in place of (a ruin's building).
    pub replaces: Option<u32>,
    /// The side saw `replaces` destroyed with nothing in its place (a crate
    /// blown away): this entry draws nothing and only removes that prop.
    #[serde(default)]
    pub destroyed: bool,
    /// Exact identity of this current or remembered physical body.
    pub id: u32,
    /// Immutable placed-building identity; absent for ordinary bodies.
    pub building: Option<u32>,
    /// This remembered state's one integrity/garrison owner.
    pub structure_owner: Option<u32>,
    /// Immutable public-map source; absent for genuinely dynamic bodies.
    pub authored_prop: Option<u32>,
    /// A wreck's unit type: what it was, so it is drawn as that unit's wreck.
    pub wreck_of: Option<TypeIndex>,
}

/// Where a body shoved this tick stands, for a side that sees it (its
/// pusher always does). Drawing only: `known_props` re-learns a shoved body
/// in `relearn_m` steps, and planning reads that.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropDisplayPose {
    /// The body's `KnownProp::id`.
    pub prop_id: u32,
    pub center: [f64; 2],
    pub yaw: f64,
    pub base_z: f64,
}

/// Where a hull grinds a metal body it shoves this tick: halfway up the
/// lower of the two, with the pusher's horizontal travel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grind {
    pub point: [f64; 3],
    pub heading: [f64; 2],
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
    /// A guided launcher needs a current shared target and its own clear LOS (P05).
    NoOwnSight,
    /// Garrisoned: every perimeter slot facing the target is taken.
    NoFacingSlot,
    /// Entering or leaving a building: weapons wait.
    ChangingPosition,
}

/// One weapon mount's readiness: what the rings and panel show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MountReadiness {
    /// Index into the unit type's mount list.
    pub mount: u8,
    /// Ammunition kind ready in any surviving physical gun.
    pub loaded: Option<u8>,
    /// Total rounds left per kind (including magazines); `None` is unlimited.
    pub ammo: Vec<Option<u32>>,
    /// Aim progress in [0, 1]; 1 once acquired.
    pub aim: f64,
    /// Soonest reload progress in [0, 1], when no gun is ready (empty or topping up).
    pub reload: f64,
    /// The ammunition kind being reloaded, if any.
    pub reloading: Option<u8>,
    pub target: Option<TargetRef>,
    pub reason: ActionReason,
    /// This mount is guiding a missile in flight.
    pub guiding: bool,
}

/// One of this side's own guided missiles: where it is and its commanded
/// point; `supported` while its launcher still guides it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuidedMissile {
    /// Stable while it flies: this side's own round, so no disclosure.
    pub id: u64,
    pub position: [f64; 3],
    /// Its commanded point; a top-attack missile climbs above it before
    /// diving onto it, so this is not where it is heading now.
    pub point: [f64; 3],
    pub supported: bool,
}

/// Which way an aircraft crosses the map edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AirTransitDirection {
    /// Arriving onto the map.
    In,
    /// Leaving it.
    Out,
}

/// One of this side's own aircraft entering or leaving over the map edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AirTransit {
    /// Its unit type.
    pub kind: TypeIndex,
    /// Where it crosses the edge: for `In`, the entry point at cruise height
    /// above the ground; for `Out`, the jet's position as it crosses.
    pub point: [f64; 3],
    /// Radians, counter-clockwise from +X.
    pub heading: f64,
    /// Horizontal speed: for `In`, its cruise speed; for `Out`, its speed as
    /// it leaves.
    pub speed_mps: f64,
    /// The tick it crosses: for `In`, the future arrival tick; for `Out`,
    /// this tick.
    pub edge_tick: Tick,
    pub dir: AirTransitDirection,
}

/// What a weapon mount is doing, for posing its model. The pose itself is
/// derived only in the renderer (Q7). Published for own mounts and for the
/// mounts of identified enemies.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeaponPose {
    /// Index into the unit type's mount list.
    pub mount: u8,
    /// Current infantry operator, if present in this side's visible members.
    /// None for hull/squad weapons and unseen enemy operators.
    pub operator: Option<u32>,
    /// World bearing (radians, counter-clockwise from +X): a turret's heading,
    /// or a hand weapon's last aim.
    pub bearing: f64,
    /// Elevation above the horizontal of the mount's last launched round
    /// (radians); 0 until it first fires.
    pub elevation: f64,
    /// Rounds this mount has launched since the battle began, wrapping at
    /// 2³²; each soldier's individual round counts. A rise between two
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
    /// A building, wall, hedge or other prop that is not metal.
    Prop,
    Soldier,
    /// A prop whose body is metal (`PropBody.metal`): a car, wreck, lamp
    /// post or railing.
    MetalProp,
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
    /// Yielding to vehicle traffic; `blocker` says who.
    Waiting,
    /// No known route; the destination is kept and retried on relevant change.
    RouteBlocked,
    /// An attack-move holding its advance while a weapon engages (W16).
    Halted,
    /// Has somewhere to go but must finish packing before it may move (L01).
    Packing,
    /// Has somewhere to go and holds where it is while its route is worked out.
    Planning,
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

/// The phase of a unit's stationary hold (L08).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldPhase {
    /// Stationary beside the host, entering.
    Entering,
    /// Inside or aboard the host.
    Inside,
    /// Stationary inside or aboard, leaving.
    Exiting,
}

/// The physical host of a unit's hold.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostRef {
    /// A placed building and its ground footprint.
    Building {
        id: u32,
        center: [f64; 2],
        half: [f64; 2],
    },
    /// A carrier unit. Its geometry is read from the carrier's own row.
    Carrier { id: u32 },
}

/// A unit's host, phase and timer progress.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoldState {
    pub host: HostRef,
    pub phase: HoldPhase,
    /// Entering or leaving progress in [0, 1]; 1 while inside.
    pub progress: f64,
}

/// Defensive readiness is visible only to its owner.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProtectionReadiness {
    pub charges: u32,
    /// Running cooldown progress in [0,1], absent when ready.
    pub cooldown: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlareReadiness {
    pub charges: u32,
    pub cooldown: Option<f64>,
}

/// A unit of the observing side: its own state is complete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnUnit {
    pub withdrawing: bool,
    /// Remaining fuel seconds for an aircraft; absent for ground units.
    #[serde(default)]
    pub fuel_s: Option<f64>,
    /// Whether the aircraft's persistent afterburner toggle is enabled.
    #[serde(default)]
    pub afterburner: bool,
    /// Own aircraft is currently in a close air engagement. The opponent's
    /// authority id never crosses the observation boundary.
    #[serde(default)]
    pub in_dogfight: bool,
    #[serde(default)]
    pub flares: Option<FlareReadiness>,
    pub protection: Option<ProtectionReadiness>,
    pub id: UnitId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<crate::players::PlayerIndex>,
    /// Its unit type: its rank in the catalog, the layout's `unitKinds`.
    pub kind: TypeIndex,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Destination of the current movement order, if any.
    pub goal: Option<[f64; 2]>,
    /// The current movement order lets the passengers out at `goal`.
    pub goal_unloads: bool,
    pub policy: Option<RoutePolicy>,
    /// Which way the current move drives (Q31); `None` without a move.
    pub direction: Option<crate::command::MoveDirection>,
    /// Driving backwards this tick: a reverse move, or a three-point turn's
    /// reversing leg (the reverse beeper's cue).
    pub reversing: bool,
    pub state: MoveState,
    /// The friendly unit this one is waiting for (an enemy it waits for,
    /// Q14, is not named).
    pub blocker: Option<UnitId>,
    /// Remaining waypoints of the current route.
    pub route: Vec<[f64; 2]>,
    /// Destinations of the orders queued behind the current one.
    pub queue: Vec<[f64; 2]>,
    /// Whether each `queue` order lets the passengers out at its destination.
    pub queue_unloads: Vec<bool>,
    /// Living squad members' body poses (infantry only).
    /// Garrison bodies stand indoors behind their abstract window exposure points.
    pub members: Vec<[f64; 3]>,
    /// Living squad members' ids (`Soldier.id`), in `members` order.
    pub member_ids: Vec<u32>,
    /// Each living soldier's slot in his squad type, in `members` order.
    pub member_slots: Vec<u8>,
    /// Each published soldier's selected mount, in `members` order; absent
    /// when he is using no weapon. Visibility follows the member identity.
    pub member_active_mounts: Vec<Option<u8>>,
    /// Each living soldier's place in the order (D2+), in `members` order.
    pub member_orders: Vec<MemberOrder>,
    /// Each living soldier's lean, in `members` order.
    pub member_leans: Vec<Option<MemberLean>>,
    /// A squad's area round its anchor; `None` for a vehicle. What
    /// its markers show while it holds: the anchor never drifts.
    pub area: Option<SquadArea>,
    /// The bearing the unit will face where its move ends (D2, Q9): the
    /// ordered facing, else the way it travels at the end (a reverse move's
    /// held facing, Q31); its current yaw without a move.
    pub final_facing: f64,
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
    /// It trails smoke (D34), by the same rule an enemy's is published.
    #[serde(default)]
    pub smoking: bool,
    /// Health of each living soldier, in `members` order.
    pub member_hp: Vec<f64>,
    /// The squad's suppression tier (P14), from its hidden level by the
    /// rules' thresholds: the one state the sim's penalties and the UI read.
    pub suppression: SuppressionTier,
    /// Enough of its visibility samples stand on concealing ground with net
    /// concealment below 1, and no visible enemy observer or known attacker
    /// reveals it.
    pub concealed: bool,
    /// A supply vehicle's remaining stock.
    pub stock: Option<u32>,
    pub service: ServiceStatus,
    /// The host, while entering, inside or leaving it.
    pub hold: Option<HoldState>,
    pub sight: UnitSight,
}

/// A soldier's resolved place (D2+, D4): where his current move or hold
/// ends, and the cover he has now and will have there.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemberOrder {
    /// His spot, or his post while the squad holds; where he stands without one.
    pub spot: [f64; 2],
    /// The cover he has where he stands, against his squad's threat.
    pub cover_now: Option<crate::scenario::CoverTier>,
    /// The cover his spot gives, as resolved.
    pub cover_there: Option<crate::scenario::CoverTier>,
}

/// Where a unit's own sight reaches at the published tick: what spotting and
/// the fog sweep used this tick, for renderer fog. Never interpolated.
///
/// Reach toward world bearing `b` is `range * m`, where with
/// `c = cos(b - forward)`, `m = side * (1 - c²) + (c ≥ 0 ? front : rear) * c²`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitSight {
    /// Where sight starts: the unit's eye, or one eye per facade a garrison holds.
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

/// How suppressed a squad is (P14). The sim accumulates a hidden level in
/// [0, 1] from near misses and lets it fade after a lull; the rules'
/// thresholds turn it into these tiers, each with one fixed penalty
/// ([`crate::scenario::SuppressionRules`]). Vehicles are always `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionTier {
    /// Below the suppressed threshold: no penalty.
    None,
    /// Slowed: the suppressed tier's movement and reload penalties.
    Suppressed,
    /// Pinned down: the bigger penalties, and soldiers hug the ground.
    Pinned,
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
}

/// A fallen soldier: a permanent record that blocks nothing (M06).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Corpse {
    pub position: [f64; 3],
    pub own: bool,
    /// The fallen soldier's id (`Soldier.id`).
    pub soldier: u32,
    /// The unit type of the squad the soldier fought in, and his slot in it.
    pub kind: TypeIndex,
    pub slot: u8,
    /// The squad's heading when the soldier fell (radians).
    pub yaw: f64,
}

/// A body that toppled (its row `topples`: a tree knocked down by a vehicle
/// or felled by fire): a permanent record that blocks nothing and gives no
/// cover. Presentation draws the fall and what lies after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FallenBody {
    /// The body's exact identity (`KnownProp::id`, a trunk's prop id).
    pub prop: u32,
    /// Where its foot stood.
    pub at: [f64; 2],
    /// The horizontal unit direction it fell toward: the pushing hull's
    /// travel, the felling round's flight, or away from the felling burst.
    pub toward: [f64; 2],
    /// The tick it fell, whenever this side learned of it.
    pub tick: Tick,
}

/// A downed airframe still falling (D3), seen by a side that saw it go down:
/// the side that flew it and any side that identified it as it died. It is
/// no unit and no body; where it lands, its wreck takes over.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FallingAirframe {
    /// The id the side knew it by: its unit id when its own, the identified
    /// handle (`ObservedTargetId`) it last had otherwise.
    pub id: u32,
    pub own: bool,
    /// Its unit type, so it is drawn as itself.
    pub kind: TypeIndex,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Nose up, radians: negative as its nose drops into the fall.
    pub pitch: f64,
    /// Right side down, radians: it leans into its spin.
    pub roll: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub skirmish: Option<crate::skirmish::SkirmishView>,
    pub tick: Tick,
    pub own: Vec<OwnUnit>,
    /// Enemies identified by any friendly sensor (team-shared).
    pub identified: Vec<IdentifiedUnit>,
    /// Uncertain evidence not explained by a current identification.
    pub contacts: Vec<ApproximateContact>,
    /// Sounds heard this bucket (empty between buckets).
    pub audible: Vec<SoundCue>,
    /// Simulation-emitted barks for this completed tick.
    #[serde(default)]
    pub shouts: Vec<Shout>,
    /// Obstacles added after setup that this side has seen or run into.
    pub known_props: Vec<KnownProp>,
    /// The bodies shoved this tick that this side sees, where they stand.
    #[serde(default)]
    pub prop_display_poses: Vec<PropDisplayPose>,
    /// Where hulls grind the metal bodies shoved this tick that this side sees.
    #[serde(default)]
    pub grinds: Vec<Grind>,
    /// Round flight this side may draw this tick (own rounds whole, enemy
    /// rounds only over ground it sees).
    pub projectiles: Vec<VisibleSegment>,
    /// Rounds that burst this tick (own everywhere, enemy over seen ground).
    pub blasts: Vec<Blast>,
    /// Own fallen, and enemy fallen this side has seen.
    pub corpses: Vec<Corpse>,
    /// Toppled bodies this side knocked down or has seen where they stood.
    pub fallen_bodies: Vec<FallenBody>,
    /// Downed airframes still falling that this side saw go down.
    #[serde(default)]
    pub crashes: Vec<FallingAirframe>,
    /// This side's own guided missiles in flight.
    pub guided: Vec<GuidedMissile>,
    /// This side's own aircraft crossing the map edge this tick: arrivals
    /// still waiting off the map, announced every tick until they cross, and
    /// withdrawing jets on the tick they leave. Recorded frames older than
    /// the field carry none.
    #[serde(default)]
    pub air_transits: Vec<AirTransit>,
    /// The fixture's completion condition, when it has one.
    pub encounter: Option<EncounterStatus>,
    /// Per-unit refusals that happened during this completed tick.
    pub refusals: Vec<crate::command::Refusal>,
    pub ground_visibility: VisibilityField,
}
