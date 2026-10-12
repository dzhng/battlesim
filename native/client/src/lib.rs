//! The native client: the browser's own shaders and passes on `wgpu`, drawing
//! what the presentation crate's `Frame` lists (slice 25).
pub mod capture;
pub mod frame;
pub mod gpu;
pub mod pipelines;
pub mod post;
pub mod sky;
