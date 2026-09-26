//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: the header, then each group in layout order. A group is
//! `countField` rows of `fields`, followed by each row's variable sections in
//! row order (section by section, each `count` points of `fields`). Then comes
//! the ground-visibility bitset, 16 bits per float so every value is exact,
//! and last the side's ground patch: its cells, each two 16-bit limbs of the
//! cell index and two floats of two 8-bit marks.
//!
//! The ground patch is the one part of a record that depends on what the
//! consumer already holds, so a [`Publisher`] (the transport's end) packs
//! records and keeps the cursor; the battle never sees it.
//!
//! Integers that grow without bound (soldier ids, shot counters) would lose
//! exactness in one float past 2²⁴, so they travel as two 16-bit limbs: a
//! field pair `<name>Lo`, `<name>Hi` holding `lo + hi · 2^limbBits`, both -1
//! when absent.
use contract::command::{Engagement, RoutePolicy, TargetRef};
use contract::ids::Side;
use contract::observation::{
    ActionReason, ContactSource, EncounterResult, GarrisonPhase, GroundPatch, MoveState,
    ObservationFrame, Posture, SegmentHit, ServiceStatus, SoundBand, SoundCategory, WeaponPose,
};
use contract::scenario::UnitKind;

use crate::battle::Battle;

pub const UNIT_KINDS: [UnitKind; 6] = [
    UnitKind::Rifle,
    UnitKind::Recon,
    UnitKind::At,
    UnitKind::Tank,
    UnitKind::Supply,
    UnitKind::Jeep,
];
const MOVE_STATES: [MoveState; 6] = [
    MoveState::Idle,
    MoveState::Moving,
    MoveState::Waiting,
    MoveState::RouteBlocked,
    MoveState::Halted,
    MoveState::Packing,
];
const POSTURES: [Posture; 2] = [Posture::Packed, Posture::Deployed];
const POLICIES: [RoutePolicy; 2] = [RoutePolicy::Shortest, RoutePolicy::Fastest];
const CONTACT_SOURCES: [ContactSource; 2] = [ContactSource::Firing, ContactSource::LastSeen];
const SOUND_CATEGORIES: [SoundCategory; 3] = [
    SoundCategory::Infantry,
    SoundCategory::Vehicle,
    SoundCategory::Shot,
];
const SOUND_BANDS: [SoundBand; 2] = [SoundBand::Near, SoundBand::Far];
const FOG_BITS_PER_FLOAT: usize = 16;
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
const SERVICE_STATUSES: [ServiceStatus; 8] = [
    ServiceStatus::OutOfRange,
    ServiceStatus::SourceNotDeployed,
    ServiceStatus::Moving,
    ServiceStatus::Firing,
    ServiceStatus::Serving,
    ServiceStatus::NoStock,
    ServiceStatus::Full,
    ServiceStatus::Garrisoned,
];
const GARRISON_PHASES: [GarrisonPhase; 4] = [
    GarrisonPhase::Entering,
    GarrisonPhase::WaitingForRoom,
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

const HEADER: [&str; 22] = [
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
    "groundEpoch",
    "groundSide",
    "groundBase",
    "groundRevision",
    "groundFull",
    "groundCellCount",
];
const GROUND_FIELDS: [&str; 4] = ["cellLo", "cellHi", "craterScorch", "tracksTrampled"];
const OWN_FIELDS: [&str; 32] = [
    "id",
    "kind",
    "x",
    "y",
    "z",
    "yaw",
    "goalX",
    "goalY",
    "policy",
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
    "stock",
    "service",
    "sightForward",
    "sightFront",
    "sightSide",
    "sightRear",
    "sightRange",
    "sightEyeCount",
];
const IDENTIFIED_FIELDS: [&str; 11] = [
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
        .map(|w| w.name.as_str())
        .collect();
    let ground = battle.ground();
    serde_json::json!({
        "header": HEADER,
        "groups": [
            {
                "name": "own",
                "count": "ownCount",
                "fields": OWN_FIELDS,
                "sections": [
                    { "name": "route", "count": "routeCount", "fields": ["x", "y"] },
                    { "name": "queue", "count": "queueCount", "fields": ["x", "y"] },
                    { "name": "members", "count": "memberCount", "fields": ["x", "y", "z"] },
                    { "name": "memberHp", "count": "memberCount", "fields": ["hp"] },
                    { "name": "memberIds", "count": "memberCount", "fields": ["idLo", "idHi"] },
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
                    { "name": "memberIds", "count": "memberCount", "fields": ["idLo", "idHi"] },
                    { "name": "weaponPoses", "count": "poseCount", "fields": POSE_FIELDS },
                ],
            },
            {
                "name": "contacts",
                "count": "contactCount",
                "fields": ["id", "source", "x", "y", "radius", "evidenceTick", "expiresTick"],
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
                "fields": ["id", "x", "y", "z", "px", "py", "pz", "supported"],
                "sections": [],
            },
            {
                "name": "corpses",
                "count": "corpseCount",
                "fields": ["x", "y", "z", "own", "soldierLo", "soldierHi", "kind", "yaw"],
                "sections": [],
            },
            {
                "name": "knownProps",
                "count": "knownPropCount",
                "fields": ["kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ", "replaces"],
                "sections": [],
            },
        ],
        "fog": { "bitsPerFloat": FOG_BITS_PER_FLOAT, "count": "fogFloats" },
        // A cell index is limbs (row-major over cols x rows cells of cellM);
        // craterScorch is crater + scorch * 256, tracksTrampled likewise.
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
        "unitKinds": names(&UNIT_KINDS),
        "moveStates": names(&MOVE_STATES),
        "policies": names(&POLICIES),
        "contactSources": names(&CONTACT_SOURCES),
        "soundCategories": names(&SOUND_CATEGORIES),
        "soundBands": names(&SOUND_BANDS),
        "propKinds": names(&crate::world::export::PROP_KINDS),
        "engagements": names(&ENGAGEMENTS),
        "actionReasons": names(&REASONS),
        "targetKinds": TARGET_KINDS,
        "postures": names(&POSTURES),
        "garrisonPhases": names(&GARRISON_PHASES),
        "serviceStatuses": names(&SERVICE_STATUSES),
        "encounterResults": names(&ENCOUNTER_RESULTS),
        // Mount ammo is rounds left per kind: -1 unlimited, -2 no such kind.
        // goalX/goalY are NaN without a movement order; policy and blocker are -1 when absent.
        // deployProgress and deployTarget are -1 for units that never deploy.
        // garrisonBuilding, garrisonPhase and garrisonProgress are -1 without a building.
        // A known prop's replaces is the authored prop it stands in place of, or -1.
        // sightForward is the bearing sight looks along at this tick (never
        // interpolated); reach toward bearing b is sightRange * m, with
        // c = cos(b - sightForward), m = side·(1 − c²) + (c ≥ 0 ? front : rear)·c².
        // A projectile's or blast's kind indexes roundKinds; a segment's
        // shooter is absent (-1) for a vehicle's gun, and nx, ny, nz are 0
        // when hit is none. A segment's path is a polyline of at least two
        // points; each ricochet names the path point where the round glanced
        // off a hull, with the outward normal there, and hit is at the path's
        // last point. A pose's shots rise by one per round launched.
    })
    .to_string()
}

/// The transport's end of a side's publications: it packs each record with
/// the ground cells its consumer lacks. A stream (an epoch) opens with a full
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
        let base = match self.cursor {
            Some((s, revision)) if s == side => Some(revision),
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
        pack(battle.observe(side), &patch, &mut self.out);
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

/// Overwrites `out` with the packed frame and ground patch.
pub fn pack(frame: &ObservationFrame, ground: &GroundPatch, out: &mut Vec<f32>) {
    out.clear();
    let fog = &frame.ground_visibility;
    let cells = (fog.nx * fog.ny) as usize;
    let fog_floats = cells.div_ceil(FOG_BITS_PER_FLOAT);
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
            tag(&UNIT_KINDS, &u.kind),
            u.position[0] as f32,
            u.position[1] as f32,
            u.position[2] as f32,
            u.yaw as f32,
            gx,
            gy,
            u.policy.map_or(-1.0, |p| tag(&POLICIES, &p)),
            tag(&MOVE_STATES, &u.state),
            u.blocker.map_or(-1.0, |b| b.0 as f32),
            u.route.len() as f32,
            u.queue.len() as f32,
            u.members.len() as f32,
            u.sees.len() as f32,
            tag(&ENGAGEMENTS, &u.engagement),
            u.mounts.len() as f32,
            u.hp as f32,
            u.suppression as f32,
            u.deployment.map_or(-1.0, |d| d.progress as f32),
            u.deployment.map_or(-1.0, |d| tag(&POSTURES, &d.target)),
            u.garrison.map_or(-1.0, |g| g.building as f32),
            u.garrison.map_or(-1.0, |g| tag(&GARRISON_PHASES, &g.phase)),
            u.garrison.map_or(-1.0, |g| g.progress as f32),
            u.stock.map_or(-1.0, |n| n as f32),
            tag(&SERVICE_STATUSES, &u.service),
            u.sight.forward as f32,
            u.sight.shape.front as f32,
            u.sight.shape.side as f32,
            u.sight.shape.rear as f32,
            u.sight.range as f32,
            u.sight.eyes.len() as f32,
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
        out.extend(u.member_ids.iter().flat_map(|&id| limbs(id)));
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
            tag(&UNIT_KINDS, &e.kind),
            e.cost as f32,
            e.position[0] as f32,
            e.position[1] as f32,
            e.position[2] as f32,
            e.yaw as f32,
            e.velocity[0] as f32,
            e.velocity[1] as f32,
            e.members.len() as f32,
            e.weapon_poses.len() as f32,
        ]);
    }
    for e in &frame.identified {
        out.extend(
            e.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(e.member_ids.iter().flat_map(|&id| limbs(id)));
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
        out.extend([
            g.id as f32,
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
            tag(&UNIT_KINDS, &c.kind),
            c.yaw as f32,
        ]);
    }
    for p in &frame.known_props {
        out.extend([
            tag(&crate::world::export::PROP_KINDS, &p.kind),
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.half_extents[0] as f32,
            p.half_extents[1] as f32,
            p.half_extents[2] as f32,
            p.base_z as f32,
            p.replaces.map_or(-1.0, |id| id as f32),
        ]);
    }
    for w in 0..fog_floats {
        let mut v = 0u32;
        for b in 0..FOG_BITS_PER_FLOAT {
            let k = w * FOG_BITS_PER_FLOAT + b;
            if k < cells && fog.bits[k / 32] & (1 << (k % 32)) != 0 {
                v |= 1 << b;
            }
        }
        out.push(v as f32);
    }
    for c in &ground.cells {
        let [lo, hi] = limbs(c.cell);
        out.extend([
            lo,
            hi,
            (c.crater as u32 | (c.scorch as u32) << 8) as f32,
            (c.tracks as u32 | (c.trampled as u32) << 8) as f32,
        ]);
    }
}
