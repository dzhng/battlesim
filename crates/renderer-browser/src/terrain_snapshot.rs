#[derive(Clone)]
pub(super) struct TerrainSnapshot {
    pub(super) params: Vec<u32>,
    pub(super) nodes: Vec<u32>,
    pub(super) plots: Vec<u32>,
    pub(super) surfaces: Vec<f32>,
    pub(super) surface_index: Vec<u32>,
    pub(super) scars: Vec<u32>,
}
