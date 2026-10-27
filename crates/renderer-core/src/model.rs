//! Shared indexed model GPU ownership.
//!
//! Platform shells decode appearance bundles and provide packed vertex/index
//! bytes; this module owns the GPU lifetime and draw contract used by both
//! native and browser shells.
use wgpu::*;

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
        let mut bytes = Vec::with_capacity(positions.len() / 3 * 28);
        for i in 0..positions.len() / 3 {
            if i >= colours.len() / 4 {
                return Err("model fixture index out of range".into());
            }
            bytes.extend_from_slice(bytemuck::cast_slice(&positions[i * 3..i * 3 + 3]));
            bytes.extend_from_slice(&colours[i * 4..i * 4 + 4]);
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
    pub fn draw<'a>(&'a self, pass: &mut RenderPass<'a>) {
        self.indexed.draw(pass, 0);
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
