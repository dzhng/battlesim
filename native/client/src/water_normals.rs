//! The water's normal map (`frame/waterNormals.ts`), baked once by the
//! exported `waterNormalsBake` and bound with the terrain's tables, with the
//! sampler that tiles it.
use wgpu::*;

use crate::frame::HDR_FORMAT;
use crate::pipelines::Pipelines;

/// Texels a side, as the browser bakes it.
const SIZE: u32 = 256;

pub struct WaterNormals {
    pub view: TextureView,
    pub sampler: Sampler,
}

impl WaterNormals {
    /// The map, baked before this returns its view.
    pub fn new(device: &Device, queue: &Queue, pipelines: &Pipelines) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("water-normals"),
            size: Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("water-normals-bake"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipelines.render("waterNormalsBake"));
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("water-normals"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        Self { view, sampler }
    }
}
