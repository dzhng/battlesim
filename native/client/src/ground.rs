//! The ground (`worldPass.ts`'s terrain, skirt and backdrop, and
//! `terrainMaterial.ts`'s `createTerrainSource`): the presentation crate's
//! ground tables and meshes, uploaded once, drawn into the depth prepass,
//! the sun's cascades and the lit world. The terrain's group (`terrainLayout`)
//! is the grass build's too.
use presentation::static_world::files::MapGround;
use wgpu::*;

use crate::gpu::{bind, buffer};
use crate::pending::mesh::{self, VERTEX_FLOATS};
use crate::pending::shadow::map_box;
use crate::pipelines::{Bound, Pipelines};

/// `TerrainParams`' last `vec4u`, the frame's view: the lit ground (not its
/// classes), the roads worn.
const VIEW: [u32; 4] = [0, 0, 0, 0];
/// `ScarParams`: the grid and cache words, then the biome's look from `full`.
const SCAR_WORDS: usize = 52;
const SCAR_LOOK: usize = 20;
/// The scar pool's texture (`scarTexture.ts`), its first texels the
/// normalisation table the shader reads.
const SCAR_POOL: [u32; 2] = [2048, 1024];
const SCAR_NORMALIZATION_WORDS: usize = 4096;
/// The identity instance (`identityInstance`): no offset, unit scale, no tint.
const IDENTITY: [f32; 8] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0];

/// A mesh of world vertices (`mesh.ts`).
struct Mesh {
    buffer: Option<Buffer>,
    vertices: u32,
}

impl Mesh {
    fn new(device: &Device, label: &str, words: &[f32]) -> Self {
        Self {
            buffer: (!words.is_empty()).then(|| {
                buffer(
                    device,
                    label,
                    BufferUsages::VERTEX,
                    bytemuck::cast_slice(words),
                )
            }),
            vertices: (words.len() / VERTEX_FLOATS) as u32,
        }
    }

    fn draw(&self, pass: &mut RenderPass, identity: &Buffer) {
        let Some(buffer) = &self.buffer else { return };
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.set_vertex_buffer(1, identity.slice(..));
        pass.draw(0..self.vertices, 0..1);
    }
}

/// A storage table holding at least one record of `record` bytes.
fn table(device: &Device, label: &str, bytes: &[u8], record: usize) -> Buffer {
    let mut padded = bytes.to_vec();
    padded.resize(bytes.len().max(record), 0);
    buffer(device, label, BufferUsages::STORAGE, &padded)
}

fn texture(device: &Device, label: &str, size: [u32; 2], format: TextureFormat) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub struct Ground {
    /// `terrainLayout`: the terrain's tables and the side's scars.
    pub group: BindGroup,
    /// The height pages fog and grass read (`Grid::pages`).
    pub heights: Buffer,
    /// The box the map's ground and what stands on it fill (`mapBox`).
    pub map_box: Option<[f64; 6]>,
    terrain: Mesh,
    skirt: Mesh,
    backdrop: Mesh,
    identity: Buffer,
    prepass: Bound,
    caster: Bound,
    lit_terrain: Bound,
    lit_skirt: Bound,
    lit_backdrop: Bound,
}

impl Ground {
    /// Uploads `map`'s ground, its backdrop reaching `backdrop_m` past the map.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        map: &MapGround,
        backdrop_m: f64,
    ) -> Result<Self, String> {
        let ground = &map.ground;
        let tables = ground.terrain_tables(ground.terrain_field()?);
        let uniform = BufferUsages::UNIFORM;
        let params: Vec<u32> = tables.params.iter().copied().chain(VIEW).collect();
        let params = buffer(
            device,
            "terrain-params",
            uniform,
            bytemuck::cast_slice(&params),
        );
        let nodes = table(
            device,
            "terrain-nodes",
            bytemuck::cast_slice(&tables.nodes),
            32,
        );
        let plots = ground.plot_records(true);
        let plots = table(device, "terrain-plots", bytemuck::cast_slice(&plots), 96);
        let surfaces = table(
            device,
            "terrain-surfaces",
            bytemuck::cast_slice(&tables.surfaces),
            32,
        );
        let index = table(
            device,
            "terrain-index",
            bytemuck::cast_slice(&tables.surface_index),
            4,
        );
        // No scars yet: an empty grid (`grid.w` 0), the biome's look after it.
        let mut scars = [0u32; SCAR_WORDS];
        scars[SCAR_LOOK..SCAR_LOOK + tables.scars.len()].copy_from_slice(&tables.scars);
        let scar_params = buffer(device, "scar-params", uniform, bytemuck::cast_slice(&scars));
        let pool = texture(
            device,
            "ground-scar-words",
            SCAR_POOL,
            TextureFormat::Rgba8Unorm,
        );
        let normalization: Vec<f32> = (0..SCAR_NORMALIZATION_WORDS)
            .map(|q| q.min(4080) as f32 / 4080.0)
            .collect();
        queue.write_texture(
            pool.as_image_copy(),
            bytemuck::cast_slice(&normalization),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SCAR_POOL[0] * 4),
                rows_per_image: None,
            },
            Extent3d {
                width: SCAR_POOL[0],
                height: SCAR_NORMALIZATION_WORDS as u32 / SCAR_POOL[0],
                depth_or_array_layers: 1,
            },
        );
        let directory = texture(
            device,
            "ground-scar-directory",
            [1, 1],
            TextureFormat::Rg32Uint,
        );
        let group = bind(
            device,
            pipelines.layout("terrainLayout"),
            "terrain",
            &[
                params.as_entire_binding(),
                nodes.as_entire_binding(),
                plots.as_entire_binding(),
                surfaces.as_entire_binding(),
                index.as_entire_binding(),
                scar_params.as_entire_binding(),
                BindingResource::TextureView(&directory.create_view(&Default::default())),
                BindingResource::TextureView(&pool.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..Default::default()
                })),
            ],
        );
        let heights = buffer(
            device,
            "terrain-heights",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&ground.source.grid.pages()),
        );
        let terrain = mesh::terrain(&map.positions, &map.indices, None);
        let map_box = map_box(&terrain);
        let backdrop = map_box.map_or_else(Vec::new, |b| mesh::backdrop(&b, backdrop_m));
        Ok(Self {
            group,
            heights,
            map_box,
            terrain: Mesh::new(device, "terrain", &terrain),
            skirt: Mesh::new(device, "skirt", &mesh::skirt(&map.positions)),
            backdrop: Mesh::new(device, "backdrop", &backdrop),
            identity: buffer(
                device,
                "identity",
                BufferUsages::VERTEX,
                bytemuck::cast_slice(&IDENTITY),
            ),
            prepass: pipelines.bound("worldPrepass"),
            caster: pipelines.bound("worldCaster"),
            lit_terrain: pipelines.bound("worldTerrain"),
            lit_skirt: pipelines.bound("worldOpaque"),
            lit_backdrop: pipelines.bound("worldBackdrop"),
        })
    }

    /// The group by the layout it fills.
    pub fn layout_group(&self) -> (&'static str, &BindGroup) {
        ("terrainLayout", &self.group)
    }

    /// The ground, the skirt and the backdrop into the depth prepass.
    pub fn draw_depth(&self, pass: &mut RenderPass, camera: (&str, &BindGroup)) {
        self.prepass.set(pass, &[camera]);
        for mesh in [&self.terrain, &self.skirt, &self.backdrop] {
            mesh.draw(pass, &self.identity);
        }
    }

    /// The ground and the skirt into one cascade (the backdrop casts nothing).
    pub fn draw_casters(&self, pass: &mut RenderPass, camera: (&str, &BindGroup)) {
        self.caster.set(pass, &[camera]);
        for mesh in [&self.terrain, &self.skirt] {
            mesh.draw(pass, &self.identity);
        }
    }

    /// The lit terrain, `groups` the frame's (camera, environment, fog).
    pub fn draw_terrain(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_terrain.set(pass, &all);
        self.terrain.draw(pass, &self.identity);
    }

    /// The map's skirt, lit as the world's faces.
    pub fn draw_skirt(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        self.lit_skirt.set(pass, groups);
        self.skirt.draw(pass, &self.identity);
    }

    /// The backdrop: the ground's material past the map, never shadowed.
    pub fn draw_backdrop(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_backdrop.set(pass, &all);
        self.backdrop.draw(pass, &self.identity);
    }
}
