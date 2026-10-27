//! The frame: one ordered list of passes, encoded in order each drawn frame
//! (Bevy 0.19+'s fixed pass order without a render graph; `battleFrame.ts`
//! is the browser's). Each presentation `Frame` first prepares the layers
//! (its camera and view, the sun's cascades it fitted, the cast lights, the
//! grass's window, the models' palettes and instances, the effects'
//! instances); then each pass reads the frame's `View`, the sized targets
//! and the layers, and encodes its work. The list is the whole frame.
use presentation::presentation::Presentation;
use presentation::records::View;
use wgpu::*;

use crate::effects::Effects;
use crate::environment::Environment;
use crate::fog::Fog;
use crate::grass::Grass;
use crate::ground::Ground;
use crate::models::Models;
use crate::models::Surface;
use crate::scenery::Scenery;
use crate::shadow::SunShadow;

struct NativeFrame<'a> {
    passes: &'a Passes,
    inputs: &'a Inputs<'a>,
}

fn native_pass_name(stage: renderer_core::frame::FrameStage) -> &'static str {
    match stage {
        renderer_core::frame::FrameStage::SunShadow => "sun-shadow",
        renderer_core::frame::FrameStage::Depth => "depth-prepass",
        renderer_core::frame::FrameStage::FogSight => "fog-sight",
        renderer_core::frame::FrameStage::World => "world",
        renderer_core::frame::FrameStage::Effects => "effects",
        renderer_core::frame::FrameStage::FogMask => "fog-mask",
        renderer_core::frame::FrameStage::Overlays => "ghosts",
        renderer_core::frame::FrameStage::Post => "post",
        renderer_core::frame::FrameStage::FogRim => "fog-rim",
    }
}

impl renderer_core::frame::FrameInputs for NativeFrame<'_> {
    fn encode_stage(
        &mut self,
        stage: renderer_core::frame::FrameStage,
        encoder: &mut CommandEncoder,
    ) {
        let name = native_pass_name(stage);
        let pass = self
            .passes
            .passes
            .iter()
            .find(|pass| pass.name() == name)
            .unwrap_or_else(|| panic!("native frame is missing shared stage {name}"));
        pass.encode(self.inputs, encoder);
    }
}

impl renderer_core::depth::DepthInputs for Inputs<'_> {
    fn depth_view(&self) -> &TextureView {
        &self.targets.depth
    }

    fn camera_group(&self) -> (&'static str, &BindGroup) {
        let [camera, ..] = self.environment.groups();
        camera
    }

    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>) {
        self.ground.draw_depth(pass, self.camera_group());
    }

    fn draw_scenery_depth(&self, pass: &mut RenderPass<'_>) {
        self.scenery
            .draw_depth(pass, self.camera_group(), self.models.statics());
    }

    fn draw_models_depth(
        &self,
        pass: &mut RenderPass<'_>,
        pipeline: &renderer_core::pipelines::Bound,
        cutout: bool,
    ) {
        pipeline.set(pass, &[self.camera_group(), self.models.group()]);
        self.models.draw(
            pass,
            if cutout {
                Surface::Cutout
            } else {
                Surface::Solid
            },
        );
    }
}

pub use renderer_core::{
    TargetOwner, Targets, DEPTH_FORMAT, DISPLAY_FORMAT, FOG_DISTANCE_FORMAT, FOG_MASK_FORMAT,
    HDR_FORMAT, MSAA,
};

/// What a pass draws from.
pub struct Inputs<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub view: &'a View,
    pub targets: &'a Targets,
    pub environment: &'a Environment,
    pub shadow: &'a SunShadow,
    pub ground: &'a Ground,
    pub grass: &'a Grass,
    pub models: &'a Models,
    pub scenery: &'a Scenery,
    pub effects: &'a Effects,
    pub fog: &'a Fog,
}

pub trait Pass {
    fn name(&self) -> &'static str;
    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder);
}

/// The layers and the passes in the order they draw.
pub struct Passes {
    pub targets: TargetOwner,
    pub environment: Environment,
    pub shadow: SunShadow,
    pub ground: Ground,
    pub grass: Grass,
    pub models: Models,
    pub scenery: Scenery,
    pub effects: Effects,
    pub fog: Fog,
    pub passes: Vec<Box<dyn Pass>>,
}

impl Passes {
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
        let inputs = Inputs {
            device,
            queue,
            view,
            targets: &self.targets,
            environment: &self.environment,
            shadow: &self.shadow,
            ground: &self.ground,
            grass: &self.grass,
            models: &self.models,
            scenery: &self.scenery,
            effects: &self.effects,
            fog: &self.fog,
        };
        let mut frame = NativeFrame {
            passes: self,
            inputs: &inputs,
        };
        for pass in &self.passes {
            assert!(
                renderer_core::frame::FrameStage::ORDER
                    .iter()
                    .any(|stage| native_pass_name(*stage) == pass.name()),
                "native pass is missing from the shared frame order: {}",
                pass.name()
            );
        }
        renderer_core::frame::encode_frame(&mut encoder, &mut frame);
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
