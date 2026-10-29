//! The native client: the browser's own shaders and passes on `wgpu`, drawing
//! what the presentation crate's `Frame` lists.
pub mod audio;
pub mod bench;
pub mod bundle;
pub mod capture;
pub mod counts;
pub mod effects;
pub mod environment;
pub mod fog;
pub mod fog_mask;
pub mod frame;
pub mod gpu;
pub mod grass;
pub mod ground;
pub mod models;
pub mod pipelines;
pub mod post;
pub mod scars;
pub mod scenery;
pub mod shadow;
pub mod sky;
pub mod statics;
pub mod water_normals;
pub mod world;
