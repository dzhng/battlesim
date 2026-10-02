//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: a complete header, then independent non-map groups carrying either
//! complete f32 words, group-local replacements, or exact fixed-row source copies. Finally
//! visibility snapshots/indexed replacements carry exact 16-bit limbs, followed
//! by ground's exact tile-local runs and packed byte marks.
//!
//! [`Publisher`] retains one bounded side/epoch baseline outside the battle and
//! its digest. A new side, epoch or resync opens every delivery cursor together.
//!
//! Integers that grow without bound (soldier ids, shot counters) would lose
//! exactness in one float past 2²⁴, so they travel as two 16-bit limbs: a
//! field pair `<name>Lo`, `<name>Hi` holding `lo + hi · 2^limbBits`, both -1
//! when absent.
use contract::command::{Engagement, MoveDirection, RoutePolicy, TargetRef};
use contract::ids::Side;
use contract::observation::{
    ActionReason, ContactSource, EncounterResult, GarrisonPhase, LeanSide, MemberLean, MoveState,
    ObservationFrame, Posture, SegmentHit, ServiceStatus, SoundBand, SoundCategory,
    SuppressionTier, WeaponPose,
};
use contract::scenario::CoverTier;

use crate::battle::Battle;
use crate::ground::GroundRunPatch;

/// Atomic publication allowance; uniform Large ground plus finest fog is about
/// 40.5 MB. Higher-entropy workloads fail admission explicitly, never lose cells.
pub const MAX_PUBLICATION_BYTES: usize = 64 * 1024 * 1024;

const MOVE_STATES: [MoveState; 7] = [
    MoveState::Idle,
    MoveState::Moving,
    MoveState::Waiting,
    MoveState::RouteBlocked,
    MoveState::Halted,
    MoveState::Packing,
    MoveState::Planning,
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
const GROUP_ENCODINGS: [&str; 3] = ["replacement", "snapshot", "copies"];
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
    "groundRunCount",
];
const GROUND_FIELDS: [&str; 4] = ["tile", "span", "craterScorch", "tracksTrampledCleared"];
const CONTACT_FIELDS: [&str; 10] = [
    "id",
    "source",
    "x",
    "y",
    "radius",
    "evidenceTick",
    "expiresTick",
    "kind",
    "heard",
    "primaryLabel",
];
const AUDIBLE_FIELDS: [&str; 5] = ["listener", "category", "sector", "band", "moving"];
const PROJECTILE_FIELDS: [&str; 10] = [
    "pointCount",
    "ricochetCount",
    "own",
    "kind",
    "shooterLo",
    "shooterHi",
    "hit",
    "nx",
    "ny",
    "nz",
];
const BLAST_FIELDS: [&str; 5] = ["x", "y", "z", "radius", "kind"];
const GUIDED_FIELDS: [&str; 9] = ["idLo", "idHi", "x", "y", "z", "px", "py", "pz", "supported"];
const CORPSE_FIELDS: [&str; 9] = [
    "x",
    "y",
    "z",
    "own",
    "soldierLo",
    "soldierHi",
    "kind",
    "slot",
    "yaw",
];
const KNOWN_PROP_FIELDS: [&str; 19] = [
    "kind",
    "x",
    "y",
    "yaw",
    "hx",
    "hy",
    "hz",
    "baseZ",
    "replacesLo",
    "replacesHi",
    "destroyed",
    "idLo",
    "idHi",
    "buildingLo",
    "buildingHi",
    "structureOwnerLo",
    "structureOwnerHi",
    "authoredPropLo",
    "authoredPropHi",
];
const OWN_FIELDS: [&str; 44] = [
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
    "concealed",
    "deployProgress",
    "deployTarget",
    "garrisonBuildingLo",
    "garrisonBuildingHi",
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
pub(crate) fn limbs(n: u32) -> [f32; 2] {
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
                "fields": CONTACT_FIELDS,
                "sections": [],
            },
            {
                "name": "audible",
                "count": "audibleCount",
                "fields": AUDIBLE_FIELDS,
                "sections": [],
            },
            {
                "name": "projectiles",
                "count": "projectileCount",
                "fields": PROJECTILE_FIELDS,
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
                "fields": BLAST_FIELDS,
                "sections": [],
            },
            {
                "name": "guided",
                "count": "guidedCount",
                "fields": GUIDED_FIELDS,
                "sections": [],
            },
            {
                "name": "corpses",
                "count": "corpseCount",
                "fields": CORPSE_FIELDS,
                "sections": [],
            },
            {
                "name": "knownProps",
                "count": "knownPropCount",
                "fields": KNOWN_PROP_FIELDS,
                "sections": [],
            },
        ],
        "groupDelivery": { "fields": ["length", "encoding", "floats"], "range": ["start", "length"], "copy": ["source", "length"], "copyAlignments": (0..9).map(|g| fixed_row_width(g).max(1)).collect::<Vec<_>>(), "encodings": GROUP_ENCODINGS },
        "fog": { "count": "fogFloats", "maxWords": MAX_FOG_WORDS },
        // A run is one 16×16 tile and start + len * 256 within it;
        // craterScorch is crater + scorch * 256; tracksTrampledCleared is tracks + trampled * 256 + cleared * 65536.
        "ground": {
            "count": "groundRunCount",
            "fields": GROUND_FIELDS,
            "tileSize": 16,
            "maxRecordBytes": MAX_PUBLICATION_BYTES,
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
        // Both garrisonBuilding limbs, phase and progress are -1 without a
        // building; garrisonX, garrisonY (its centre) and garrisonHalfX,
        // garrisonHalfY (its footprint's half extents) are NaN.
        // Known body IDs and associations use exact limbs; absent pairs are (-1,-1).
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

/// The transport's end of a side's publications: it packs group word changes,
/// visibility changes and the ground cells its consumer lacks. A stream (an epoch) opens with a full
/// snapshot when publishing starts, when the side changes and on
/// [`Publisher::resync`], then carries only cells learned or changed since
/// the previous record. The cursor is transport state, outside the digest.
#[derive(Default)]
pub struct Publisher {
    epoch: u32,
    /// The side and knowledge revision the consumer holds.
    cursor: Option<(Side, u32)>,
    ground_bytes: usize,
    out: Vec<f32>,
    fog: Vec<u32>,
    fog_revision: u32,
    fog_grid: Option<(f64, u32, u32)>,
    logical: Vec<f32>,
    encoded: Vec<f32>,
    groups: Vec<f32>,
    group_ends: Vec<usize>,
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
    pub fn publish(&mut self, battle: &Battle, side: Side) -> Result<&[f32], String> {
        let ground_grid = battle.ground();
        if ground_grid
            .cols()
            .checked_mul(ground_grid.rows())
            .is_none_or(|cells| cells > u32::MAX as usize)
            || ground_grid
                .cols()
                .div_ceil(16)
                .checked_mul(ground_grid.rows().div_ceil(16))
                .is_none_or(|tiles| tiles > 1 << 24)
        {
            return Err("ground grid exceeds its exact delivery address range".into());
        }
        let field = &battle.observe(side).ground_visibility;
        let grid = (field.cell_m, field.nx, field.ny);
        let base = match self.cursor {
            Some((s, revision)) if s == side && self.fog_grid == Some(grid) => Some(revision),
            _ => None,
        };
        let known = battle.known_ground(side);
        let epoch = if base.is_none() {
            self.epoch
                .checked_add(1)
                .ok_or("publication epoch exhausted")?
        } else {
            self.epoch
        };
        let baseline = base.unwrap_or(0);
        // A current cursor needs no walk over retained tiles. A snapshot/delta
        // counts runs without collecting a second ground array.
        let count = if known.revision() == baseline {
            0
        } else {
            known
                .change_runs_since(baseline)
                .take(MAX_PUBLICATION_BYTES / 16 + 1)
                .count()
        };
        let ground = GroundHeader {
            epoch,
            side,
            base: baseline,
            revision: known.revision(),
            full: base.is_none(),
            count,
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
            .ok_or("fog publication revision exhausted")?;
        let fog = FogPatch {
            full,
            base: if base.is_some() { self.fog_revision } else { 0 },
            revision,
            changed: &changed,
        };
        let mut ends = Vec::with_capacity(9);
        pack_record(
            frame,
            &ground,
            &fog,
            known.change_runs_since(baseline).take(count),
            &mut self.logical,
            Some(&mut ends),
        )?;
        encode_groups(
            &self.logical,
            &ends,
            if base.is_some() {
                &self.groups[..]
            } else {
                &[]
            },
            if base.is_some() {
                &self.group_ends[..]
            } else {
                &[]
            },
            &mut self.encoded,
        )?;
        self.groups
            .try_reserve_exact(ends.last().unwrap().saturating_sub(self.groups.len()))
            .map_err(|e| format!("publication baseline allocation: {e}"))?;
        std::mem::swap(&mut self.out, &mut self.encoded);
        self.groups.clear();
        self.groups
            .extend_from_slice(&self.logical[..*ends.last().unwrap()]);
        self.group_ends = ends;
        if full {
            self.fog.clone_from(&frame.ground_visibility.bits);
        } else {
            for &i in &changed {
                self.fog[i] = frame.ground_visibility.bits[i];
            }
        }
        self.fog_revision = revision;
        self.fog_grid = Some(grid);
        self.epoch = epoch;
        self.cursor = Some((side, ground.revision));
        self.ground_bytes = count * GROUND_FIELDS.len() * 4;
        Ok(&self.out)
    }

    /// The latest packed record.
    pub fn record(&self) -> &[f32] {
        &self.out
    }

    /// Encoded ground bytes in the latest successful record, for reports.
    pub fn ground_patch_bytes(&self) -> usize {
        self.ground_bytes
    }
}

/// Cursor metadata for the same side/epoch as observation visibility.
pub struct GroundHeader {
    pub epoch: u32,
    pub side: Side,
    pub base: u32,
    pub revision: u32,
    pub full: bool,
    pub count: usize,
}

/// Visibility payload decision. Revisions are transport state, not battle state.
pub struct FogPatch<'a> {
    pub full: bool,
    pub base: u32,
    pub revision: u32,
    pub changed: &'a [usize],
}

/// Canonical complete logical record, before group delivery encoding. This is
/// the reconstruction oracle; [`Publisher::publish`] owns the transport record.
/// Fog/ground payloads still follow their supplied cursor headers.
/// Admission precedes output allocation.
pub fn pack_logical(
    frame: &ObservationFrame,
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
) -> Result<(), String> {
    pack_record(frame, ground, patch, runs, out, None)
}

fn pack_record(
    frame: &ObservationFrame,
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
    mut ends: Option<&mut Vec<usize>>,
) -> Result<(), String> {
    let fog = &frame.ground_visibility;
    let cells = u64::from(fog.nx) * u64::from(fog.ny);
    let words = cells.div_ceil(32);
    if words > MAX_FOG_WORDS as u64 {
        return Err(format!(
            "fog delivery requires {words} words; admitted bound is {MAX_FOG_WORDS}"
        ));
    }
    let (cells, words) = (cells as usize, words as usize);
    if fog.bits.len() != words {
        return Err("fog field dimensions must match its words".into());
    }
    let fog_floats = if patch.full {
        words * 2
    } else {
        patch.changed.len() * 3
    };
    let [base_lo, base_hi] = limbs(patch.base);
    let [revision_lo, revision_hi] = limbs(patch.revision);
    // Plain ground counters remain exact inside their existing lifetime bound.
    if ground.epoch >= 1 << 24 || ground.revision >= 1 << 24 || ground.base >= 1 << 24 {
        return Err("ground cursor exceeds its exact float range".into());
    }
    let length = packed_len(frame, fog_floats, ground.count)?;
    out.try_reserve_exact(length.saturating_sub(out.len()))
        .map_err(|e| format!("publication allocation: {e}"))?;
    out.clear();
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
        ground.base as f32,
        ground.revision as f32,
        ground.full as u8 as f32,
        ground.count as f32,
    ]);
    for u in &frame.own {
        let [garrison_lo, garrison_hi] = limbs_or_absent(u.garrison.map(|g| g.building));
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
            u.concealed as u8 as f32,
            u.deployment.map_or(-1.0, |d| d.progress as f32),
            u.deployment.map_or(-1.0, |d| tag(&POSTURES, &d.target)),
            garrison_lo,
            garrison_hi,
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.known_props {
        let [replaces_lo, replaces_hi] = limbs_or_absent(p.replaces);
        let [id_lo, id_hi] = limbs(p.id);
        let [building_lo, building_hi] = limbs_or_absent(p.building);
        let [owner_lo, owner_hi] = limbs_or_absent(p.structure_owner);
        let [authored_lo, authored_hi] = limbs_or_absent(p.authored_prop);
        out.extend([
            p.kind.0 as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.half_extents[0] as f32,
            p.half_extents[1] as f32,
            p.half_extents[2] as f32,
            p.base_z as f32,
            replaces_lo,
            replaces_hi,
            p.destroyed as u8 as f32,
            id_lo,
            id_hi,
            building_lo,
            building_hi,
            owner_lo,
            owner_hi,
            authored_lo,
            authored_hi,
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
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
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
    let mut encoded = 0;
    for run in runs {
        if encoded >= ground.count
            || run.tile >= 1 << 24
            || run.len == 0
            || usize::from(run.start) + usize::from(run.len) > 256
        {
            out.clear();
            return Err("ground run exceeds its exact tile-local encoding".into());
        }
        let c = run.marks;
        out.extend([
            run.tile as f32,
            u32::from(run.start) as f32 + u32::from(run.len) as f32 * 256.0,
            (u32::from(c.crater) + (u32::from(c.scorch) << 8)) as f32,
            (u32::from(c.tracks) + (u32::from(c.trampled) << 8) + (u32::from(c.cleared) << 16))
                as f32,
        ]);
        encoded += 1;
    }
    if encoded != ground.count || out.len() != length {
        out.clear();
        return Err("publication values do not match their counts".into());
    }
    Ok(())
}

fn packed_len(frame: &ObservationFrame, fog: usize, runs: usize) -> Result<usize, String> {
    let mut length = HEADER.len();
    let mut add = |rows: usize, width: usize| -> Result<(), String> {
        length = rows
            .checked_mul(width)
            .and_then(|n| length.checked_add(n))
            .filter(|n| *n <= MAX_PUBLICATION_BYTES / 4)
            .ok_or("publication exceeds its 64 MiB atomic allocation allowance")?;
        Ok(())
    };
    add(frame.own.len(), OWN_FIELDS.len())?;
    for u in &frame.own {
        add(u.route.len(), 2)?;
        add(u.queue.len(), 2)?;
        add(u.members.len(), 3)?;
        add(u.member_hp.len(), 1)?;
        add(u.member_ids.len().min(u.member_slots.len()), 3)?;
        add(u.member_orders.len(), 4)?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.sees.len(), 1)?;
        add(u.mounts.len(), MOUNT_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
        add(u.sight.eyes.len(), 3)?;
    }
    add(frame.identified.len(), IDENTIFIED_FIELDS.len())?;
    for u in &frame.identified {
        add(u.members.len(), 3)?;
        add(u.member_ids.len().min(u.member_slots.len()), 3)?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
    }
    add(frame.contacts.len(), CONTACT_FIELDS.len())?;
    add(frame.audible.len(), AUDIBLE_FIELDS.len())?;
    add(frame.projectiles.len(), PROJECTILE_FIELDS.len())?;
    for p in &frame.projectiles {
        add(p.path.len(), 3)?;
        add(p.ricochets.len(), 4)?;
    }
    add(frame.blasts.len(), BLAST_FIELDS.len())?;
    add(frame.guided.len(), GUIDED_FIELDS.len())?;
    add(frame.corpses.len(), CORPSE_FIELDS.len())?;
    add(frame.known_props.len(), KNOWN_PROP_FIELDS.len())?;
    add(fog, 1)?;
    add(runs, GROUND_FIELDS.len())?;
    Ok(length)
}

/// Each group keeps its own addresses: changing a squad's variable rows never
/// shifts an unchanged corpse or known body. Values are copied as f32 words;
/// equality is bitwise, including NaN payloads and signed zero.
fn encode_groups(
    logical: &[f32],
    ends: &[usize],
    previous: &[f32],
    previous_ends: &[usize],
    out: &mut Vec<f32>,
) -> Result<(), String> {
    let mut plans = Vec::with_capacity(ends.len());
    let mut start = HEADER.len();
    let mut length = HEADER.len() + logical.len() - ends.last().copied().unwrap_or(HEADER.len());
    for (g, &end) in ends.iter().enumerate() {
        let values = &logical[start..end];
        let old = previous_ends.get(g).map_or(&[][..], |&end| {
            let start = if g == 0 {
                HEADER.len()
            } else {
                previous_ends[g - 1]
            };
            &previous[start..end]
        });
        let delta: usize = if previous.is_empty() {
            values.len()
        } else {
            changed_ranges(values, old).map(|(a, b)| 2 + b - a).sum()
        };
        let mut encoding = u8::from(previous.is_empty() || delta >= values.len());
        let mut payload = if encoding == 1 { values.len() } else { delta };
        let stride = fixed_row_width(g);
        let mut index = Vec::new();
        if !previous.is_empty()
            && payload > 2
            && !old.is_empty()
            && (stride == 0
                || (old.len().is_multiple_of(stride) && values.len().is_multiple_of(stride)))
        {
            // Fixed rows retain row alignment; variable sections use sparse exact
            // eight-word anchors. Both share one source-span wire grammar.
            let anchor = if stride == 0 { 8 } else { stride };
            index
                .try_reserve_exact(old.len() / anchor)
                .map_err(|e| format!("publication row index allocation: {e}"))?;
            index.extend(
                (0..old.len().saturating_sub(anchor - 1))
                    .step_by(anchor)
                    .map(|i| i as u32),
            );
            index.sort_unstable_by(|&a, &b| {
                compare_rows(
                    &old[a as usize..a as usize + anchor],
                    &old[b as usize..b as usize + anchor],
                )
                .then_with(|| a.cmp(&b))
            });
            let copies: usize = source_copies(values, old, stride, &index)
                .map(|(source, from, to)| 2 + if source.is_some() { 0 } else { to - from })
                .sum();
            if copies < payload {
                encoding = 2;
                payload = copies;
            } else {
                index = Vec::new();
            }
        }
        length = length
            .checked_add(3 + payload)
            .filter(|n| *n <= MAX_PUBLICATION_BYTES / 4)
            .ok_or("publication exceeds its 64 MiB atomic allocation allowance")?;
        plans.push((encoding, payload, old, stride, index));
        start = end;
    }
    // Admission and exact reservation precede writing. Geometric growth must
    // not turn the record allowance into larger retained scratch allocations.
    out.try_reserve_exact(length.saturating_sub(out.len()))
        .map_err(|e| format!("publication encoding allocation: {e}"))?;
    out.clear();
    out.extend_from_slice(&logical[..HEADER.len()]);
    start = HEADER.len();
    for (&end, (encoding, payload, old, stride, index)) in ends.iter().zip(plans) {
        let values = &logical[start..end];
        out.extend([values.len() as f32, encoding as f32, payload as f32]);
        if encoding == 1 {
            out.extend_from_slice(values);
        } else if encoding == 2 {
            for (source, from, to) in source_copies(values, old, stride, &index) {
                out.extend([source.map_or(-1.0, |s| s as f32), (to - from) as f32]);
                if source.is_none() {
                    out.extend_from_slice(&values[from..to]);
                }
            }
        } else {
            for (from, to) in changed_ranges(values, old) {
                out.extend([from as f32, (to - from) as f32]);
                out.extend_from_slice(&values[from..to]);
            }
        }
        start = end;
    }
    out.extend_from_slice(&logical[start..]);
    Ok(())
}

/// Variable-section groups cannot address complete rows by one fixed width.
fn fixed_row_width(group: usize) -> usize {
    [
        0,
        0,
        CONTACT_FIELDS.len(),
        AUDIBLE_FIELDS.len(),
        0,
        BLAST_FIELDS.len(),
        GUIDED_FIELDS.len(),
        CORPSE_FIELDS.len(),
        KNOWN_PROP_FIELDS.len(),
    ][group]
}

fn compare_rows(a: &[f32], b: &[f32]) -> std::cmp::Ordering {
    a.iter()
        .map(|v| v.to_bits())
        .cmp(b.iter().map(|v| v.to_bits()))
}

/// Assemble canonical order from exact old spans and new literals. Fixed rows
/// retain their alignment; variable spans use sparse eight-word anchors and
/// extend wordwise. Bit comparisons never depend on entity identity or hashes.
fn source_copies<'a>(
    values: &'a [f32],
    old: &'a [f32],
    stride: usize,
    index: &'a [u32],
) -> impl Iterator<Item = (Option<usize>, usize, usize)> + 'a {
    let anchor = if stride == 0 { 8 } else { stride };
    let source = move |at: usize| {
        let row = values.get(at..at + anchor)?;
        if stride == 0
            && old
                .get(at..at + anchor)
                .is_some_and(|v| compare_rows(v, row).is_eq())
        {
            return Some(at);
        }
        let i = index
            .partition_point(|&p| compare_rows(&old[p as usize..p as usize + anchor], row).is_lt());
        let p = *index.get(i)? as usize;
        compare_rows(&old[p..p + anchor], row).is_eq().then_some(p)
    };
    let mut at = 0;
    std::iter::from_fn(move || {
        if at == values.len() {
            return None;
        }
        let start = at;
        let from = source(at);
        at += if from.is_some() {
            anchor
        } else {
            stride.max(1)
        };
        while at < values.len() {
            let step = stride.max(1);
            let follows = match from {
                Some(p) => old
                    .get(p + at - start..p + at - start + step)
                    .is_some_and(|row| compare_rows(row, &values[at..at + step]).is_eq()),
                None => source(at).is_none(),
            };
            if !follows {
                break;
            }
            at += step;
        }
        Some((from, start, at))
    })
}

/// Two scans choose a smaller payload without storing a range for every word.
fn changed_ranges<'a>(
    values: &'a [f32],
    old: &'a [f32],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < values.len()
            && old
                .get(i)
                .is_some_and(|v| v.to_bits() == values[i].to_bits())
        {
            i += 1;
        }
        if i == values.len() {
            return None;
        }
        let start = i;
        i += 1;
        while i < values.len()
            && old
                .get(i)
                .is_none_or(|v| v.to_bits() != values[i].to_bits())
        {
            i += 1;
        }
        Some((start, i))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scattered_fixed_row_insertions_do_not_resend_the_retained_tail() {
        let row = |i: usize| {
            [
                i as f32,
                i as f32 + 0.25,
                -0.0,
                1.0,
                i as f32,
                0.0,
                0.0,
                0.0,
                f32::from_bits(0x7fc0_0001),
            ]
        };
        let old_rows: Vec<_> = (0..100).map(row).collect();
        let mut rows = old_rows.clone();
        for (at, id) in [(75, 102), (40, 101), (5, 100)] {
            rows.insert(at, row(id));
        }
        let record = |rows: &[[f32; 9]]| {
            let mut data = vec![0.0; HEADER.len()];
            data.extend(rows.iter().flatten());
            let mut ends = vec![HEADER.len(); 7];
            ends.extend([data.len(), data.len()]);
            (data, ends)
        };
        let (old, old_ends) = record(&old_rows);
        let (logical, ends) = record(&rows);
        let mut encoded = Vec::new();
        encode_groups(&logical, &ends, &old, &old_ends, &mut encoded).unwrap();
        assert!(
            encoded.len() * 4 < 512,
            "three new rows must not resend 100 retained rows: {} B",
            encoded.len() * 4
        );
        assert_eq!(
            decode_copies(&encoded, &old),
            rows.iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
    }

    fn decode_copies(encoded: &[f32], old: &[f32]) -> Vec<u32> {
        let mut reconstructed = Vec::new();
        let mut at = HEADER.len() + 7 * 3;
        assert_eq!(encoded[at + 1], 2.0);
        let end = at + 3 + encoded[at + 2] as usize;
        at += 3;
        while at < end {
            let source = encoded[at];
            let count = encoded[at + 1] as usize;
            at += 2;
            if source == -1.0 {
                reconstructed.extend(encoded[at..at + count].iter().map(|v| v.to_bits()));
                at += count;
            } else {
                let start = HEADER.len() + source as usize;
                reconstructed.extend(old[start..start + count].iter().map(|v| v.to_bits()));
            }
        }
        reconstructed
    }

    #[test]
    fn fixed_row_removals_and_reordering_copy_canonical_bits_without_new_literals() {
        let mut old = vec![0.0; HEADER.len()];
        for i in 0..100 {
            old.extend([
                i as f32,
                -0.0,
                0.0,
                1.0,
                i as f32,
                0.0,
                0.0,
                0.0,
                f32::from_bits(0x7fc0_0001),
            ]);
        }
        let mut old_ends = vec![HEADER.len(); 7];
        old_ends.extend([old.len(), old.len()]);
        for reverse in [false, true] {
            let mut logical = vec![0.0; HEADER.len()];
            for offset in 0..80 {
                let i = if reverse { 89 - offset } else { 10 + offset };
                logical.extend_from_slice(&old[HEADER.len() + i * 9..HEADER.len() + (i + 1) * 9]);
            }
            let mut ends = vec![HEADER.len(); 7];
            ends.extend([logical.len(), logical.len()]);
            let mut encoded = Vec::new();
            encode_groups(&logical, &ends, &old, &old_ends, &mut encoded).unwrap();
            assert!(
                encoded.len() * 4 < 1024,
                "retained 80-row reorder should copy source rows: {} B",
                encoded.len() * 4
            );
            assert_eq!(
                decode_copies(&encoded, &old),
                logical[HEADER.len()..]
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn group_addresses_survive_earlier_group_shrink_and_keep_float_bits() {
        let words = [f32::from_bits(0x7fc0_0001), -0.0, f32::INFINITY];
        let mut previous = vec![0.0; HEADER.len()];
        previous.extend([1.0, 2.0, 3.0]);
        previous.extend(words);
        let previous_ends = [HEADER.len() + 3, previous.len()];
        let mut logical = vec![0.0; HEADER.len()];
        logical.push(1.0);
        logical.extend(words);
        let mut encoded = Vec::new();
        encode_groups(
            &logical,
            &[HEADER.len() + 1, logical.len()],
            &previous,
            &previous_ends,
            &mut encoded,
        )
        .unwrap();
        assert_eq!(&encoded[HEADER.len()..], &[1.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
        logical[HEADER.len() + 2] = 0.0;
        logical[HEADER.len() + 1] = f32::from_bits(0x7fc0_0002);
        encode_groups(
            &logical,
            &[HEADER.len() + 1, logical.len()],
            &previous,
            &previous_ends,
            &mut encoded,
        )
        .unwrap();
        // Dense changes choose a full group; exact NaN payload and +0 survive.
        assert_eq!(encoded[HEADER.len() + 6].to_bits(), 0x7fc0_0002);
        assert_eq!(encoded[HEADER.len() + 7].to_bits(), 0);
        assert_eq!(encoded[HEADER.len() + 8], f32::INFINITY);
    }
}
