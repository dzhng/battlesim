//! Shared terrain material table ownership.
//!
//! Presentation resolves the terrain material into packed tables.  The
//! native and browser shells only provide those bytes; this owner keeps the
//! `terrainLayout` binding order and the placeholder scar textures together
//! so the world pass cannot silently fall back to a second table graph.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::{bind, buffer};

fn storage(device: &Device, label: &str, bytes: &[u8]) -> Buffer {
    let mut data = bytes.to_vec();
    data.resize(data.len().max(16), 0);
    buffer(
        device,
        label,
        BufferUsages::STORAGE | BufferUsages::COPY_DST,
        &data,
    )
}

fn uniform(device: &Device, label: &str, bytes: &[u8]) -> Buffer {
    let mut data = bytes.to_vec();
    data.resize(data.len().max(16), 0);
    buffer(
        device,
        label,
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        &data,
    )
}

fn texture(device: &Device, label: &str, format: TextureFormat) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: 1,
            height: 1,
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

/// The presentation-owned table bytes consumed by `terrainLayout`.
pub struct TerrainUpload<'a> {
    pub params: &'a [u8],
    pub nodes: &'a [u8],
    pub plots: &'a [u8],
    pub surfaces: &'a [u8],
    pub surface_index: &'a [u8],
    /// The look suffix of `ScarParams`; the grid/cache prefix is supplied by
    /// the scar owner when that stream is wired.  Keeping the suffix here
    /// already removes the neutral terrain-material fallback.
    pub scars: &'a [u8],
}

/// GPU buffers and bind group for one resolved terrain material.
pub struct TerrainResources {
    group: BindGroup,
    params: Buffer,
    nodes: Buffer,
    plots: Buffer,
    surfaces: Buffer,
    surface_index: Buffer,
    scar_params: Buffer,
    scar_pages: Texture,
    scars: Texture,
}

impl TerrainResources {
    pub fn new(device: &Device, pipelines: &Pipelines) -> Self {
        let params = uniform(device, "terrain-params", &[]);
        let nodes = storage(device, "terrain-nodes", &[]);
        let plots = storage(device, "terrain-plots", &[]);
        let surfaces = storage(device, "terrain-surfaces", &[]);
        let surface_index = storage(device, "terrain-index", &[]);
        let scar_params = uniform(device, "terrain-scar-params", &[]);
        let scar_pages = texture(device, "terrain-scar-directory", TextureFormat::Rg32Uint);
        let scars = texture(device, "terrain-scar-pages", TextureFormat::Rgba8Unorm);
        let group = Self::bind(
            device,
            pipelines,
            &params,
            &nodes,
            &plots,
            &surfaces,
            &surface_index,
            &scar_params,
            &scar_pages,
            &scars,
        );
        Self {
            group,
            params,
            nodes,
            plots,
            surfaces,
            surface_index,
            scar_params,
            scar_pages,
            scars,
        }
    }

    #[allow(clippy::too_many_arguments)] // Bindings mirror the exported terrain layout one-for-one.
    fn bind(
        device: &Device,
        pipelines: &Pipelines,
        params: &Buffer,
        nodes: &Buffer,
        plots: &Buffer,
        surfaces: &Buffer,
        surface_index: &Buffer,
        scar_params: &Buffer,
        scar_pages: &Texture,
        scars: &Texture,
    ) -> BindGroup {
        bind(
            device,
            pipelines.layout("terrainLayout"),
            "shared-terrain-material",
            &[
                params.as_entire_binding(),
                nodes.as_entire_binding(),
                plots.as_entire_binding(),
                surfaces.as_entire_binding(),
                surface_index.as_entire_binding(),
                scar_params.as_entire_binding(),
                BindingResource::TextureView(&scar_pages.create_view(&Default::default())),
                BindingResource::TextureView(&scars.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..Default::default()
                })),
            ],
        )
    }

    /// Replace the table buffers with the resolved presentation records.
    /// Rebuilding at world-load boundaries keeps this owner simple and makes
    /// device-loss replay a pure CPU snapshot operation.
    pub fn upload(&mut self, device: &Device, pipelines: &Pipelines, upload: TerrainUpload<'_>) {
        self.params = uniform(device, "terrain-params", upload.params);
        self.nodes = storage(device, "terrain-nodes", upload.nodes);
        self.plots = storage(device, "terrain-plots", upload.plots);
        self.surfaces = storage(device, "terrain-surfaces", upload.surfaces);
        self.surface_index = storage(device, "terrain-index", upload.surface_index);
        let mut scar_words = vec![0u8; 20 * 4];
        scar_words.extend_from_slice(upload.scars);
        self.scar_params = uniform(device, "terrain-scar-params", &scar_words);
        self.group = Self::bind(
            device,
            pipelines,
            &self.params,
            &self.nodes,
            &self.plots,
            &self.surfaces,
            &self.surface_index,
            &self.scar_params,
            &self.scar_pages,
            &self.scars,
        );
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.group
    }
}
