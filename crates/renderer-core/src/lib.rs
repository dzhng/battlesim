//! GPU resources shared by the native and browser renderer shells.
//!
//! This crate deliberately stops at the device-facing boundary. It owns the
//! target formats, resize allocation and small resource helpers used by every
//! pass and consumes the shared presentation records. Surface creation,
//! adapter policy, runtime files and audio stay in the platform shells. It has
//! no filesystem or windowing dependencies and therefore checks for
//! `wasm32-unknown-unknown`.

use std::ops::{Deref, DerefMut};
use wgpu::*;

pub mod bundle;
pub mod depth;
pub mod effects;
pub mod environment;
pub mod fog;
pub mod fog_mask;
pub mod fog_sight;
pub mod frame;
pub mod grass;
pub mod ground;
pub mod high_air;
pub mod impostor;
pub mod memory;
pub mod model_material;
pub mod models;
pub mod overlay;
pub mod paint;
pub mod pipelines;
pub mod post;
pub mod probes;
pub mod readback;
pub mod runtime;
pub mod scars;
pub mod scenery;
pub mod shadow;
pub mod sky;
pub mod statics;
pub mod terrain;
pub mod timing;
pub mod world;
pub mod xray;

/// Encode an ordered pass list with one shared debug-group boundary.
///
/// The closure carries the platform shell's concrete pass/input types; this
/// function owns the ordering instrumentation without importing those types.
pub fn encode_passes<'a, I, F>(encoder: &mut CommandEncoder, names: I, mut encode: F)
where
    I: IntoIterator<Item = &'a str>,
    F: FnMut(usize, &mut CommandEncoder),
{
    for (index, name) in names.into_iter().enumerate() {
        encoder.push_debug_group(name);
        encode(index, encoder);
        encoder.pop_debug_group();
    }
}

/// Encode one pass inside the shared debug-group boundary.
pub fn encode_grouped(
    encoder: &mut CommandEncoder,
    name: &'static str,
    encode: impl FnOnce(&mut CommandEncoder),
) {
    encoder.push_debug_group(name);
    encode(encoder);
    encoder.pop_debug_group();
}

/// The lit world before post.
pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// What post writes: the browser's canvas format.
pub const DISPLAY_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
/// The sample count shared by the renderable targets.
pub const MSAA: u32 = 4;
/// Reverse-Z: cleared to 0 (the far plane), nearer is greater.
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
/// The fog mask the lit passes write beside their colour.
pub const FOG_MASK_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
/// The fog mask passes' distances to the other side of the edge.
pub const FOG_DISTANCE_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
/// Display-space overlay colour, including ground paint and glow.
pub const OVERLAY_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// The frame's size-dependent targets.
///
/// Both platform shells construct this exact set. Passes may add a surface
/// view at the shell boundary, but they do not choose formats or silently
/// allocate a second target set.
pub struct Targets {
    pub size: [u32; 2],
    /// The world, multisampled: resolved into `lit`, then the fogged world.
    pub hdr_msaa: TextureView,
    /// The lit world, with effects drawn over it.
    pub lit: TextureView,
    /// The world with fog applied: post's input.
    pub hdr: TextureView,
    /// The world's depth, multisampled; effects read it.
    pub depth: TextureView,
    /// The fog mask, multisampled and resolved into `fog_mask`.
    pub fog_mask_msaa: TextureView,
    pub fog_mask: TextureView,
    /// Distances to the other side of the fog edge.
    pub fog_distance_rows: TextureView,
    pub fog_distance: TextureView,
    /// The prepass depth copied before overlay geometry is drawn.
    pub overlay_depth: TextureView,
    /// Display-space overlay geometry and its resolved colour.
    pub overlay_msaa: TextureView,
    pub overlay: TextureView,
    /// Ground paint sampled by the painted world layers.
    pub paint: TextureView,
    /// Half-resolution overlay halo targets.
    pub overlay_glow_rows: TextureView,
    pub overlay_glow: TextureView,
    /// Post's output, presented and optionally read back by a shell.
    pub display: TextureView,
    pub display_texture: Texture,
}

impl Targets {
    /// Allocate the shared target set for a render size.
    pub fn new(device: &Device, size: [u32; 2]) -> Self {
        let texture = |label, format, samples, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let view = |t: Texture| t.create_view(&Default::default());
        let render = TextureUsages::RENDER_ATTACHMENT;
        let sampled = render | TextureUsages::TEXTURE_BINDING;
        let copy = TextureUsages::COPY_SRC | TextureUsages::COPY_DST;
        let half = [size[0].div_ceil(2), size[1].div_ceil(2)];
        let small_texture = |label, format, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: half[0],
                    height: half[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let display_texture = texture(
            "display",
            DISPLAY_FORMAT,
            1,
            render | TextureUsages::COPY_SRC | TextureUsages::TEXTURE_BINDING,
        );
        Self {
            size,
            hdr_msaa: view(texture("hdr-msaa", HDR_FORMAT, MSAA, render)),
            lit: view(texture("lit", HDR_FORMAT, 1, sampled)),
            hdr: view(texture("hdr", HDR_FORMAT, 1, sampled)),
            depth: view(texture(
                "depth",
                DEPTH_FORMAT,
                MSAA,
                sampled | TextureUsages::COPY_SRC,
            )),
            fog_mask_msaa: view(texture("fog-mask-msaa", FOG_MASK_FORMAT, MSAA, render)),
            fog_mask: view(texture("fog-mask", FOG_MASK_FORMAT, 1, sampled)),
            fog_distance_rows: view(texture(
                "fog-distance-rows",
                FOG_DISTANCE_FORMAT,
                1,
                sampled,
            )),
            fog_distance: view(texture("fog-distance", FOG_DISTANCE_FORMAT, 1, sampled)),
            overlay_depth: view(texture("overlay-depth", DEPTH_FORMAT, MSAA, sampled | copy)),
            overlay_msaa: view(texture("overlay-msaa", OVERLAY_FORMAT, MSAA, render)),
            overlay: view(texture("overlay", OVERLAY_FORMAT, 1, sampled)),
            paint: view(texture("ground-paint", OVERLAY_FORMAT, 1, sampled)),
            overlay_glow_rows: view(small_texture(
                "overlay-glow-rows",
                OVERLAY_FORMAT,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            )),
            overlay_glow: view(small_texture(
                "overlay-glow",
                OVERLAY_FORMAT,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            )),
            display: display_texture.create_view(&Default::default()),
            display_texture,
        }
    }
}

/// Owns the size-dependent target set and its rebuild generation.
///
/// A shell keeps this owner alongside its surface. Rebuilding swaps the whole
/// set before the next frame, so old views cannot leak into a resized pass;
/// the generation gives static/resource adapters one shared invalidation
/// signal without importing a window or presentation type into this crate.
pub struct TargetOwner {
    targets: Targets,
    generation: u64,
}

impl TargetOwner {
    pub fn new(device: &Device, size: [u32; 2]) -> Self {
        Self {
            targets: Targets::new(device, size),
            generation: 0,
        }
    }

    /// Rebuild the complete target set when the pixel size changes. Returns
    /// whether a swap occurred; callers should invalidate size-bound groups
    /// only on `true`.
    pub fn rebuild(&mut self, device: &Device, size: [u32; 2]) -> bool {
        if self.targets.size == size {
            return false;
        }
        self.targets = Targets::new(device, size);
        self.generation = self.generation.wrapping_add(1);
        true
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn as_targets(&self) -> &Targets {
        &self.targets
    }
}

impl Deref for TargetOwner {
    type Target = Targets;
    fn deref(&self) -> &Self::Target {
        &self.targets
    }
}

impl DerefMut for TargetOwner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.targets
    }
}

/// A buffer holding `bytes` from the start.
pub fn buffer(device: &Device, label: &str, usage: BufferUsages, bytes: &[u8]) -> Buffer {
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage,
        mapped_at_creation: true,
    });
    buffer
        .get_mapped_range_mut(..)
        .expect("mapped at creation")
        .copy_from_slice(bytes);
    buffer.unmap();
    buffer
}

/// A bind group whose resources occupy bindings 0, 1, … in order.
pub fn bind(
    device: &Device,
    layout: &BindGroupLayout,
    label: &str,
    resources: &[BindingResource],
) -> BindGroup {
    let entries: Vec<BindGroupEntry> = resources
        .iter()
        .enumerate()
        .map(|(i, resource)| BindGroupEntry {
            binding: i as u32,
            resource: resource.clone(),
        })
        .collect();
    device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &entries,
    })
}

impl Targets {
    pub fn record_memory(&self, memory: &mut memory::ResourceMemory) {
        for view in [
            &self.hdr_msaa,
            &self.lit,
            &self.hdr,
            &self.depth,
            &self.fog_mask_msaa,
            &self.fog_mask,
            &self.fog_distance_rows,
            &self.fog_distance,
            &self.overlay_depth,
            &self.overlay_msaa,
            &self.overlay,
            &self.paint,
            &self.overlay_glow_rows,
            &self.overlay_glow,
            &self.display,
        ] {
            memory.view(view);
        }
        memory.texture(&self.display_texture);
    }
}
