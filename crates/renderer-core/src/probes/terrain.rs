//! Explicit material diagnostics using the composed production terrain shader.
use super::{check_storage, storage, workgroups};
use crate::pipelines::{builtin_source, Pipelines};
use crate::readback::Readback;
use crate::terrain::TerrainResources;
use wgpu::*;

#[derive(Clone, Copy)]
pub enum Query {
    Distances,
    Paved,
    Filters,
}
impl Query {
    fn entry(self) -> &'static str {
        match self {
            Self::Distances => "terrainDistances",
            Self::Paved => "terrainPaved",
            Self::Filters => "terrainFilters",
        }
    }
    fn vectors(self) -> u64 {
        if matches!(self, Self::Filters) {
            4
        } else {
            1
        }
    }
}

pub struct TerrainProbe {
    pipeline: ComputePipeline,
    layout: BindGroupLayout,
    source: String,
    query: Query,
}
impl TerrainProbe {
    pub fn new(device: &Device, pipelines: &Pipelines, query: Query) -> Result<Self, String> {
        let source = format!(
            "{}\n{}",
            builtin_source("worldTerrain.wgsl").ok_or("missing production terrain shader")?,
            include_str!("terrain_query.wgsl")
        );
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("terrain-material-query"),
            source: ShaderSource::Wgsl(source.clone().into()),
        });
        let storage_entry = |binding, read_only| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("terrain-query"),
            entries: &[
                storage_entry(1, true),
                storage_entry(2, false),
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("terrain-query"),
            bind_group_layouts: &[Some(&layout), Some(pipelines.layout("terrainLayout"))],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("terrain-query"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some(query.entry()),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(Self {
            pipeline,
            layout,
            source,
            query,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// The copy is submitted before this returns; the future retains no terrain borrow.
    pub fn queue(
        &self,
        device: &Device,
        queue: &Queue,
        material: &TerrainResources,
        rows: &[[f32; 4]],
        reference: &[u8],
        reference_side: u32,
    ) -> Result<Readback, String> {
        let groups = workgroups(device, rows.len(), 64)?;
        check_storage(device, "terrain-query-input", rows.len() as u64 * 16)?;
        let bytes = rows.len() as u64 * 16 * self.query.vectors();
        let output = storage(device, "terrain-query-output", bytes)?;
        let input = crate::buffer(
            device,
            queue,
            "terrain-query-input",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(rows),
        );
        let side = reference_side.max(1);
        if side > device.limits().max_texture_dimension_2d
            || (!reference.is_empty()
                && reference.len() as u64 != u64::from(side) * u64::from(side) * 4)
            || (matches!(self.query, Query::Filters) && reference.is_empty())
        {
            return Err(
                "terrain reference must be a square RGBA8 texture within device limits".into(),
            );
        }
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("terrain-query-reference"),
            size: Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if !reference.is_empty() {
            queue.write_texture(
                texture.as_image_copy(),
                reference,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(side * 4),
                    rows_per_image: Some(side),
                },
                texture.size(),
            );
        }
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        let group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("terrain-query"),
            layout: &self.layout,
            entries: &[
                BindGroupEntry {
                    binding: 1,
                    resource: input.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: output.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.set_bind_group(1, material.bind_group(), &[]);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        let read = Readback::buffer(device, &mut encoder, &output, 0..bytes)?;
        queue.submit([encoder.finish()]);
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn diagnostic_entries_validate_with_the_actual_production_material() {
        let source = format!(
            "{}\n{}",
            crate::pipelines::builtin_source("worldTerrain.wgsl").unwrap(),
            include_str!("terrain_query.wgsl")
        );
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
