use super::{CardFrame, Spec};
use crate::fog::Layer;
use crate::pipelines::Pipelines;
use crate::texture_array::SharedArray;
use presentation::records::Instance;
use std::collections::BTreeMap;
use std::sync::Arc;
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
    pub(crate) array: Arc<SharedArray>,
    pub(crate) metadata: BTreeMap<u32, CardFrame>,
    frames: Buffer,
    instances: Buffer,
    runs: Vec<Run>,
}
/// The atlases are destroyed with their final shared owner, as the frame's
/// targets are (`Targets`), rather than left for the browser's garbage collector.
impl Cards {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        array: Arc<SharedArray>,
        metadata: BTreeMap<u32, CardFrame>,
    ) -> Result<Self, String> {
        let spec = Spec::runtime();
        let (width, height) = (spec.yaws * spec.cell, spec.pitches.len() as u32 * spec.cell);
        if array.texture().width() != width || array.texture().height() != height {
            return Err("runtime card atlases must use the runtime specification".into());
        }
        let mut frames = vec![
            0_f32;
            (metadata
                .keys()
                .copied()
                .max()
                .map_or(1, |layer| layer as usize + 1))
                * 4
        ];
        for (&layer, frame) in &metadata {
            let at = layer as usize * 4;
            frames[at..at + 3].copy_from_slice(&frame.center.map(|n| n as f32));
            frames[at + 3] = frame.radius as f32;
        }
        let frames = crate::buffer(
            device,
            queue,
            "card-frames",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&frames),
        );
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
                BindingResource::TextureView(array.view(0)),
                BindingResource::TextureView(array.view(1)),
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
            array,
            metadata,
            frames,
            instances,
            runs: Vec::new(),
        })
    }
    pub fn prepare(&mut self, device: &Device, queue: &Queue, records: &[CardInstance]) {
        let instances: Vec<_> = records.iter().map(|r| r.record).collect();
        let bytes = bytemuck::cast_slice::<_, u8>(&instances);
        if bytes.len() as u64 > self.instances.size() {
            self.instances.destroy();
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
        self.array.texture().depth_or_array_layers()
    }
    pub fn has_pass(&self, pass: u32) -> bool {
        self.runs
            .iter()
            .any(|run| run.pass == pass && run.count > 0)
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.frames);
        memory.buffer(&self.instances);
        self.array.record_memory(memory);
    }
}

impl Drop for Cards {
    fn drop(&mut self) {
        self.frames.destroy();
        self.instances.destroy();
    }
}
