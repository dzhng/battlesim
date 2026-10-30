//! Shared GPU resources, preparation and complete frame encoding over canonical
//! presentation. Platform shells own surfaces, adapter policy, asset transport
//! and audio; the renderer has no filesystem or windowing dependencies.

use wgpu::*;

/// Full-frame targets and platform surfaces share the camera's minimum extent.
pub fn render_size(size: [u32; 2]) -> [u32; 2] {
    size.map(|value| value.max(presentation::records::MIN_TARGET_PX))
}

/// Shared feature and limit policy for desktop and browser GPU admission.
pub fn device_descriptor(adapter: &Adapter) -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        label: Some("battle"),
        required_features: adapter.features() & Features::TIMESTAMP_QUERY,
        required_limits: adapter.limits(),
        experimental_features: ExperimentalFeatures::disabled(),
        memory_hints: MemoryHints::Performance,
        trace: Trace::Off,
    }
}

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
mod pipeline_registry;
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
pub mod submitted;
pub mod terrain;
pub mod timing;
pub mod water;
pub mod world;
pub mod xray;

/// The lit world before post.
pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// The display output format written by post.
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
/// The water's guard: how unseen and how painted what lies under it is.
pub const GUARD_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;

/// The frame's size-dependent targets.
///
/// The shared runtime constructs this set. Passes may add a surface
/// view at the shell boundary, but they do not choose formats or silently
/// allocate a second target set.
pub struct Targets {
    pub size: [u32; 2],
    /// The world, multisampled: resolved into `lit`, then the fogged world.
    pub hdr_msaa: TextureView,
    /// The lit world, with effects drawn over it.
    pub lit: TextureView,
    pub lit_texture: Texture,
    /// `lit` as the world pass left it, before the water, with its mips:
    /// what the water shows of its bed, refracted and blurred.
    pub scene_colour: TextureView,
    pub scene_colour_texture: Texture,
    /// The world with fog applied: post's input.
    pub hdr: TextureView,
    /// The world's depth, multisampled; effects read it.
    pub depth: TextureView,
    /// The fog mask, multisampled and resolved into `fog_mask`.
    pub fog_mask_msaa: TextureView,
    pub fog_mask: TextureView,
    /// The water's guard: how unseen the ground is before the water (the
    /// fog mask's unseen share) and how much paint covers it, each mip the
    /// worst of its footprint in `scene_colour`'s mips.
    pub fog_mask_under: TextureView,
    pub fog_mask_under_texture: Texture,
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
        // Never more levels than the smallest side halves to.
        let mips = pipelines::builtin_sizes()
            .scene_colour_mips
            .min(32 - size[0].min(size[1]).leading_zeros());
        let lit_texture = texture("lit", HDR_FORMAT, 1, sampled | TextureUsages::COPY_SRC);
        let scene_colour_texture = mipped(
            "scene-colour",
            HDR_FORMAT,
            1,
            sampled | TextureUsages::COPY_DST,
            mips,
        );
        let fog_mask_under_texture = mipped("fog-mask-under", GUARD_FORMAT, 1, sampled, mips);
        let display_texture = texture(
            "display",
            DISPLAY_FORMAT,
            1,
            render | TextureUsages::COPY_SRC | TextureUsages::TEXTURE_BINDING,
        );
        Self {
            size,
            hdr_msaa: view(texture("hdr-msaa", HDR_FORMAT, MSAA, render)),
            lit: lit_texture.create_view(&Default::default()),
            lit_texture,
            scene_colour: scene_colour_texture.create_view(&Default::default()),
            scene_colour_texture,
            hdr: view(texture("hdr", HDR_FORMAT, 1, sampled)),
            depth: view(texture(
                "depth",
                DEPTH_FORMAT,
                MSAA,
                sampled | TextureUsages::COPY_SRC,
            )),
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

impl Targets {
    /// Rebuild the complete target set when the pixel size changes. Returns
    /// whether a swap occurred; callers should invalidate size-bound groups
    /// only on `true`.
    pub fn rebuild(&mut self, device: &Device, size: [u32; 2]) -> bool {
        if self.size == size {
            return false;
        }
        *self = Self::new(device, size);
        true
    }
}

/// A buffer holding `bytes` from the start.
pub fn buffer(
    device: &Device,
    queue: &Queue,
    label: &str,
    usage: BufferUsages,
    bytes: &[u8],
) -> Buffer {
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage: usage | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !bytes.is_empty() {
        queue.write_buffer(&buffer, 0, bytes);
    }
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
            &self.scene_colour,
            &self.fog_mask_under,
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
