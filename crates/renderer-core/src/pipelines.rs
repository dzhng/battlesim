//! The shared pipeline catalog: WGSL sources and their `wgpu` descriptors are
//! embedded once by this crate's build script and used by native and browser.
//! Production uses the typed `build_builtin` catalog; the display compositor
//! uses the small typed `build_display` descriptor.
pub use crate::pipeline_registry::Sizes;
use crate::pipeline_registry::{self, PipelineKind, PipelineSpec};
use std::collections::HashMap;
use wgpu::*;

type Result<T> = std::result::Result<T, String>;

pub enum Pipeline {
    Render(RenderPipeline),
    Compute(ComputePipeline),
}

/// Installed typed depth facts used by frame diagnostics.
#[derive(Clone, Copy)]
struct DepthStateInfo {
    format: TextureFormat,
    compare: CompareFunction,
}

pub struct Pipelines {
    pipelines: HashMap<String, Pipeline>,
    layouts: HashMap<String, BindGroupLayout>,
    /// Each pipeline's bind group layouts, by slot.
    slots: HashMap<String, Vec<String>>,
    depth_states: HashMap<String, DepthStateInfo>,
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
    /// Installed descriptor facts, used by diagnostics rather than a second
    /// hand-maintained statement of the engine's depth convention.
    pub fn depth_state(&self, name: &str) -> serde_json::Value {
        let state = self
            .depth_states
            .get(name)
            .expect("pipeline has a depth state");
        serde_json::json!({ "format": state.format, "compare": state.compare })
    }

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

/// The common built-in pipeline catalog used by native and browser shells.
/// The source and descriptor are embedded by this crate's build script, so no
/// platform reads the browser export or carries a second shader owner.
pub fn build_builtin(device: &Device) -> Result<Pipelines> {
    build_typed(
        device,
        pipeline_registry::layouts(),
        pipeline_registry::pipelines(),
        |_, shader| {
            builtin_source(shader).ok_or_else(|| format!("missing built-in shader {shader}"))
        },
    )
}

/// Build the platform shell's tiny present/composite pipeline without a JSON
/// descriptor. The production catalog remains the only shared pipeline owner.
pub fn build_display(device: &Device, shader: &str, format: TextureFormat) -> Result<Pipelines> {
    let layout = pipeline_registry::LayoutSpec {
        name: "sampled",
        entries: vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    };
    let spec = PipelineSpec {
        name: "composite",
        kind: PipelineKind::Render,
        shader: "display.wgsl",
        layouts: vec!["sampled"],
        vertex: Some(pipeline_registry::VertexSpec {
            entry_point: Some("fullscreen"),
            buffers: vec![],
        }),
        fragment: Some(pipeline_registry::FragmentSpec {
            entry_point: Some("composite"),
            targets: vec![pipeline_registry::TargetSpec {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            }],
        }),
        primitive: pipeline_registry::PrimitiveSpec {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
        },
        depth_stencil: None,
        multisample_count: 1,
        compute_entry: None,
    };
    build_typed(device, vec![layout], vec![spec], |_, _| {
        Ok(shader.to_owned())
    })
}

pub fn builtin_pipeline_count() -> usize {
    BUILTIN_PIPELINE_COUNT
}

pub fn builtin_sizes() -> Sizes {
    pipeline_registry::sizes()
}

pub fn builtin_shaders() -> &'static [&'static str] {
    BUILTIN_SHADERS
}

fn build_typed<F>(
    device: &Device,
    layout_specs: Vec<pipeline_registry::LayoutSpec>,
    pipeline_specs: Vec<PipelineSpec>,
    mut source: F,
) -> Result<Pipelines>
where
    F: FnMut(&str, &str) -> Result<String>,
{
    let mut layouts = HashMap::new();
    for spec in &layout_specs {
        layouts.insert(
            spec.name.to_owned(),
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(spec.name),
                entries: &spec.entries,
            }),
        );
    }
    let slots = pipeline_specs
        .iter()
        .map(|spec| {
            (
                spec.name.to_owned(),
                spec.layouts.iter().map(|s| (*s).to_owned()).collect(),
            )
        })
        .collect();
    let mut pipelines = HashMap::new();
    let mut depth_states = HashMap::new();
    for spec in &pipeline_specs {
        let built = build_typed_pipeline(device, spec, &layouts, &mut source)?;
        pipelines.insert(spec.name.to_owned(), built);
        if let Some(depth) = &spec.depth_stencil {
            depth_states.insert(
                spec.name.to_owned(),
                DepthStateInfo {
                    format: depth.format,
                    compare: depth.depth_compare,
                },
            );
        }
    }
    Ok(Pipelines {
        pipelines,
        layouts,
        slots,
        depth_states,
        sizes: pipeline_registry::sizes(),
    })
}

fn build_typed_pipeline<F>(
    device: &Device,
    spec: &PipelineSpec,
    layouts: &HashMap<String, BindGroupLayout>,
    source: &mut F,
) -> Result<Pipeline>
where
    F: FnMut(&str, &str) -> Result<String>,
{
    let shader = source(spec.name, spec.shader)?;
    #[cfg(windows)]
    let shader = native_shader_source(spec.name, shader);
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(spec.shader),
        source: ShaderSource::Wgsl(shader.into()),
    });
    let groups: Vec<Option<&BindGroupLayout>> = spec
        .layouts
        .iter()
        .map(|name| {
            layouts
                .get(*name)
                .map(Some)
                .ok_or_else(|| format!("no layout {name}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(spec.name),
        bind_group_layouts: &groups,
        immediate_size: 0,
    });
    if spec.kind == PipelineKind::Compute {
        return Ok(Pipeline::Compute(device.create_compute_pipeline(
            &ComputePipelineDescriptor {
                label: Some(spec.name),
                layout: Some(&layout),
                module: &module,
                entry_point: spec.compute_entry,
                compilation_options: Default::default(),
                cache: None,
            },
        )));
    }
    let vertex = spec.vertex.as_ref().ok_or("no vertex stage")?;
    let attributes: Vec<Vec<VertexAttribute>> = vertex
        .buffers
        .iter()
        .map(|buffer| {
            buffer
                .attributes
                .iter()
                .map(|attribute| VertexAttribute {
                    format: attribute.format,
                    offset: attribute.offset,
                    shader_location: attribute.shader_location,
                })
                .collect()
        })
        .collect();
    let buffers: Vec<Option<VertexBufferLayout>> = vertex
        .buffers
        .iter()
        .zip(&attributes)
        .map(|(buffer, attributes)| {
            Some(VertexBufferLayout {
                array_stride: buffer.array_stride,
                step_mode: buffer.step_mode,
                attributes,
            })
        })
        .collect();
    let fragment = spec.fragment.as_ref();
    let targets: Vec<Option<ColorTargetState>> = fragment
        .map(|fragment| {
            fragment
                .targets
                .iter()
                .map(|target| {
                    Some(ColorTargetState {
                        format: target.format,
                        blend: target.blend,
                        write_mask: target.write_mask,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let depth_stencil = spec.depth_stencil.as_ref().map(|depth| DepthStencilState {
        format: depth.format,
        depth_write_enabled: Some(depth.depth_write_enabled),
        depth_compare: Some(depth.depth_compare),
        stencil: StencilState::default(),
        bias: DepthBiasState {
            constant: depth.depth_bias,
            ..Default::default()
        },
    });
    Ok(Pipeline::Render(device.create_render_pipeline(
        &RenderPipelineDescriptor {
            label: Some(spec.name),
            layout: Some(&layout),
            vertex: VertexState {
                module: &module,
                entry_point: vertex.entry_point,
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: PrimitiveState {
                topology: spec.primitive.topology,
                cull_mode: spec.primitive.cull_mode,
                ..Default::default()
            },
            depth_stencil,
            multisample: MultisampleState {
                count: spec.multisample_count,
                ..Default::default()
            },
            fragment: fragment.map(|fragment| FragmentState {
                module: &module,
                entry_point: fragment.entry_point,
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        },
    )))
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

include!(concat!(env!("OUT_DIR"), "/pipeline_export.rs"));

/// Built-in shader `name`'s WGSL: its composed modules after the
/// presentation's look numbers they read by name, so a shader and the
/// presentation code that bounds or clocks the same motion cannot disagree.
pub fn builtin_source(name: &str) -> Option<String> {
    use presentation::shader_constants::{Scalar, CONSTANTS};
    let mut source: String = CONSTANTS
        .iter()
        .map(|(name, value)| match value {
            Scalar::F32(value) => format!("const {name}: f32 = {value:?};\n"),
            Scalar::U32(value) => format!("const {name}: u32 = {value}u;\n"),
        })
        .collect();
    source.push_str(assembled_source(name)?);
    Some(source)
}
