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
pub mod environment;
pub mod fog;
pub mod fog_mask;
pub mod frame;
pub mod gpu;
pub mod grass;
pub mod ground;
pub mod pipelines;
pub mod post;
pub mod scars;
pub mod shadow;
pub mod sky;
pub mod world;

pub use renderer_core::{models, scenery, statics};
