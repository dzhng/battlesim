//! Shared fog-of-war sight resources and bind-group ownership.
//!
//! The presentation layer computes plain records (`FogParams`, eyes, foliage
//! and known occluders). This owner keeps the GPU buffers, texture and four
//! compute bind-group layouts in one place for native and browser shells. It
//! does not decide when sight is active or expose presentation types.

use wgpu::*;

use crate::fog::{FogComputeOwner, FogStage};
use crate::pipelines::Pipelines;

const WORD: u64 = 4;
const MAX_LAYERS: usize = 5;
const WHOLE_WIDTH: u32 = 256;

// The five fragment groups are a renderer contract, not per-frame data:
// painted ground is fogged and painted, ordinary ground is fogged, faces are
// fogged without ground treatment, and units stay visible.
const LAYER_RECORDS: [[u32; 4]; MAX_LAYERS] = [
    [1, 0, 1, 0],
    [0, 0, 1, 0],
    [1, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 1, 0, 0],
];

/// A packed upload from a presentation fog sight record.
///
/// All byte slices use the generated presentation record layout. The core
/// deliberately keeps this boundary byte-based so it remains independent of
/// the presentation crate and can be called from browser WASM.
pub struct FogSightUpload<'a> {
    pub params: &'a [u8],
    pub eyes: &'a [u8],
    pub foliage: &'a [u8],
    pub occluders: &'a [u8],
    pub rebuild: &'a [u8],
    pub map_words: [u32; 2],
    pub tiles: [u32; 3],
    pub wholes: &'a [u32],
    pub whole_items: &'a [u8],
    pub whole_uniform: &'a [u8],
}

fn storage(device: &Device, label: &str, size: u64) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn uniform(device: &Device, label: &str, bytes: &[u8]) -> Buffer {
    crate::buffer(
        device,
        label,
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        bytes,
    )
}

fn grow(device: &Device, buffer: &mut Buffer, label: &str, bytes: &[u8]) -> bool {
    let needed = bytes.len().max(16) as u64;
    if buffer.size() >= needed {
        return false;
    }
    *buffer = storage(device, label, needed.next_power_of_two());
    true
}

/// The shared GPU owner for fog sight's compute and material groups.
pub struct FogSightResources {
    params: Buffer,
    layers: [Buffer; MAX_LAYERS],
    eyes: Buffer,
    maps: Buffer,
    terrain_maps: Buffer,
    foliage: Buffer,
    occluders: Buffer,
    rebuild: Buffer,
    lists: Buffer,
    counts: Buffer,
    paint_style: Buffer,
    height: Buffer,
    whole_uniform: Buffer,
    flags: Buffer,
    items: Buffer,
    paint: Texture,
    wholes: Texture,
    depth: TextureView,
    fog_layout: BindGroupLayout,
    build_layout: BindGroupLayout,
    cull_layout: BindGroupLayout,
    whole_layout: BindGroupLayout,
    groups: [Option<BindGroup>; MAX_LAYERS],
    build: Option<BindGroup>,
    cull: Option<BindGroup>,
    whole: Option<BindGroup>,
    compute: FogComputeOwner,
    map_words: [u32; 2],
    tiles: [u32; 3],
}

impl FogSightResources {
    /// Allocate a valid zeroed fog graph. Zero records mean fog is disabled;
    /// no browser frame needs to bind an absent or guessed resource.
    pub fn new(device: &Device, pipelines: &Pipelines, depth: &TextureView) -> Self {
        let params = uniform(device, "fog-params", &[0; 256]);
        let layers = std::array::from_fn(|i| {
            uniform(device, "fog-layer", bytemuck::cast_slice(&LAYER_RECORDS[i]))
        });
        let paint_style = uniform(device, "fog-paint-style", &[0; 32]);
        let whole_uniform = uniform(device, "fog-whole", &[0; 16]);
        let height = storage(device, "fog-heights", 16);
        let eyes = storage(device, "fog-eyes", 16);
        let maps = storage(device, "fog-maps", 16);
        let terrain_maps = storage(device, "fog-terrain-maps", 16);
        let foliage = storage(device, "fog-foliage", 16);
        let occluders = storage(device, "fog-occluders", 32);
        let rebuild = storage(device, "fog-rebuild", 16);
        let lists = storage(device, "fog-tile-lists", 16);
        let counts = storage(device, "fog-tile-counts", 16);
        let flags = storage(device, "fog-whole-flags", WHOLE_WIDTH as u64 * WORD);
        let items = storage(device, "fog-whole-items", 16);
        let paint = device.create_texture(&TextureDescriptor {
            label: Some("fog-paint"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let wholes = Self::wholes_texture(device, 1);
        let fog_layout = pipelines.layout("fogLayout").clone();
        let build_layout = pipelines.layout("buildLayout").clone();
        let cull_layout = pipelines.layout("cullLayout").clone();
        let whole_layout = pipelines.layout("wholeLayout").clone();
        let mut owner = Self {
            params,
            layers,
            eyes,
            maps,
            terrain_maps,
            foliage,
            occluders,
            rebuild,
            lists,
            counts,
            paint_style,
            height,
            whole_uniform,
            flags,
            items,
            paint,
            wholes,
            depth: depth.clone(),
            fog_layout,
            build_layout,
            cull_layout,
            whole_layout,
            groups: std::array::from_fn(|_| None),
            build: None,
            cull: None,
            whole: None,
            compute: FogComputeOwner::new(pipelines),
            map_words: [0; 2],
            tiles: [0; 3],
        };
        owner.rebind(device);
        owner
    }

    fn wholes_texture(device: &Device, rows: u32) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("fog-wholes"),
            size: Extent3d {
                width: WHOLE_WIDTH,
                height: rows.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Uint,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn rebind(&mut self, device: &Device) {
        let paint = self.paint.create_view(&TextureViewDescriptor::default());
        let wholes = self.wholes.create_view(&TextureViewDescriptor::default());
        self.groups = std::array::from_fn(|i| {
            Some(crate::bind(
                device,
                &self.fog_layout,
                "fog",
                &[
                    self.params.as_entire_binding(),
                    self.layers[i].as_entire_binding(),
                    self.eyes.as_entire_binding(),
                    self.maps.as_entire_binding(),
                    self.lists.as_entire_binding(),
                    self.counts.as_entire_binding(),
                    self.paint_style.as_entire_binding(),
                    BindingResource::TextureView(&paint),
                    BindingResource::TextureView(&wholes),
                ],
            ))
        });
        self.build = Some(crate::bind(
            device,
            &self.build_layout,
            "fog-build",
            &[
                self.params.as_entire_binding(),
                self.eyes.as_entire_binding(),
                self.height.as_entire_binding(),
                self.foliage.as_entire_binding(),
                self.occluders.as_entire_binding(),
                self.rebuild.as_entire_binding(),
                self.terrain_maps.as_entire_binding(),
                self.maps.as_entire_binding(),
            ],
        ));
        self.cull = Some(crate::bind(
            device,
            &self.cull_layout,
            "fog-cull",
            &[
                self.params.as_entire_binding(),
                self.eyes.as_entire_binding(),
                BindingResource::TextureView(&self.depth),
                self.lists.as_entire_binding(),
                self.counts.as_entire_binding(),
            ],
        ));
        self.whole = Some(crate::bind(
            device,
            &self.whole_layout,
            "fog-whole",
            &[
                self.occluders.as_entire_binding(),
                self.whole_uniform.as_entire_binding(),
                self.flags.as_entire_binding(),
                self.items.as_entire_binding(),
            ],
        ));
    }

    /// Upload one presentation sight record and rebuild groups if a growable
    /// resource changed size. This validates resource ownership without
    /// dispatching; shells can use `dispatch` only once their visibility
    /// inputs are real.
    pub fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        up: &FogSightUpload<'_>,
    ) -> Result<(), String> {
        if up.params.len() > self.params.size() as usize {
            return Err(format!(
                "fog params record is {} bytes, buffer is {} bytes",
                up.params.len(),
                self.params.size()
            ));
        }
        if up.whole_uniform.len() > self.whole_uniform.size() as usize {
            return Err(format!(
                "fog whole record is {} bytes, buffer is {} bytes",
                up.whole_uniform.len(),
                self.whole_uniform.size()
            ));
        }
        let mut rebind = false;
        queue.write_buffer(&self.params, 0, up.params);
        if !up.eyes.is_empty() {
            rebind |= grow(device, &mut self.eyes, "fog-eyes", up.eyes);
            queue.write_buffer(&self.eyes, 0, up.eyes);
        }
        if !up.foliage.is_empty() {
            rebind |= grow(device, &mut self.foliage, "fog-foliage", up.foliage);
            queue.write_buffer(&self.foliage, 0, up.foliage);
        }
        if !up.occluders.is_empty() {
            rebind |= grow(device, &mut self.occluders, "fog-occluders", up.occluders);
            queue.write_buffer(&self.occluders, 0, up.occluders);
        }
        if !up.rebuild.is_empty() {
            rebind |= grow(device, &mut self.rebuild, "fog-rebuild", up.rebuild);
            queue.write_buffer(&self.rebuild, 0, up.rebuild);
        }
        let map_sizes = [up.map_words[0] as u64 * WORD, up.map_words[1] as u64 * WORD];
        if map_sizes[0] > self.maps.size() {
            self.maps = storage(device, "fog-maps", map_sizes[0]);
            rebind = true;
        }
        if map_sizes[1] > self.terrain_maps.size() {
            self.terrain_maps = storage(device, "fog-terrain-maps", map_sizes[1]);
            rebind = true;
        }
        let list_size = u64::from(
            up.tiles[0]
                .saturating_mul(up.tiles[1])
                .saturating_mul(up.tiles[2]),
        ) * WORD;
        if list_size > self.lists.size() {
            self.lists = storage(device, "fog-tile-lists", list_size);
            rebind = true;
        }
        let count_size = u64::from(up.tiles[0].saturating_mul(up.tiles[1])) * WORD;
        if count_size > self.counts.size() {
            self.counts = storage(device, "fog-tile-counts", count_size);
            rebind = true;
        }
        if !up.whole_uniform.is_empty() {
            queue.write_buffer(&self.whole_uniform, 0, up.whole_uniform);
        }
        if !up.wholes.is_empty() {
            self.wholes =
                Self::wholes_texture(device, (up.wholes.len() as u32).div_ceil(WHOLE_WIDTH));
            queue.write_texture(
                self.wholes.as_image_copy(),
                bytemuck::cast_slice(up.wholes),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(WHOLE_WIDTH * WORD as u32),
                    rows_per_image: None,
                },
                self.wholes.size(),
            );
            let flags_size =
                (up.wholes.len() as u64).div_ceil(WHOLE_WIDTH as u64) * WHOLE_WIDTH as u64 * WORD;
            if flags_size > self.flags.size() {
                self.flags = storage(device, "fog-whole-flags", flags_size);
            }
            rebind = true;
        }
        if !up.whole_items.is_empty() {
            rebind |= grow(device, &mut self.items, "fog-whole-items", up.whole_items);
            queue.write_buffer(&self.items, 0, up.whole_items);
        }
        self.map_words = up.map_words;
        self.tiles = up.tiles;
        if rebind {
            self.rebind(device);
        }
        Ok(())
    }

    /// Upload the static ground height pages once per ground generation.
    pub fn upload_heights(
        &mut self,
        device: &Device,
        queue: &Queue,
        heights: &[u8],
    ) -> Result<(), String> {
        if heights.is_empty() {
            return Ok(());
        }
        let rebind = grow(device, &mut self.height, "fog-heights", heights);
        queue.write_buffer(&self.height, 0, heights);
        if rebind {
            self.rebind(device);
        }
        Ok(())
    }

    /// Dispatch terrain/merge and cull/whole in native order. This remains an
    /// explicit opt-in so a shell can keep its visible fixture unchanged while
    /// it proves all uploaded resources on a real device.
    pub fn dispatch(
        &self,
        encoder: &mut CommandEncoder,
        rebuild: Option<[u32; 2]>,
        whole: Option<[u32; 2]>,
    ) {
        if let Some([terrain, merge]) = rebuild {
            self.compute.dispatch(
                encoder,
                FogStage::Terrain,
                &[(
                    "buildLayout",
                    self.build.as_ref().expect("fog build group bound"),
                )],
                [terrain, 1],
            );
            self.compute.dispatch(
                encoder,
                FogStage::Merge,
                &[(
                    "buildLayout",
                    self.build.as_ref().expect("fog build group bound"),
                )],
                [merge, 1],
            );
        }
        if let Some([x, y]) = whole {
            self.compute.dispatch(
                encoder,
                FogStage::Whole,
                &[
                    (
                        "wholeLayout",
                        self.whole.as_ref().expect("fog whole group bound"),
                    ),
                    (
                        "fogLayout",
                        self.groups[3].as_ref().expect("fog material group bound"),
                    ),
                ],
                [x, y],
            );
        }
        self.compute.dispatch(
            encoder,
            FogStage::Cull,
            &[(
                "cullLayout",
                self.cull.as_ref().expect("fog cull group bound"),
            )],
            [self.tiles[0], self.tiles[1]],
        );
    }

    pub fn group(&self, layer: usize) -> &BindGroup {
        self.groups[layer.min(MAX_LAYERS - 1)]
            .as_ref()
            .expect("fog material group bound")
    }
}
