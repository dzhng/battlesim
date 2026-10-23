//! Spatial terrain draws shared with the native renderer.
use presentation::static_world::ground::partition::TerrainPartition as Inner;
use std::ops::Range;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TerrainPartition {
    inner: Inner,
    ranges: Vec<Range<u32>>,
    visited: usize,
}

#[wasm_bindgen]
impl TerrainPartition {
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: &[f32]) -> Self {
        Self {
            inner: Inner::new(
                vertices,
                presentation::static_world::ground::partition::CELL_M,
            ),
            ranges: vec![],
            visited: 0,
        }
    }

    pub fn vertices(&self) -> Vec<f32> {
        self.inner.vertices().to_vec()
    }

    /// Pairs of first vertex and vertex count, in buffer order.
    pub fn select(&mut self, matrix: &[f32]) -> Vec<u32> {
        self.visited = self.inner.select(
            matrix.try_into().expect("view projection has 16 words"),
            &mut self.ranges,
        );
        self.ranges
            .iter()
            .flat_map(|r| [r.start, r.end - r.start])
            .collect()
    }

    pub fn visited(&self) -> usize {
        self.visited
    }
}
