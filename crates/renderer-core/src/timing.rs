//! Optional whole-frame GPU timestamps. Marker render passes match the browser
//! control: empty compute passes do not write reliable timestamps on Metal.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wgpu::*;

const READBACKS: usize = 3;
const WINDOW: usize = 240;

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuFrameTime {
    pub frames: usize,
    pub mean_ms: f64,
    pub p95_ms: f64,
}

#[derive(Default)]
struct Samples {
    generation: u64,
    times: VecDeque<f64>,
}

struct Slot {
    readback: Buffer,
    busy: Arc<std::sync::atomic::AtomicBool>,
}

/// Three readbacks bound pending work; a saturated frame goes unmeasured.
/// Allocation is opt-in, so ordinary rendering pays no instrumentation cost.
pub struct FrameTimer {
    slots: Vec<Slot>,
    queries: QuerySet,
    resolve: Buffer,
    marker: TextureView,
    pending: Option<(usize, u64)>,
    period: f64,
    samples: Arc<Mutex<Samples>>,
}

impl FrameTimer {
    pub fn new(device: &Device, queue: &Queue) -> Option<Self> {
        if !device.features().contains(Features::TIMESTAMP_QUERY) {
            return None;
        }
        let buffer = |label, usage| {
            device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size: 16,
                usage,
                mapped_at_creation: false,
            })
        };
        let slots = (0..READBACKS)
            .map(|_| Slot {
                readback: buffer(
                    "frame-timer-readback",
                    BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                ),
                busy: Arc::default(),
            })
            .collect();
        let marker = device
            .create_texture(&TextureDescriptor {
                label: Some("frame-timer-marker"),
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Some(Self {
            slots,
            queries: device.create_query_set(&QuerySetDescriptor {
                label: Some("frame-timer"),
                ty: QueryType::Timestamp,
                count: 2,
            }),
            resolve: buffer(
                "frame-timer-resolve",
                BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            ),
            marker,
            pending: None,
            period: f64::from(queue.get_timestamp_period()),
            samples: Arc::default(),
        })
    }

    pub fn begin(&mut self, encoder: &mut CommandEncoder) {
        use std::sync::atomic::Ordering;
        self.pending = self
            .slots
            .iter()
            .position(|slot| !slot.busy.load(Ordering::Acquire))
            .map(|slot| {
                (
                    slot,
                    self.samples.lock().expect("GPU timing samples").generation,
                )
            });
        if self.pending.is_some() {
            self.mark(encoder, 0);
        }
    }

    fn mark(&self, encoder: &mut CommandEncoder, index: u32) {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(if index == 0 {
                "frame-begin"
            } else {
                "frame-end"
            }),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.marker,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            timestamp_writes: Some(RenderPassTimestampWrites {
                query_set: &self.queries,
                beginning_of_pass_write_index: (index == 0).then_some(0),
                end_of_pass_write_index: (index == 1).then_some(1),
            }),
            ..Default::default()
        });
    }

    pub fn end(&self, encoder: &mut CommandEncoder) {
        let Some((index, _)) = self.pending else {
            return;
        };
        self.mark(encoder, 1);
        let slot = &self.slots[index];
        encoder.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &slot.readback, 0, 16);
    }

    /// Call after submission. Mapping resolves asynchronously; it never drains
    /// the GPU queue or delays another frame to obtain a sample.
    pub fn submitted(&mut self) {
        use std::sync::atomic::Ordering;
        let Some((index, generation)) = self.pending.take() else {
            return;
        };
        let slot = &self.slots[index];
        let readback = slot.readback.clone();
        let busy = slot.busy.clone();
        let samples = self.samples.clone();
        let period = self.period;
        busy.store(true, Ordering::Release);
        slot.readback.map_async(MapMode::Read, .., move |result| {
            if result.is_ok() {
                let ticks: [u64; 2] = {
                    let view = readback
                        .get_mapped_range(..)
                        .expect("mapped GPU timestamps");
                    bytemuck::pod_read_unaligned(&view[..16])
                };
                readback.unmap();
                let mut samples = samples.lock().expect("GPU timing samples");
                if samples.generation == generation && ticks[0] != 0 && ticks[1] > ticks[0] {
                    samples
                        .times
                        .push_back((ticks[1] - ticks[0]) as f64 * period / 1e6);
                    if samples.times.len() > WINDOW {
                        samples.times.pop_front();
                    }
                }
            }
            busy.store(false, Ordering::Release);
        });
    }

    pub fn reset(&mut self) {
        let mut samples = self.samples.lock().expect("GPU timing samples");
        samples.generation += 1;
        samples.times.clear();
    }

    pub fn stats(&self) -> GpuFrameTime {
        let samples = self.samples.lock().expect("GPU timing samples");
        let mut sorted: Vec<_> = samples.times.iter().copied().collect();
        sorted.sort_by(f64::total_cmp);
        GpuFrameTime {
            frames: sorted.len(),
            mean_ms: sorted.iter().sum::<f64>() / sorted.len().max(1) as f64,
            p95_ms: sorted
                .get((sorted.len().saturating_sub(1) as f64 * 0.95) as usize)
                .copied()
                .unwrap_or(0.0),
        }
    }
}

impl FrameTimer {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.resolve);
        for slot in &self.slots {
            memory.buffer(&slot.readback);
        }
        memory.view(&self.marker);
    }
}
