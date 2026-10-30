//! Shared model material bind-group ownership.
//!
//! Model admission uploads material rows, palette storage, and texture
//! arrays. This owner keeps the shared `modelLayout` binding order and
//! rebuilds the group when a growable palette buffer is replaced.

use wgpu::*;

pub struct ModelMaterialBindings {
    group: BindGroup,
}

impl ModelMaterialBindings {
    pub fn new(
        device: &Device,
        layout: &BindGroupLayout,
        palette: &Buffer,
        materials: &Buffer,
        albedo: &TextureView,
        surface: &TextureView,
        sampler: &Sampler,
    ) -> Self {
        Self {
            group: bind(device, layout, palette, materials, albedo, surface, sampler),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rebuild(
        &mut self,
        device: &Device,
        layout: &BindGroupLayout,
        palette: &Buffer,
        materials: &Buffer,
        albedo: &TextureView,
        surface: &TextureView,
        sampler: &Sampler,
    ) {
        self.group = bind(device, layout, palette, materials, albedo, surface, sampler);
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.group
    }
}

fn bind(
    device: &Device,
    layout: &BindGroupLayout,
    palette: &Buffer,
    materials: &Buffer,
    albedo: &TextureView,
    surface: &TextureView,
    sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("models"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: palette.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: materials.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(albedo),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(surface),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    })
}
