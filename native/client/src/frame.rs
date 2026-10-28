//! Native scene preparation and display readback around the shared GPU frame.
//! Presentation supplies canonical records; the shared `FrameRenderer` owns
//! every render-pass decision and the completed display texture.
use presentation::presentation::Presentation;
use presentation::records::View;
use wgpu::*;

use crate::effects::Effects;
use crate::environment::Environment;
use crate::fog::Fog;
use crate::grass::Grass;
use crate::ground::Ground;
use crate::models::Models;
use crate::scenery::Scenery;
use crate::shadow::SunShadow;

pub use renderer_core::{
    TargetOwner, Targets, DEPTH_FORMAT, DISPLAY_FORMAT, FOG_DISTANCE_FORMAT, FOG_MASK_FORMAT,
    HDR_FORMAT, MSAA,
};

/// Native retention and presentation adapters for the shared scene owners.
pub struct Scene {
    pub targets: TargetOwner,
    pub environment: Environment,
    pub shadow: SunShadow,
    pub ground: Ground,
    pub grass: Grass,
    pub models: Models,
    pub scenery: Scenery,
    pub effects: Effects,
    pub fog: Fog,
    pub sky: renderer_core::sky::Sky,
    pub frame: renderer_core::frame::FrameRenderer,
    pub post: renderer_core::post::PostResources,
    pub fog_mask: renderer_core::fog_mask::FogMaskResources,
}

impl Scene {
    /// Prepares the layers for `presentation`'s last frame.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &Presentation,
    ) -> Result<(), String> {
        let frame = presentation.last_frame();
        self.environment.prepare(queue, &frame.view, &frame.camera);
        self.ground.prepare(&frame.camera, frame.shadow.as_ref());
        if let Some(fit) = &frame.shadow {
            self.shadow.prepare(queue, fit);
        }
        self.grass.prepare(queue, presentation);
        self.models.prepare(device, queue, frame)?;
        self.scenery.prepare(self.models.statics());
        Ok(())
    }

    /// Takes this frame's scars from `presentation`: the grass regrows over
    /// them when they changed (the next frame's growth reads them).
    pub fn prepare_scars(&mut self, device: &Device, presentation: &mut Presentation) {
        let size = self.targets.size;
        self.ground
            .prepare_scars(device, presentation, self.grass.scar_bounds(), size);
    }

    /// Takes this frame's fog of war from `presentation`, for its last
    /// frame's view.
    pub fn prepare_fog(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &mut Presentation,
    ) -> Result<(), String> {
        let camera = presentation.last_frame().camera;
        self.fog.prepare(device, queue, presentation, &camera)
    }

    /// Prepares the effects and the cast lights `presentation` made for
    /// its last frame, whose cast lights reach `light_wrap` round a surface.
    pub fn prepare_effects(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &Presentation,
        light_wrap: f32,
    ) {
        self.environment
            .set_cast_lights(queue, presentation.lights(), light_wrap);
        self.effects.prepare(device, queue, presentation.effects());
    }

    /// Encodes every pass for `view` into the display target.
    pub fn encode(&self, device: &Device, queue: &Queue, view: &View) -> CommandEncoder {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("battle-frame"),
        });
        let inputs = renderer_core::frame::FrameInputs {
            device,
            queue,
            view,
            targets: &self.targets,
            sky: &self.sky,
            environment: &self.environment,
            shadow: self.shadow.resources(),
            ground: &self.ground,
            grass: Some(&self.grass),
            models: &self.models,
            scenery: &self.scenery,
            effects: self.effects.resources(),
            fog: &self.fog,
            fog_mask: &self.fog_mask,
            post: &self.post,
            overlays: None,
        };
        self.frame.encode(&inputs, &mut encoder);
        encoder
    }

    /// The display target's pixels after `encoder` runs, as RGBA rows.
    pub fn read_display(
        &self,
        device: &Device,
        queue: &Queue,
        mut encoder: CommandEncoder,
    ) -> Vec<u8> {
        let [w, h] = self.targets.size;
        let row = (w * 4).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("plate"),
            size: u64::from(row * h),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            self.targets.display_texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.targets.display_texture.size(),
        );
        queue.submit([encoder.finish()]);
        buffer
            .slice(..)
            .map_async(MapMode::Read, |r| r.expect("plate readback"));
        device
            .poll(PollType::wait_indefinitely())
            .expect("plate readback");
        let mapped = buffer.slice(..).get_mapped_range().expect("mapped");
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let start = (y * row) as usize;
            for bgra in mapped[start..start + (w * 4) as usize].as_chunks::<4>().0 {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], 255]);
            }
        }
        rgba
    }
}
