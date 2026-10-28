//! The native client: the browser's own shaders and passes on `wgpu`, drawing
//! what the presentation crate's `Frame` lists.
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
#[cfg(not(target_arch = "wasm32"))]
pub mod bench;
pub mod bundle;
#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
#[cfg(not(target_arch = "wasm32"))]
pub mod counts;
pub mod effects;
pub mod gpu;
pub mod pipelines;
pub mod readback;

pub use renderer_core::{environment, fog, grass, ground, models, scars, scenery, sky, statics};
