//! Native's half of the frame-rate comparison (`web/fps-compare.mjs`): a
//! replay's frame records drawn back to back, each through the whole pass
//! list, timed as the browser's graphics test times its reel. Each frame
//! records its interval (start to next start), its CPU time (framing,
//! preparing, encoding and submitting) and its GPU time between
//! the shared core timer's marker passes. Its AfterCompletion mode resolves
//! Metal stage counters in a separate telemetry submission after the frame
//! completes. The report is the browser
//! file's shape with `client: "native"`.
//!
//! The bench skips window presentation and holds at most `IN_FLIGHT` frames on
//! the GPU, as a presenting swap chain would. Intervals include telemetry waits;
//! timed CPU phases end at the render submission and exclude those waits.
use std::collections::VecDeque;
use std::time::Instant;

use renderer_core::timing::{FrameTimer, PendingFrame, Resolve};
use serde_json::{json, Value};
use wgpu::*;

/// Drawn frames left out of the timing: the browser's `WARMUP_FRAMES`.
pub const WARMUP_FRAMES: usize = 60;
/// Frames submitted and not yet finished, at most (the surface's
/// `desired_maximum_frame_latency`).
pub const IN_FLIGHT: usize = 2;
/// The CPU frame's parts, in order.
pub const PHASES: [&str; 4] = ["frame", "prepare", "encode", "submit"];

/// Draws with both a completed warm-up and a following frame interval.
pub fn reported_draws(draws: usize) -> Result<std::ops::Range<usize>, String> {
    let end = draws.saturating_sub(1);
    if end <= WARMUP_FRAMES {
        return Err(format!(
            "{draws} frames: none after the {WARMUP_FRAMES}-frame warm-up and final interval boundary"
        ));
    }
    Ok(WARMUP_FRAMES..end)
}

/// One timed frame.
struct Sample {
    interval_ms: f64,
    cpu_ms: f64,
    gpu_ms: Option<f64>,
    phases_ms: [f64; PHASES.len()],
}

/// The frames of one bench run.
pub struct Bench {
    timer: Option<FrameTimer>,
    in_flight: VecDeque<(usize, SubmissionIndex, Option<PendingFrame>)>,
    samples: Vec<Sample>,
    /// The frame being drawn, its start and its phase marks.
    frame: usize,
    started: Option<Instant>,
    marks: Vec<Instant>,
}

impl Bench {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            timer: FrameTimer::new(device, queue, Resolve::AfterCompletion),
            in_flight: VecDeque::new(),
            samples: Vec::new(),
            frame: 0,
            started: None,
            marks: Vec::new(),
        }
    }

    /// A frame starts: its interval runs from the last frame's start.
    pub fn start(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.started.replace(now) {
            let interval_ms = ms(now - last);
            if let Some(s) = self.samples.last_mut() {
                s.interval_ms = interval_ms;
            }
        }
        self.marks.clear();
    }

    /// The end of the current phase (`PHASES`, in order).
    pub fn mark(&mut self) {
        self.marks.push(Instant::now());
    }

    /// Opens the frame in its first encoder, including any regional submissions.
    pub fn begin(&mut self, encoder: &mut CommandEncoder) {
        let Some(timer) = &mut self.timer else { return };
        timer.begin(encoder, self.frame);
    }

    /// Closes the frame's GPU time at the end of `encoder`.
    pub fn end(&self, encoder: &mut CommandEncoder) {
        let Some(timer) = &self.timer else { return };
        timer.end(encoder);
    }

    /// The frame was submitted as `index`: its CPU time
    /// ends here. Then waits until no more than `IN_FLIGHT` frames are
    /// unfinished, collecting the finished frames' GPU times.
    pub fn submitted(&mut self, device: &Device, index: SubmissionIndex) -> Result<(), String> {
        self.mark();
        let start = self.started.ok_or("a frame submitted before it started")?;
        let mut phases_ms = [0.0; PHASES.len()];
        let mut from = start;
        for (phase, &at) in phases_ms.iter_mut().zip(&self.marks) {
            *phase = ms(at - from);
            from = at;
        }
        self.samples.push(Sample {
            interval_ms: 0.0,
            cpu_ms: ms(from - start),
            gpu_ms: None,
            phases_ms,
        });
        let pending = self
            .timer
            .as_mut()
            .and_then(|timer| timer.submitted(index.clone()));
        self.in_flight.push_back((self.frame, index, pending));
        self.frame += 1;
        while self.in_flight.len() > IN_FLIGHT {
            self.finish_oldest(device)?;
        }
        Ok(())
    }

    fn finish_oldest(&mut self, device: &Device) -> Result<(), String> {
        let Some((frame, index, pending)) = self.in_flight.pop_front() else {
            return Ok(());
        };
        if let Some(pending) = pending {
            let sample = pending
                .wait()
                .map_err(|e| format!("timing frame {frame}: {e}"))?;
            self.samples[sample.frame].gpu_ms = sample.ms;
        } else {
            device
                .poll(PollType::Wait {
                    submission_index: Some(index),
                    timeout: None,
                })
                .map_err(|e| format!("waiting on frame {frame}: {e}"))?;
        }
        Ok(())
    }

    /// Finishes every frame and writes the report for `scene`: the browser
    /// file's shape (`graphics-fps.mjs`), its frames after the warm-up.
    /// The last frame has no interval (nothing started after it), so it is
    /// left out.
    pub fn report(
        &mut self,
        device: &Device,
        scene: &str,
        size: [u32; 2],
        conditions: Value,
    ) -> Result<Value, String> {
        while !self.in_flight.is_empty() {
            self.finish_oldest(device)?;
        }
        let timed = &self.samples[reported_draws(self.samples.len())?];
        let mut gpu: Vec<f64> = timed.iter().filter_map(|s| s.gpu_ms).collect();
        gpu.sort_by(f64::total_cmp);
        let measured_gpu_frames = gpu.len();
        let gpu = (gpu.len() == timed.len()).then(|| {
            json!({
                "frames": gpu.len(),
                "meanMs": gpu.iter().sum::<f64>() / gpu.len() as f64,
                "p95Ms": gpu[((gpu.len() - 1) as f64 * 0.95) as usize],
            })
        });
        let phases: serde_json::Map<String, Value> = PHASES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let ms: Vec<f64> = timed.iter().map(|s| s.phases_ms[i]).collect();
                ((*name).to_owned(), json!(ms))
            })
            .collect();
        Ok(json!({
            "client": "native",
            "conditions": conditions,
            "viewport": { "width": size[0], "height": size[1] },
            "scenes": [{
                "scene": scene,
                "warmupFrames": WARMUP_FRAMES,
                "intervalsMs": timed.iter().map(|s| s.interval_ms).collect::<Vec<_>>(),
                "cpuMs": timed.iter().map(|s| s.cpu_ms).collect::<Vec<_>>(),
                "gpu": gpu,
                "gpuMs": timed.iter().map(|s| s.gpu_ms).collect::<Vec<_>>(),
                "gpuCoverage": { "expected": timed.len(), "measured": measured_gpu_frames },
                "gpuTiming": self.timer.as_ref().map(|_| "render-pass stage counters; post-completion resolve submission and wait"),
                "phasesMs": phases,
            }],
        }))
    }
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// macOS's power source; "unknown" where `pmset` is absent (another OS).
pub fn power_source() -> String {
    std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .ok()
        .and_then(|o| {
            let text = String::from_utf8_lossy(&o.stdout).into_owned();
            let rest = text.split("drawing from '").nth(1)?;
            Some(rest.split('\'').next()?.to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// A report's numbers in a line: average FPS, CPU ms p50 and p95 with each
/// phase's median, the GPU ms, and what the frames drew on average.
pub fn summary(report: &Value) -> String {
    let scene = &report["scenes"][0];
    let sorted = |v: &Value| {
        let mut v: Vec<f64> = v
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_f64)
            .collect();
        v.sort_by(f64::total_cmp);
        v
    };
    // Nearest rank, as `summarize` in `recording.ts`.
    let rank = |v: &[f64], q: f64| v[((v.len() as f64 * q).ceil() as usize).clamp(1, v.len()) - 1];
    let intervals = sorted(&scene["intervalsMs"]);
    let cpu = sorted(&scene["cpuMs"]);
    let phases: Vec<String> = PHASES
        .iter()
        .map(|name| format!("{name} {:.2}", rank(&sorted(&scene["phasesMs"][name]), 0.5)))
        .collect();
    format!(
        "bench {}: {} frames, {:.1} FPS average, CPU ms p50 {:.2} p95 {:.2} (medians: {}), GPU {}; drew on average {}",
        scene["scene"].as_str().unwrap_or("?"),
        intervals.len(),
        1000.0 * intervals.len() as f64 / intervals.iter().sum::<f64>(),
        rank(&cpu, 0.5),
        rank(&cpu, 0.95),
        phases.join(", "),
        scene["gpu"],
        report["conditions"]["meanDrawn"],
    )
}

#[cfg(test)]
mod tests {
    use super::reported_draws;

    #[test]
    fn reported_draws_keep_the_first_complete_interval_through_the_penultimate_draw() {
        let indices: Vec<_> = reported_draws(181).unwrap().collect();
        assert_eq!(indices.len(), 120);
        assert_eq!(indices.first(), Some(&60));
        assert_eq!(indices.last(), Some(&179));
        assert!(reported_draws(61).is_err());
        assert_eq!(reported_draws(62).unwrap().collect::<Vec<_>>(), [60]);
    }
}
