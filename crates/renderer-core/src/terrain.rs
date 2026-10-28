//! Terrain material tables and the presentation's bounded scar cache.
use wgpu::*;

use crate::pipelines::Pipelines;
use crate::scars::Scars;
use crate::{bind, buffer};

fn table(device: &Device, label: &str, usage: BufferUsages, bytes: &[u8], record: usize) -> Buffer {
    let mut data = bytes.to_vec();
    data.resize(data.len().max(record), 0);
    buffer(device, label, usage, &data)
}

/// The presentation-owned table bytes consumed by `terrainLayout`.
#[derive(Clone, Copy, Default)]
pub struct TerrainUpload<'a> {
    pub params: &'a [u8],
    pub nodes: &'a [u8],
    pub plots: &'a [u8],
    pub surfaces: &'a [u8],
    pub surface_index: &'a [u8],
    /// The biome look suffix of `ScarParams`; the scar cache owns its prefix.
    pub scars: &'a [u8],
}

/// One owner for terrain tables, scar textures and their shared binding order.
pub struct TerrainResources {
    group: BindGroup,
    layout: BindGroupLayout,
    tables: [Buffer; 5],
    pub scars: Scars,
}

fn tables(device: &Device, upload: TerrainUpload<'_>) -> [Buffer; 5] {
    let storage = BufferUsages::STORAGE;
    [
        table(
            device,
            "terrain-params",
            BufferUsages::UNIFORM,
            upload.params,
            16,
        ),
        table(device, "terrain-nodes", storage, upload.nodes, 32),
        table(device, "terrain-plots", storage, upload.plots, 96),
        table(device, "terrain-surfaces", storage, upload.surfaces, 32),
        table(device, "terrain-index", storage, upload.surface_index, 4),
    ]
}

impl TerrainResources {
    pub fn from_upload(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        upload: TerrainUpload<'_>,
    ) -> Self {
        let tables = tables(device, upload);
        let scars = Scars::new(device, queue, bytemuck::cast_slice(upload.scars));
        let layout = pipelines.layout("terrainLayout").clone();
        let group = Self::bind(device, &layout, &tables, &scars);
        Self {
            group,
            layout,
            tables,
            scars,
        }
    }

    fn bind(
        device: &Device,
        layout: &BindGroupLayout,
        tables: &[Buffer; 5],
        scars: &Scars,
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
            ],
        )
    }

    /// A scar directory resize replaces its texture and invalidates this group.
    pub fn rebind_scars(&mut self, device: &Device) {
        self.group = Self::bind(device, &self.layout, &self.tables, &self.scars);
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.group
    }
}

impl TerrainResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for table in &self.tables {
            memory.buffer(table);
        }
        self.scars.record_memory(memory);
    }
}
