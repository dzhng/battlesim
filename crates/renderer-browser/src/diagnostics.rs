//! Opt-in GPU timestamps and snapshots of renderer-owned memory.
use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Enable the same whole-frame timestamp markers as the browser control.
    /// Returns false when timestamps are unsupported or profiling is disabled.
    pub fn set_gpu_timing(&mut self, enabled: bool) -> bool {
        if !enabled {
            self.gpu_timer = None;
        } else if self.gpu_timer.is_none() {
            self.gpu_timer = renderer_core::timing::FrameTimer::new(&self.device, &self.queue);
        }
        self.gpu_timer.is_some()
    }

    pub fn reset_gpu_timing(&mut self) {
        if let Some(timer) = &mut self.gpu_timer {
            timer.reset();
        }
    }

    pub fn gpu_timing(&self) -> Result<JsValue, JsValue> {
        match &self.gpu_timer {
            Some(timer) => serde_wasm_bindgen::to_value(&timer.stats()).map_err(js_error),
            None => Ok(JsValue::NULL),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStats {
    gpu: renderer_core::memory::AllocationCounts,
    cpu_snapshot_payload_bytes: usize,
    cpu_bundle_payload_bytes: usize,
}

#[wasm_bindgen]
impl Renderer {
    /// GPU buffer/texture ownership plus retained CPU recovery payloads.
    /// CPU payloads cover browser recovery snapshots and decoded asset arrays;
    /// they exclude core selection caches, container/allocator overhead and presentation's
    /// separate WASM memory; GPU counts exclude driver and canvas storage.
    pub fn memory(&self) -> Result<JsValue, JsValue> {
        let mut memory = renderer_core::memory::ResourceMemory::default();
        self.targets.record_memory(&mut memory);
        self.environment.record_memory(&mut memory);
        self.sky.record_memory(&mut memory);
        self.shared_shadow.record_memory(&mut memory);
        self.shared_post.record_memory(&mut memory);
        self.shared_fog_mask.record_memory(&mut memory);
        self.shared_fog_sight.record_memory(&mut memory);
        self.shared_effects.record_memory(&mut memory);
        self.shared_overlay_resources.record_memory(&mut memory);
        memory.buffer(&self.overlay_vertices);
        memory.buffer(&self.overlay_instances);
        if let Some(ground) = &self.ground {
            ground.record_memory(&mut memory);
        }
        if let Some(grass) = &self.grass {
            grass.record_memory(&mut memory);
        }
        if let Some(models) = &self.models {
            models.record_memory(&mut memory);
        }
        if let Some(scenery) = &self.scenery {
            scenery.record_memory(&mut memory);
        }
        if let Some(timer) = &self.gpu_timer {
            timer.record_memory(&mut memory);
        }
        let mut snapshot_bytes = self.model_world.as_ref().map_or(0, |s| s.payload_bytes())
            + self.model_frame.as_ref().map_or(0, |s| s.payload_bytes())
            + self.scenery_world.as_ref().map_or(0, |s| s.payload_bytes())
            + self
                .ground_snapshot
                .as_ref()
                .map_or(0, |s| s.payload_bytes())
            + self
                .grass_snapshot
                .as_ref()
                .map_or(0, |s| s.payload_bytes())
            + size_of_val(self.post_bloom.as_slice())
            + size_of_val(self.post_grade.as_slice())
            + self
                .shared_effects
                .layers()
                .iter()
                .map(Vec::len)
                .sum::<usize>();
        if let Some(presentation) = &self.presentation_snapshot {
            for values in [
                &presentation.camera,
                &presentation.view,
                &presentation.lights,
                &presentation.fog,
                &presentation.overlay,
                &presentation.effects,
                &presentation.shadow,
                &presentation.casters,
            ] {
                snapshot_bytes += size_of_val(values.as_slice());
            }
            snapshot_bytes += size_of_val(presentation.batches.as_slice());
        }
        serde_wasm_bindgen::to_value(&MemoryStats {
            gpu: memory.counts(),
            cpu_snapshot_payload_bytes: snapshot_bytes,
            cpu_bundle_payload_bytes: self.model_library.payload_bytes(),
        })
        .map_err(js_error)
    }
}
