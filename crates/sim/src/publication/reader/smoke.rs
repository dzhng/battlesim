//! A smoke cloud's row, decoded by the writer's field names.
use super::{at, Reader, Result, Row, SmokeCloud, SmokeCloudSpec};
use crate::publication::SMOKE_CLOUD_FIELDS;

impl Reader {
    pub(super) fn smoke_cloud(&self, r: &Row) -> Result<SmokeCloud> {
        const FIELDS: &[&str] = &SMOKE_CLOUD_FIELDS;
        let f = r.fields;
        Ok(SmokeCloud {
            id: v!(f, "id"),
            centre: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            popped: v!(f, "popped"),
            spec: SmokeCloudSpec {
                radius_m: v!(f, "radiusM"),
                height_m: v!(f, "heightM"),
                life_s: v!(f, "lifeS"),
                bloom_s: v!(f, "bloomS"),
                fade_s: v!(f, "fadeS"),
                bloom_steps: v!(f, "bloomSteps"),
                fade_steps: v!(f, "fadeSteps"),
                depth_per_m: v!(f, "depthPerM"),
                concealment: v!(f, "concealment"),
                look: self.name("smokeLooks", v!(f, "look"))?,
            },
        })
    }
}
