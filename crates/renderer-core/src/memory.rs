//! Allocation sizes of the buffers and textures held by renderer owners.
//! Counts describe owned GPU payloads, excluding driver, pipeline and surface
//! storage. Temporary handles only live during a snapshot; no registry extends
//! a resource's lifetime or prevents WebGPU garbage collection.
use std::collections::HashSet;
use wgpu::*;

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationCounts {
    pub buffers: usize,
    pub textures: usize,
    pub buffer_bytes: u64,
    pub texture_bytes: u64,
}

#[derive(Default)]
pub struct ResourceMemory {
    buffers: HashSet<Buffer>,
    textures: HashSet<Texture>,
}

impl ResourceMemory {
    pub fn buffer(&mut self, buffer: &Buffer) {
        self.buffers.insert(buffer.clone());
    }

    pub fn texture(&mut self, texture: &Texture) {
        self.textures.insert(texture.clone());
    }

    pub fn view(&mut self, view: &TextureView) {
        self.texture(view.texture());
    }

    pub fn counts(&self) -> AllocationCounts {
        AllocationCounts {
            buffers: self.buffers.len(),
            textures: self.textures.len(),
            buffer_bytes: self.buffers.iter().map(Buffer::size).sum(),
            texture_bytes: self.textures.iter().map(texture_bytes).sum(),
        }
    }
}

fn texture_bytes(texture: &Texture) -> u64 {
    let size = texture.size();
    let (bw, bh) = texture.format().block_dimensions();
    let block_bytes = texture
        .format()
        .block_copy_size(None)
        .expect("renderer textures have a definite texel block size");
    (0..texture.mip_level_count())
        .map(|level| {
            let w = (size.width >> level).max(1).div_ceil(bw);
            let h = (size.height >> level).max(1).div_ceil(bh);
            let d = if texture.dimension() == TextureDimension::D3 {
                (size.depth_or_array_layers >> level).max(1)
            } else {
                size.depth_or_array_layers
            };
            u64::from(w)
                * u64::from(h)
                * u64::from(d)
                * u64::from(block_bytes)
                * u64::from(texture.sample_count())
        })
        .sum()
}
