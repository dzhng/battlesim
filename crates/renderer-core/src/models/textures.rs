//! Content-addressed model texture layers and unchanged authored mip uploads.
use crate::bundle::Texture as SourceTexture;
use crate::runtime::AdmissionWork;
use crate::texture_array::{ArrayShape, LayerArray, LayerPressure, SharedArray};
use std::collections::BTreeMap;
use std::sync::Arc;
use wgpu::*;

/// The installed textures, one layer each in two arrays:
/// albedo (sRGB), and surface (normal and ORM, linear).
#[derive(Default)]
pub(super) struct TextureStore {
    albedo: LayerArray<String>,
    surface: LayerArray<String>,
}

impl TextureStore {
    pub fn pressures(&self, wanted: &BTreeMap<String, Arc<SourceTexture>>) -> [LayerPressure; 2] {
        [(true, &self.albedo), (false, &self.surface)].map(|(srgb, array)| {
            let ids = wanted
                .values()
                .filter(|t| t.srgb == srgb)
                .map(|t| t.id.clone())
                .collect();
            array.pressure(&ids)
        })
    }

    pub fn admit(
        &mut self,
        device: &Device,
        queue: &Queue,
        wanted: &BTreeMap<String, Arc<SourceTexture>>,
        spare: u32,
        report: fn(&str),
        work: &mut AdmissionWork,
    ) -> [Arc<SharedArray>; 2] {
        [(true, &mut self.albedo), (false, &mut self.surface)].map(|(srgb, array)| {
            let textures: Vec<_> = wanted.values().filter(|t| t.srgb == srgb).collect();
            let ids = textures.iter().map(|t| t.id.clone()).collect();
            let size = textures.iter().map(|t| t.size).max().unwrap_or(1);
            let shape = ArrayShape {
                size: [size, size],
                levels: size.ilog2() + 1,
                formats: vec![if srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                }],
            };
            let admitted = array.admit(
                device,
                queue,
                &ids,
                &shape,
                spare,
                if srgb {
                    "model-albedo"
                } else {
                    "model-surface"
                },
                report,
            );
            work.texture_bytes_copied += admitted.copied_bytes;
            work.texture_arrays_rebuilt += u32::from(admitted.rebuilt);
            for texture in textures {
                if !array.written(&admitted.owner, &texture.id) {
                    write_layer(
                        queue,
                        admitted.owner.texture(),
                        array.layer(&texture.id),
                        texture,
                        work,
                    );
                    array.mark_written(&admitted.owner, texture.id.clone());
                }
            }
            admitted.owner
        })
    }
    pub fn layer(&self, texture: Option<&Arc<SourceTexture>>) -> f32 {
        let Some(texture) = texture else { return -1.0 };
        (if texture.srgb {
            &self.albedo
        } else {
            &self.surface
        })
        .layer(&texture.id) as f32
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        self.albedo.record_memory(memory);
        self.surface.record_memory(memory);
    }
}

/// One layer of the array texture, holding its source with every level:
/// the edge is the largest's, a smaller one upsampled into the finer levels.
fn write_layer(
    queue: &Queue,
    texture: &Texture,
    layer: u32,
    t: &SourceTexture,
    work: &mut AdmissionWork,
) {
    let size = texture.width();
    let levels = texture.mip_level_count();
    work.texture_layers_written += 1;
    let shift = (size / t.size).ilog2();
    for level in 0..levels {
        let edge = size >> level;
        let upsampled;
        let data = if level >= shift {
            &t.levels[(level - shift) as usize]
        } else {
            let (from, k) = (t.size as usize, t.size as f64 / f64::from(edge));
            let src = &t.levels[0];
            upsampled = (0..edge as usize * edge as usize)
                .flat_map(|i| {
                    let (y, x) = (i / edge as usize, i % edge as usize);
                    let s = ((y as f64 * k) as usize * from + (x as f64 * k) as usize) * 4;
                    src[s..s + 4].to_vec()
                })
                .collect::<Vec<u8>>();
            &upsampled
        };
        queue.write_texture(
            TexelCopyTextureInfo {
                texture,
                mip_level: level,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(edge * 4),
                rows_per_image: Some(edge),
            },
            Extent3d {
                width: edge,
                height: edge,
                depth_or_array_layers: 1,
            },
        );
        work.texture_bytes_uploaded += data.len() as u64;
    }
}
