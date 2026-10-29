use super::*;
#[path = "effects.rs"]
mod effects;
#[path = "fog.rs"]
mod fog;
#[path = "models.rs"]
mod models;
#[path = "post.rs"]
mod post;
#[path = "water.rs"]
mod water;
#[path = "world.rs"]
mod world;
pub fn all() -> Vec<PipelineSpec> {
    let mut out = Vec::new();
    out.extend(post::all());
    out.extend(world::all());
    out.extend(water::all());
    out.extend(models::all());
    out.extend(fog::all());
    out.extend(effects::all());
    out
}
