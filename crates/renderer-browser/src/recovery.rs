use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Recreate the device asynchronously. The next synchronous render borrows
    /// current presentation to rebuild resources and refresh canonical knowledge.
    pub async fn recover_device(&mut self, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let mut replacement = Self::admit(
            canvas,
            self.high_surface.as_ref().map(|high| high.canvas.clone()),
            self.config.clone(),
        )
        .await?;
        replacement.stats = self.stats;
        replacement.stats.device_recoveries += 1;
        replacement
            .device_lost
            .store(self.device_lost.load(Ordering::Relaxed), Ordering::Relaxed);
        replacement.set_gpu_timing(self.gpu_timer.is_some());
        replacement.model_library = self.model_library.clone();
        replacement.geometry = self.geometry.clone();
        replacement.effect_sheets = self.effect_sheets.clone();
        replacement.overlay_glow = self.overlay_glow;
        replacement.asset_report = self.asset_report.clone();
        replacement.player_meshes = self.player_meshes.recreate(&replacement.device);
        *self = replacement;
        Ok(())
    }
}
