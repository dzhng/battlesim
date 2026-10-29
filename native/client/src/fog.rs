//! Fog of war's sight (`frame/fogVisibility.ts`): what the presentation
//! decides each frame (`Presentation::prepare_fog`) uploaded as it is, then
//! the browser's exported compute over it, after the depth prepass and
//! before the lit world: the rebuilt eyes' horizon maps (`fogTerrain`,
//! `fogMerge`), the structures that take fog whole (`fogWhole`), and the
//! per-tile eye lists over the prepass's depth (`fogCull`). Every fogged
//! material binds one of the five fragment groups, by what it is
//! (`Layer`): the ground paint lies on the painted layers, and units are
//! never fogged.
use presentation::fog::sight::{FogUpload, WHOLE_TEXTURE_WIDTH};
use presentation::presentation::Presentation;
use presentation::records::{
    Camera as CameraRecord, FogBox, FogEye, FogParams, FOG_FACES, FOG_GROUND, FOG_PAINTED_FACES,
};
use wgpu::*;

use crate::frame::{Inputs, Pass, Targets, DEPTH_FORMAT, HDR_FORMAT, MSAA};
use crate::gpu::{bind, buffer};
use crate::pipelines::{BoundCompute, Pipelines};

const WORD: u64 = 4;

/// What a drawn surface is to fog (`FogLayer`): its `ground`, `seen` and
/// `painted` words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layer {
    /// The terrain, its grass and the backdrop.
    PaintedGround,
    /// What movers stand on that fog takes face by face: the water, a
    /// bridge deck.
    PaintedFaces,
    /// What lies on the ground as a body: the fallen.
    Ground,
    /// Buildings, props, scenery and wreck pieces, face by face.
    Faces,
    /// Units: never fogged.
    Units,
}

impl Layer {
    /// The layer a batch's fog class names (`Batch.fog`).
    pub fn of(fog: u32) -> Self {
        match fog {
            FOG_FACES => Layer::Faces,
            FOG_GROUND => Layer::Ground,
            FOG_PAINTED_FACES => Layer::PaintedFaces,
            _ => Layer::Units,
        }
    }
}

const LAYERS: [[u32; 4]; 5] = [
    [1, 0, 1, 0],
    [0, 0, 1, 0],
    [1, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 1, 0, 0],
];

/// A storage buffer that grows (by doubling) to hold what it is handed.
struct Grown {
    label: &'static str,
    buffer: Buffer,
    usage: BufferUsages,
}

impl Grown {
    /// Starts at `record` bytes: a storage binding holds at least one.
    fn new(device: &Device, label: &'static str, usage: BufferUsages, record: usize) -> Self {
        let usage = usage | BufferUsages::COPY_DST;
        Self {
            label,
            buffer: buffer(device, label, usage, &vec![0; record.max(16)]),
            usage,
        }
    }

    /// Writes `bytes` from the start; true when the buffer was replaced.
    /// An `exact` one is the bytes' size (a shader reads its length).
    fn write(&mut self, device: &Device, queue: &Queue, bytes: &[u8], exact: bool) -> bool {
        let len = (bytes.len() as u64).max(16);
        let size = self.buffer.size();
        let grown = if exact { len != size } else { len > size };
        if grown {
            self.buffer = device.create_buffer(&BufferDescriptor {
                label: Some(self.label),
                size: if exact { len } else { len.max(size * 2) },
                usage: self.usage,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytes);
        grown
    }
}

/// This frame's passes, as the last upload asked.
#[derive(Default)]
struct Due {
    rebuild: Option<[u32; 2]>,
    /// Count, workgroups x and y, the flags' first row and rows.
    whole: Option<[u32; 5]>,
    cull: Option<[u32; 2]>,
}

pub struct Fog {
    params: Buffer,
    layers: Vec<Buffer>,
    paint_style: Buffer,
    paint: TextureView,
    heights: Buffer,
    depth: TextureView,
    scene_colour: TextureView,
    seen_under: TextureView,
    scene_taps: Sampler,
    /// What the groups the world pass binds hold for what lies under the
    /// water (`fogLayout.depth`, `sceneColour`, `seenUnder`): only the
    /// water's group holds the real ones, since the world pass writes that
    /// depth.
    no_depth: TextureView,
    no_scene: TextureView,
    eyes: Grown,
    foliage: Grown,
    occluders: Grown,
    rebuild: Grown,
    items: Grown,
    maps: Buffer,
    terrain_maps: Buffer,
    map_words: [u32; 2],
    wholes: Texture,
    flags: Buffer,
    whole_uniform: Buffer,
    lists: Buffer,
    counts: Buffer,
    tiles: [u32; 3],
    pipelines: [BoundCompute; 4],
    fog_layout: BindGroupLayout,
    build_layout: BindGroupLayout,
    cull_layout: BindGroupLayout,
    whole_layout: BindGroupLayout,
    /// The five fragment groups, by `Layer`, then the water's; then the
    /// build's, the cull's and the structures pass's.
    groups: Vec<BindGroup>,
    build: Option<BindGroup>,
    cull: Option<BindGroup>,
    whole: Option<BindGroup>,
    due: Due,
}

fn storage(device: &Device, label: &str, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage: BufferUsages::STORAGE | usage,
        mapped_at_creation: false,
    })
}

fn wholes_texture(device: &Device, rows: u32) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("fog-wholes"),
        size: Extent3d {
            width: WHOLE_TEXTURE_WIDTH as u32,
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

impl Fog {
    /// The ground paint the fog's groups bind (`fogLayout.paint`).
    pub fn paint(&self) -> &TextureView {
        &self.paint
    }

    /// Fog over the ground's height pages `heights`, culled over `targets`'
    /// depth; drawn clear until the presentation hands it sight.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        heights: &Buffer,
    ) -> Self {
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let params = buffer(
            device,
            "fog-params",
            uniform,
            bytemuck::bytes_of(&FogParams::default()),
        );
        let layers = LAYERS
            .iter()
            .map(|l| buffer(device, "fog-layer", uniform, bytemuck::cast_slice(l)))
            .collect();
        // The ground paint (the UI's marks) is absent: a replay has no UI.
        let paint_style = buffer(device, "fog-paint-style", uniform, &[0; 32]);
        let paint = device
            .create_texture(&TextureDescriptor {
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
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let placeholder = |label, format, samples, usage| {
            device
                .create_texture(&TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::TEXTURE_BINDING | usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let no_depth = placeholder(
            "fog-no-depth",
            DEPTH_FORMAT,
            MSAA,
            TextureUsages::RENDER_ATTACHMENT,
        );
        let no_scene = placeholder("fog-no-scene", HDR_FORMAT, 1, TextureUsages::empty());
        let copy = BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
        let mut fog = Self {
            params,
            layers,
            paint_style,
            paint,
            heights: heights.clone(),
            depth: targets.depth.clone(),
            scene_colour: targets.scene_colour.clone(),
            seen_under: targets.fog_mask_under.clone(),
            scene_taps: device.create_sampler(&SamplerDescriptor {
                label: Some("fog-scene-taps"),
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            }),
            no_depth,
            no_scene,
            eyes: Grown::new(
                device,
                "fog-eyes",
                BufferUsages::STORAGE,
                size_of::<FogEye>(),
            ),
            foliage: Grown::new(device, "fog-foliage", BufferUsages::STORAGE, 16),
            occluders: Grown::new(
                device,
                "fog-occluders",
                BufferUsages::STORAGE,
                size_of::<FogBox>(),
            ),
            rebuild: Grown::new(device, "fog-rebuild", BufferUsages::STORAGE, 16),
            items: Grown::new(device, "fog-whole-items", BufferUsages::STORAGE, 16),
            maps: storage(device, "fog-maps", 16, BufferUsages::empty()),
            terrain_maps: storage(device, "fog-terrain-maps", 16, BufferUsages::empty()),
            map_words: [0, 0],
            wholes: wholes_texture(device, 1),
            flags: storage(device, "fog-whole-flags", 1024, copy),
            whole_uniform: buffer(device, "fog-whole", uniform, &[0; 16]),
            lists: storage(device, "fog-tile-lists", 16, BufferUsages::empty()),
            counts: storage(device, "fog-tile-counts", 16, copy),
            tiles: [0; 3],
            pipelines: ["fogTerrain", "fogMerge", "fogCull", "fogWhole"]
                .map(|n| pipelines.bound_compute(n)),
            fog_layout: pipelines.layout("fogLayout").clone(),
            build_layout: pipelines.layout("buildLayout").clone(),
            cull_layout: pipelines.layout("cullLayout").clone(),
            whole_layout: pipelines.layout("wholeLayout").clone(),
            groups: Vec::new(),
            build: None,
            cull: None,
            whole: None,
            due: Due::default(),
        };
        fog.bind(device);
        fog
    }

    /// Every group again, over the buffers as they are now.
    fn bind(&mut self, device: &Device) {
        let wholes = self.wholes.create_view(&Default::default());
        let fragment = |layer: &Buffer, [depth, scene, seen]: [&TextureView; 3]| {
            bind(
                device,
                &self.fog_layout,
                "fog",
                &[
                    self.params.as_entire_binding(),
                    layer.as_entire_binding(),
                    self.eyes.buffer.as_entire_binding(),
                    self.maps.as_entire_binding(),
                    self.lists.as_entire_binding(),
                    self.counts.as_entire_binding(),
                    self.paint_style.as_entire_binding(),
                    BindingResource::TextureView(&self.paint),
                    BindingResource::TextureView(&wholes),
                    BindingResource::TextureView(depth),
                    BindingResource::TextureView(scene),
                    BindingResource::Sampler(&self.scene_taps),
                    BindingResource::TextureView(seen),
                    // The wave field: native draws no wakes (water-native).
                    BindingResource::TextureView(&self.no_scene),
                ],
            )
        };
        let under = [&self.depth, &self.scene_colour, &self.seen_under];
        let water = fragment(&self.layers[Layer::PaintedFaces as usize], under);
        let none = [&self.no_depth, &self.no_scene, &self.no_scene];
        self.groups = self
            .layers
            .iter()
            .map(|layer| fragment(layer, none))
            .chain([water])
            .collect();
        self.build = Some(bind(
            device,
            &self.build_layout,
            "fog-build",
            &[
                self.params.as_entire_binding(),
                self.eyes.buffer.as_entire_binding(),
                self.heights.as_entire_binding(),
                self.foliage.buffer.as_entire_binding(),
                self.occluders.buffer.as_entire_binding(),
                self.rebuild.buffer.as_entire_binding(),
                self.terrain_maps.as_entire_binding(),
                self.maps.as_entire_binding(),
            ],
        ));
        self.cull = Some(bind(
            device,
            &self.cull_layout,
            "fog-cull",
            &[
                self.params.as_entire_binding(),
                self.eyes.buffer.as_entire_binding(),
                BindingResource::TextureView(&self.depth),
                self.lists.as_entire_binding(),
                self.counts.as_entire_binding(),
            ],
        ));
        self.whole = Some(bind(
            device,
            &self.whole_layout,
            "fog-whole",
            &[
                self.occluders.buffer.as_entire_binding(),
                self.whole_uniform.as_entire_binding(),
                self.flags.as_entire_binding(),
                self.items.buffer.as_entire_binding(),
            ],
        ));
    }

    /// The fragment group a surface of `layer` binds.
    pub fn group(&self, layer: Layer) -> (&'static str, &BindGroup) {
        ("fogLayout", &self.groups[layer as usize])
    }

    /// The water's fragment group: the painted faces', with what lies under
    /// the water (the world's depth and its colour before the water).
    pub fn water_group(&self) -> (&'static str, &BindGroup) {
        ("fogLayout", &self.groups[LAYERS.len()])
    }

    /// Takes this frame's fog from `presentation`, for the frame's `camera`
    /// record: uploads it, and keeps the passes it asks for.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &mut Presentation,
        camera: &CameraRecord,
    ) -> Result<(), String> {
        let size = [camera.width as u32, camera.height as u32];
        self.due = Due::default();
        let Some(up) = presentation.prepare_fog(Some(camera), size[0], size[1], 0)? else {
            return Ok(());
        };
        if self.upload(device, queue, &up) {
            self.bind(device);
        }
        Ok(())
    }

    /// Uploads `up`; true when a buffer a group binds was replaced.
    fn upload(&mut self, device: &Device, queue: &Queue, up: &FogUpload) -> bool {
        let mut rebind = false;
        let copy = BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
        if up.map_words != self.map_words {
            // A new size rebuilds every map: the old ones are not kept.
            let [maps, terrain] = up.map_words.map(|w| u64::from(w) * WORD);
            self.maps = storage(device, "fog-maps", maps, BufferUsages::empty());
            self.terrain_maps = storage(device, "fog-terrain-maps", terrain, BufferUsages::empty());
            self.map_words = up.map_words;
            rebind = true;
        }
        if let Some(foliage) = &up.foliage {
            // The foliage lookup searches the whole buffer (`arrayLength`).
            rebind |= self
                .foliage
                .write(device, queue, bytemuck::cast_slice(foliage), true);
        }
        if let Some(occluders) = &up.occluders {
            rebind |= self
                .occluders
                .write(device, queue, bytemuck::cast_slice(occluders), false);
        }
        if let Some(words) = &up.wholes {
            let rows = (words.len() / WHOLE_TEXTURE_WIDTH) as u32;
            self.wholes = wholes_texture(device, rows);
            queue.write_texture(
                self.wholes.as_image_copy(),
                bytemuck::cast_slice(words),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(WHOLE_TEXTURE_WIDTH as u32 * 4),
                    rows_per_image: None,
                },
                self.wholes.size(),
            );
            rebind = true;
        }
        if !up.eyes.is_empty() {
            rebind |= self
                .eyes
                .write(device, queue, bytemuck::cast_slice(&up.eyes), false);
        }
        if !up.rebuild.is_empty() {
            rebind |= self
                .rebuild
                .write(device, queue, bytemuck::cast_slice(&up.rebuild), false);
            self.due.rebuild = Some(up.rebuild_dispatch);
        }
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&up.params));
        if !up.active {
            return rebind;
        }
        let [x, y, eyes_max] = up.tiles;
        if self.tiles != up.tiles {
            let n = u64::from(x * y);
            self.lists = storage(
                device,
                "fog-tile-lists",
                n * u64::from(eyes_max) * WORD,
                BufferUsages::empty(),
            );
            self.counts = storage(device, "fog-tile-counts", n * WORD, copy);
            self.tiles = up.tiles;
            rebind = true;
        }
        self.due.cull = Some([x, y]);
        if let Some(whole) = &up.whole {
            let rows = u64::from(whole.flag_rows) * WHOLE_TEXTURE_WIDTH as u64 * WORD;
            if self.flags.size() != rows {
                self.flags = storage(device, "fog-whole-flags", rows, copy);
                rebind = true;
            }
            if whole.count > 0 {
                rebind |=
                    self.items
                        .write(device, queue, bytemuck::cast_slice(&whole.items), false);
                queue.write_buffer(&self.whole_uniform, 0, bytemuck::cast_slice(&whole.uniform));
            }
            self.due.whole = Some([
                whole.count,
                whole.dispatch[0],
                whole.dispatch[1],
                whole.flags_row,
                whole.flag_rows,
            ]);
        }
        rebind
    }

    /// The passes the last upload asked for, in the browser's order.
    fn encode(&self, encoder: &mut CommandEncoder) {
        let compute = |encoder: &mut CommandEncoder,
                       label,
                       bound: &BoundCompute,
                       groups: &[(&str, &BindGroup)],
                       [x, y]: [u32; 2]| {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });
            bound.set(&mut pass, groups);
            pass.dispatch_workgroups(x, y, 1);
        };
        let [terrain, merge, cull, whole] = &self.pipelines;
        let group = |g: &Option<BindGroup>| {
            g.as_ref()
                .expect("fog's groups are bound at its start")
                .clone()
        };
        if let Some([t, m]) = self.due.rebuild {
            let build = group(&self.build);
            let build = [("buildLayout", &build)];
            compute(encoder, "fog-terrain", terrain, &build, [t, 1]);
            compute(encoder, "fog-merge", merge, &build, [m, 1]);
        }
        let Some(tiles) = self.due.cull else {
            return;
        };
        if let Some([count, x, y, flags_row, rows]) = self.due.whole {
            // An eye leaving a neighbourhood must clear its last seen flags.
            encoder.clear_buffer(&self.flags, 0, None);
            if count > 0 {
                let whole_group = group(&self.whole);
                let groups = [("wholeLayout", &whole_group), self.group(Layer::Faces)];
                compute(encoder, "fog-whole", whole, &groups, [x, y]);
            }
            encoder.copy_buffer_to_texture(
                TexelCopyBufferInfo {
                    buffer: &self.flags,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(WHOLE_TEXTURE_WIDTH as u32 * 4),
                        rows_per_image: None,
                    },
                },
                TexelCopyTextureInfo {
                    texture: &self.wholes,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: flags_row,
                        z: 0,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: WHOLE_TEXTURE_WIDTH as u32,
                    height: rows,
                    depth_or_array_layers: 1,
                },
            );
        }
        encoder.clear_buffer(&self.counts, 0, None);
        compute(
            encoder,
            "fog-cull",
            cull,
            &[("cullLayout", &group(&self.cull))],
            tiles,
        );
    }
}

/// Fog's compute: after the depth prepass, before the lit world.
pub struct FogSight;

impl Pass for FogSight {
    fn name(&self) -> &'static str {
        "fog-sight"
    }

    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        inputs.fog.encode(encoder);
    }
}
