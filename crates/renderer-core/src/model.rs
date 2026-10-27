//! Shared indexed model GPU ownership.
//!
//! Platform shells decode appearance bundles and provide packed vertex/index
//! bytes; this module owns the GPU lifetime and draw contract used by both
//! native and browser shells.
use wgpu::*;

pub const BATCH_WORDS: usize = 12;
pub const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawableRun {
    pub appearance: u32,
    pub tier: u32,
    pub pass: u32,
    pub first: u32,
    pub count: u32,
    pub static_held: bool,
}

/// Decode presentation batches without inventing a second frame contract.
/// `first == NONE` is an explicit static batch held by the renderer; the
/// browser shell must retain its previous GPU range rather than read absent
/// instance words.
pub fn drawable_runs(words: &[u32]) -> Result<Vec<DrawableRun>, String> {
    if !words.len().is_multiple_of(BATCH_WORDS) {
        return Err("presentation batches are not 12-word records".into());
    }
    words
        .as_chunks::<BATCH_WORDS>()
        .0
        .iter()
        .map(|b| {
            Ok(DrawableRun {
                appearance: b[1],
                tier: b[2],
                pass: b[3],
                first: b[7],
                count: b[8],
                static_held: b[7] == NONE,
            })
        })
        .collect()
}

pub struct IndexedModel {
    pub vertices: Buffer,
    pub indices: Buffer,
    pub index_count: u32,
    pub vertex_bytes: u64,
    pub index_bytes: u64,
}

/// Packed non-indexed fixture model retained as a compatibility adapter while
/// callers move to `IndexedModel`.
pub struct Model {
    indexed: IndexedModel,
}

impl Model {
    pub fn from_indexed_fitted(
        device: &Device,
        queue: &Queue,
        label: &str,
        positions: &[f32],
        colours: &[u8],
        indices: &[u32],
    ) -> Result<Self, String> {
        if !positions.len().is_multiple_of(3) || colours.len() != positions.len() / 3 * 4 {
            return Err("model fixture attribute lengths differ".into());
        }
        // The browser fixture pipelines use the shared 28-byte stream:
        // position (vec3) followed by four normalized float colour channels.
        // Keep this compatibility constructor aligned with that layout; the
        // production authored model adapter owns its wider 48-byte stream.
        let mut bytes = Vec::with_capacity(positions.len() / 3 * 28);
        for i in 0..positions.len() / 3 {
            if i >= colours.len() / 4 {
                return Err("model fixture index out of range".into());
            }
            bytes.extend_from_slice(bytemuck::cast_slice(&positions[i * 3..i * 3 + 3]));
            for &channel in &colours[i * 4..i * 4 + 4] {
                bytes.extend_from_slice(&(channel as f32 / 255.0).to_ne_bytes());
            }
        }
        Ok(Self {
            indexed: IndexedModel::upload(
                device,
                queue,
                label,
                &bytes,
                bytemuck::cast_slice(indices),
                indices.len() as u32,
            ),
        })
    }
    pub fn vertex(&self) -> &Buffer {
        &self.indexed.vertices
    }
    pub fn vertex_count(&self) -> u32 {
        self.indexed.index_count
    }
    pub fn index(&self) -> &Buffer {
        &self.indexed.indices
    }
    pub fn index_count(&self) -> u32 {
        self.indexed.index_count
    }
}

impl IndexedModel {
    pub fn upload(
        device: &Device,
        queue: &Queue,
        label: &str,
        vertices: &[u8],
        indices: &[u8],
        index_count: u32,
    ) -> Self {
        let vertices_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("{label}-vertices")),
            size: vertices.len().max(1) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertices_buffer, 0, vertices);
        let indices_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("{label}-indices")),
            size: indices.len().max(1) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&indices_buffer, 0, indices);
        Self {
            vertices: vertices_buffer,
            indices: indices_buffer,
            index_count,
            vertex_bytes: vertices.len() as u64,
            index_bytes: indices.len() as u64,
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, vertex_slot: u32) {
        pass.set_vertex_buffer(vertex_slot, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{drawable_runs, NONE};
    #[test]
    fn separates_dynamic_and_held_static_ranges() {
        let mut words = vec![0; 24];
        words[1] = 4;
        words[2] = 1;
        words[3] = 0;
        words[7] = 2;
        words[8] = 3;
        words[13] = 5;
        words[14] = 2;
        words[15] = 0;
        words[19] = NONE;
        words[20] = 7;
        let runs = drawable_runs(&words).unwrap();
        assert_eq!(runs[0].count, 3);
        assert!(!runs[0].static_held);
        assert!(runs[1].static_held);
    }
    #[test]
    fn rejects_partial_batch() {
        assert!(drawable_runs(&[0; 11]).is_err());
    }
}
