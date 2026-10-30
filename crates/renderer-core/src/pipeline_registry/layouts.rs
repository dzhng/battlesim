mod visibility;
mod water;
use super::*;
pub fn all() -> Vec<LayoutSpec> {
    vec![
        layout(
            "skyBakeLayout",
            vec![entry(
                0,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            )],
        ),
        layout(
            "skyBackgroundLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    3,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "sampled",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
            ],
        ),
        layout(
            "filtering",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "cameraLayout",
            vec![entry(
                0,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            )],
        ),
        layout(
            "environment",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
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
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    4,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    5,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    6,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    7,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    8,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Comparison),
                ),
            ],
        ),
        visibility::sight(),
        layout(
            "terrainLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    3,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    4,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    5,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    6,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    7,
                    ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                // The water's baked normal map and the sampler that tiles it.
                entry(8, ShaderStages::FRAGMENT, float_texture()),
                entry(
                    9,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
            ],
        ),
        layout(
            "modelLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    4,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
            ],
        ),
        layout(
            "xrayReadLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "xrayCountLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                ),
            ],
        ),
        layout(
            "cardLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    3,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        visibility::horizon(),
        visibility::cull(),
        visibility::probe(),
        visibility::shape(),
        visibility::whole(),
        layout(
            "grassDrawLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    3,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "grassBuildLayout",
            vec![
                entry(
                    0,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    3,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    4,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    5,
                    ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        visibility::mask_rows(),
        visibility::mask_columns(),
        visibility::mask_compose(),
        layout(
            "overlaySource",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
            ],
        ),
        layout(
            "glowRowsLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
            ],
        ),
        layout(
            "glowColumnsLayout",
            vec![
                entry(
                    0,
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
            ],
        ),
        layout(
            "effects",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    4,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    5,
                    ShaderStages::VERTEX,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    6,
                    ShaderStages::VERTEX,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "highpassLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "blurLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "compositeLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    4,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    5,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    6,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
        layout(
            "finalLayout",
            vec![
                entry(
                    0,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Sampler(SamplerBindingType::Filtering),
                ),
                entry(
                    1,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        ),
    ]
    .into_iter()
    .chain(water::all())
    .collect()
}
pub fn layout(name: &'static str, entries: Vec<BindGroupLayoutEntry>) -> LayoutSpec {
    LayoutSpec { name, entries }
}
pub fn entry(binding: u32, visibility: ShaderStages, ty: BindingType) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty,
        count: None,
    }
}

/// A filterable float 2D texture.
pub fn float_texture() -> BindingType {
    BindingType::Texture {
        sample_type: TextureSampleType::Float { filterable: true },
        view_dimension: TextureViewDimension::D2,
        multisampled: false,
    }
}
