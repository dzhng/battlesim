//! Semantic built-in pipeline catalog shared by native and browser.
use crate::{
    DEPTH_FORMAT, DISPLAY_FORMAT, FOG_DISTANCE_FORMAT, FOG_MASK_FORMAT, GUARD_FORMAT, HDR_FORMAT,
    MSAA, OVERLAY_FORMAT,
};
use wgpu::*;
#[derive(Clone, Copy)]
pub struct Sizes {
    pub bloom_levels: usize,
    pub sky_lut: [u32; 2],
    pub effect_sheet: u32,
    pub grass_clump: u64,
    /// The levels of the world's copy the water refracts and blurs.
    pub scene_colour_mips: u32,
}
pub struct LayoutSpec {
    pub name: &'static str,
    pub entries: Vec<BindGroupLayoutEntry>,
}
pub struct VertexAttributeSpec {
    pub format: VertexFormat,
    pub offset: u64,
    pub shader_location: u32,
}
pub struct VertexBufferSpec {
    pub array_stride: u64,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttributeSpec>,
}
pub struct VertexSpec {
    pub entry_point: Option<&'static str>,
    pub buffers: Vec<VertexBufferSpec>,
}
pub struct TargetSpec {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: ColorWrites,
}
pub struct FragmentSpec {
    pub entry_point: Option<&'static str>,
    pub targets: Vec<TargetSpec>,
}
pub struct PrimitiveSpec {
    pub topology: PrimitiveTopology,
    pub cull_mode: Option<Face>,
}
pub struct DepthSpec {
    pub format: TextureFormat,
    pub depth_compare: CompareFunction,
    pub depth_write_enabled: bool,
    pub depth_bias: i32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    Render,
    Compute,
}
pub struct PipelineSpec {
    pub name: &'static str,
    pub kind: PipelineKind,
    pub shader: &'static str,
    pub layouts: Vec<&'static str>,
    pub vertex: Option<VertexSpec>,
    pub fragment: Option<FragmentSpec>,
    pub primitive: PrimitiveSpec,
    pub depth_stencil: Option<DepthSpec>,
    pub multisample_count: u32,
    pub compute_entry: Option<&'static str>,
}
pub fn attr(format: VertexFormat, offset: u64, shader_location: u32) -> VertexAttributeSpec {
    VertexAttributeSpec {
        format,
        offset,
        shader_location,
    }
}
pub fn buffer(
    array_stride: u64,
    step_mode: VertexStepMode,
    attributes: Vec<VertexAttributeSpec>,
) -> VertexBufferSpec {
    VertexBufferSpec {
        array_stride,
        step_mode,
        attributes,
    }
}
pub fn vertex(entry_point: Option<&'static str>, buffers: Vec<VertexBufferSpec>) -> VertexSpec {
    VertexSpec {
        entry_point,
        buffers,
    }
}
pub fn target(format: TextureFormat) -> TargetSpec {
    target_with(format, None, 0xf)
}
pub fn target_with(
    format: TextureFormat,
    blend: Option<BlendState>,
    write_mask: u32,
) -> TargetSpec {
    TargetSpec {
        format,
        blend,
        write_mask: ColorWrites::from_bits_retain(write_mask),
    }
}
pub fn fragment(entry_point: Option<&'static str>, targets: Vec<TargetSpec>) -> FragmentSpec {
    FragmentSpec {
        entry_point,
        targets,
    }
}
pub fn render(
    name: &'static str,
    shader: &'static str,
    layouts: Vec<&'static str>,
    vertex: Option<VertexSpec>,
    fragment: Option<FragmentSpec>,
) -> PipelineSpec {
    PipelineSpec {
        name,
        kind: PipelineKind::Render,
        shader,
        layouts,
        vertex,
        fragment,
        primitive: PrimitiveSpec {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
        },
        depth_stencil: None,
        multisample_count: 1,
        compute_entry: None,
    }
}

pub fn model_vertex() -> VertexSpec {
    vertex(
        None,
        vec![
            buffer(
                48,
                VertexStepMode::Vertex,
                vec![
                    attr(VertexFormat::Float32x3, 0, 0),
                    attr(VertexFormat::Snorm16x4, 12, 1),
                    attr(VertexFormat::Float32x2, 20, 2),
                    attr(VertexFormat::Unorm8x4, 28, 3),
                    attr(VertexFormat::Uint8x4, 32, 4),
                    attr(VertexFormat::Unorm8x4, 36, 5),
                    attr(VertexFormat::Snorm8x4, 40, 6),
                    attr(VertexFormat::Uint16x2, 44, 7),
                ],
            ),
            buffer(
                64,
                VertexStepMode::Instance,
                vec![
                    attr(VertexFormat::Float32x4, 0, 8),
                    attr(VertexFormat::Float32x4, 16, 9),
                    attr(VertexFormat::Float32x4, 32, 10),
                    attr(VertexFormat::Float32x4, 48, 11),
                ],
            ),
        ],
    )
}
pub fn compute(
    name: &'static str,
    shader: &'static str,
    layouts: Vec<&'static str>,
    compute_entry: Option<&'static str>,
) -> PipelineSpec {
    PipelineSpec {
        name,
        kind: PipelineKind::Compute,
        shader,
        layouts,
        vertex: None,
        fragment: None,
        primitive: PrimitiveSpec {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
        },
        depth_stencil: None,
        multisample_count: 1,
        compute_entry,
    }
}
impl PipelineSpec {
    pub fn primitive(mut self, topology: PrimitiveTopology, cull_mode: Option<Face>) -> Self {
        self.primitive = PrimitiveSpec {
            topology,
            cull_mode,
        };
        self
    }
    pub fn depth(
        mut self,
        format: TextureFormat,
        depth_compare: CompareFunction,
        depth_write_enabled: bool,
        depth_bias: i32,
    ) -> Self {
        self.depth_stencil = Some(DepthSpec {
            format,
            depth_compare,
            depth_write_enabled,
            depth_bias,
        });
        self
    }
    pub fn msaa(mut self, count: u32) -> Self {
        self.multisample_count = count;
        self
    }
}
mod layouts;
mod pipelines;
pub use layouts::all as layouts;
pub use pipelines::all as pipelines;
pub fn sizes() -> Sizes {
    Sizes {
        bloom_levels: 5,
        sky_lut: [384, 192],
        effect_sheet: 1024,
        grass_clump: 48,
        scene_colour_mips: 4,
    }
}
