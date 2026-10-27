use renderer_core::environment::EnvironmentBindings;
use renderer_core::pipelines::Pipelines;
use renderer_core::shadow::ShadowResources;
use wgpu::*;

/// Browser shell owner for the nine real resources consumed by the exported
/// environment bind group. Asset/sky generation can replace the sampled
/// textures later without changing the binding contract.
pub struct BrowserEnvironment {
    pub bindings: EnvironmentBindings,
    pub sky: Texture,
    pub pmrem: Texture,
    pub dfg: Texture,
    pub sampler: Sampler,
    pub cast_lights: Buffer,
}

impl BrowserEnvironment {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        presentation_environment: &Buffer,
        shadow: &ShadowResources,
    ) -> Self {
        let cast_lights = renderer_core::buffer(
            device,
            "shared-environment-cast-lights",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[0; 1024],
        );
        let sampled = |label: &str, format: TextureFormat| {
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
        };
        let sky = sampled("shared-environment-sky", TextureFormat::Rgba16Float);
        let pmrem = sampled("shared-environment-pmrem", TextureFormat::Rgba16Float);
        let dfg = sampled("shared-environment-dfg", TextureFormat::Rg16Float);
        let rgba16 = bytemuck::cast_slice(&[0x3c00u16, 0x3c00, 0x3c00, 0x3c00]);
        for texture in [&sky, &pmrem] {
            queue.write_texture(
                texture.as_image_copy(),
                rgba16,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(1),
                },
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.write_texture(
            dfg.as_image_copy(),
            bytemuck::cast_slice(&[0x3800u16, 0x3800]),
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
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("shared-environment-sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let bindings = EnvironmentBindings::new(
            device,
            pipelines.layout("environment"),
            &[
                presentation_environment.as_entire_binding(),
                cast_lights.as_entire_binding(),
                BindingResource::TextureView(&sky.create_view(&Default::default())),
                BindingResource::TextureView(&pmrem.create_view(&Default::default())),
                BindingResource::TextureView(&dfg.create_view(&Default::default())),
                BindingResource::Sampler(&sampler),
                shadow.receiver.as_entire_binding(),
                BindingResource::TextureView(&shadow.depth),
                BindingResource::Sampler(&shadow.compare),
            ],
        );
        Self {
            bindings,
            sky,
            pmrem,
            dfg,
            sampler,
            cast_lights,
        }
    }
}
