//! The ground (`worldPass.ts`'s terrain, skirt and backdrop, and
//! `terrainMaterial.ts`'s `createTerrainSource`): the presentation's ground
//! (`Presentation::ground`), its tables and meshes uploaded once, drawn into
//! the depth prepass, the sun's cascades and the lit world. The terrain's
//! group (`terrainLayout`) is the grass build's too, and holds the side's
//! scars (`scars.rs`).
use std::ops::Range;

use presentation::records::Camera;
use presentation::shadow::{map_box, SunShadowFrame, CASCADES};
use presentation::static_world::ground::mesh::{self, VERTEX_FLOATS};
use presentation::static_world::ground::partition::{TerrainPartition, CELL_M};
use presentation::static_world::ground::Ground as Resolved;
use wgpu::*;

use presentation::presentation::Presentation;

use crate::gpu::{bind, buffer};
use crate::pipelines::{Bound, Pipelines};
use crate::scars::Scars;
use crate::water_normals::WaterNormals;

/// `TerrainParams`' last `vec4u`, the frame's view: the lit ground (not its
/// classes), the roads worn.
const VIEW: [u32; 4] = [0, 0, 0, 0];
/// The identity instance (`identityInstance`): no offset, unit scale, no tint.
const IDENTITY: [f32; 8] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0];

/// A mesh of world vertices (`mesh.ts`).
struct Mesh {
    buffer: Option<Buffer>,
    partition: Option<std::sync::Arc<TerrainPartition>>,
    view: Vec<Range<u32>>,
    cascades: [Vec<Range<u32>>; CASCADES],
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
            partition: None,
            view: std::iter::once(0..(words.len() / VERTEX_FLOATS) as u32).collect(),
            cascades: std::array::from_fn(|_| Vec::new()),
        }
    }

    fn partitioned(device: &Device, label: &str, words: &[f32]) -> Self {
        let partition = std::sync::Arc::new(TerrainPartition::new(words, CELL_M));
        let mut mesh = Self::new(device, label, partition.vertices());
        mesh.partition = Some(partition);
        mesh.view.clear();
        mesh
    }

    fn prepare(&mut self, camera: &Camera, casters: &[Camera]) {
        let Some(partition) = &self.partition else {
            return;
        };
        partition.select(&camera.view_proj, &mut self.view);
        for (i, ranges) in self.cascades.iter_mut().enumerate() {
            if let Some(caster) = casters.get(i) {
                partition.select(&caster.view_proj, ranges);
            } else {
                ranges.clear();
            }
        }
    }

    fn ranges(&self, cascade: Option<usize>) -> &[Range<u32>] {
        cascade.map_or(&self.view, |i| &self.cascades[i])
    }

    fn draw(&self, pass: &mut RenderPass, identity: &Buffer, cascade: Option<usize>) {
        let Some(buffer) = &self.buffer else { return };
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.set_vertex_buffer(1, identity.slice(..));
        for range in self.ranges(cascade) {
            pass.draw(range.clone(), 0..1);
        }
    }
}

/// `terrainLayout`'s group: the tables, then the scars.
fn terrain_group(
    device: &Device,
    layout: &BindGroupLayout,
    tables: &[Buffer; 5],
    scars: &Scars,
    water: &WaterNormals,
) -> BindGroup {
    let [params, nodes, plots, surfaces, index] = tables;
    bind(
        device,
        layout,
        "terrain",
        &[
            params.as_entire_binding(),
            nodes.as_entire_binding(),
            plots.as_entire_binding(),
            surfaces.as_entire_binding(),
            index.as_entire_binding(),
            scars.params.as_entire_binding(),
            BindingResource::TextureView(&scars.directory_view()),
            BindingResource::TextureView(&scars.pool_view()),
            BindingResource::TextureView(&water.view),
            BindingResource::Sampler(&water.sampler),
        ],
    )
}

/// A storage table holding at least one record of `record` bytes.
fn table(device: &Device, label: &str, bytes: &[u8], record: usize) -> Buffer {
    let mut padded = bytes.to_vec();
    padded.resize(bytes.len().max(record), 0);
    buffer(device, label, BufferUsages::STORAGE, &padded)
}

pub struct Ground {
    /// `terrainLayout`: the terrain's tables and the side's scars.
    pub group: BindGroup,
    layout: BindGroupLayout,
    /// The terrain's tables: params, plot nodes, plots, surfaces and their index.
    tables: [Buffer; 5],
    pub scars: Scars,
    water_normals: WaterNormals,
    /// The height pages fog and grass read (`Grid::pages`).
    pub heights: Buffer,
    /// The box the map's ground and what stands on it fill (`mapBox`).
    pub map_box: Option<[f64; 6]>,
    terrain: Mesh,
    pub terrain_partition: std::sync::Arc<TerrainPartition>,
    skirt: Mesh,
    backdrop: Mesh,
    water: Mesh,
    identity: Buffer,
    prepass: Bound,
    caster: Bound,
    lit_terrain: Bound,
    lit_skirt: Bound,
    lit_backdrop: Bound,
    lit_water: Bound,
}

impl Ground {
    /// Uploads `ground` over the simulation's terrain triangles (`positions`
    /// and `indices`), its backdrop reaching `backdrop_m` past the map.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        ground: &Resolved,
        (positions, indices): (&[f32], &[u32]),
        backdrop_m: f64,
    ) -> Result<Self, String> {
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
        let scars = Scars::new(device, queue, &tables.scars);
        let tables = [params, nodes, plots, surfaces, index];
        let layout = pipelines.layout("terrainLayout").clone();
        let water_normals = WaterNormals::new(device, queue, pipelines);
        let group = terrain_group(device, &layout, &tables, &scars, &water_normals);
        let heights = buffer(
            device,
            "terrain-heights",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&ground.source.grid.pages()),
        );
        let terrain = mesh::terrain(positions, indices, None);
        let map_box = map_box(&terrain);
        // Coarse spatial batches avoid one draw per terrain cell while allowing
        // the camera and each shadow cascade to exclude distant ground.
        let terrain = Mesh::partitioned(device, "terrain", &terrain);
        let terrain_partition = terrain.partition.as_ref().unwrap().clone();
        let backdrop = map_box.map_or_else(Vec::new, |b| mesh::backdrop(&b, backdrop_m));
        let skirt = Mesh::partitioned(device, "skirt", &mesh::skirt(positions));
        let water = Mesh::partitioned(device, "water", &mesh::water(&ground.water_cells()));
        Ok(Self {
            group,
            layout,
            tables,
            scars,
            water_normals,
            heights,
            map_box,
            terrain,
            terrain_partition,
            skirt,
            backdrop: Mesh::new(device, "backdrop", &backdrop),
            water,
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
            lit_water: pipelines.bound("worldWater"),
        })
    }

    /// Select from the same camera records uploaded for the colour and shadow passes.
    pub fn prepare(&mut self, camera: &Camera, shadow: Option<&SunShadowFrame>) {
        let casters = shadow.map_or(&[][..], |fit| fit.casters.as_slice());
        self.terrain.prepare(camera, casters);
        self.skirt.prepare(camera, casters);
        self.water.prepare(camera, &[]);
    }

    /// Takes this frame's scar uploads from `presentation` for a `size`
    /// target, the grass's window `grass` first when it regrows.
    pub fn prepare_scars(
        &mut self,
        device: &Device,
        presentation: &mut Presentation,
        grass: Option<[f64; 4]>,
        size: [u32; 2],
    ) {
        if self
            .scars
            .prepare(device, presentation, grass, self.map_box.as_ref(), size)
        {
            self.group = terrain_group(
                device,
                &self.layout,
                &self.tables,
                &self.scars,
                &self.water_normals,
            );
        }
    }

    /// The group by the layout it fills.
    pub fn layout_group(&self) -> (&'static str, &BindGroup) {
        ("terrainLayout", &self.group)
    }

    /// The ground, the skirt and the backdrop into the depth prepass.
    pub fn draw_depth(&self, pass: &mut RenderPass, camera: (&str, &BindGroup)) {
        self.prepass.set(pass, &[camera]);
        self.terrain.draw(pass, &self.identity, None);
        for mesh in [&self.skirt, &self.backdrop] {
            mesh.draw(pass, &self.identity, None);
        }
    }

    /// The ground and the skirt into one cascade (the backdrop casts nothing).
    pub fn draw_casters(&self, pass: &mut RenderPass, camera: (&str, &BindGroup), cascade: usize) {
        self.caster.set(pass, &[camera]);
        self.terrain.draw(pass, &self.identity, Some(cascade));
        self.skirt.draw(pass, &self.identity, Some(cascade));
    }

    /// The lit terrain, `groups` the frame's (camera, environment, fog).
    pub fn draw_terrain(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_terrain.set(pass, &all);
        self.terrain.draw(pass, &self.identity, None);
    }

    /// The map's skirt, lit as the world's faces.
    pub fn draw_skirt(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        self.lit_skirt.set(pass, groups);
        self.skirt.draw(pass, &self.identity, None);
    }

    /// The backdrop: the ground's material past the map, never shadowed.
    pub fn draw_backdrop(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_backdrop.set(pass, &all);
        self.backdrop.draw(pass, &self.identity, None);
    }

    /// The water's surface over the rivers' cells: blended over everything
    /// opaque and glass (`world::Water`), reading depth and writing none.
    pub fn draw_water(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_water.set(pass, &all);
        self.water.draw(pass, &self.identity, None);
    }
}

#[cfg(test)]
#[path = "ground_range_tests.rs"]
mod range_tests;
