//! Terrain material tables and the presentation's bounded scar cache.
use wgpu::*;

use crate::pipelines::Pipelines;
use crate::scars::Scars;
use crate::water::{WaterDetail, WaterNormals};
use crate::{bind, buffer};
use presentation::records::TerrainParams;
use presentation::static_world::ground::Ground;

fn table(
    device: &Device,
    queue: &Queue,
    label: &str,
    usage: BufferUsages,
    bytes: &[u8],
    record: usize,
) -> Buffer {
    let mut data = bytes.to_vec();
    data.resize(data.len().max(record), 0);
    buffer(device, queue, label, usage, &data)
}

/// The presentation-owned table bytes consumed by `terrainLayout`.
#[derive(Clone, Copy, Default)]
pub struct TerrainUpload<'a> {
    /// Canonical params without the renderer-owned final view vec4u.
    pub params: &'a [u8],
    pub nodes: &'a [u8],
    pub plots: &'a [u8],
    pub surfaces: &'a [u8],
    pub surface_index: &'a [u8],
    /// The biome look suffix of `ScarParams`; the scar cache owns its prefix.
    pub scars: &'a [u8],
}

/// One owner for terrain tables, scar textures, the water's normal map and
/// their shared binding order.
pub struct TerrainResources {
    group: BindGroup,
    layout: BindGroupLayout,
    tables: [Buffer; 5],
    pub scars: Scars,
    water_normals: WaterNormals,
    class_view: bool,
    road_wear: bool,
    field_texture: bool,
    water_detail: WaterDetail,
}

fn tables(device: &Device, queue: &Queue, upload: TerrainUpload<'_>) -> [Buffer; 5] {
    let storage = BufferUsages::STORAGE;
    // The canonical table excludes the renderer's final view vec4u.
    let mut params = upload.params.to_vec();
    params.resize(TerrainParams::LAYOUT.size, 0);
    [
        table(
            device,
            queue,
            "terrain-params",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &params,
            16,
        ),
        table(device, queue, "terrain-nodes", storage, upload.nodes, 32),
        table(
            device,
            queue,
            "terrain-plots",
            storage | BufferUsages::COPY_DST,
            upload.plots,
            96,
        ),
        table(
            device,
            queue,
            "terrain-surfaces",
            storage,
            upload.surfaces,
            32,
        ),
        table(
            device,
            queue,
            "terrain-index",
            storage,
            upload.surface_index,
            4,
        ),
    ]
}

impl TerrainResources {
    pub fn from_upload(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        upload: TerrainUpload<'_>,
    ) -> Self {
        let tables = tables(device, queue, upload);
        let scars = Scars::new(device, queue, bytemuck::cast_slice(upload.scars));
        let layout = pipelines.layout("terrainLayout").clone();
        let water_normals = WaterNormals::new(device, queue, pipelines);
        let group = Self::bind(device, &layout, &tables, &scars, &water_normals);
        Self {
            group,
            layout,
            tables,
            scars,
            water_normals,
            class_view: false,
            road_wear: true,
            field_texture: true,
            water_detail: WaterDetail::Full,
        }
    }

    fn bind(
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

    /// A scar directory resize replaces its texture and invalidates this group.
    pub fn rebind_scars(&mut self, device: &Device) {
        self.group = Self::bind(
            device,
            &self.layout,
            &self.tables,
            &self.scars,
            &self.water_normals,
        );
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.group
    }

    fn write_view(&self, queue: &Queue) {
        let words = [
            u32::from(self.class_view),
            u32::from(!self.road_wear),
            u32::from(self.water_detail == WaterDetail::Depth),
            0,
        ];
        queue.write_buffer(
            &self.tables[0],
            std::mem::offset_of!(TerrainParams, view) as u64,
            bytemuck::cast_slice(&words),
        );
    }

    pub fn set_class_view(&mut self, queue: &Queue, on: bool) -> bool {
        if self.class_view == on {
            return false;
        }
        self.class_view = on;
        self.write_view(queue);
        true
    }

    /// The water reads its bed refracted, or straight below at its pixel.
    pub fn set_water_detail(&mut self, queue: &Queue, detail: WaterDetail) {
        self.water_detail = detail;
        self.write_view(queue);
    }

    /// Disabling wear draws each road's flat colour to its authored edge.
    /// A changed shoulder material also requires the caller to regrow grass.
    pub fn set_road_wear(&mut self, queue: &Queue, on: bool) -> bool {
        if self.road_wear == on {
            return false;
        }
        self.road_wear = on;
        self.write_view(queue);
        true
    }

    /// Resolve plot rows through their canonical owner. Fine grain, broken
    /// rows and wheelings toggle together; their authored mean colour stays.
    pub fn set_field_texture(
        &mut self,
        queue: &Queue,
        ground: &Ground,
        on: bool,
    ) -> Result<bool, String> {
        if self.field_texture == on {
            return Ok(false);
        }
        let plots = ground.plot_records(on);
        let bytes = bytemuck::cast_slice(&plots);
        if bytes.len() as u64 != self.tables[2].size() {
            return Err("field texture update belongs to a different ground generation".into());
        }
        queue.write_buffer(&self.tables[2], 0, bytes);
        self.field_texture = on;
        Ok(true)
    }
}

impl TerrainResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for table in &self.tables {
            memory.buffer(table);
        }
        self.scars.record_memory(memory);
        self.water_normals.record_memory(memory);
    }
}
