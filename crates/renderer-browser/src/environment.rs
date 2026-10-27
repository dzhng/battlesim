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

const SKY_LUT: [u32; 2] = [384, 192];
const PMREM: [u32; 2] = [336, 256];
const DFG: [u32; 2] = [16, 16];

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
        let sampled = |label: &str, format: TextureFormat, size: [u32; 2]| {
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
        };
        let sky = sampled(
            "shared-environment-sky",
            TextureFormat::Rgba16Float,
            SKY_LUT,
        );
        let pmrem = sampled(
            "shared-environment-pmrem",
            TextureFormat::Rgba16Float,
            PMREM,
        );
        let dfg = sampled("shared-environment-dfg", TextureFormat::Rg16Float, DFG);
        let sky_words = vec![0x3c00u16; (SKY_LUT[0] * SKY_LUT[1] * 4) as usize];
        let pmrem_words = vec![0x3c00u16; (PMREM[0] * PMREM[1] * 4) as usize];
        let sky_bytes = bytemuck::cast_slice(&sky_words);
        let pmrem_bytes = bytemuck::cast_slice(&pmrem_words);
        for (texture, bytes, size, row_bytes) in [
            (&sky, sky_bytes, SKY_LUT, SKY_LUT[0] * 8),
            (&pmrem, pmrem_bytes, PMREM, PMREM[0] * 8),
        ] {
            queue.write_texture(
                texture.as_image_copy(),
                bytes,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(size[1]),
                },
                Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.write_texture(
            dfg.as_image_copy(),
            bytemuck::cast_slice(&vec![0x3800u16; (DFG[0] * DFG[1] * 2) as usize]),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(DFG[0] * 4),
                rows_per_image: Some(DFG[1]),
            },
            Extent3d {
                width: DFG[0],
                height: DFG[1],
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
