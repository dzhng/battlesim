use super::*;
#[wasm_bindgen]
impl Renderer {
    pub fn frame_stats(&self) -> Result<String, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before inspecting frame stats"))?;
        let mut stats = runtime.frame_stats();
        stats["frames"] = self.frames.into();
        stats["memory"] = serde_json::to_value(self.memory_snapshot().gpu).map_err(js_error)?;
        stats["gpu"] = serde_json::to_value(self.gpu_timer.as_ref().map(|timer| timer.stats()))
            .map_err(js_error)?;
        serde_json::to_string(&stats).map_err(js_error)
    }

    pub fn drawn_instances(&self) -> Result<String, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before inspecting drawn instances"))?;
        serde_json::to_string(&runtime.drawn_instances()).map_err(js_error)
    }
}
