use super::*;

/// Upload the presentation's fitted sun receiver, cascade cameras and batch
/// admission. Presentation owns the fit and spatial culling; this shell only
/// copies the records into the shared shadow owner and retains the routing
/// decision for the eventual caster draw pass.
#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_shadow(
        &mut self,
        receiver: &[f32],
        casters: &[f32],
        batches: &[u32],
    ) -> Result<(), JsValue> {
        if receiver.len() < renderer_core::shadow::RECEIVER_BYTES / 4 {
            return Err(js_error("presentation sun shadow record needs 100 words"));
        }
        if casters.len() > renderer_core::shadow::CASCADES * CAMERA_BYTES / 4
            || !casters.len().is_multiple_of(CAMERA_BYTES / 4)
        {
            return Err(js_error(
                "presentation caster records need 40 words per cascade",
            ));
        }
        let cameras: Vec<&[u8]> = casters
            .as_chunks::<{ CAMERA_BYTES / 4 }>()
            .0
            .iter()
            .map(|camera| bytemuck::cast_slice(camera.as_slice()))
            .collect();
        self.shared_shadow.prepare(
            &self.queue,
            bytemuck::cast_slice(&receiver[..renderer_core::shadow::RECEIVER_BYTES / 4]),
            &cameras,
        );
        let Some(snapshot) = &mut self.presentation_snapshot else {
            return Err(js_error("upload presentation records before shadow frame"));
        };
        snapshot.shadow = receiver[..renderer_core::shadow::RECEIVER_BYTES / 4].to_vec();
        snapshot.casters = casters.to_vec();
        snapshot.batches = batches.to_vec();
        Ok(())
    }
}
