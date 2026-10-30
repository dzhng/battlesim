//! Optional whole-frame GPU timestamps. Marker render passes match the browser
//! control: empty compute passes do not write reliable timestamps on Metal.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wgpu::*;

const WINDOW: usize = 240;

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuFrameTime {
    pub frames: usize,
    pub mean_ms: f64,
    pub p95_ms: f64,
}

/// Where timestamp counters are resolved. AfterCompletion uses a separate
/// submission once the frame finishes, avoiding stale Metal stage counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolve {
    InFrame,
    AfterCompletion,
}

#[derive(Default)]
struct Samples {
    generation: u64,
    times: VecDeque<f64>,
}

impl Samples {
    fn reset(&mut self) {
        self.generation += 1;
        self.times.clear();
    }
}

#[derive(Clone)]
struct Measurement {
    generation: u64,
    samples: Arc<Mutex<Samples>>,
    result: Arc<Mutex<Option<f64>>>,
}

impl Measurement {
    fn new(samples: &Arc<Mutex<Samples>>) -> Self {
        Self {
            generation: samples.lock().expect("GPU timing samples").generation,
            samples: samples.clone(),
            result: Arc::default(),
        }
    }

    fn complete(&self, ms: Option<f64>) {
        let mut samples = self.samples.lock().expect("GPU timing samples");
        if self.generation == samples.generation {
            if let Some(ms) = ms {
                samples.times.push_back(ms);
                if samples.times.len() > WINDOW {
                    samples.times.pop_front();
                }
            }
        }
        *self.result.lock().expect("GPU timing result") = ms;
    }
}

#[derive(Clone)]
struct Slot {
    queries: QuerySet,
    resolve: Buffer,
    readback: Buffer,
    busy: Arc<AtomicBool>,
}

impl Slot {
    fn resolve(&self, encoder: &mut CommandEncoder) {
        encoder.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16);
    }

    fn map(&self, measurement: Measurement, period: f64) {
        let slot = self.clone();
        self.readback.map_async(MapMode::Read, .., move |result| {
            let ms = if result.is_ok() {
                let ticks: [u64; 2] = {
                    let view = slot
                        .readback
                        .get_mapped_range(..)
                        .expect("mapped GPU timestamps");
                    bytemuck::pod_read_unaligned(&view[..16])
                };
                slot.readback.unmap();
                (ticks[0] != 0 && ticks[1] > ticks[0])
                    .then(|| (ticks[1] - ticks[0]) as f64 * period / 1e6)
            } else {
                None
            };
            measurement.complete(ms);
            slot.busy.store(false, Ordering::Release);
        });
    }
}

/// An AfterCompletion measurement owns its result across later resets and
/// completions. Consume it with wait to resolve counters and release its slot.
pub struct PendingFrame {
    render: SubmissionIndex,
    slot: Slot,
    measurement: Measurement,
    period: f64,
    device: Device,
    queue: Queue,
}

impl PendingFrame {
    /// Native's explicit profiling wait: first the render, then its telemetry.
    pub fn wait(self) -> Result<Option<f64>, String> {
        let wait = |index| {
            self.device
                .poll(PollType::Wait {
                    submission_index: Some(index),
                    timeout: None,
                })
                .map_err(|e| e.to_string())
        };
        wait(self.render.clone())?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.slot.resolve(&mut encoder);
        let index = self.queue.submit([encoder.finish()]);
        self.slot.map(self.measurement.clone(), self.period);
        wait(index)?;
        Ok(*self.measurement.result.lock().expect("GPU timing result"))
    }
}

/// Caller-sized readbacks bound pending work; a saturated frame goes unmeasured.
/// Allocation is opt-in, so ordinary rendering pays no instrumentation cost.
pub struct FrameTimer {
    slots: Vec<Slot>,
    device: Device,
    queue: Queue,
    mode: Resolve,
    marker: TextureView,
    pending: Option<(usize, Measurement)>,
    period: f64,
    samples: Arc<Mutex<Samples>>,
}

impl FrameTimer {
    pub fn new(device: &Device, queue: &Queue, mode: Resolve, capacity: usize) -> Option<Self> {
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
        let queries = || {
            device.create_query_set(&QuerySetDescriptor {
                label: Some("frame-timer"),
                ty: QueryType::Timestamp,
                count: 2,
            })
        };
        // InFrame copies its counters before the next frame can overwrite them.
        // AfterCompletion retains each frame's queries until its ticket waits.
        let shared_queries = (mode == Resolve::InFrame).then(queries);
        let resolve = buffer(
            "frame-timer-resolve",
            BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
        );
        let slots = (0..capacity)
            .map(|_| Slot {
                queries: shared_queries.clone().unwrap_or_else(queries),
                resolve: resolve.clone(),
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
                size: Extent3d::default(),
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
            device: device.clone(),
            queue: queue.clone(),
            mode,
            marker,
            pending: None,
            period: f64::from(queue.get_timestamp_period()),
            samples: Arc::default(),
        })
    }

    /// Reserve a slot in the current reset window. Pair every
    /// begin with end and submitted; saturation skips the measurement only.
    pub fn begin(&mut self, encoder: &mut CommandEncoder) {
        self.pending = self
            .slots
            .iter()
            .position(|slot| !slot.busy.swap(true, Ordering::AcqRel))
            .map(|slot| (slot, Measurement::new(&self.samples)));
        if self.pending.is_some() {
            self.mark(encoder, 0);
        }
    }

    fn mark(&self, encoder: &mut CommandEncoder, index: u32) {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(["frame-begin", "frame-end"][index as usize]),
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
                query_set: &self.slots[self.pending.as_ref().expect("timed frame").0].queries,
                beginning_of_pass_write_index: (index == 0).then_some(0),
                end_of_pass_write_index: (index == 1).then_some(1),
            }),
            ..Default::default()
        });
    }

    pub fn end(&self, encoder: &mut CommandEncoder) {
        let Some((index, _)) = &self.pending else {
            return;
        };
        self.mark(encoder, 1);
        if self.mode == Resolve::InFrame {
            self.slots[*index].resolve(encoder);
        }
    }

    /// After submission, InFrame maps without waiting; AfterCompletion requires
    /// waiting on its ticket. The slot stays reserved until mapping completes.
    pub fn submitted(&mut self, render: SubmissionIndex) -> Option<PendingFrame> {
        let (index, measurement) = self.pending.take()?;
        let slot = self.slots[index].clone();
        if self.mode == Resolve::InFrame {
            slot.map(measurement, self.period);
            return None;
        }
        Some(PendingFrame {
            render,
            slot,
            measurement,
            period: self.period,
            device: self.device.clone(),
            queue: self.queue.clone(),
        })
    }

    pub fn reset(&mut self) {
        self.samples.lock().expect("GPU timing samples").reset();
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

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for slot in &self.slots {
            memory.buffer(&slot.resolve);
            memory.buffer(&slot.readback);
        }
        memory.view(&self.marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_samples_keep_their_result_and_reset_window() {
        let gpu_samples = Arc::default();
        let first = Measurement::new(&gpu_samples);
        let delayed = Measurement::new(&gpu_samples);
        let newest = Measurement::new(&gpu_samples);
        // Completion order need not be frame order.
        newest.complete(Some(3.0));
        first.complete(Some(1.0));
        assert_eq!(*first.result.lock().unwrap(), Some(1.0));
        assert_eq!(*newest.result.lock().unwrap(), Some(3.0));
        gpu_samples.lock().unwrap().reset();
        let current = Measurement::new(&gpu_samples);
        current.complete(Some(4.0));
        delayed.complete(Some(2.0));
        assert_eq!(*delayed.result.lock().unwrap(), Some(2.0));
        assert_eq!(*current.result.lock().unwrap(), Some(4.0));
        assert_eq!(gpu_samples.lock().unwrap().times, [4.0]);
    }
}
