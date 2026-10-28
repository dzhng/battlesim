//! The sun's cascades (`world/shadow.ts`): a depth array, one layer per
//! cascade, each cleared and drawn from its caster camera, and the receiver
//! block every lit material samples them by. The presentation crate fits
//! both to the frame's camera; this only uploads and draws.
use wgpu::*;

use crate::pipelines::Pipelines;
use presentation::records::{Camera, SunShadow as Receiver};
use presentation::shadow::SunShadowFrame;
use renderer_core::shadow::ShadowResources;

pub struct SunShadow {
    resources: ShadowResources,
}

impl SunShadow {
    /// The cascades at `map_size` texels a side, as `cold` fits them.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        map_size: u32,
        cold: &SunShadowFrame,
    ) -> Self {
        Self {
            resources: ShadowResources::new(
                device,
                pipelines,
                map_size,
                bytemuck::bytes_of(&cold.receiver),
                std::mem::size_of::<Camera>(),
            ),
        }
    }

    /// Uploads the frame's fit.
    pub fn prepare(&mut self, queue: &Queue, fit: &SunShadowFrame) {
        let cameras: Vec<&[u8]> = fit.casters.iter().map(bytemuck::bytes_of).collect();
        self.resources.prepare(
            queue,
            bytemuck::bytes_of::<Receiver>(&fit.receiver),
            &cameras,
        );
    }

    pub fn resources(&self) -> &ShadowResources {
        &self.resources
    }
}
