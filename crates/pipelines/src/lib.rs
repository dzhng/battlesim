//! The browser's shaders and pipeline settings, exported for a native client.
//!
//! `web/tests/shaderExport.test.ts` writes them from the browser's own code
//! into `packages/battle-renderer/pipelines/`: one WGSL file per pipeline, and
//! `pipelines.json` with each pipeline's stages, targets, depth, multisampling,
//! vertex buffers and bind group layouts. This crate's tests hold that export
//! to what `wgpu` accepts.

/// The export's directory.
pub const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/battle-renderer/pipelines"
);
