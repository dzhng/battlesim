//! Owned external world geometry, independent of its platform transport.
use presentation::shadow::map_box;
use presentation::static_world::ground::mesh;

/// World vertices use the canonical ten-float layout. Geometry is retained for
/// device recreation; selection and GPU allocation belong to the runtime.
#[derive(Clone, Default)]
pub struct WorldGeometry {
    pub terrain: Vec<f32>,
    pub skirt: Vec<f32>,
    pub backdrop: Vec<f32>,
    pub water: Vec<f32>,
}

impl super::Runtime {
    /// Keep recovery bytes and the drawn geometry under the same lifetime.
    pub fn set_world_props(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, words: Vec<f32>) {
        self.ground.set_skirt(device, queue, &words);
        std::sync::Arc::make_mut(&mut self.geometry).skirt = words;
    }
}

impl WorldGeometry {
    /// Native physical triangles become the same meshes a browser may provide
    /// directly (including lab-specific disconnected decorative surrounds).
    pub fn from_triangles(
        positions: &[f32],
        indices: &[u32],
        water_cells: &[f32],
        backdrop_m: f64,
    ) -> Self {
        let terrain = mesh::terrain(positions, indices, None);
        let backdrop = map_box(&terrain).map_or_else(Vec::new, |b| mesh::backdrop(&b, backdrop_m));
        Self {
            terrain,
            skirt: mesh::skirt(positions),
            backdrop,
            water: mesh::water(water_cells),
        }
    }

    pub fn payload_bytes(&self) -> usize {
        [&self.terrain, &self.skirt, &self.backdrop, &self.water]
            .into_iter()
            .map(|words| words.len() * 4)
            .sum()
    }
}
