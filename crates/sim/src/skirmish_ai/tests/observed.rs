//! Test observations used by the skirmish AI's target-selection cases.
use contract::catalog::TypeIndex;
use contract::observation::{IdentifiedUnit, ObservedTargetId};

pub(super) fn identified(id: u32, kind: TypeIndex, position: [f64; 3]) -> IdentifiedUnit {
    IdentifiedUnit {
        id: ObservedTargetId(id),
        kind,
        cost: 100,
        position,
        yaw: 0.0,
        velocity: [0.0, 0.0],
        members: vec![],
        member_ids: vec![],
        member_slots: vec![],
        member_active_mounts: vec![],
        weapon_poses: vec![],
        reversing: false,
        deployment: None,
        grounded: false,
        smoking: false,
        member_leans: vec![],
        building: None,
    }
}
