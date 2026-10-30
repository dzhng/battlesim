//! Optional whole-frame GPU timestamps. Marker render passes match the browser
//! control: empty compute passes do not write reliable timestamps on Metal.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Where timestamp counters are resolved. AfterCompletion uses a separate
/// submission once the frame finishes, avoiding stale Metal stage counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolve {
    InFrame,
    AfterCompletion,
}

/// A measured frame, even when its counters were invalid or mapping failed.
#[derive(Clone, Copy, Debug)]
pub struct FrameTime {
    pub frame: usize,
    pub ms: Option<f64>,
}

#[derive(Clone, Copy)]
struct Measurement {
    frame: usize,
    generation: u64,
}

#[derive(Default)]
struct Samples {
    generation: u64,
    times: VecDeque<f64>,
}

impl Samples {
    fn begin(&self, frame: usize) -> Measurement {
        Measurement {
            frame,
            generation: self.generation,
        }
    }

    fn complete(&mut self, measurement: Measurement, ms: Option<f64>) -> FrameTime {
        if measurement.generation == self.generation {
            if let Some(ms) = ms {
                self.times.push_back(ms);
                if self.times.len() > WINDOW {
                    self.times.pop_front();
                }
            }
        }
        FrameTime {
            frame: measurement.frame,
            ms,
        }
    }

    fn reset(&mut self) {
        self.generation += 1;
        self.times.clear();
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

    fn map(
        &self,
        measurement: Measurement,
        period: f64,
        samples: Arc<Mutex<Samples>>,
        completion: Arc<Mutex<Completion>>,
    ) {
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
            let sample = samples
                .lock()
                .expect("GPU timing samples")
                .complete(measurement, ms);
            completion.lock().expect("GPU timing completion").sample = Some(sample);
            slot.busy.store(false, Ordering::Release);
        });
    }
}

#[derive(Default)]
struct Completion {
    resolve: Option<SubmissionIndex>,
    sample: Option<FrameTime>,
}

/// The result belongs to the frame passed to begin, independent of completion
/// order and later resets. Dropping this handle does not cancel rolling stats.
#[derive(Clone)]
pub struct PendingFrame {
    render: SubmissionIndex,
    ready: Arc<AtomicBool>,
    completion: Arc<Mutex<Completion>>,
    slot: Slot,
    measurement: Measurement,
    period: f64,
    samples: Arc<Mutex<Samples>>,
    device: Device,
    queue: Queue,
}

impl PendingFrame {
    // Queue completion callbacks require Send even on WebGPU, whose device
    // handles are thread-local. The callback only signals; a later frame (or
    // native's explicit wait) submits telemetry without holding a live borrow.
    fn resolve(&self) -> bool {
        if !self.ready.load(Ordering::Acquire) {
            return false;
        }
        let mut completion = self.completion.lock().expect("GPU timing completion");
        if completion.resolve.is_none() {
            let mut encoder = self
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("frame-timer-resolve"),
                });
            self.slot.resolve(&mut encoder);
            completion.resolve = Some(self.queue.submit([encoder.finish()]));
            drop(completion);
            self.slot.map(
                self.measurement,
                self.period,
                self.samples.clone(),
                self.completion.clone(),
            );
        }
        true
    }

    /// Native's explicit profiling wait: first the render, then its telemetry
    /// submission. Browser callers drop the handle and never poll or wait.
    pub fn wait(self) -> Result<FrameTime, String> {
        let wait = |index| {
            self.device
                .poll(PollType::Wait {
                    submission_index: Some(index),
                    timeout: None,
                })
                .map_err(|e| e.to_string())
        };
        wait(self.render.clone())?;
        self.resolve();
        let resolve = self
            .completion
            .lock()
            .expect("GPU timing completion")
            .resolve
            .clone();
        if let Some(index) = resolve {
            wait(index)?;
        }
        self.completion
            .lock()
            .expect("GPU timing completion")
            .sample
            .ok_or_else(|| "GPU timing readback did not complete".into())
    }
}

/// Three readbacks bound pending work; a saturated frame goes unmeasured.
/// Allocation is opt-in, so ordinary rendering pays no instrumentation cost.
pub struct FrameTimer {
    slots: Vec<Slot>,
    device: Device,
    queue: Queue,
    mode: Resolve,
    marker: TextureView,
    pending: Option<(usize, Measurement)>,
    resolving: Vec<PendingFrame>,
    period: f64,
    samples: Arc<Mutex<Samples>>,
}

impl FrameTimer {
    pub fn new(device: &Device, queue: &Queue, mode: Resolve) -> Option<Self> {
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
                queries: device.create_query_set(&QuerySetDescriptor {
                    label: Some("frame-timer"),
                    ty: QueryType::Timestamp,
                    count: 2,
                }),
                resolve: buffer(
                    "frame-timer-resolve",
                    BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
                ),
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
            device: device.clone(),
            queue: queue.clone(),
            mode,
            marker,
            pending: None,
            resolving: Vec::new(),
            period: f64::from(queue.get_timestamp_period()),
            samples: Arc::default(),
        })
    }

    /// Reserve a slot and capture the frame identity/reset window. Pair every
    /// begin with end and submitted; saturation skips the measurement only.
    pub fn begin(&mut self, encoder: &mut CommandEncoder, frame: usize) {
        self.resolving.retain(|pending| {
            pending
                .completion
                .lock()
                .expect("GPU timing completion")
                .resolve
                .is_none()
        });
        self.pending = self
            .slots
            .iter()
            .position(|slot| !slot.busy.swap(true, Ordering::AcqRel))
            .map(|slot| {
                (
                    slot,
                    self.samples
                        .lock()
                        .expect("GPU timing samples")
                        .begin(frame),
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
                query_set: &self.slots[self.pending.expect("timed frame").0].queries,
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
        if self.mode == Resolve::InFrame {
            self.slots[index].resolve(encoder);
        }
    }

    /// Call immediately after submitting the marked frame. In AfterCompletion
    /// mode, advance with resolve_completed or wait on the returned ticket.
    /// Slots stay reserved until their asynchronous map completes.
    pub fn submitted(&mut self, render: SubmissionIndex) -> Option<PendingFrame> {
        let (index, measurement) = self.pending.take()?;
        let result = PendingFrame {
            render,
            ready: Arc::default(),
            completion: Arc::default(),
            slot: self.slots[index].clone(),
            measurement,
            period: self.period,
            samples: self.samples.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
        };
        match self.mode {
            Resolve::InFrame => result.slot.map(
                measurement,
                self.period,
                self.samples.clone(),
                result.completion.clone(),
            ),
            Resolve::AfterCompletion => {
                let ready = result.ready.clone();
                self.queue
                    .on_submitted_work_done(move || ready.store(true, Ordering::Release));
                self.resolving.push(result.clone());
            }
        }
        Some(result)
    }

    /// Submit any post-completion resolves whose render has finished, without
    /// waiting. Browser frame loops call this; native resolves in its wait so
    /// telemetry submissions stay outside the timed CPU phases.
    pub fn resolve_completed(&mut self) {
        self.resolving.retain(|pending| !pending.resolve());
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
}

impl FrameTimer {
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
    fn delayed_samples_keep_their_frame_and_reset_window() {
        let mut samples = Samples::default();
        let first = samples.begin(17);
        let delayed = samples.begin(18);
        let newest = samples.begin(19);
        // Completion order need not be frame order.
        let result = samples.complete(newest, Some(3.0));
        assert_eq!((result.frame, result.ms), (19, Some(3.0)));
        let result = samples.complete(first, Some(1.0));
        assert_eq!((result.frame, result.ms), (17, Some(1.0)));
        samples.reset();
        let current = samples.begin(20);
        let result = samples.complete(current, Some(4.0));
        assert_eq!((result.frame, result.ms), (20, Some(4.0)));
        let result = samples.complete(delayed, Some(2.0));
        assert_eq!((result.frame, result.ms), (18, Some(2.0)));
        assert_eq!(samples.times.iter().copied().collect::<Vec<_>>(), [4.0]);
    }
}
