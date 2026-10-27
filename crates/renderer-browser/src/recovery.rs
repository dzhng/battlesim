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
        let static_world = self.static_world_snapshot.clone();
        let terrain = self.terrain_snapshot.clone();
        let post_bloom = self.post_bloom.clone();
        let post_grade = self.post_grade.clone();
        let fog_heights = self.fog_height_snapshot.clone();
        let presentation = self.presentation_snapshot.clone();
        let model_words = presentation
            .as_ref()
            .map(|p| (p.palettes.clone(), p.instances.clone(), p.batches.clone()));
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
            replacement.upload_ground(&ground)?;
        }
        if let Some(static_world) = static_world {
            replacement.upload_static_world(&static_world)?;
        }
        if let Some(terrain) = terrain {
            replacement.upload_terrain_tables(
                &terrain.params,
                &terrain.nodes,
                &terrain.plots,
                &terrain.surfaces,
                &terrain.surface_index,
                &terrain.scars,
            )?;
        }
        if let Some(presentation) = presentation {
            replacement.model_batch_state = presentation.batch_state.clone();
            replacement.upload_frame(
                &presentation.camera,
                &presentation.environment,
                &presentation.fog,
                &presentation.overlay,
            )?;
            replacement.upload_effects(&presentation.effects)?;
            if !presentation.sky.is_empty() {
                replacement.upload_sky(&presentation.sky)?;
            }
            if !presentation.shadow.is_empty() {
                replacement.upload_shadow(
                    &presentation.shadow,
                    &presentation.casters,
                    &presentation.batches,
                )?;
            }
            if let Some((palettes, instances, batches)) = model_words {
                replacement.upload_model_frame(&palettes, &instances, &batches, &[])?;
            }
        }
        if !post_bloom.is_empty() || !post_grade.is_empty() {
            replacement.upload_post(&post_bloom, &post_grade)?;
        }
        if let Some(heights) = fog_heights {
            replacement.upload_fog_heights(&heights)?;
        }
        *self = replacement;
        Ok(())
    }
}
