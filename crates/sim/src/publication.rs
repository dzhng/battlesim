//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: the header, then each group in layout order. A group is
//! `countField` rows of `fields`, followed by each row's variable sections in
//! row order (section by section, each `count` points of `fields`). Last comes
//! the ground-visibility bitset, 16 bits per float so every value is exact.
use contract::command::{Engagement, RoutePolicy, TargetRef};
use contract::observation::{
    ActionReason, ContactSource, EncounterResult, GarrisonPhase, MoveState, ObservationFrame,
    Posture, ServiceStatus, SoundBand, SoundCategory,
};
use contract::scenario::UnitKind;

pub const UNIT_KINDS: [UnitKind; 5] = [
    UnitKind::Rifle,
    UnitKind::Recon,
    UnitKind::At,
    UnitKind::Tank,
    UnitKind::Supply,
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

const HEADER: [&str; 15] = [
    "tick",
    "ownCount",
    "identifiedCount",
    "contactCount",
    "audibleCount",
    "knownPropCount",
    "projectileCount",
    "corpseCount",
    "guidedCount",
    "encounterHeldS",
    "encounterResult",
    "fogCellM",
    "fogNx",
    "fogNy",
    "fogFloats",
];
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
const IDENTIFIED_FIELDS: [&str; 10] = [
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
];

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

pub fn layout_json() -> String {
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
                    { "name": "sees", "count": "seesCount", "fields": ["id"] },
                    { "name": "mounts", "count": "mountCount", "fields": MOUNT_FIELDS },
                    { "name": "sightEyes", "count": "sightEyeCount", "fields": ["x", "y", "z"] },
                ],
            },
            {
                "name": "identified",
                "count": "identifiedCount",
                "fields": IDENTIFIED_FIELDS,
                "sections": [
                    { "name": "members", "count": "memberCount", "fields": ["x", "y", "z"] },
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
                "fields": ["x0", "y0", "z0", "x1", "y1", "z1", "own", "impact"],
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
                "fields": ["x", "y", "z", "own"],
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
    })
    .to_string()
}

/// Overwrites `out` with the packed frame.
pub fn pack(frame: &ObservationFrame, out: &mut Vec<f32>) {
    out.clear();
    let fog = &frame.ground_visibility;
    let cells = (fog.nx * fog.ny) as usize;
    let fog_floats = cells.div_ceil(FOG_BITS_PER_FLOAT);
    out.extend([
        frame.tick as f32,
        frame.own.len() as f32,
        frame.identified.len() as f32,
        frame.contacts.len() as f32,
        frame.audible.len() as f32,
        frame.known_props.len() as f32,
        frame.projectiles.len() as f32,
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
        ]);
    }
    for e in &frame.identified {
        out.extend(
            e.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
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
        out.extend([
            p.from[0] as f32,
            p.from[1] as f32,
            p.from[2] as f32,
            p.to[0] as f32,
            p.to[1] as f32,
            p.to[2] as f32,
            p.own as u8 as f32,
            p.impact as u8 as f32,
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
        out.extend([
            c.position[0] as f32,
            c.position[1] as f32,
            c.position[2] as f32,
            c.own as u8 as f32,
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
}
