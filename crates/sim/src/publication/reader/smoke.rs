//! A smoke cloud's row, decoded by the writer's field names, and the cloud
//! it is under `contract::smoke`'s law.
use super::{at, Reader, Result, Row, SmokeCloud, SmokeCloudSpec};
use crate::publication::SMOKE_CLOUD_FIELDS;
use contract::smoke;

impl SmokeCloud {
    /// The popped cloud the row was written from, for `contract::smoke::state`.
    pub fn cloud(&self) -> smoke::SmokeCloud {
        let s = &self.spec;
        smoke::SmokeCloud {
            id: self.id as u32,
            centre: self.centre,
            popped: self.popped as contract::ids::Tick,
            spec: smoke::SmokeCloudSpec {
                radius_m: s.radius_m,
                height_m: s.height_m,
                life_s: s.life_s,
                bloom_s: s.bloom_s,
                fade_s: s.fade_s,
                bloom_steps: s.bloom_steps as u8,
                fade_steps: s.fade_steps as u8,
                depth_per_m: s.depth_per_m,
                concealment: s.concealment,
                look: s.look.to_string(),
            },
        }
    }
}

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
