//! Smoke clouds in the publication: one row for each cloud a side sees, and
//! the look names those rows index (the layout's `smokeLooks`). A row says
//! where and when its cloud was popped and what it is, never who popped it.
use contract::scenario::Rules;
use contract::smoke::SmokeCloud;

pub(super) const SMOKE_CLOUD_FIELDS: [&str; 15] = [
    "id",
    "x",
    "y",
    "z",
    "popped",
    "radiusM",
    "heightM",
    "lifeS",
    "bloomS",
    "fadeS",
    "bloomSteps",
    "fadeSteps",
    "depthPerM",
    "concealment",
    "look",
];

/// Every look the rules' smoke capabilities name, in name order: what a
/// cloud row's `look` indexes.
pub fn smoke_looks(rules: &Rules) -> Vec<&str> {
    let catalog = &rules.catalog;
    let mut looks: Vec<&str> = catalog
        .indices()
        .filter_map(|t| catalog.get(t).capabilities.smoke.as_ref())
        .map(|s| s.cloud.look.as_str())
        .collect();
    looks.sort_unstable();
    looks.dedup();
    looks
}

/// `cloud`'s row, its look indexed in `looks` ([`smoke_looks`]).
pub(super) fn smoke_cloud_row(cloud: &SmokeCloud, looks: &[&str]) -> Result<[f32; 15], String> {
    let s = &cloud.spec;
    let look = looks.iter().position(|l| *l == s.look).ok_or_else(|| {
        format!(
            "smoke look {} is not one of the layout's smokeLooks",
            s.look
        )
    })?;
    Ok([
        cloud.id as f32,
        cloud.centre[0] as f32,
        cloud.centre[1] as f32,
        cloud.centre[2] as f32,
        cloud.popped as f32,
        s.radius_m as f32,
        s.height_m as f32,
        s.life_s as f32,
        s.bloom_s as f32,
        s.fade_s as f32,
        f32::from(s.bloom_steps),
        f32::from(s.fade_steps),
        s.depth_per_m as f32,
        s.concealment as f32,
        look as f32,
    ])
}
