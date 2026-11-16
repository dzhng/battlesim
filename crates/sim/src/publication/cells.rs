//! Cells more than one row packs the same way: a target (`targetKind`
//! indexes targetKinds; an id for a unit or contact, a point for ground) and
//! a host (`hostKind` indexes hostKinds; its id in two limbs).
use contract::command::TargetRef;

use super::limbs_or_absent;
use super::tags::{tag, HOST_KINDS};

/// A target as `targetKind`, `targetId`, `targetX`, `targetY`, `targetZ`:
/// kind 0 and id -1 for none.
pub(super) fn target_cells(target: Option<TargetRef>) -> [f32; 5] {
    let (kind, id, p) = match target {
        None => (0.0, -1.0, [0.0; 3]),
        Some(TargetRef::Identified { id }) => (1.0, id.0 as f32, [0.0; 3]),
        Some(TargetRef::Contact { id }) => (2.0, id.0 as f32, [0.0; 3]),
        Some(TargetRef::Ground { point }) => (3.0, -1.0, point.map(|v| v as f32)),
    };
    [kind, id, p[0], p[1], p[2]]
}

/// A host as `hostKind`, `hostLo`, `hostHi`: all -1 for none.
pub(super) fn host_cells(host: Option<(&str, u32)>) -> [f32; 3] {
    let kind = host.map_or(-1.0, |(kind, _)| tag(&HOST_KINDS, &kind));
    let [lo, hi] = limbs_or_absent(host.map(|(_, id)| id));
    [kind, lo, hi]
}
