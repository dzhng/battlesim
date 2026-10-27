use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Recreate the complete renderer after a WebGPU device loss.
    ///
    /// A lost device invalidates every pipeline, bind group, buffer and target
    /// view. Reconfiguring the old surface cannot make those handles usable,
    /// so this swaps in a fresh renderer built through the same constructor
    /// used at startup, then reuploads the cached CPU asset snapshot.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub async fn recover_device(&mut self, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let previous_stats = self.stats;
        let previous_lost = self.device_lost.load(Ordering::Relaxed);
        let snapshot = self.asset_snapshot.clone();
        let ground = self.ground_snapshot.clone();
        let presentation = self.presentation_snapshot.clone();
        let mut replacement = Self::new(canvas).await?;
        replacement.stats = previous_stats;
        replacement.stats.device_recoveries = replacement.stats.device_recoveries.saturating_add(1);
        replacement
            .device_lost
            .store(previous_lost, Ordering::Relaxed);
        if let Some(snapshot) = snapshot {
            replacement.load_bundle_bytes(snapshot.name, snapshot.bundle, snapshot.textures)?;
        }
        if let Some(ground) = ground {
            replacement.upload_shared_ground(&ground)?;
        }
        if let Some(presentation) = presentation {
            replacement.upload_presentation_frame(
                &presentation.camera,
                &presentation.environment,
                &presentation.fog,
                &presentation.overlay,
            )?;
            if !presentation.shadow.is_empty() {
                replacement.upload_shadow_frame(
                    &presentation.shadow,
                    &presentation.casters,
                    &presentation.batches,
                )?;
            }
        }
        *self = replacement;
        Ok(())
    }
}
