use super::{Atlas, Spec};
use crate::fog::Layer;
use crate::pipelines::Pipelines;
use presentation::records::Instance;
use wgpu::*;

pub(crate) struct CardInstance {
    pub record: Instance,
    pub pass: u32,
    pub fog: Layer,
}
struct Run {
    pass: u32,
    fog: Layer,
    first: u32,
    count: u32,
}
pub(crate) struct Cards {
    pub group: BindGroup,
    albedo: TextureView,
    normal: TextureView,
    frames: Buffer,
    instances: Buffer,
    runs: Vec<Run>,
}
impl Cards {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        atlases: &[Atlas],
    ) -> Result<Self, String> {
        let spec = Spec::runtime();
        let (width, height) = (spec.yaws * spec.cell, spec.pitches.len() as u32 * spec.cell);
        if atlases
            .iter()
            .any(|a| a.spec != spec || a.width != width || a.height != height)
        {
            return Err("runtime card atlases must use the runtime specification".into());
        }
        let texture = |label| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: atlases.len().max(1) as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let (albedo, normal) = (texture("card-albedo"), texture("card-normal"));
        let mut frames = Vec::new();
        for (layer, atlas) in atlases.iter().enumerate() {
            let mut packed = atlas.normal.clone();
            for (normal, mask) in packed
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(atlas.mask.as_chunks::<4>().0.iter())
            {
                normal[3] = mask[0];
            }
            for (texture, bytes) in [(&albedo, &atlas.albedo), (&normal, &packed)] {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    bytes,
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            frames.extend(atlas.center.map(|n| n as f32));
            frames.push(atlas.radius as f32);
        }
        if frames.is_empty() {
            frames = vec![0., 0., 0., 1.];
        }
        let frames = crate::buffer(
            device,
            queue,
            "card-frames",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&frames),
        );
        let view = |texture: &Texture| {
            texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            })
        };
        let (albedo, normal) = (view(&albedo), view(&normal));
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("card-linear"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let group = crate::bind(
            device,
            pipelines.layout("cardLayout"),
            "model-cards",
            &[
                BindingResource::TextureView(&albedo),
                BindingResource::TextureView(&normal),
                BindingResource::Sampler(&sampler),
                frames.as_entire_binding(),
            ],
        );
        let instances = crate::buffer(
            device,
            queue,
            "card-instances",
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            &[0; 256],
        );
        Ok(Self {
            group,
            albedo,
            normal,
            frames,
            instances,
            runs: Vec::new(),
        })
    }
    pub fn prepare(&mut self, device: &Device, queue: &Queue, records: &[CardInstance]) {
        let instances: Vec<_> = records.iter().map(|r| r.record).collect();
        let bytes = bytemuck::cast_slice::<_, u8>(&instances);
        if bytes.len() as u64 > self.instances.size() {
            self.instances = device.create_buffer(&BufferDescriptor {
                label: Some("card-instances"),
                size: (bytes.len() * 2) as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.instances, 0, bytes);
        }
        self.runs.clear();
        for (first, record) in records.iter().enumerate() {
            if let Some(run) = self
                .runs
                .last_mut()
                .filter(|run| run.pass == record.pass && run.fog == record.fog)
            {
                run.count += 1;
            } else {
                self.runs.push(Run {
                    pass: record.pass,
                    fog: record.fog,
                    first: first as u32,
                    count: 1,
                });
            }
        }
    }
    pub fn draw(
        &self,
        pass: &mut RenderPass,
        layer: Layer,
        selected: impl Fn(u32) -> bool,
    ) -> crate::submitted::SubmittedDraws {
        let mut submitted = crate::submitted::SubmittedDraws::default();
        pass.set_vertex_buffer(0, self.instances.slice(..));
        for run in self
            .runs
            .iter()
            .filter(|run| run.fog == layer && selected(run.pass))
        {
            pass.draw(0..6, run.first..run.first + run.count);
            submitted.record(6, run.count);
        }
        submitted
    }
    pub fn layers(&self) -> u32 {
        self.albedo.texture().depth_or_array_layers()
    }
    pub fn has_pass(&self, pass: u32) -> bool {
        self.runs
            .iter()
            .any(|run| run.pass == pass && run.count > 0)
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.frames);
        memory.buffer(&self.instances);
        memory.view(&self.albedo);
        memory.view(&self.normal);
    }
}
