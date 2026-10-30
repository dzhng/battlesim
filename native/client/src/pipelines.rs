//! Native filesystem shell for the shared pipeline constructor.

pub use renderer_core::pipelines::Pipelines;
use wgpu::Device;

type Result<T> = std::result::Result<T, String>;

pub use renderer_core::pipelines::{Bound, BoundCompute, Sizes};

/// Build the shared pipeline graph from the built-in catalog.
pub fn build(device: &Device) -> Result<Pipelines> {
    renderer_core::pipelines::build_builtin(device)
}
