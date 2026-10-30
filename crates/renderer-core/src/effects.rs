//! Shared effect sheets, atlas mips, material bindings and instance drawing.
//! Platform transports supply PNG bytes; decoding, premultiplication and GPU
//! resource policy stay identical across all renderer targets.
use crate::environment::Environment;
use presentation::records::Effect;
use std::sync::Arc;
use wgpu::*;

use crate::pipelines::Pipelines;
use crate::Targets;

/// Decoded, premultiplied base sheets retained for device reconstruction.
/// GPU upload derives every mip with the same encoded-RGBA filter.
pub struct EffectSheets {
    side: u32,
    layers: Vec<Vec<u8>>,
}

impl EffectSheets {
    pub fn decode(side: u32, pngs: &[&[u8]]) -> Result<Self, String> {
        if pngs.is_empty() {
            return Err("effect atlas contains no sheets".into());
        }
        let layers = pngs
            .iter()
            .enumerate()
            .map(|(index, bytes)| {
                let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
                decoder.set_transformations(png::Transformations::normalize_to_color8());
                let mut reader = decoder
                    .read_info()
                    .map_err(|error| format!("effect sheet {index}: {error}"))?;
                let mut rgba = vec![0; reader.output_buffer_size()];
                let info = reader
                    .next_frame(&mut rgba)
                    .map_err(|error| format!("effect sheet {index}: {error}"))?;
                if (info.width, info.height) != (side, side)
                    || info.color_type != png::ColorType::Rgba
                {
                    return Err(format!(
                        "effect sheet {index} is {}×{} {:?}, not {side}² RGBA",
                        info.width, info.height, info.color_type
                    ));
                }
                for pixel in rgba.as_chunks_mut::<4>().0 {
                    let alpha = u32::from(pixel[3]);
                    for channel in &mut pixel[..3] {
                        *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                    }
                }
                Ok(rgba)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self { side, layers })
    }

    pub fn payload_bytes(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }
}

/// Mean encoded premultiplied RGBA over the covered 2×2 texels.
fn halve(level: &[u8], side: usize) -> Vec<u8> {
    let half = side / 2;
    let mut out = vec![0; half * half * 4];
    for y in 0..half {
        for x in 0..half {
            for channel in 0..4 {
                let at = |dx: usize, dy: usize| {
                    u32::from(level[((2 * y + dy) * side + 2 * x + dx) * 4 + channel])
                };
                out[(y * half + x) * 4 + channel] =
                    ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
            }
        }
    }
    out
}

fn atlas(device: &Device, queue: &Queue, sheets: &EffectSheets) -> Texture {
    let side = sheets.side;
    let levels = side.ilog2() + 1;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("effect-atlas"),
        size: Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: sheets.layers.len() as u32,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, rgba) in sheets.layers.iter().enumerate() {
        let mut level = rgba.clone();
        for mip in 0..levels {
            let edge = side >> mip;
            if mip > 0 {
                level = halve(&level, (edge * 2) as usize);
            }
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: TextureAspect::All,
                },
                &level,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(edge * 4),
                    rows_per_image: None,
                },
                Extent3d {
                    width: edge,
                    height: edge,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    texture
}

pub struct EffectsResources {
    atlas: Texture,
    sampler: Sampler,
    sheets: Arc<EffectSheets>,
    pipeline: RenderPipeline,
    group: BindGroup,
    instances: Buffer,
    instance_stride: u64,
    count: u32,
}

fn bindings(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    environment: &Environment,
    atlas: &Texture,
    sampler: &Sampler,
) -> BindGroup {
    let atlas = atlas.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });
    crate::bind(
        device,
        pipelines.layout("effects"),
        "effects",
        &[
            environment.camera_buffer.as_entire_binding(),
            BindingResource::TextureView(&targets.depth),
            BindingResource::TextureView(&atlas),
            BindingResource::Sampler(sampler),
            environment.data.as_entire_binding(),
            BindingResource::TextureView(&environment.pmrem),
            environment.cast_lights.as_entire_binding(),
        ],
    )
}

impl EffectsResources {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        targets: &Targets,
        environment: &Environment,
        sheets: Arc<EffectSheets>,
    ) -> Self {
        let atlas = atlas(device, queue, &sheets);
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("effects"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });
        let instance_stride = size_of::<Effect>() as u64;
        let instances = device.create_buffer(&BufferDescriptor {
            label: Some("effect-instances"),
            size: 1024 * instance_stride,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = bindings(device, pipelines, targets, environment, &atlas, &sampler);
        Self {
            atlas,
            sampler,
            sheets,
            pipeline: pipelines.render("effects").clone(),
            group,
            instances,
            instance_stride,
            count: 0,
        }
    }

    /// Replace target-dependent bindings without recreating atlas or instances.
    pub fn rebind(
        &mut self,
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        environment: &Environment,
    ) {
        self.group = bindings(
            device,
            pipelines,
            targets,
            environment,
            &self.atlas,
            &self.sampler,
        );
    }

    pub fn sheets(&self) -> Arc<EffectSheets> {
        self.sheets.clone()
    }

    /// Replace this frame's packed instances, growing by doubling as needed.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, bytes: &[u8]) {
        self.count = (bytes.len() as u64 / self.instance_stride) as u32;
        if bytes.is_empty() {
            return;
        }
        let need = bytes.len() as u64;
        if self.instances.size() < need {
            self.instances = device.create_buffer(&BufferDescriptor {
                label: Some("effect-instances"),
                size: need.saturating_mul(2),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.instances, 0, bytes);
    }

    pub fn capacity(&self) -> u64 {
        self.instances.size() / self.instance_stride
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    /// Draw effects over lit colour and the fog mask using shared attachments.
    pub fn encode(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        if self.count == 0 {
            return;
        }
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("effects"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &targets.lit,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &targets.fog_mask,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
            ],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}

impl EffectsResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.instances);
        memory.texture(&self.atlas);
    }
}
