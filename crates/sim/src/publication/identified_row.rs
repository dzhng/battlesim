//! An identified enemy's fixed publication row: its observed identity, pose,
//! visible setup posture, landing latch and building limbs.
use contract::observation::IdentifiedUnit;

use super::cells::limbs_or_absent;

pub(super) const IDENTIFIED_FIELDS: [&str; 18] = [
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
    "smoking",
    "deployProgress",
    "deployTarget",
    "grounded",
    "buildingLo",
    "buildingHi",
];

pub(super) fn identified_row(e: &IdentifiedUnit) -> [f32; IDENTIFIED_FIELDS.len()] {
    let [building_lo, building_hi] = limbs_or_absent(e.building);
    let [deploy_progress, deploy_target] = super::deployment::cells(e.deployment);
    [
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
        e.smoking as u8 as f32,
        deploy_progress,
        deploy_target,
        e.grounded as u8 as f32,
        building_lo,
        building_hi,
    ]
}
