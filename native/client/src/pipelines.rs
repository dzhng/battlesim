//! The browser's pipelines, built on this device from its export
//! (`crates/pipelines`): each pipeline's WGSL file and its settings in
//! `pipelines.json`, with the bind group layouts by name and the sizes the
//! passes allocate by. Nothing is mirrored by hand, and the export's schema is
//! read strictly, so a new setting fails here rather than being dropped.
use std::collections::{BTreeMap, HashMap};
use std::fs;

use serde::Deserialize;
use wgpu::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    sizes: Sizes,
    layouts: BTreeMap<String, Vec<Entry>>,
    pipelines: Vec<Desc>,
}

/// What the passes allocate by, as the browser's frame code sizes it.
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Sizes {
    /// The bloom's halving levels, one `bloomBlur<level>` pipeline each.
    pub bloom_levels: usize,
    /// The sky lookup's width and height in texels.
    pub sky_lut: [u32; 2],
    /// Every effect sheet's side in texels: each sheet is square.
    pub effect_sheet: u32,
    /// A grass clump's stride in the clump buffer, in bytes.
    pub grass_clump: u64,
    /// The levels of the world's copy the water refracts (`world::Water`).
    pub scene_colour_mips: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    binding: u32,
    visibility: Vec<String>,
    buffer: Option<Kind>,
    sampler: Option<Kind>,
    texture: Option<TextureEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Kind {
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TextureEntry {
    sample_type: Option<String>,
    view_dimension: Option<String>,
    multisampled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Desc {
    name: String,
    kind: String,
    shader: String,
    layout: Vec<String>,
    vertex: Option<VertexDesc>,
    fragment: Option<FragmentDesc>,
    primitive: Option<PrimitiveDesc>,
    depth_stencil: Option<DepthDesc>,
    multisample: Option<MultisampleDesc>,
    compute: Option<ComputeDesc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct VertexDesc {
    entry_point: Option<String>,
    buffers: Vec<BufferDesc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BufferDesc {
    array_stride: u64,
    step_mode: String,
    attributes: Vec<AttributeDesc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AttributeDesc {
    format: String,
    offset: u64,
    shader_location: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FragmentDesc {
    entry_point: Option<String>,
    targets: Vec<TargetDesc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TargetDesc {
    format: String,
    blend: Option<BlendDesc>,
    write_mask: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlendDesc {
    color: ComponentDesc,
    alpha: ComponentDesc,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ComponentDesc {
    src_factor: Option<String>,
    dst_factor: Option<String>,
    operation: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PrimitiveDesc {
    topology: Option<String>,
    cull_mode: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DepthDesc {
    format: String,
    depth_write_enabled: Option<bool>,
    depth_compare: Option<String>,
    depth_bias: Option<i32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MultisampleDesc {
    count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ComputeDesc {
    entry_point: Option<String>,
}

type Result<T> = std::result::Result<T, String>;

pub fn texture_format(s: &str) -> Result<TextureFormat> {
    Ok(match s {
        "rgba16float" => TextureFormat::Rgba16Float,
        "rgba8unorm" => TextureFormat::Rgba8Unorm,
        "bgra8unorm" => TextureFormat::Bgra8Unorm,
        "rg8unorm" => TextureFormat::Rg8Unorm,
        "r8unorm" => TextureFormat::R8Unorm,
        "rg16float" => TextureFormat::Rg16Float,
        "r32uint" => TextureFormat::R32Uint,
        "rg32uint" => TextureFormat::Rg32Uint,
        "depth32float" => TextureFormat::Depth32Float,
        other => return Err(format!("texture format {other}")),
    })
}

fn vertex_format(s: &str) -> Result<VertexFormat> {
    Ok(match s {
        "float32" => VertexFormat::Float32,
        "float32x2" => VertexFormat::Float32x2,
        "float32x3" => VertexFormat::Float32x3,
        "float32x4" => VertexFormat::Float32x4,
        "unorm8x4" => VertexFormat::Unorm8x4,
        "uint8x4" => VertexFormat::Uint8x4,
        "uint16x2" => VertexFormat::Uint16x2,
        "snorm8x4" => VertexFormat::Snorm8x4,
        "snorm16x4" => VertexFormat::Snorm16x4,
        other => return Err(format!("vertex format {other}")),
    })
}

fn compare(s: &str) -> Result<CompareFunction> {
    Ok(match s {
        "less" => CompareFunction::Less,
        "equal" => CompareFunction::Equal,
        "less-equal" => CompareFunction::LessEqual,
        "greater" => CompareFunction::Greater,
        "greater-equal" => CompareFunction::GreaterEqual,
        "always" => CompareFunction::Always,
        other => return Err(format!("depth compare {other}")),
    })
}

fn factor(s: &str) -> Result<BlendFactor> {
    Ok(match s {
        "zero" => BlendFactor::Zero,
        "one" => BlendFactor::One,
        "src-alpha" => BlendFactor::SrcAlpha,
        "one-minus-src-alpha" => BlendFactor::OneMinusSrcAlpha,
        other => return Err(format!("blend factor {other}")),
    })
}

/// WebGPU's defaults where the export leaves a field out.
fn component(c: &ComponentDesc) -> Result<BlendComponent> {
    Ok(BlendComponent {
        src_factor: factor(c.src_factor.as_deref().unwrap_or("one"))?,
        dst_factor: factor(c.dst_factor.as_deref().unwrap_or("zero"))?,
        operation: match c.operation.as_deref().unwrap_or("add") {
            "add" => BlendOperation::Add,
            "max" => BlendOperation::Max,
            other => return Err(format!("blend operation {other}")),
        },
    })
}

/// WebGPU's defaults where the export leaves a field out.
fn entry(e: &Entry) -> Result<BindGroupLayoutEntry> {
    let mut visibility = ShaderStages::NONE;
    for stage in &e.visibility {
        visibility |= match stage.as_str() {
            "vertex" => ShaderStages::VERTEX,
            "fragment" => ShaderStages::FRAGMENT,
            "compute" => ShaderStages::COMPUTE,
            other => return Err(format!("stage {other}")),
        };
    }
    let ty = match (&e.buffer, &e.sampler, &e.texture) {
        (Some(b), None, None) => BindingType::Buffer {
            ty: match b.kind.as_deref().unwrap_or("uniform") {
                "uniform" => BufferBindingType::Uniform,
                "storage" => BufferBindingType::Storage { read_only: false },
                "read-only-storage" => BufferBindingType::Storage { read_only: true },
                other => return Err(format!("buffer type {other}")),
            },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        (None, Some(s), None) => {
            BindingType::Sampler(match s.kind.as_deref().unwrap_or("filtering") {
                "filtering" => SamplerBindingType::Filtering,
                "comparison" => SamplerBindingType::Comparison,
                other => return Err(format!("sampler type {other}")),
            })
        }
        (None, None, Some(t)) => BindingType::Texture {
            sample_type: match t.sample_type.as_deref().unwrap_or("float") {
                "float" => TextureSampleType::Float { filterable: true },
                "depth" => TextureSampleType::Depth,
                "uint" => TextureSampleType::Uint,
                other => return Err(format!("sample type {other}")),
            },
            view_dimension: match t.view_dimension.as_deref().unwrap_or("2d") {
                "2d" => TextureViewDimension::D2,
                "2d-array" => TextureViewDimension::D2Array,
                other => return Err(format!("view dimension {other}")),
            },
            multisampled: t.multisampled.unwrap_or(false),
        },
        _ => return Err(format!("binding {} is not one resource", e.binding)),
    };
    Ok(BindGroupLayoutEntry {
        binding: e.binding,
        visibility,
        ty,
        count: None,
    })
}

pub enum Pipeline {
    Render(RenderPipeline),
    Compute(ComputePipeline),
}

/// Every exported pipeline, built, and its bind group layouts.
pub struct Pipelines {
    pipelines: HashMap<String, Pipeline>,
    layouts: HashMap<String, BindGroupLayout>,
    /// Each pipeline's bind group layouts, by slot.
    slots: HashMap<String, Vec<String>>,
    pub sizes: Sizes,
}

/// A render pipeline and the layouts its bind group slots take, so a pass
/// binds groups by the layout they fill rather than by number.
#[derive(Clone)]
pub struct Bound {
    pub pipeline: RenderPipeline,
    slots: Vec<String>,
}

impl Bound {
    /// Sets the pipeline, and each slot's group from `groups` by layout name.
    pub fn set(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        pass.set_pipeline(&self.pipeline);
        for (i, group) in by_layout(&self.slots, groups) {
            pass.set_bind_group(i, group, &[]);
        }
    }
}

/// Each slot's group from `groups`, found by its layout's name.
fn by_layout<'g>(
    slots: &'g [String],
    groups: &'g [(&str, &BindGroup)],
) -> impl Iterator<Item = (u32, &'g BindGroup)> {
    slots.iter().enumerate().map(|(i, slot)| {
        let (_, group) = groups
            .iter()
            .find(|(name, _)| name == slot)
            .unwrap_or_else(|| panic!("no bind group for layout {slot}"));
        (i as u32, *group)
    })
}

/// A compute pipeline and its slots' layouts, as [`Bound`].
#[derive(Clone)]
pub struct BoundCompute {
    pub pipeline: ComputePipeline,
    slots: Vec<String>,
}

impl BoundCompute {
    /// Sets the pipeline, and each slot's group from `groups` by layout name.
    pub fn set(&self, pass: &mut ComputePass, groups: &[(&str, &BindGroup)]) {
        pass.set_pipeline(&self.pipeline);
        for (i, group) in by_layout(&self.slots, groups) {
            pass.set_bind_group(i, group, &[]);
        }
    }
}

impl Pipelines {
    /// The compute pipeline `name` with its slots' layouts.
    pub fn bound_compute(&self, name: &str) -> BoundCompute {
        match self.pipelines.get(name) {
            Some(Pipeline::Compute(p)) => BoundCompute {
                pipeline: p.clone(),
                slots: self.slots[name].clone(),
            },
            _ => panic!("no compute pipeline {name} in the export"),
        }
    }

    /// Builds every pipeline in `dir`; fails listing each one the device refused.
    pub fn build(device: &Device, dir: &str) -> Result<Self> {
        let text =
            fs::read_to_string(format!("{dir}/pipelines.json")).map_err(|e| e.to_string())?;
        let export: Export =
            serde_json::from_str(&text).map_err(|e| format!("pipelines.json: {e}"))?;
        let mut layouts = HashMap::new();
        for (name, entries) in &export.layouts {
            let entries = entries.iter().map(entry).collect::<Result<Vec<_>>>()?;
            let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(name),
                entries: &entries,
            });
            layouts.insert(name.clone(), layout);
        }
        let mut pipelines = HashMap::new();
        let slots = export
            .pipelines
            .iter()
            .map(|d| (d.name.clone(), d.layout.clone()))
            .collect();
        let mut failures = Vec::new();
        for desc in &export.pipelines {
            let scope = device.push_error_scope(ErrorFilter::Validation);
            let built = build(device, dir, desc, &layouts);
            let error = pollster::block_on(scope.pop());
            match (built, error) {
                (Ok(p), None) => {
                    pipelines.insert(desc.name.clone(), p);
                }
                (Err(e), _) => failures.push(format!("{}: {e}", desc.name)),
                (_, Some(e)) => failures.push(format!("{}: {e:?}", desc.name)),
            }
        }
        if !failures.is_empty() {
            return Err(format!(
                "{} of {} pipelines fail:\n{}",
                failures.len(),
                export.pipelines.len(),
                failures.join("\n")
            ));
        }
        Ok(Self {
            pipelines,
            layouts,
            slots,
            sizes: export.sizes,
        })
    }

    /// The names of every pipeline built.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.pipelines.keys().map(String::as_str)
    }

    pub fn render(&self, name: &str) -> &RenderPipeline {
        match self.pipelines.get(name) {
            Some(Pipeline::Render(p)) => p,
            _ => panic!("no render pipeline {name} in the export"),
        }
    }

    /// The render pipeline `name` with its slots' layouts.
    pub fn bound(&self, name: &str) -> Bound {
        Bound {
            pipeline: self.render(name).clone(),
            slots: self.slots[name].clone(),
        }
    }

    pub fn layout(&self, name: &str) -> &BindGroupLayout {
        self.layouts
            .get(name)
            .unwrap_or_else(|| panic!("no bind group layout {name} in the export"))
    }
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

fn build(
    device: &Device,
    dir: &str,
    desc: &Desc,
    layouts: &HashMap<String, BindGroupLayout>,
) -> Result<Pipeline> {
    let source = fs::read_to_string(format!("{dir}/{}", desc.shader)).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let source = native_shader_source(&desc.name, source);
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&desc.shader),
        source: ShaderSource::Wgsl(source.into()),
    });
    let groups = desc
        .layout
        .iter()
        .map(|n| layouts.get(n).map(Some).ok_or(format!("no layout {n}")))
        .collect::<Result<Vec<_>>>()?;
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(&desc.name),
        bind_group_layouts: &groups,
        immediate_size: 0,
    });
    if desc.kind == "compute" {
        let compute = desc.compute.as_ref().ok_or("no compute stage")?;
        return Ok(Pipeline::Compute(device.create_compute_pipeline(
            &ComputePipelineDescriptor {
                label: Some(&desc.name),
                layout: Some(&layout),
                module: &module,
                entry_point: compute.entry_point.as_deref(),
                compilation_options: Default::default(),
                cache: None,
            },
        )));
    }
    if desc.kind != "render" {
        return Err(format!("kind {}", desc.kind));
    }
    let vertex = desc.vertex.as_ref().ok_or("no vertex stage")?;
    let attributes = vertex
        .buffers
        .iter()
        .map(|b| {
            b.attributes
                .iter()
                .map(|a| {
                    Ok(VertexAttribute {
                        format: vertex_format(&a.format)?,
                        offset: a.offset,
                        shader_location: a.shader_location,
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let buffers = vertex
        .buffers
        .iter()
        .zip(&attributes)
        .map(|(b, attributes)| {
            Ok(Some(VertexBufferLayout {
                array_stride: b.array_stride,
                step_mode: match b.step_mode.as_str() {
                    "vertex" => VertexStepMode::Vertex,
                    "instance" => VertexStepMode::Instance,
                    other => return Err(format!("step mode {other}")),
                },
                attributes,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let targets = desc
        .fragment
        .iter()
        .flat_map(|f| &f.targets)
        .map(|t| {
            Ok(Some(ColorTargetState {
                format: texture_format(&t.format)?,
                blend: match &t.blend {
                    Some(b) => Some(BlendState {
                        color: component(&b.color)?,
                        alpha: component(&b.alpha)?,
                    }),
                    None => None,
                },
                write_mask: ColorWrites::from_bits(t.write_mask.unwrap_or(0xf))
                    .ok_or("write mask")?,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let depth_stencil = match &desc.depth_stencil {
        Some(d) => Some(DepthStencilState {
            format: texture_format(&d.format)?,
            depth_write_enabled: d.depth_write_enabled,
            depth_compare: d.depth_compare.as_deref().map(compare).transpose()?,
            stencil: StencilState::default(),
            bias: DepthBiasState {
                constant: d.depth_bias.unwrap_or(0),
                ..Default::default()
            },
        }),
        None => None,
    };
    let primitive = desc.primitive.as_ref();
    let primitive = PrimitiveState {
        topology: match primitive.and_then(|p| p.topology.as_deref()) {
            None | Some("triangle-list") => PrimitiveTopology::TriangleList,
            Some(other) => return Err(format!("topology {other}")),
        },
        cull_mode: match primitive.and_then(|p| p.cull_mode.as_deref()) {
            None | Some("none") => None,
            Some("back") => Some(Face::Back),
            Some("front") => Some(Face::Front),
            Some(other) => return Err(format!("cull mode {other}")),
        },
        ..Default::default()
    };
    Ok(Pipeline::Render(device.create_render_pipeline(
        &RenderPipelineDescriptor {
            label: Some(&desc.name),
            layout: Some(&layout),
            vertex: VertexState {
                module: &module,
                entry_point: vertex.entry_point.as_deref(),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive,
            depth_stencil,
            multisample: MultisampleState {
                count: desc.multisample.as_ref().map_or(1, |m| m.count),
                ..Default::default()
            },
            fragment: desc.fragment.as_ref().map(|f| FragmentState {
                module: &module,
                entry_point: f.entry_point.as_deref(),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        },
    )))
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
