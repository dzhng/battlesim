//! An own mount's readiness row: its load, ammunition per kind (-1
//! unlimited, -2 no such kind), aim and reload, its target (`targetKind`
//! indexes targetKinds; an id for a unit or contact, a point for ground) and
//! why it holds fire.
use contract::observation::MountReadiness;

use super::cells::target_cells;
use super::tags::{tag, REASONS};

pub(super) const MOUNT_FIELDS: [&str; 15] = [
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

pub(super) fn mount_row(m: &MountReadiness) -> [f32; 15] {
    let ammo = |k: usize| m.ammo.get(k).map_or(-2.0, |a| a.map_or(-1.0, |n| n as f32));
    let [kind, id, x, y, z] = target_cells(m.target);
    [
        m.mount as f32,
        m.loaded.map_or(-1.0, |k| k as f32),
        ammo(0),
        ammo(1),
        m.aim as f32,
        m.reload as f32,
        kind,
        id,
        x,
        y,
        z,
        tag(&REASONS, &m.reason),
        m.ammo.len() as f32,
        m.guiding as u8 as f32,
        m.reloading.map_or(-1.0, |k| k as f32),
    ]
}
