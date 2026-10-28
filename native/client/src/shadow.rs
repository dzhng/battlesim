//! The sun's cascades (`world/shadow.ts`): a depth array, one layer per
//! cascade, each cleared and drawn from its caster camera, and the receiver
//! block every lit material samples them by. The presentation crate fits
//! both to the frame's camera; this only uploads and draws.
use wgpu::*;

use crate::frame::{Inputs, Pass};
use crate::models::Surface;
use crate::pipelines::{Bound, Pipelines};
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

    pub fn depth(&self) -> &TextureView {
        &self.resources.depth
    }
    pub fn receiver(&self) -> &Buffer {
        &self.resources.receiver
    }
    pub fn compare(&self) -> &Sampler {
        &self.resources.compare
    }
}

/// One clear-to-far depth pass per fitted cascade, drawing every caster:
/// the ground's, the models' and the scenery's (`encodeShadows`).
pub struct ShadowPass {
    solid: Bound,
    cutout: Bound,
}

impl ShadowPass {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            solid: pipelines.bound("modelCaster"),
            cutout: pipelines.bound("modelCasterCutout"),
        }
    }
}

impl Pass for ShadowPass {
    fn name(&self) -> &'static str {
        "sun-shadow"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let shadow = inputs.shadow;
        shadow
            .resources
            .encode_cascades(encoder, |cascade, camera, pass| {
                let camera = ("typegpuCameraLayout", camera);
                inputs.ground.draw_casters(pass, camera, cascade);
                let models = inputs.models;
                for (bound, surface) in [
                    (&self.solid, Surface::Solid),
                    (&self.cutout, Surface::Cutout),
                ] {
                    bound.set(pass, &[camera, models.group()]);
                    models.draw_casters(pass, surface, cascade);
                }
                inputs
                    .scenery
                    .draw_casters(pass, camera, models.statics(), cascade);
            });
    }
}
