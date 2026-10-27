//! Native shell for the shared bloom and tone-map pass resources.
use presentation::records::{Bloom, Grade};
use wgpu::*;

use crate::frame::{Inputs, Pass, Targets};
use crate::pipelines::Pipelines;

pub struct Post(renderer_core::post::PostResources);

impl Post {
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        bloom: &Bloom,
        grade: &Grade,
    ) -> Self {
        Self(renderer_core::post::new(
            device,
            pipelines,
            targets,
            bytemuck::bytes_of(bloom),
            bytemuck::bytes_of(grade),
        ))
    }
}

impl Pass for Post {
    fn name(&self) -> &'static str {
        "post"
    }
    fn encode(&self, _: &Inputs, encoder: &mut CommandEncoder) {
        self.0.encode(encoder);
    }
}
