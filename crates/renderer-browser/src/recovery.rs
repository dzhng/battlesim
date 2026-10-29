use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Recreate the device asynchronously. The next synchronous render borrows
    /// current presentation to rebuild resources and refresh canonical knowledge.
    pub async fn recover_device(
        &mut self,
        gpu: &BrowserGpu,
        canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        let mut replacement = Self::admit(
            gpu.inner.clone(),
            canvas,
            self.high_surface.as_ref().map(|high| high.canvas.clone()),
            self.config.clone(),
        )
        .await?;
        replacement.frames = self.frames;
        replacement.set_gpu_timing(self.gpu_timer.is_some());
        replacement.model_library = self.model_library.clone();
        replacement.geometry = self.geometry.clone();
        replacement.effect_sheets = self.effect_sheets.clone();
        replacement.overlay_glow = self.overlay_glow;
        replacement.player_meshes = self
            .player_meshes
            .recreate(&replacement.gpu.device, &replacement.gpu.queue);
        *self = replacement;
        Ok(())
    }
}
