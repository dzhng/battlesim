//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: header, then `ownCount` own-unit rows, then the variable sections
//! in own-unit order: each unit's `routeCount` route points, `queueCount`
//! queued destinations and `memberCount` member positions.
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
const HEADER_FIELDS: [&str; 2] = ["tick", "ownCount"];
const OWN_FIELDS: [&str; 13] = [
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
];
const MEMBER_COUNT: &str = "memberCount";

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
    let mut own: Vec<&str> = OWN_FIELDS.to_vec();
    own.push(MEMBER_COUNT);
    serde_json::json!({
        "header": HEADER_FIELDS,
        "own": { "stride": own.len(), "fields": own },
        "sections": [
            { "name": "route", "count": "routeCount", "fields": ["x", "y"] },
            { "name": "queue", "count": "queueCount", "fields": ["x", "y"] },
            { "name": "members", "count": MEMBER_COUNT, "fields": ["x", "y", "z"] },
        ],
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
    out.extend([frame.tick as f32, frame.own.len() as f32]);
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
    }
}
