//! The water's own passes' resource layouts.
use super::*;

/// The world copy's mips, the guard seeded from the
/// fog mask and paint and reduced by mips, and the wave field's step.
pub(super) fn all() -> Vec<LayoutSpec> {
    vec![
        layout(
            "sceneMipLayout",
            vec![
                entry(0, ShaderStages::FRAGMENT, float_texture()),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
            ],
        ),
        layout(
            "guardSeedLayout",
            vec![
                entry(0, ShaderStages::FRAGMENT, float_texture()),
                entry(1, ShaderStages::FRAGMENT, float_texture()),
            ],
        ),
        layout(
            "guardMipLayout",
            vec![entry(0, ShaderStages::FRAGMENT, float_texture())],
        ),
        layout(
            "waveLayout",
            vec![
                entry(0, ShaderStages::FRAGMENT, float_texture()),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
    ]
}
