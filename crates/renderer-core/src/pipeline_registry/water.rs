//! The water's own passes: its world copy's mips, its guard, its normal map
//! and its wave field.
use super::*;
pub fn all() -> Vec<PipelineSpec> {
    let full_screen = |name, shader, layout: Vec<&'static str>, format| {
        render(
            name,
            shader,
            layout,
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(format)])),
        )
    };
    vec![
        full_screen(
            "sceneColourMip",
            "sceneColourMip.wgsl",
            vec!["sceneMipLayout"],
            TextureFormat::Rgba16Float,
        ),
        full_screen(
            "waterGuardSeed",
            "waterGuardSeed.wgsl",
            vec!["guardSeedLayout"],
            TextureFormat::Rg8Unorm,
        ),
        full_screen(
            "waterGuardMip",
            "waterGuardMip.wgsl",
            vec!["guardMipLayout"],
            TextureFormat::Rg8Unorm,
        ),
        full_screen(
            "waterNormalsBake",
            "waterNormalsBake.wgsl",
            vec![],
            TextureFormat::Rgba16Float,
        ),
        full_screen(
            "waveStep",
            "waveStep.wgsl",
            vec!["waveLayout"],
            TextureFormat::Rgba16Float,
        ),
    ]
}
