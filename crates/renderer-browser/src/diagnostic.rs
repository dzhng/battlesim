use super::*;
use renderer_core::model_material::ModelMaterialBindings;

/// Seed the minimal valid palette/material records consumed by the opt-in
/// exported opaque model probe. Production asset loading can replace the
/// texture views while retaining these record buffers.
pub(super) fn initialize_material_probe(
    queue: &Queue,
    palette: &Buffer,
    rows: &Buffer,
    albedo: &Texture,
    surface: &Texture,
) {
    queue.write_buffer(
        palette,
        0,
        bytemuck::cast_slice(&[
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ]),
    );
    queue.write_buffer(
        rows,
        0,
        bytemuck::cast_slice(&[
            1.0f32, 1.0, 1.0, 1.0, 0.0, 0.8, 1.0, 1.0, -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ]),
    );
    for texture in [albedo, surface] {
        queue.write_texture(
            texture.as_image_copy(),
            &[255, 255, 255, 255],
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
}

pub(super) fn material_probe(
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
) -> (
    Buffer,
    Buffer,
    Texture,
    Texture,
    Sampler,
    ModelMaterialBindings,
) {
    let palette = renderer_core::buffer(
        device,
        "shared-material-palette",
        BufferUsages::STORAGE | BufferUsages::COPY_DST,
        &[0; 48],
    );
    let rows = renderer_core::buffer(
        device,
        "shared-material-rows",
        BufferUsages::STORAGE | BufferUsages::COPY_DST,
        &[0; 80],
    );
    let texture = |label| {
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
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let albedo = texture("shared-material-albedo");
    let surface = texture("shared-material-surface");
    let sampler = device.create_sampler(&SamplerDescriptor::default());
    initialize_material_probe(queue, &palette, &rows, &albedo, &surface);
    let bindings = ModelMaterialBindings::new(
        device,
        layout,
        &palette,
        &rows,
        &array_view(&albedo),
        &array_view(&surface),
        &sampler,
    );
    (palette, rows, albedo, surface, sampler, bindings)
}

#[wasm_bindgen]
impl Renderer {
    /// Enable the exported lit-model pass for a screenshot/validation probe.
    /// The normal shared fixture remains unchanged until this probe is ready.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn enable_model_opaque_diagnostic(&mut self) {
        self.model_opaque_diagnostic = true;
    }
}
