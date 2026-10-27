//! The observation as a client reads it: every view the [`Reader`](super::Reader)
//! decodes into, under the browser's `ObservationView` names.
use std::sync::Arc;

use serde::Serialize;

pub type Point2 = [f64; 2];
pub type Point3 = [f64; 3];
/// A name from one of the layout's tables.
pub type Name = Arc<str>;
pub type Shared<T> = Arc<Vec<T>>;

/// Declares a view struct, serialized under the browser's camelCase names.
macro_rules! view {
    ($name:ident { $($(#[$attr:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[$attr])* pub $field: $ty),* }
    };
}

view!(Observation { tick: f64, own: Vec<OwnUnit>, identified: Vec<Identified>, contacts: Vec<Contact>,
    audible: Vec<SoundCue>, #[serde(skip)] known_props: Shared<KnownProp>, projectiles: Vec<Projectile>,
    blasts: Vec<Blast>, air_transits: Vec<AirTransit>, #[serde(skip)] corpses: Shared<Corpse>, #[serde(skip)] fallen_bodies: Shared<FallenBody>,
    refusals: Vec<Refusal>, crashes: Vec<Crash>, shouts: Vec<Shout>, guided: Vec<Guided>, encounter: Option<Encounter>, skirmish: Option<Skirmish>,
    fog: Arc<Fog>, ground_patch: GroundPatch, prop_display_poses: Vec<PropDisplayPose>, grinds: Vec<Grind> });
view!(PropDisplayPose {
    prop_id: f64,
    center: Point2,
    yaw: f64,
    base_z: f64
});
view!(Grind {
    point: Point3,
    heading: Point2
});
view!(OwnUnit { id: f64, #[serde(skip_serializing_if = "Option::is_none")] owner: Option<f64>, kind: Name, position: Point3, yaw: f64, goal: Option<Point2>, goal_unloads: bool, policy: Option<Name>,
    direction: Option<Name>, reversing: bool, withdrawing: bool, protection: Option<Protection>, state: Name,
    fuel_s: Option<f64>, afterburner: bool, in_dogfight: bool,
    flares: Option<Protection>,
    blocker: Option<f64>, route: Vec<Point2>, queue: Vec<Point2>, queue_unloads: Vec<bool>, members: Vec<Point3>, member_ids: Vec<Option<f64>>,
    member_slots: Vec<f64>, member_active_mounts: Vec<Option<f64>>, member_orders: Vec<MemberOrder>,
    member_leans: Vec<Option<MemberLean>>, area: Option<SquadArea>, final_facing: f64, sees: Vec<f64>,
    engagement: Name, mounts: Vec<Mount>, weapon_poses: Vec<WeaponPose>, hp: f64, smoking: bool,
    member_hp: Vec<f64>, suppression: Name, concealed: bool, deployment: Option<Deployment>,
    hold: Option<Hold>, stock: Option<f64>, service: Name, sight: Sight });
view!(Protection { charges: f64, cooldown: Option<f64> });
view!(MemberOrder { spot: Point2, cover_now: Option<Name>, cover_there: Option<Name> });
view!(MemberLean {
    side: Name,
    at: Point2
});
view!(SquadArea {
    anchor: Point2,
    radius: f64
});
view!(Sight { eyes: Vec<Point3>, forward: f64, shape: SightShape, range: f64 });
view!(SightShape {
    front: f64,
    side: f64,
    rear: f64
});
view!(Hold {
    host: Host,
    phase: Name,
    progress: f64
});
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Host {
    Building {
        id: f64,
        center: Point2,
        half: Point2,
    },
    Carrier {
        id: f64,
    },
}
view!(Refusal {
    unit: f64,
    reason: Name
});
view!(Deployment {
    progress: f64,
    target: Name
});
view!(WeaponPose { mount: f64, bearing: f64, elevation: f64, shots: Option<f64>, operator: Option<f64> });
view!(Mount { mount: f64, loaded: Option<f64>, ammo: Vec<Option<f64>>, aim: f64, reload: f64,
    target: Option<MountTarget>, reason: Name, guiding: bool, reloading: Option<f64> });
view!(MountTarget { kind: Name, #[serde(skip_serializing_if = "Option::is_none")] id: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] point: Option<Point3> });
view!(Identified { id: f64, kind: Name, cost: f64, position: Point3, yaw: f64, velocity: Point2,
    members: Vec<Point3>, member_ids: Vec<Option<f64>>, member_slots: Vec<f64>,
    member_active_mounts: Vec<Option<f64>>, member_leans: Vec<Option<MemberLean>>,
    weapon_poses: Vec<WeaponPose>, reversing: bool, smoking: bool });
view!(Contact { id: f64, source: Name, center: Point3, layer: Name, aloft: bool, radius: f64,
    evidence_tick: f64, expires_tick: f64, kind: Option<Name>, heard: Vec<Name>,
    #[serde(skip_serializing_if = "Option::is_none")] bearing_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] range_m: Option<f64> });
view!(SoundCue {
    listener: f64,
    category: Name,
    sector: f64,
    band: Name,
    moving: bool
});
view!(KnownProp { kind: Name, center: Point2, yaw: f64, half: Point3, base_z: f64, replaces: Option<f64>,
    id: Option<f64>, building: Option<f64>, structure_owner: Option<f64>, authored_prop: Option<f64>,
    destroyed: bool, wreck_of: Option<Name> });
view!(Projectile { path: Vec<Point3>, ricochets: Vec<Ricochet>, own: bool, kind: Name,
    shooter_member: Option<f64>, hit: Name, impact_normal: Option<Point3> });
view!(Ricochet {
    point: f64,
    normal: Point3
});
view!(Blast {
    point: Point3,
    radius: f64,
    kind: Name
});
view!(Guided { id: Option<f64>, position: Point3, point: Point3, supported: bool });
view!(AirTransit {
    kind: Name,
    point: Point3,
    heading: f64,
    speed_mps: f64,
    edge_tick: f64,
    dir: Name
});
view!(Corpse { position: Point3, own: bool, soldier: Option<f64>, kind: Name, slot: f64, yaw: f64 });
view!(FallenBody { prop: Option<f64>, at: Point2, toward: Point2, tick: f64 });
view!(Crash {
    id: f64,
    own: bool,
    kind: Name,
    position: Point3,
    yaw: f64,
    pitch: f64,
    roll: f64
});
view!(Shout {
    own: bool,
    id: f64,
    kind: Name,
    position: Point3,
    trigger: Name,
    intensity: Name
});
view!(Encounter {
    held_s: f64,
    result: Name
});
view!(Skirmish { phase: Option<&'static str>, players: Vec<Player>, preparation_remaining_s: f64,
    scores: [f64; 2], result: Option<&'static str>, objectives: Vec<Objective>, report: Option<Report> });
view!(Player { index: f64, id: Name, controller: Name, ready: bool, credits: f64, unit_count: f64, cap: f64, pending: Vec<PendingPurchase> });
view!(Objective { id: Name, center: Point2, radius_m: f64, owner: Option<&'static str>,
    capturing: Option<&'static str>, capture_progress: f64, contested: bool });
view!(PendingPurchase {
    owner: f64,
    id: f64,
    kind: Name,
    destination: Point2,
    confirmed_tick: f64,
    blocked: bool,
    // The tick a waiting aircraft reaches the map; 0 when it doesn't wait.
    arrival_tick: f64
});
view!(Report { totals: Vec<ReportTotals>, events: Vec<ReportEvent> });
view!(ReportTotals {
    spawned: f64,
    spent: f64,
    killed: f64,
    refunded: f64
});
view!(ReportEvent { kind: &'static str, tick: Option<f64>, side: Name, unit_id: Option<f64>, unit_kind: Name, owner: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] victim_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] killer: Option<Option<Killer>> });
view!(Killer { side: Name, unit_id: Option<f64>, unit_kind: Name, owner: Option<f64>, cost: f64 });
view!(Fog { cell_m: f64, nx: f64, ny: f64, #[serde(skip)] bits: Vec<u32> });
// The runs, four words a run: tile, `start + len * 256`,
// `crater + scorch * 256` and `tracks + trampled * 256 + cleared * 65536`.
view!(GroundPatch { epoch: f64, side: Name, base_revision: f64, revision: f64, full: bool,
    #[serde(skip)] runs: Vec<f32> });
