//! The presentation's ground (`Presentation::ground`): its terrain, skirt,
//! backdrop, tables and meshes uploaded once, drawn into
//! the depth prepass, the sun's cascades and the lit world. The terrain's
//! group (`terrainLayout`) is the grass build's too, and holds the side's
//! scars (`scars.rs`).
use std::ops::Range;

use presentation::records::Camera;
use presentation::shadow::{map_box, SunShadowFrame, CASCADES};
use presentation::static_world::ground::mesh::VERTEX_FLOATS;
use presentation::static_world::ground::partition::{TerrainPartition, CELL_M};
use wgpu::*;

use presentation::presentation::Presentation;

use crate::buffer;
use crate::pipelines::{Bound, Pipelines};
use crate::terrain::{TerrainResources, TerrainUpload};

/// The identity instance: no offset, unit scale, no tint.
const IDENTITY: [f32; 8] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0];

/// A mesh of world vertices.
struct Mesh {
    buffer: Option<Buffer>,
    partition: Option<std::sync::Arc<TerrainPartition>>,
    visited: usize,
    view: Vec<Range<u32>>,
    cascades: [Vec<Range<u32>>; CASCADES],
}

impl Mesh {
    fn new(device: &Device, queue: &Queue, label: &str, words: &[f32]) -> Self {
        Self {
            buffer: (!words.is_empty()).then(|| {
                buffer(
                    device,
                    queue,
                    label,
                    BufferUsages::VERTEX,
                    bytemuck::cast_slice(words),
                )
            }),
            partition: None,
            visited: 0,
            view: std::iter::once(0..(words.len() / VERTEX_FLOATS) as u32).collect(),
            cascades: std::array::from_fn(|_| Vec::new()),
        }
    }

    fn partitioned(device: &Device, queue: &Queue, label: &str, words: &[f32]) -> Self {
        let partition = std::sync::Arc::new(TerrainPartition::new(words, CELL_M));
        let mut mesh = Self::new(device, queue, label, partition.vertices());
        mesh.partition = Some(partition);
        mesh.view.clear();
        mesh
    }

    fn prepare(&mut self, camera: &Camera, casters: &[Camera]) {
        let Some(partition) = &self.partition else {
            return;
        };
        self.visited = partition.select(&camera.view_proj, &mut self.view);
        for (i, ranges) in self.cascades.iter_mut().enumerate() {
            if let Some(caster) = casters.get(i) {
                self.visited += partition.select(&caster.view_proj, ranges);
            } else {
                ranges.clear();
            }
        }
    }

    fn ranges(&self, cascade: Option<usize>) -> &[Range<u32>] {
        cascade.map_or(&self.view, |i| &self.cascades[i])
    }

    fn draw(
        &self,
        pass: &mut RenderPass,
        identity: &Buffer,
        cascade: Option<usize>,
    ) -> crate::submitted::SubmittedDraws {
        let mut submitted = crate::submitted::SubmittedDraws::default();
        let Some(buffer) = &self.buffer else {
            return submitted;
        };
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.set_vertex_buffer(1, identity.slice(..));
        for range in self.ranges(cascade) {
            pass.draw(range.clone(), 0..1);
            submitted.record(range.end - range.start, 1);
        }
        submitted
    }
}

/// Borrowed presentation tables and world meshes, uploaded once per world.
/// Meshes use the shared ten-float world vertex layout; partitioning and all
/// per-frame camera/cascade selection stay inside this owner.
pub struct GroundData<'a> {
    pub material: TerrainUpload<'a>,
    pub heights: &'a [u32],
    pub terrain: &'a [f32],
    pub skirt: &'a [f32],
    pub backdrop: &'a [f32],
    pub water: &'a [f32],
}

pub struct Ground {
    pub submissions: crate::submitted::Submissions,
    pub material: TerrainResources,
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
    /// Replace external world-face geometry without rebuilding terrain tables,
    /// targets or appearance resources. Selection is prepared on the next draw.
    pub fn set_skirt(&mut self, device: &Device, queue: &Queue, words: &[f32]) {
        self.skirt = Mesh::partitioned(device, queue, "skirt", words);
    }

    pub fn from_data(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        data: GroundData<'_>,
    ) -> Self {
        let material = TerrainResources::from_upload(device, queue, pipelines, data.material);
        let heights = buffer(
            device,
            queue,
            "terrain-heights",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(data.heights),
        );
        let map_box = map_box(data.terrain);
        // Coarse spatial batches avoid one draw per terrain cell while allowing
        // the camera and each shadow cascade to exclude distant ground.
        let terrain = Mesh::partitioned(device, queue, "terrain", data.terrain);
        let terrain_partition = terrain.partition.as_ref().unwrap().clone();
        let skirt = Mesh::partitioned(device, queue, "skirt", data.skirt);
        let water = Mesh::partitioned(device, queue, "water", data.water);
        Self {
            submissions: Default::default(),
            material,
            heights,
            map_box,
            terrain,
            terrain_partition,
            skirt,
            backdrop: Mesh::new(device, queue, "backdrop", data.backdrop),
            water,
            identity: buffer(
                device,
                queue,
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
        }
    }

    /// Select from the same camera records uploaded for the colour and shadow passes.
    pub fn prepare(&mut self, camera: &Camera, shadow: Option<&SunShadowFrame>) {
        let casters = shadow.map_or(&[][..], |fit| fit.casters.as_slice());
        self.prepare_records(camera, casters);
    }

    pub fn set_class_view(&mut self, queue: &Queue, on: bool) -> bool {
        self.material.set_class_view(queue, on)
    }

    pub fn set_road_wear(&mut self, queue: &Queue, on: bool) -> bool {
        self.material.set_road_wear(queue, on)
    }

    /// True when the changed material requires grass to regrow.
    pub fn set_field_texture(
        &mut self,
        queue: &Queue,
        presentation: &Presentation,
        on: bool,
    ) -> Result<bool, String> {
        let ground = presentation
            .ground()
            .ok_or("renderer world has no presentation ground")?;
        self.material.set_field_texture(queue, ground, on)
    }

    /// Select bounded ranges from the camera and cascade camera publications.
    pub fn prepare_records(&mut self, camera: &Camera, casters: &[Camera]) {
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
            .material
            .scars
            .prepare(device, presentation, grass, self.map_box.as_ref(), size)
        {
            self.material.rebind_scars(device);
        }
    }

    /// The group by the layout it fills.
    pub fn layout_group(&self) -> (&'static str, &BindGroup) {
        ("terrainLayout", self.material.bind_group())
    }

    /// The ground, the skirt and the backdrop into the depth prepass.
    pub fn draw_depth(&self, pass: &mut RenderPass, camera: (&str, &BindGroup)) {
        self.prepass.set(pass, &[camera]);
        self.submissions
            .record(crate::submitted::Pass::Depth, |stats| {
                stats
                    .terrain
                    .add(self.terrain.draw(pass, &self.identity, None));
                stats
                    .skirt_and_debug_props
                    .add(self.skirt.draw(pass, &self.identity, None));
                stats
                    .backdrop
                    .add(self.backdrop.draw(pass, &self.identity, None));
            });
    }

    /// The ground and the skirt into one cascade (the backdrop casts nothing).
    pub fn draw_casters(&self, pass: &mut RenderPass, camera: (&str, &BindGroup), cascade: usize) {
        self.caster.set(pass, &[camera]);
        self.submissions
            .record(crate::submitted::Pass::Cascade(cascade), |stats| {
                stats
                    .terrain
                    .add(self.terrain.draw(pass, &self.identity, Some(cascade)));
                stats.skirt_and_debug_props.add(self.skirt.draw(
                    pass,
                    &self.identity,
                    Some(cascade),
                ));
            });
    }

    /// The lit terrain, `groups` the frame's (camera, environment, fog).
    pub fn draw_terrain(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_terrain.set(pass, &all);
        self.submissions
            .record(crate::submitted::Pass::Color, |stats| {
                stats
                    .terrain
                    .add(self.terrain.draw(pass, &self.identity, None));
            });
    }

    /// The map's skirt, lit as the world's faces.
    pub fn draw_skirt(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        self.lit_skirt.set(pass, groups);
        self.submissions
            .record(crate::submitted::Pass::Color, |stats| {
                stats
                    .skirt_and_debug_props
                    .add(self.skirt.draw(pass, &self.identity, None));
            });
    }

    /// The backdrop: the ground's material past the map, never shadowed.
    pub fn draw_backdrop(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_backdrop.set(pass, &all);
        self.submissions
            .record(crate::submitted::Pass::Color, |stats| {
                stats
                    .backdrop
                    .add(self.backdrop.draw(pass, &self.identity, None));
            });
    }

    /// The water's surface over the rivers' cells: blended over everything
    /// opaque and glass, reading depth and writing none.
    pub fn draw_water(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        let mut all = groups.to_vec();
        all.push(self.layout_group());
        self.lit_water.set(pass, &all);
        self.submissions
            .record(crate::submitted::Pass::Color, |stats| {
                stats.water.add(self.water.draw(pass, &self.identity, None));
            });
    }
}

#[cfg(test)]
#[path = "ground_range_tests.rs"]
mod range_tests;

impl Ground {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.heights);
        memory.buffer(&self.identity);
        for mesh in [&self.terrain, &self.skirt, &self.backdrop, &self.water] {
            if let Some(buffer) = &mesh.buffer {
                memory.buffer(buffer);
            }
        }
        self.material.record_memory(memory);
    }
}

impl Ground {
    pub fn mesh_diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "terrainVisitedNodes": self.terrain.visited,
            "skirtVisitedNodes": self.skirt.visited,
            "waterVisitedNodes": self.water.visited,
            "worldVertices": ([&self.terrain, &self.skirt, &self.water, &self.backdrop].iter()
                .filter_map(|mesh| mesh.buffer.as_ref()).map(|buffer| buffer.size() / (VERTEX_FLOATS * 4) as u64).sum::<u64>()),
        })
    }
}
