//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
use contract::observation::ObservationFrame;
use contract::scenario::UnitKind;

pub const UNIT_KINDS: [UnitKind; 5] = [
    UnitKind::Rifle,
    UnitKind::Recon,
    UnitKind::At,
    UnitKind::Tank,
    UnitKind::Supply,
];
const HEADER_FIELDS: [&str; 2] = ["tick", "ownCount"];
const OWN_FIELDS: [&str; 9] = [
    "id", "kind", "x", "y", "z", "yaw", "goalX", "goalY", "queued",
];

pub fn layout_json() -> String {
    serde_json::json!({
        "header": HEADER_FIELDS,
        "own": { "stride": OWN_FIELDS.len(), "fields": OWN_FIELDS },
        "unitKinds": UNIT_KINDS.iter().map(|k| format!("{k:?}").to_lowercase()).collect::<Vec<_>>(),
        // goalX/goalY are NaN when the unit has no movement order.
    })
    .to_string()
}

/// Overwrites `out` with the packed frame.
pub fn pack(frame: &ObservationFrame, out: &mut Vec<f32>) {
    out.clear();
    out.extend([frame.tick as f32, frame.own.len() as f32]);
    for u in &frame.own {
        let kind = UNIT_KINDS.iter().position(|k| *k == u.kind).unwrap() as f32;
        let [gx, gy] = u.goal.map_or([f32::NAN; 2], |g| [g[0] as f32, g[1] as f32]);
        out.extend([
            u.id.0 as f32,
            kind,
            u.position[0] as f32,
            u.position[1] as f32,
            u.position[2] as f32,
            u.yaw as f32,
            gx,
            gy,
            u.queued as f32,
        ]);
    }
}
