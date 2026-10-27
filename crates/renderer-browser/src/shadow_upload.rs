use super::*;

/// Upload the presentation's fitted sun receiver, cascade cameras and batch
/// admission. Presentation owns the fit and spatial culling; this shell only
/// copies the records into the shared shadow owner and retains the routing
/// decision for the eventual caster draw pass.
#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_shadow_frame(
        &mut self,
        receiver: &[f32],
        casters: &[f32],
        batches: &[u32],
    ) -> Result<(), JsValue> {
        if receiver.len() < renderer_core::shadow::RECEIVER_BYTES / 4 {
            return Err(js_error("presentation sun shadow record needs 100 words"));
        }
        if casters.len() > renderer_core::shadow::CASCADES * CAMERA_BYTES / 4
            || casters.len() % (CAMERA_BYTES / 4) != 0
        {
            return Err(js_error(
                "presentation caster records need 40 words per cascade",
            ));
        }
        let cameras: Vec<&[u8]> = casters
            .chunks_exact(CAMERA_BYTES / 4)
            .map(bytemuck::cast_slice)
            .collect();
        self.shared_shadow.prepare(
            &self.queue,
            bytemuck::cast_slice(&receiver[..renderer_core::shadow::RECEIVER_BYTES / 4]),
            &cameras,
        );
        self.shadow_routing = CasterRouting::from_batch_words(batches, 12, 3, 4, 9, 1, u32::MAX);
        self.stats.shadow_caster_batches = self.shadow_routing.counts();
        let Some(snapshot) = &mut self.presentation_snapshot else {
            return Err(js_error("upload presentation records before shadow frame"));
        };
        snapshot.shadow = receiver[..renderer_core::shadow::RECEIVER_BYTES / 4].to_vec();
        snapshot.casters = casters.to_vec();
        snapshot.batches = batches.to_vec();
        Ok(())
    }
}
