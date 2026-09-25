//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: the header, then each group in layout order. A group is
//! `countField` rows of `fields`, followed by each row's variable sections in
//! row order (section by section, each `count` points of `fields`). Last comes
//! the ground-visibility bitset, 16 bits per float so every value is exact.
use contract::command::RoutePolicy;
use contract::observation::{MoveState, ObservationFrame};
use contract::scenario::UnitKind;

pub const UNIT_KINDS: [UnitKind; 5] = [
    UnitKind::Rifle,
    UnitKind::Recon,
    UnitKind::At,
    UnitKind::Tank,
    UnitKind::Supply,
];
const MOVE_STATES: [MoveState; 4] = [
    MoveState::Idle,
    MoveState::Moving,
    MoveState::Waiting,
    MoveState::RouteBlocked,
];
const POLICIES: [RoutePolicy; 2] = [RoutePolicy::Shortest, RoutePolicy::Fastest];
const FOG_BITS_PER_FLOAT: usize = 16;

const HEADER: [&str; 7] = [
    "tick",
    "ownCount",
    "identifiedCount",
    "fogCellM",
    "fogNx",
    "fogNy",
    "fogFloats",
];
const OWN_FIELDS: [&str; 15] = [
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
                    { "name": "sees", "count": "seesCount", "fields": ["id"] },
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
        ],
        "fog": { "bitsPerFloat": FOG_BITS_PER_FLOAT, "count": "fogFloats" },
        "unitKinds": names(&UNIT_KINDS),
        "moveStates": names(&MOVE_STATES),
        "policies": names(&POLICIES),
        // goalX/goalY are NaN without a movement order; policy and blocker are -1 when absent.
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
        out.extend(u.sees.iter().map(|id| id.0 as f32));
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
