//! Opt-in GPU timestamps and snapshots of renderer-owned memory.
use super::*;

#[wasm_bindgen(inline_js = "export function report_panic(message) {
  setTimeout(() => { throw new Error(message); });
}")]
extern "C" {
    fn report_panic(message: &str);
}

/// Report the original panic before its trap leaves exported handles borrowed.
#[wasm_bindgen(start)]
pub fn report_panics() {
    std::panic::set_hook(Box::new(|info| {
        report_panic(&format!("presentation/renderer panic: {info}"));
    }));
}

#[wasm_bindgen]
impl Renderer {
    /// Enable the same whole-frame timestamp markers as the browser control.
    /// Returns false when timestamps are unsupported or profiling is disabled.
    pub fn set_gpu_timing(&mut self, enabled: bool) -> bool {
        if !enabled {
            self.gpu_timer = None;
        } else if self.gpu_timer.is_none() {
            self.gpu_timer =
                renderer_core::timing::FrameTimer::new(&self.gpu.device, &self.gpu.queue);
        }
        self.gpu_timer.is_some()
    }

    pub fn reset_gpu_timing(&mut self) {
        if let Some(timer) = &mut self.gpu_timer {
            timer.reset();
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MemoryStats {
    pub(super) gpu: renderer_core::memory::AllocationCounts,
    cpu_snapshot_payload_bytes: usize,
    cpu_bundle_payload_bytes: usize,
}

impl Renderer {
    pub(super) fn memory_snapshot(&self) -> MemoryStats {
        let mut memory = renderer_core::memory::ResourceMemory::default();
        if let Some(runtime) = &self.runtime {
            runtime.record_memory(&mut memory);
        }
        let player_bytes = self.player_meshes.record_memory(&mut memory);
        memory.buffer(&self.overlay_instances);
        if let Some(timer) = &self.gpu_timer {
            timer.record_memory(&mut memory);
        }
        let snapshot_bytes = self.geometry.as_ref().map_or(0, |g| g.payload_bytes()) + player_bytes;
        MemoryStats {
            gpu: memory.counts(),
            cpu_snapshot_payload_bytes: snapshot_bytes,
            cpu_bundle_payload_bytes: self.model_library.payload_bytes()
                + self.effect_sheets.as_ref().map_or(0, |s| s.payload_bytes()),
        }
    }
}
