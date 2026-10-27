//! Native filesystem shell for the shared pipeline constructor.

use std::fs;

pub use renderer_core::pipelines::Pipelines;
use wgpu::Device;

type Result<T> = std::result::Result<T, String>;

pub use renderer_core::pipelines::{Bound, BoundCompute, Sizes};

pub const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/battle-renderer/pipelines"
);

/// Build the shared pipeline graph from the native runtime export.
pub fn build(device: &Device, dir: &str) -> Result<Pipelines> {
    let text = fs::read_to_string(format!("{dir}/pipelines.json")).map_err(|e| e.to_string())?;
    Pipelines::build(device, &text, |_name, shader| {
        let source = fs::read_to_string(format!("{dir}/{shader}")).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        let source = native_shader_source(name, source);
        Ok(source)
    })
}

#[cfg(windows)]
fn native_shader_source(name: &str, source: String) -> String {
    if !name.ends_with("Cutout") {
        return source;
    }
    source
        .replace("-> @builtin(sample_mask) u32 {", " {")
        .replace(
            "return coveredSamples(coverage, fwidth(coverage), pixelNoise(clip.xy));",
            "if (coverage <= 0.5f) { discard; }",
        )
        .replace(
            "return u32((cutoutCoverage(_arg_0.material, _arg_0.uv) > pixelNoise(clip.xy)));",
            "if (!(cutoutCoverage(_arg_0.material, _arg_0.uv) > pixelNoise(clip.xy))) { discard; }",
        )
}

#[cfg(all(test, windows))]
mod tests {
    use super::native_shader_source;

    #[test]
    fn amd_cutout_shader_uses_discard_instead_of_sample_mask() {
        let source = "@fragment fn cut() -> @builtin(sample_mask) u32 { return coveredSamples(coverage, fwidth(coverage), pixelNoise(clip.xy)); } return u32((cutoutCoverage(_arg_0.material, _arg_0.uv) > pixelNoise(clip.xy)));";
        let adapted = native_shader_source("modelPrepassCutout", source.to_owned());
        assert!(!adapted.contains("sample_mask"));
        assert!(!adapted.contains("return coveredSamples"));
        assert!(!adapted.contains("return u32"));
        assert!(adapted.contains("discard"));
    }
}
