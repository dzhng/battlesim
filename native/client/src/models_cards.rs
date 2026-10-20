//! GPU upload of prepared cards for the existing modelCards pipeline.
use crate::{gpu, pipelines::Pipelines};
use presentation::assets::cards::{Card, HEIGHT, WIDTH};
use wgpu::*;

pub fn upload(device: &Device, queue: &Queue, pipelines: &Pipelines, cards: &[Card]) -> BindGroup {
    let texture = |normal: bool| {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("model-card-atlas"),
            size: Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: cards.len().max(1) as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer, card) in cards.iter().enumerate() {
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: TextureAspect::All,
                },
                if normal { &card.normal } else { &card.albedo },
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(WIDTH * 4),
                    rows_per_image: Some(HEIGHT),
                },
                Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
        }
        texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        })
    };
    let albedo = texture(false);
    let normal = texture(true);
    let frames: Vec<[f32; 4]> = if cards.is_empty() {
        vec![[0.0, 0.0, 0.0, 1.0]]
    } else {
        cards.iter().map(|c| c.frame).collect()
    };
    let frames = gpu::buffer(
        device,
        "card-frames",
        BufferUsages::STORAGE,
        bytemuck::cast_slice(&frames),
    );
    let sampler = device.create_sampler(&SamplerDescriptor {
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..Default::default()
    });
    gpu::bind(
        device,
        pipelines.layout("cardLayout"),
        "cards",
        &[
            BindingResource::TextureView(&albedo),
            BindingResource::TextureView(&normal),
            BindingResource::Sampler(&sampler),
            frames.as_entire_binding(),
        ],
    )
}

/// Patch only the renderer-owned layer word on changed static view batches.
pub fn card_records<'a>(
    records: &'a [presentation::records::Instance],
    batch: &presentation::records::Batch,
    layers: &[Option<u32>],
) -> std::borrow::Cow<'a, [presentation::records::Instance]> {
    use presentation::{records::VIEW_PASS, view::IMPOSTOR};
    if batch.pass == VIEW_PASS && batch.tier == IMPOSTOR as u32 {
        if let Some(layer) = layers.get(batch.appearance as usize).copied().flatten() {
            return records
                .iter()
                .map(|r| {
                    let mut r = *r;
                    r.tint[3] = layer as f32;
                    r
                })
                .collect::<Vec<_>>()
                .into();
        }
    }
    records.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use presentation::records::{Batch, Instance, CAST_PASS, VIEW_PASS};
    #[test]
    fn card_upload_preserves_nonuniform_placement_and_mesh_casters() {
        let records = [Instance {
            placement: [7.0, -2.0, 3.0, 1.2],
            data: [0.0; 4],
            tint: [0.2, 0.3, 0.4, -1.0],
            scale: [2.0, 0.5, 3.0, 0.0],
        }];
        let mut batch = Batch {
            tier: presentation::view::IMPOSTOR as u32,
            pass: VIEW_PASS,
            appearance: 0,
            ..Default::default()
        };
        let prepared = card_records(&records, &batch, &[Some(3)]);
        assert_eq!(prepared[0].placement, records[0].placement);
        assert_eq!(prepared[0].scale, records[0].scale);
        assert_eq!(prepared[0].tint, [0.2, 0.3, 0.4, 3.0]);
        assert_eq!(records[0].tint[3], -1.0);
        assert_eq!(
            card_records(&records, &batch, &[None])[0].tint,
            records[0].tint
        );
        batch.pass = CAST_PASS;
        assert_eq!(
            card_records(&records, &batch, &[Some(3)])[0].tint,
            records[0].tint
        );
        batch.pass = VIEW_PASS;
        batch.tier = 3;
        assert_eq!(
            card_records(&records, &batch, &[Some(3)])[0].tint,
            records[0].tint
        );
    }
}
