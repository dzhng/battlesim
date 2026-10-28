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
        let timing_enabled = self.gpu_timer.is_some();
        let lighting = self.lighting;
        let previous_lost = self.device_lost.load(Ordering::Relaxed);
        let library = self.model_library.clone();
        let model_world = self.model_world.clone();
        let model_frame = self.model_frame.clone();
        let scenery_world = self.scenery_world.clone();
        let ground = self.ground_snapshot.clone();
        let grass = self.grass_snapshot.clone();
        let post_bloom = self.post_bloom.clone();
        let post_grade = self.post_grade.clone();
        let effect_layers = self.shared_effects.layers().to_vec();
        let presentation = self.presentation_snapshot.clone();
        let mut replacement = Self::new(
            canvas,
            bytemuck::cast_slice(&[lighting.sky]).to_vec(),
            bytemuck::cast_slice(&[lighting.sun]).to_vec(),
            bytemuck::cast_slice(&[lighting.aerial]).to_vec(),
        )
        .await?;
        replacement.set_gpu_timing(timing_enabled);
        replacement.stats = previous_stats;
        replacement.stats.device_recoveries = replacement.stats.device_recoveries.saturating_add(1);
        replacement
            .device_lost
            .store(previous_lost, Ordering::Relaxed);
        replacement.model_library = library;
        if let Some(world) = model_world {
            replacement.install_model_world(world)?;
        }
        if let Some(scenery) = scenery_world {
            replacement.install_scenery_world(scenery)?;
        }
        if let Some(frame) = model_frame {
            replacement.restore_model_frame(frame)?;
        }
        for (layer, bytes) in effect_layers.iter().enumerate() {
            if !bytes.is_empty() {
                replacement.upload_effect_atlas(layer as u32, bytes)?;
            }
        }
        if let Some(ground) = ground {
            replacement.install_ground_world(ground)?;
        }
        if let Some(grass) = grass {
            replacement.install_grass_world(grass)?;
        }
        if let Some(presentation) = presentation {
            replacement.upload_frame(
                &presentation.camera,
                &presentation.view,
                &presentation.fog,
                &presentation.overlay,
                &presentation.lights,
                presentation.light_wrap,
            )?;
            replacement.upload_effects(&presentation.effects)?;
            if !presentation.shadow.is_empty() {
                replacement.upload_shadow(
                    &presentation.shadow,
                    &presentation.casters,
                    &presentation.batches,
                )?;
            }
        }
        if !post_bloom.is_empty() || !post_grade.is_empty() {
            replacement.upload_post(&post_bloom, &post_grade)?;
        }
        *self = replacement;
        Ok(())
    }
}
