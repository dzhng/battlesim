//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: the header, then each group in layout order. A group is
//! `countField` rows of `fields`, followed by each row's variable sections in
//! row order (section by section, each `count` points of `fields`). Then comes
//! visibility snapshots or indexed word replacements, each as exact 16-bit limbs,
//! and last the side's ground patch: its cells, each two 16-bit limbs of the
//! cell index and two floats of 8-bit marks (two, then three).
//!
//! Visibility and ground patches depend on what the consumer already holds.
//! [`Publisher`] keeps their cursors at the transport end; the battle never
//! sees them. Both open together on a new side/epoch.
//!
//! Integers that grow without bound (soldier ids, shot counters) would lose
//! exactness in one float past 2²⁴, so they travel as two 16-bit limbs: a
//! field pair `<name>Lo`, `<name>Hi` holding `lo + hi · 2^limbBits`, both -1
//! when absent.
use contract::command::{Engagement, MoveDirection, RoutePolicy, TargetRef};
use contract::ids::Side;
use contract::observation::{
    ActionReason, ContactSource, EncounterResult, GarrisonPhase, GroundPatch, LeanSide, MemberLean,
    MoveState, ObservationFrame, Posture, SegmentHit, ServiceStatus, SoundBand, SoundCategory,
    SuppressionTier, WeaponPose,
};
use contract::scenario::CoverTier;

use crate::battle::Battle;

const MOVE_STATES: [MoveState; 6] = [
    MoveState::Idle,
    MoveState::Moving,
    MoveState::Waiting,
    MoveState::RouteBlocked,
    MoveState::Halted,
    MoveState::Packing,
];
const COVER_TIERS: [CoverTier; 3] = [CoverTier::Light, CoverTier::Medium, CoverTier::Heavy];
const LEAN_SIDES: [LeanSide; 2] = [LeanSide::Left, LeanSide::Right];
const POSTURES: [Posture; 2] = [Posture::Packed, Posture::Deployed];
const POLICIES: [RoutePolicy; 2] = [RoutePolicy::Shortest, RoutePolicy::Fastest];
const DIRECTIONS: [MoveDirection; 2] = [MoveDirection::Forward, MoveDirection::Reverse];
const CONTACT_SOURCES: [ContactSource; 2] = [ContactSource::Firing, ContactSource::LastSeen];
const SOUND_CATEGORIES: [SoundCategory; 3] = [
    SoundCategory::Infantry,
    SoundCategory::Vehicle,
    SoundCategory::Shot,
];
const SOUND_BANDS: [SoundBand; 2] = [SoundBand::Near, SoundBand::Far];
/// At most the fixed 18 km extent at the finest accepted 2 m fog grid.
/// A snapshot holds two exact 16-bit limbs per word: at most 20,250,000 bytes.
pub const MAX_FOG_WORDS: usize = 2_531_250;
const LIMB_BITS: u32 = 16;
const SEGMENT_HITS: [SegmentHit; 5] = [
    SegmentHit::None,
    SegmentHit::Ground,
    SegmentHit::Hull,
    SegmentHit::Prop,
    SegmentHit::Soldier,
];
const ENGAGEMENTS: [Engagement; 2] = [Engagement::FireAtWill, Engagement::ReturnFireOnly];
const ENCOUNTER_RESULTS: [EncounterResult; 4] = [
    EncounterResult::Running,
    EncounterResult::Captured,
    EncounterResult::Defeated,
    EncounterResult::Inconclusive,
];
const SERVICE_STATUSES: [ServiceStatus; 7] = [
    ServiceStatus::OutOfRange,
    ServiceStatus::SourceNotDeployed,
    ServiceStatus::Moving,
    ServiceStatus::Firing,
    ServiceStatus::Serving,
    ServiceStatus::NoStock,
    ServiceStatus::Full,
];
const SUPPRESSION_TIERS: [SuppressionTier; 3] = [
    SuppressionTier::None,
    SuppressionTier::Suppressed,
    SuppressionTier::Pinned,
];
const GARRISON_PHASES: [GarrisonPhase; 3] = [
    GarrisonPhase::Entering,
    GarrisonPhase::Inside,
    GarrisonPhase::Exiting,
];
const REASONS: [ActionReason; 16] = [
    ActionReason::Firing,
    ActionReason::NoCompatibleTarget,
    ActionReason::HoldingFire,
    ActionReason::OutOfRange,
    ActionReason::BlockedTrajectory,
    ActionReason::FriendlyInLine,
    ActionReason::Aiming,
    ActionReason::Reloading,
    ActionReason::TurretTraversing,
    ActionReason::MovingStationaryWeapon,
    ActionReason::OutOfAmmo,
    ActionReason::TrackingLastSighting,
    ActionReason::Guiding,
    ActionReason::NoOwnSight,
    ActionReason::NoFacingSlot,
    ActionReason::ChangingPosition,
];
const TARGET_KINDS: [&str; 4] = ["none", "identified", "contact", "ground"];
/// Ammunition kinds per mount the record carries (the cannon's AP and HE).
pub const MAX_AMMO_KINDS: usize = 2;
/// Weapon rows a firing report's `heard` mask can name: a float holds an
/// integer exactly below 2^24.
pub const MAX_WEAPON_ROWS: usize = 24;
const MOUNT_FIELDS: [&str; 15] = [
    "mount",
    "loaded",
    "ammo0",
    "ammo1",
    "aim",
    "reload",
    "targetKind",
    "targetId",
    "targetX",
    "targetY",
    "targetZ",
    "reason",
    "kinds",
    "guiding",
    "reloadKind",
];

const POSE_FIELDS: [&str; 5] = ["mount", "bearing", "elevation", "shotsLo", "shotsHi"];

const HEADER: [&str; 27] = [
    "tick",
    "ownCount",
    "identifiedCount",
    "contactCount",
    "audibleCount",
    "knownPropCount",
    "projectileCount",
    "blastCount",
    "corpseCount",
    "guidedCount",
    "encounterHeldS",
    "encounterResult",
    "fogCellM",
    "fogNx",
    "fogNy",
    "fogFloats",
    "fogFull",
    "fogBaseLo",
    "fogBaseHi",
    "fogRevisionLo",
    "fogRevisionHi",
    "groundEpoch",
    "groundSide",
    "groundBase",
    "groundRevision",
    "groundFull",
    "groundCellCount",
];
const GROUND_FIELDS: [&str; 4] = ["cellLo", "cellHi", "craterScorch", "tracksTrampledCleared"];
const OWN_FIELDS: [&str; 42] = [
    "id",
    "kind",
    "x",
    "y",
    "z",
    "yaw",
    "goalX",
    "goalY",
    "policy",
    "direction",
    "reversing",
    "state",
    "blocker",
    "routeCount",
    "queueCount",
    "memberCount",
    "seesCount",
    "engagement",
    "mountCount",
    "hp",
    "suppression",
    "deployProgress",
    "deployTarget",
    "garrisonBuilding",
    "garrisonPhase",
    "garrisonProgress",
    "garrisonX",
    "garrisonY",
    "garrisonHalfX",
    "garrisonHalfY",
    "stock",
    "service",
    "sightForward",
    "sightFront",
    "sightSide",
    "sightRear",
    "sightRange",
    "sightEyeCount",
    "finalFacing",
    "areaX",
    "areaY",
    "areaM",
];
const IDENTIFIED_FIELDS: [&str; 12] = [
    "id",
    "kind",
    "cost",
    "x",
    "y",
    "z",
    "yaw",
    "vx",
    "vy",
    "memberCount",
    "poseCount",
    "reversing",
];

/// An integer as its two exact 16-bit limbs.
fn limbs(n: u32) -> [f32; 2] {
    [(n & 0xffff) as f32, (n >> LIMB_BITS) as f32]
}

fn limbs_or_absent(n: Option<u32>) -> [f32; 2] {
    n.map_or([-1.0; 2], limbs)
}

fn pose(p: &WeaponPose) -> [f32; 5] {
    let [lo, hi] = limbs(p.shots);
    [p.mount as f32, p.bearing as f32, p.elevation as f32, lo, hi]
}

const LEAN_FIELDS: [&str; 3] = ["side", "x", "y"];

/// A member's lean, or none (side -1, NaN point).
fn lean(l: &Option<MemberLean>) -> [f32; 3] {
    l.map_or([-1.0, f32::NAN, f32::NAN], |l| {
        [tag(&LEAN_SIDES, &l.side), l.at[0] as f32, l.at[1] as f32]
    })
}

/// The `memberIds` rows: each soldier's id limbs and his slot.
fn member_ids<'a>(ids: &'a [u32], slots: &'a [u8]) -> impl Iterator<Item = f32> + 'a {
    ids.iter().zip(slots).flat_map(|(&id, &slot)| {
        let [lo, hi] = limbs(id);
        [lo, hi, slot as f32]
    })
}

fn tag<T: PartialEq>(all: &[T], v: &T) -> f32 {
    all.iter().position(|k| k == v).unwrap() as f32
}

fn names<T: std::fmt::Debug>(all: &[T]) -> Vec<String> {
    all.iter()
        .map(|k| {
            let s = format!("{k:?}");
            // CamelCase → snake_case, matching the serde names.
            s.chars()
                .enumerate()
                .fold(String::new(), |mut out, (i, c)| {
                    if c.is_uppercase() && i > 0 {
                        out.push('_');
                    }
                    out.push(c.to_ascii_lowercase());
                    out
                })
        })
        .collect()
}

/// The layout of every publication in `battle`: its round kinds are the
/// rules' weapon rows (in name order), its ground grid the ground layer's.
pub fn layout_json(battle: &Battle) -> String {
    let round_kinds: Vec<&str> = battle
        .arsenal()
        .weapons
        .iter()
        .map(|w| w.id.as_str())
        .collect();
    let ground = battle.ground();
    serde_json::json!({
        "header": HEADER,
        "groups": [
            {
                "name": "own",
                "count": "ownCount",
                "fields": &OWN_FIELDS[..],
                "sections": [
                    { "name": "route", "count": "routeCount", "fields": ["x", "y"] },
                    { "name": "queue", "count": "queueCount", "fields": ["x", "y"] },
                    { "name": "members", "count": "memberCount", "fields": ["x", "y", "z"] },
                    { "name": "memberHp", "count": "memberCount", "fields": ["hp"] },
                    { "name": "memberIds", "count": "memberCount", "fields": ["idLo", "idHi", "slot"] },
                    {
                        "name": "memberOrders",
                        "count": "memberCount",
                        "fields": ["x", "y", "coverNow", "coverThere"],
                    },
                    { "name": "memberLeans", "count": "memberCount", "fields": LEAN_FIELDS },
                    { "name": "sees", "count": "seesCount", "fields": ["id"] },
                    { "name": "mounts", "count": "mountCount", "fields": MOUNT_FIELDS },
                    { "name": "weaponPoses", "count": "mountCount", "fields": POSE_FIELDS },
                    { "name": "sightEyes", "count": "sightEyeCount", "fields": ["x", "y", "z"] },
                ],
            },
            {
                "name": "identified",
                "count": "identifiedCount",
                "fields": IDENTIFIED_FIELDS,
                "sections": [
                    { "name": "members", "count": "memberCount", "fields": ["x", "y", "z"] },
                    { "name": "memberIds", "count": "memberCount", "fields": ["idLo", "idHi", "slot"] },
                    { "name": "memberLeans", "count": "memberCount", "fields": LEAN_FIELDS },
                    { "name": "weaponPoses", "count": "poseCount", "fields": POSE_FIELDS },
                ],
            },
            {
                "name": "contacts",
                "count": "contactCount",
                "fields": [
                    "id", "source", "x", "y", "radius", "evidenceTick", "expiresTick", "kind", "heard", "primaryLabel",
                ],
                "sections": [],
            },
            {
                "name": "audible",
                "count": "audibleCount",
                "fields": ["listener", "category", "sector", "band", "moving"],
                "sections": [],
            },
            {
                "name": "projectiles",
                "count": "projectileCount",
                "fields": [
                    "pointCount", "ricochetCount", "own", "kind", "shooterLo", "shooterHi", "hit",
                    "nx", "ny", "nz",
                ],
                "sections": [
                    { "name": "path", "count": "pointCount", "fields": ["x", "y", "z"] },
                    {
                        "name": "ricochets",
                        "count": "ricochetCount",
                        "fields": ["point", "nx", "ny", "nz"],
                    },
                ],
            },
            {
                "name": "blasts",
                "count": "blastCount",
                "fields": ["x", "y", "z", "radius", "kind"],
                "sections": [],
            },
            {
                "name": "guided",
                "count": "guidedCount",
                "fields": ["idLo", "idHi", "x", "y", "z", "px", "py", "pz", "supported"],
                "sections": [],
            },
            {
                "name": "corpses",
                "count": "corpseCount",
                "fields": ["x", "y", "z", "own", "soldierLo", "soldierHi", "kind", "slot", "yaw"],
                "sections": [],
            },
            {
                "name": "knownProps",
                "count": "knownPropCount",
                "fields": ["kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ", "replaces", "destroyed"],
                "sections": [],
            },
        ],
        "fog": { "count": "fogFloats", "maxWords": MAX_FOG_WORDS },
        // A cell index is limbs (row-major over cols x rows cells of cellM);
        // craterScorch is crater + scorch * 256; tracksTrampledCleared is tracks + trampled * 256 + cleared * 65536.
        "ground": {
            "count": "groundCellCount",
            "fields": GROUND_FIELDS,
            "cellM": ground.cell_m(),
            "cols": ground.cols(),
            "rows": ground.rows(),
            "sides": names(&Side::ALL),
        },
        "limbBits": LIMB_BITS,
        "roundKinds": round_kinds,
        "hitKinds": names(&SEGMENT_HITS),
        "unitKinds": battle.rules().catalog.ids(),
        "moveStates": names(&MOVE_STATES),
        "policies": names(&POLICIES),
        "directions": names(&DIRECTIONS),
        "contactSources": names(&CONTACT_SOURCES),
        "soundCategories": names(&SOUND_CATEGORIES),
        "soundBands": names(&SOUND_BANDS),
        "propKinds": battle.world().types().ids(),
        "engagements": names(&ENGAGEMENTS),
        "actionReasons": names(&REASONS),
        "targetKinds": TARGET_KINDS,
        "postures": names(&POSTURES),
        "coverTiers": names(&COVER_TIERS),
        "leanSides": names(&LEAN_SIDES),
        "garrisonPhases": names(&GARRISON_PHASES),
        "serviceStatuses": names(&SERVICE_STATUSES),
        "suppressionTiers": names(&SUPPRESSION_TIERS),
        "encounterResults": names(&ENCOUNTER_RESULTS),
        // Mount ammo is rounds left per kind: -1 unlimited, -2 no such kind.
        // goalX/goalY are NaN without a movement order; policy, direction and blocker are -1 when absent.
        // reversing is 1 while the unit drives backwards this tick, else 0.
        // finalFacing is the bearing the unit ends its move at (its yaw
        // without one). A member order is the soldier's spot (his post while
        // holding, where he stands without either); coverNow and coverThere
        // index coverTiers, -1 for none.
        // A member lean is a soldier out past his cover's edge while he fires:
        // side indexes leanSides and x, y is where his body stands meanwhile
        // (his members position stays where he tucks in); side -1 and x, y
        // NaN while he is tucked in. areaX, areaY and areaM are a squad's
        // anchor and area radius, NaN for a vehicle.
        // deployProgress and deployTarget are -1 for units that never deploy.
        // garrisonBuilding, garrisonPhase and garrisonProgress are -1 without a
        // building; garrisonX, garrisonY (its centre) and garrisonHalfX,
        // garrisonHalfY (its footprint's half extents) are NaN.
        // A known prop's replaces is the authored prop it stands in place of, or -1.
        // sightForward is the bearing sight looks along at this tick (never
        // interpolated); reach toward bearing b is sightRange * m, with
        // c = cos(b - sightForward), m = side·(1 − c²) + (c ≥ 0 ? front : rear)·c².
        // A contact's kind indexes unitKinds (-1 for a firing report); its
        // heard is a bitmask over roundKinds (bit k for row k; 0 for a last
        // sighting).
        // A projectile's or blast's kind indexes roundKinds; a segment's
        // shooter is absent (-1) for a vehicle's gun, and nx, ny, nz are 0
        // when hit is none. A segment's path is a polyline of at least two
        // points; each ricochet names the path point where the round glanced
        // off a hull, with the outward normal there, and hit is at the path's
        // last point. A pose's shots rise by one per round launched.
    })
    .to_string()
}

/// The transport's end of a side's publications: it packs visibility changes
/// and the ground cells its consumer lacks. A stream (an epoch) opens with a full
/// snapshot when publishing starts, when the side changes and on
/// [`Publisher::resync`], then carries only cells learned or changed since
/// the previous record. The cursor is transport state, outside the digest.
#[derive(Default)]
pub struct Publisher {
    epoch: u32,
    /// The side and knowledge revision the consumer holds.
    cursor: Option<(Side, u32)>,
    patch: Option<GroundPatch>,
    out: Vec<f32>,
    fog: Vec<u32>,
    fog_revision: u32,
    fog_grid: Option<(f64, u32, u32)>,
}

impl Publisher {
    pub fn new() -> Self {
        Self::default()
    }

    /// The next record opens a new epoch with a full snapshot (a consumer
    /// reconnected or reset its view).
    pub fn resync(&mut self) {
        self.cursor = None;
    }

    /// Pack `side`'s observation at the battle's tick with its ground patch.
    pub fn publish(&mut self, battle: &Battle, side: Side) -> &[f32] {
        let field = &battle.observe(side).ground_visibility;
        let grid = (field.cell_m, field.nx, field.ny);
        let base = match self.cursor {
            Some((s, revision)) if s == side && self.fog_grid == Some(grid) => Some(revision),
            _ => {
                self.epoch += 1;
                None
            }
        };
        let known = battle.known_ground(side);
        // Reuse the last patch's cell buffer.
        let mut cells = self.patch.take().map(|p| p.cells).unwrap_or_default();
        cells.clear();
        cells.extend(known.changes_since(base.unwrap_or(0)));
        let patch = GroundPatch {
            epoch: self.epoch,
            side,
            base_revision: base.unwrap_or(0),
            revision: known.revision(),
            full: base.is_none(),
            cells,
        };
        let frame = battle.observe(side);
        let changed: Vec<usize> = if base.is_some() {
            frame
                .ground_visibility
                .bits
                .iter()
                .zip(&self.fog)
                .enumerate()
                .filter_map(|(i, (now, before))| (now != before).then_some(i))
                .collect()
        } else {
            Vec::new()
        };
        let full = base.is_none() || changed.len() * 3 >= frame.ground_visibility.bits.len() * 2;
        let revision = self
            .fog_revision
            .checked_add(1)
            .expect("fog publication revision exhausted");
        let fog = FogPatch {
            full,
            base: if base.is_some() { self.fog_revision } else { 0 },
            revision,
            changed: &changed,
        };
        pack_frame(frame, &patch, &fog, &mut self.out);
        if full {
            self.fog.clone_from(&frame.ground_visibility.bits);
        } else {
            for &i in &changed {
                self.fog[i] = frame.ground_visibility.bits[i];
            }
        }
        self.fog_revision = revision;
        self.fog_grid = Some(grid);
        self.cursor = Some((side, patch.revision));
        self.patch = Some(patch);
        &self.out
    }

    /// The latest packed record.
    pub fn record(&self) -> &[f32] {
        &self.out
    }

    /// The ground patch of the latest record.
    pub fn last_patch(&self) -> Option<&GroundPatch> {
        self.patch.as_ref()
    }
}

/// Overwrites `out` with the packed frame and ground patch. Unit and prop
/// kinds travel as the frame carries them: their catalog ranks, which index
/// the layout's `unitKinds` and `propKinds`.
pub fn pack(frame: &ObservationFrame, ground: &GroundPatch, out: &mut Vec<f32>) {
    pack_frame(
        frame,
        ground,
        &FogPatch {
            full: true,
            base: 0,
            revision: 1,
            changed: &[],
        },
        out,
    );
}

struct FogPatch<'a> {
    full: bool,
    base: u32,
    revision: u32,
    changed: &'a [usize],
}

fn pack_frame(
    frame: &ObservationFrame,
    ground: &GroundPatch,
    patch: &FogPatch<'_>,
    out: &mut Vec<f32>,
) {
    out.clear();
    let fog = &frame.ground_visibility;
    let cells = u64::from(fog.nx) * u64::from(fog.ny);
    let words = cells.div_ceil(32);
    assert!(
        words <= MAX_FOG_WORDS as u64,
        "fog delivery requires {words} words; admitted bound is {MAX_FOG_WORDS}"
    );
    let (cells, words) = (cells as usize, words as usize);
    assert_eq!(
        fog.bits.len(),
        words,
        "fog field dimensions must match its words"
    );
    let fog_floats = if patch.full {
        words * 2
    } else {
        patch.changed.len() * 3
    };
    let [base_lo, base_hi] = limbs(patch.base);
    let [revision_lo, revision_hi] = limbs(patch.revision);
    // Plain floats hold counters exactly below 2^24: an epoch per resync,
    // a revision per fog sweep that learned something.
    debug_assert!(ground.epoch < 1 << 24 && ground.revision < 1 << 24);
    out.extend([
        frame.tick as f32,
        frame.own.len() as f32,
        frame.identified.len() as f32,
        frame.contacts.len() as f32,
        frame.audible.len() as f32,
        frame.known_props.len() as f32,
        frame.projectiles.len() as f32,
        frame.blasts.len() as f32,
        frame.corpses.len() as f32,
        frame.guided.len() as f32,
        frame.encounter.map_or(-1.0, |e| e.held_s as f32),
        frame
            .encounter
            .map_or(-1.0, |e| tag(&ENCOUNTER_RESULTS, &e.result)),
        fog.cell_m as f32,
        fog.nx as f32,
        fog.ny as f32,
        fog_floats as f32,
        patch.full as u8 as f32,
        base_lo,
        base_hi,
        revision_lo,
        revision_hi,
        ground.epoch as f32,
        tag(&Side::ALL, &ground.side),
        ground.base_revision as f32,
        ground.revision as f32,
        ground.full as u8 as f32,
        ground.cells.len() as f32,
    ]);
    for u in &frame.own {
        let [gx, gy] = u.goal.map_or([f32::NAN; 2], |g| [g[0] as f32, g[1] as f32]);
        out.extend([
            u.id.0 as f32,
            u.kind.0 as f32,
            u.position[0] as f32,
            u.position[1] as f32,
            u.position[2] as f32,
            u.yaw as f32,
            gx,
            gy,
            u.policy.map_or(-1.0, |p| tag(&POLICIES, &p)),
            u.direction.map_or(-1.0, |d| tag(&DIRECTIONS, &d)),
            u.reversing as u8 as f32,
            tag(&MOVE_STATES, &u.state),
            u.blocker.map_or(-1.0, |b| b.0 as f32),
            u.route.len() as f32,
            u.queue.len() as f32,
            u.members.len() as f32,
            u.sees.len() as f32,
            tag(&ENGAGEMENTS, &u.engagement),
            u.mounts.len() as f32,
            u.hp as f32,
            tag(&SUPPRESSION_TIERS, &u.suppression),
            u.deployment.map_or(-1.0, |d| d.progress as f32),
            u.deployment.map_or(-1.0, |d| tag(&POSTURES, &d.target)),
            u.garrison.map_or(-1.0, |g| g.building as f32),
            u.garrison.map_or(-1.0, |g| tag(&GARRISON_PHASES, &g.phase)),
            u.garrison.map_or(-1.0, |g| g.progress as f32),
            u.garrison.map_or(f32::NAN, |g| g.center[0] as f32),
            u.garrison.map_or(f32::NAN, |g| g.center[1] as f32),
            u.garrison.map_or(f32::NAN, |g| g.half[0] as f32),
            u.garrison.map_or(f32::NAN, |g| g.half[1] as f32),
            u.stock.map_or(-1.0, |n| n as f32),
            tag(&SERVICE_STATUSES, &u.service),
            u.sight.forward as f32,
            u.sight.shape.front as f32,
            u.sight.shape.side as f32,
            u.sight.shape.rear as f32,
            u.sight.range as f32,
            u.sight.eyes.len() as f32,
            u.final_facing as f32,
            u.area.map_or(f32::NAN, |a| a.anchor[0] as f32),
            u.area.map_or(f32::NAN, |a| a.anchor[1] as f32),
            u.area.map_or(f32::NAN, |a| a.radius as f32),
        ]);
    }
    for u in &frame.own {
        out.extend(u.route.iter().flat_map(|p| [p[0] as f32, p[1] as f32]));
        out.extend(u.queue.iter().flat_map(|p| [p[0] as f32, p[1] as f32]));
        out.extend(
            u.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(u.member_hp.iter().map(|&hp| hp as f32));
        out.extend(member_ids(&u.member_ids, &u.member_slots));
        let tier = |t: Option<CoverTier>| t.map_or(-1.0, |t| tag(&COVER_TIERS, &t));
        out.extend(u.member_orders.iter().flat_map(|m| {
            [
                m.spot[0] as f32,
                m.spot[1] as f32,
                tier(m.cover_now),
                tier(m.cover_there),
            ]
        }));
        out.extend(u.member_leans.iter().flat_map(lean));
        out.extend(u.sees.iter().map(|id| id.0 as f32));
        for m in &u.mounts {
            let ammo = |k: usize| m.ammo.get(k).map_or(-2.0, |a| a.map_or(-1.0, |n| n as f32));
            let (kind, id, p) = match m.target {
                None => (0.0, -1.0, [0.0; 3]),
                Some(TargetRef::Identified { id }) => (1.0, id.0 as f32, [0.0; 3]),
                Some(TargetRef::Contact { id }) => (2.0, id.0 as f32, [0.0; 3]),
                Some(TargetRef::Ground { point }) => (3.0, -1.0, point.map(|v| v as f32)),
            };
            out.extend([
                m.mount as f32,
                m.loaded.map_or(-1.0, |k| k as f32),
                ammo(0),
                ammo(1),
                m.aim as f32,
                m.reload as f32,
                kind,
                id,
                p[0],
                p[1],
                p[2],
                tag(&REASONS, &m.reason),
                m.ammo.len() as f32,
                m.guiding as u8 as f32,
                m.reloading.map_or(-1.0, |k| k as f32),
            ]);
        }
        // One pose per mount: the section is counted by mountCount.
        assert_eq!(u.weapon_poses.len(), u.mounts.len());
        out.extend(u.weapon_poses.iter().flat_map(pose));
        out.extend(
            u.sight
                .eyes
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
    }
    for e in &frame.identified {
        out.extend([
            e.id.0 as f32,
            e.kind.0 as f32,
            e.cost as f32,
            e.position[0] as f32,
            e.position[1] as f32,
            e.position[2] as f32,
            e.yaw as f32,
            e.velocity[0] as f32,
            e.velocity[1] as f32,
            e.members.len() as f32,
            e.weapon_poses.len() as f32,
            e.reversing as u8 as f32,
        ]);
    }
    for e in &frame.identified {
        out.extend(
            e.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(member_ids(&e.member_ids, &e.member_slots));
        out.extend(e.member_leans.iter().flat_map(lean));
        out.extend(e.weapon_poses.iter().flat_map(pose));
    }
    for c in &frame.contacts {
        out.extend([
            c.id.0 as f32,
            tag(&CONTACT_SOURCES, &c.source),
            c.center[0] as f32,
            c.center[1] as f32,
            c.radius as f32,
            c.evidence_tick as f32,
            c.expires_tick as f32,
            c.kind.map_or(-1.0, |k| k.0 as f32),
            c.heard as f32,
            u8::from(c.primary_label) as f32,
        ]);
    }
    for a in &frame.audible {
        out.extend([
            a.listener.0 as f32,
            tag(&SOUND_CATEGORIES, &a.category),
            a.sector as f32,
            tag(&SOUND_BANDS, &a.band),
            a.moving as u8 as f32,
        ]);
    }
    for p in &frame.projectiles {
        let [lo, hi] = limbs_or_absent(p.shooter_member);
        let n = p.impact_normal.unwrap_or([0.0; 3]);
        out.extend([
            p.path.len() as f32,
            p.ricochets.len() as f32,
            p.own as u8 as f32,
            p.kind as f32,
            lo,
            hi,
            tag(&SEGMENT_HITS, &p.hit),
            n[0] as f32,
            n[1] as f32,
            n[2] as f32,
        ]);
    }
    for p in &frame.projectiles {
        out.extend(
            p.path
                .iter()
                .flat_map(|q| [q[0] as f32, q[1] as f32, q[2] as f32]),
        );
        out.extend(p.ricochets.iter().flat_map(|r| {
            [
                r.point as f32,
                r.normal[0] as f32,
                r.normal[1] as f32,
                r.normal[2] as f32,
            ]
        }));
    }
    for b in &frame.blasts {
        out.extend([
            b.point[0] as f32,
            b.point[1] as f32,
            b.point[2] as f32,
            b.radius as f32,
            b.kind as f32,
        ]);
    }
    for g in &frame.guided {
        let [lo, hi] = limbs(u32::try_from(g.id).expect("a guided id fits two limbs"));
        out.extend([
            lo,
            hi,
            g.position[0] as f32,
            g.position[1] as f32,
            g.position[2] as f32,
            g.point[0] as f32,
            g.point[1] as f32,
            g.point[2] as f32,
            g.supported as u8 as f32,
        ]);
    }
    for c in &frame.corpses {
        let [lo, hi] = limbs(c.soldier);
        out.extend([
            c.position[0] as f32,
            c.position[1] as f32,
            c.position[2] as f32,
            c.own as u8 as f32,
            lo,
            hi,
            c.kind.0 as f32,
            c.slot as f32,
            c.yaw as f32,
        ]);
    }
    for p in &frame.known_props {
        out.extend([
            p.kind.0 as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.half_extents[0] as f32,
            p.half_extents[1] as f32,
            p.half_extents[2] as f32,
            p.base_z as f32,
            p.replaces.map_or(-1.0, |id| id as f32),
            p.destroyed as u8 as f32,
        ]);
    }
    let word = |i: usize| {
        let value = fog.bits[i];
        if i + 1 == words && !cells.is_multiple_of(32) {
            value & ((1u32 << (cells % 32)) - 1)
        } else {
            value
        }
    };
    if patch.full {
        for i in 0..words {
            out.extend(limbs(word(i)));
        }
    } else {
        for &i in patch.changed {
            // MAX_FOG_WORDS keeps the word index exact in a single float.
            out.push(i as f32);
            out.extend(limbs(word(i)));
        }
    }
    for c in &ground.cells {
        let [lo, hi] = limbs(c.cell);
        out.extend([
            lo,
            hi,
            (c.crater as u32 | (c.scorch as u32) << 8) as f32,
            (c.tracks as u32 | (c.trampled as u32) << 8 | (c.cleared as u32) << 16) as f32,
        ]);
    }
}
