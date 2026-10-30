//! Published mount poses, decoded by the writer's field names.
use super::{at, each, limbs, WeaponPose};
use crate::publication::POSE_FIELDS;

pub(super) fn weapon_poses(flat: &[f32]) -> Vec<WeaponPose> {
    const FIELDS: &[&str] = &POSE_FIELDS;
    each::<{ POSE_FIELDS.len() }>(flat)
        .map(|p| WeaponPose {
            mount: v!(p, "mount"),
            bearing: v!(p, "bearing"),
            elevation: v!(p, "elevation"),
            round: v!(p, "round"),
            shots: limbs(v!(p, "shotsLo"), v!(p, "shotsHi")),
            operator: limbs(v!(p, "operatorLo"), v!(p, "operatorHi")),
        })
        .collect()
}
