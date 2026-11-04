//! Sight, horizon and screen-mask resource layouts.
use super::*;

pub(super) fn sight() -> LayoutSpec {
    layout(
        "fogLayout",
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
                    ty: BufferBindingType::Uniform,
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
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
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
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            entry(
                8,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Texture {
                    sample_type: TextureSampleType::Uint,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            // What lies under the water: the world's depth, its colour before
            // the water with its sampler, the guard, and the wave field.
            entry(
                9,
                ShaderStages::FRAGMENT,
                BindingType::Texture {
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: true,
                },
            ),
            entry(10, ShaderStages::FRAGMENT, float_texture()),
            entry(
                11,
                ShaderStages::FRAGMENT,
                BindingType::Sampler(SamplerBindingType::Filtering),
            ),
            entry(12, ShaderStages::FRAGMENT, float_texture()),
            entry(13, ShaderStages::FRAGMENT, float_texture()),
        ],
    )
}

pub(super) fn horizon() -> LayoutSpec {
    layout(
        "buildLayout",
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
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                2,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                3,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                4,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                5,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                6,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                7,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ],
    )
}

pub(super) fn cull() -> LayoutSpec {
    layout(
        "cullLayout",
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
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                2,
                ShaderStages::COMPUTE,
                BindingType::Texture {
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: true,
                },
            ),
            entry(
                3,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                4,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ],
    )
}

pub(super) fn probe() -> LayoutSpec {
    layout(
        "probeLayout",
        vec![
            entry(
                0,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                1,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ],
    )
}

pub(super) fn shape() -> LayoutSpec {
    layout(
        "shapeLayout",
        vec![
            entry(
                0,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                1,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ],
    )
}

pub(super) fn whole() -> LayoutSpec {
    layout(
        "wholeLayout",
        vec![
            entry(
                0,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                1,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                2,
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                3,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ],
    )
}

pub(super) fn mask_rows() -> LayoutSpec {
    layout(
        "rowsLayout",
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
    )
}

pub(super) fn mask_columns() -> LayoutSpec {
    layout(
        "columnsLayout",
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
    )
}

pub(super) fn mask_compose() -> LayoutSpec {
    layout(
        "composeLayout",
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
        ],
    )
}
