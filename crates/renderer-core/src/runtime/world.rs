//! Owned external world geometry, independent of its platform transport.
use presentation::shadow::map_box;
use presentation::static_world::ground::mesh;

/// World vertices use the canonical ten-float layout. Geometry is retained for
/// device recreation; selection and GPU allocation belong to the runtime.
#[derive(Clone, Default)]
pub struct WorldGeometry {
    pub terrain: Vec<f32>,
    pub skirt_and_props: Vec<f32>,
    pub backdrop: Vec<f32>,
    pub water: Vec<f32>,
}

impl super::Runtime {
    /// Keep recovery bytes and the drawn geometry under the same lifetime.
    pub fn set_skirt_and_props(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        words: Vec<f32>,
    ) {
        self.ground.set_skirt_and_props(device, queue, &words);
        std::sync::Arc::make_mut(&mut self.geometry).skirt_and_props = words;
    }
}

impl WorldGeometry {
    /// Physical triangles become the shared world layers. Traversal supplies
    /// per-triangle tint and extra faces; surface views leave both empty.
    pub fn from_triangles(
        positions: &[f32],
        indices: &[u32],
        water_cells: &[f32],
        backdrop_m: f64,
        tint: Option<(&[u8], u8)>,
        extra_faces: &[f32],
        water: bool,
    ) -> Self {
        let terrain = mesh::terrain(positions, indices, tint);
        Self::from_mesh(
            terrain,
            positions,
            extra_faces,
            water_cells,
            backdrop_m,
            water,
        )
    }

    /// Labs may supply terrain faces directly, including a disconnected mesh
    /// whose backdrop still surrounds only the local patch.
    pub fn from_mesh(
        terrain: Vec<f32>,
        skirt_positions: &[f32],
        extra_faces: &[f32],
        water_cells: &[f32],
        backdrop_m: f64,
        water: bool,
    ) -> Self {
        let mut geometry = Self {
            terrain,
            skirt_and_props: Self::faces(skirt_positions, extra_faces),
            backdrop: Vec::new(),
            water: if water {
                mesh::water(water_cells)
            } else {
                Vec::new()
            },
        };
        geometry.set_backdrop(None, backdrop_m);
        geometry
    }

    /// Boundary skirt followed by the caller's additional world faces.
    pub fn faces(positions: &[f32], extra_faces: &[f32]) -> Vec<f32> {
        let mut faces = mesh::skirt(positions);
        faces.extend_from_slice(extra_faces);
        faces
    }

    pub fn set_backdrop(&mut self, terrain: Option<&[f32]>, reach_m: f64) {
        self.backdrop = map_box(terrain.unwrap_or(&self.terrain))
            .map_or_else(Vec::new, |b| mesh::backdrop(&b, reach_m));
    }

    pub fn payload_bytes(&self) -> usize {
        [
            &self.terrain,
            &self.skirt_and_props,
            &self.backdrop,
            &self.water,
        ]
        .into_iter()
        .map(|words| words.len() * 4)
        .sum()
    }
}
