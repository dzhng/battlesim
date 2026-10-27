//! Shared indexed model geometry ownership.
//!
//! The browser and native shells may decode different asset containers, but
//! once a mesh is reduced to positions, vertex colours and indices this owner
//! performs the same GPU expansion, upload and draw operation in both targets.

use bytemuck::{Pod, Zeroable};
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub colour: [f32; 4],
}

/// A shell-independent triangle-list model. Materials and skinning remain
/// higher-level adapters; this owner is the first shared model resource seam.
pub struct Model {
    vertex: Buffer,
    vertex_count: u32,
}

impl Model {
    pub fn from_indexed(
        device: &Device,
        label: &str,
        positions: &[f32],
        colours: &[u8],
        indices: &[u32],
    ) -> Result<Self, String> {
        Self::upload(
            device,
            label,
            Self::indexed_vertices(positions, colours, indices)?,
        )
    }

    /// Fixture-only helper that fits a mesh into a clip-space inspection view.
    pub fn from_indexed_fitted(
        device: &Device,
        label: &str,
        positions: &[f32],
        colours: &[u8],
        indices: &[u32],
    ) -> Result<Self, String> {
        let mut vertices = Self::indexed_vertices(positions, colours, indices)?;
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for vertex in &vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }
        let center: [f32; 3] = std::array::from_fn(|axis| (min[axis] + max[axis]) * 0.5);
        let extent = (0..3)
            .map(|axis| max[axis] - min[axis])
            .fold(0.0f32, f32::max)
            .max(1e-6);
        let scale = 1.6 / extent;
        for vertex in &mut vertices {
            vertex.position =
                std::array::from_fn(|axis| (vertex.position[axis] - center[axis]) * scale);
        }
        Self::upload(device, label, vertices)
    }

    fn indexed_vertices(
        positions: &[f32],
        colours: &[u8],
        indices: &[u32],
    ) -> Result<Vec<Vertex>, String> {
        if !positions.len().is_multiple_of(3) || colours.len() != positions.len() / 3 * 4 {
            return Err("model vertex arrays have different lengths".into());
        }
        if indices
            .iter()
            .any(|&index| index as usize >= positions.len() / 3)
        {
            return Err("model index is outside the vertex array".into());
        }
        let vertices = indices
            .iter()
            .map(|&index| {
                let vertex = index as usize;
                Vertex {
                    position: positions[vertex * 3..vertex * 3 + 3]
                        .try_into()
                        .expect("validated position width"),
                    colour: std::array::from_fn(|channel| {
                        f32::from(colours[vertex * 4 + channel]) / 255.0
                    }),
                }
            })
            .collect::<Vec<_>>();
        if vertices.is_empty() {
            Err("model has no indexed triangles".into())
        } else {
            Ok(vertices)
        }
    }

    fn upload(device: &Device, label: &str, vertices: Vec<Vertex>) -> Result<Self, String> {
        let vertex = crate::buffer(
            device,
            label,
            BufferUsages::VERTEX,
            bytemuck::cast_slice(&vertices),
        );
        Ok(Self {
            vertex,
            vertex_count: vertices.len() as u32,
        })
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn vertex(&self) -> &Buffer {
        &self.vertex
    }
}
