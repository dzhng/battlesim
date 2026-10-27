//! GPU resources shared by the native and browser renderer shells.
//!
//! This crate deliberately stops at the device-facing boundary. It owns the
//! target formats, resize allocation and small resource helpers used by every
//! pass, while surface creation, adapter policy, runtime files and audio stay
//! in the platform shells. It has no filesystem, windowing or presentation
//! feature dependencies and therefore checks for `wasm32-unknown-unknown`.

use wgpu::*;

pub mod depth;
pub mod fog_mask;
pub mod model;
pub mod pipelines;
pub mod shadow;
pub mod world;

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
        let display_texture = texture(
            "display",
            DISPLAY_FORMAT,
            1,
            render | TextureUsages::COPY_SRC,
        );
        Self {
            size,
            hdr_msaa: view(texture("hdr-msaa", HDR_FORMAT, MSAA, render)),
            lit: view(texture("lit", HDR_FORMAT, 1, sampled)),
            hdr: view(texture("hdr", HDR_FORMAT, 1, sampled)),
            depth: view(texture("depth", DEPTH_FORMAT, MSAA, sampled)),
            fog_mask_msaa: view(texture("fog-mask-msaa", FOG_MASK_FORMAT, MSAA, render)),
            fog_mask: view(texture("fog-mask", FOG_MASK_FORMAT, 1, sampled)),
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
