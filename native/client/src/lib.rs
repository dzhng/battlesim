//! The native client: the browser's own shaders and passes on `wgpu`, drawing
//! what the presentation crate's `Frame` lists (slice 25).
pub mod audio;
pub mod bundle;
pub mod capture;
pub mod effects;
pub mod environment;
pub mod fog_mask;
pub mod frame;
pub mod gpu;
pub mod grass;
pub mod ground;
pub mod models;
pub mod pending;
pub mod pipelines;
pub mod post;
pub mod shadow;
pub mod sky;
pub mod world;
