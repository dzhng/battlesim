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
use crate::pipelines::Sizes;
use crate::scenery::Scenery;
use crate::shadow::SunShadow;

/// The lit world before post.
pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// What post writes: the browser's canvas format.
pub const DISPLAY_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
/// 4× is the count WebGPU guarantees for every renderable format.
pub const MSAA: u32 = 4;
/// Reverse-Z: cleared to 0 (the far plane), nearer is greater.
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
/// The fog mask the lit passes write beside their colour (`WORLD_OUT`).
pub const FOG_MASK_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
/// The water's guard (`Targets::fog_mask_under`).
pub const GUARD_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
/// The fog mask passes' distances to each side of the edge.
pub const FOG_DISTANCE_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;

/// The frame's size-dependent targets.
pub struct Targets {
    pub size: [u32; 2],
    /// The world, multisampled: resolved into `lit`, then the fogged world
    /// (and the ghosts over it) resolved into `hdr`.
    pub hdr_msaa: TextureView,
    /// The lit world, the effects drawn over it.
    pub lit: TextureView,
    pub lit_texture: Texture,
    /// `lit` as the world pass left it, before the water, with its mips:
    /// what the water shows of the bed through it (`world::Water`). Native
    /// draws the water `full` (`WaterDetail`), refracted.
    pub scene_colour: TextureView,
    pub scene_colour_texture: Texture,
    /// The world with fog applied: post's input.
    pub hdr: TextureView,
    /// The world's depth, multisampled; the effects read it.
    pub depth: TextureView,
    /// The fog mask, multisampled, resolved into `fog_mask`.
    pub fog_mask_msaa: TextureView,
    pub fog_mask: TextureView,
    /// The water's guard: how unseen the ground is before the water
    /// (`fog_mask`'s unseen share) and how much the paint covers it, each
    /// mip the worst of its footprint in `scene_colour`'s mips.
    pub fog_mask_under: TextureView,
    pub fog_mask_under_texture: Texture,
    /// The distances to the other side of the fog edge: along rows, then
    /// the Euclidean result.
    pub fog_distance_rows: TextureView,
    pub fog_distance: TextureView,
    /// Post's output: presented, and read back for a plate.
    pub display: TextureView,
    pub display_texture: Texture,
}

impl Targets {
    pub fn new(device: &Device, size: [u32; 2], sizes: &Sizes) -> Self {
        let mipped = |label, format, samples, usage, mips| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: mips,
                sample_count: samples,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let texture = |label, format, samples, usage| mipped(label, format, samples, usage, 1);
        let view = |t: Texture| t.create_view(&Default::default());
        let render = TextureUsages::RENDER_ATTACHMENT;
        let sampled = render | TextureUsages::TEXTURE_BINDING;
        let display_texture = texture(
            "display",
            DISPLAY_FORMAT,
            1,
            render | TextureUsages::COPY_SRC,
        );
        let lit_texture = texture("lit", HDR_FORMAT, 1, sampled | TextureUsages::COPY_SRC);
        let scene_colour_texture = mipped(
            "scene-colour",
            HDR_FORMAT,
            1,
            sampled | TextureUsages::COPY_DST,
            sizes.scene_colour_mips,
        );

        let fog_mask_under_texture = mipped(
            "fog-mask-under",
            GUARD_FORMAT,
            1,
            sampled,
            sizes.scene_colour_mips,
        );
        Self {
            size,
            hdr_msaa: view(texture("hdr-msaa", HDR_FORMAT, MSAA, render)),
            lit: lit_texture.create_view(&Default::default()),
            lit_texture,
            scene_colour: scene_colour_texture.create_view(&Default::default()),
            scene_colour_texture,
            hdr: view(texture("hdr", HDR_FORMAT, 1, sampled)),
            depth: view(texture("depth", DEPTH_FORMAT, MSAA, sampled)),
            fog_mask_msaa: view(texture("fog-mask-msaa", FOG_MASK_FORMAT, MSAA, render)),
            fog_mask: view(texture("fog-mask", FOG_MASK_FORMAT, 1, sampled)),
            fog_mask_under: fog_mask_under_texture.create_view(&Default::default()),
            fog_mask_under_texture,
            fog_distance_rows: view(texture(
                "fog-distance-rows",
                FOG_DISTANCE_FORMAT,
                1,
                sampled,
            )),
            fog_distance: view(texture("fog-distance", FOG_DISTANCE_FORMAT, 1, sampled)),
            display: display_texture.create_view(&Default::default()),
            display_texture,
        }
    }
}

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
    pub targets: Targets,
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
        for pass in &self.passes {
            encoder.push_debug_group(pass.name());
            pass.encode(&inputs, &mut encoder);
            encoder.pop_debug_group();
        }
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
