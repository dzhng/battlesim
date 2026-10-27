use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Upload the packed presentation camera/environment/fog/overlay records.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_presentation_frame(
        &mut self,
        camera: &[f32],
        environment: &[f32],
        fog: &[f32],
        overlay: &[f32],
    ) -> Result<(), JsValue> {
        if camera.len() < 40 {
            return Err(js_error("presentation camera record needs 40 words"));
        }
        let write = |queue: &Queue, buffer: &Buffer, words: &[f32], label: &str| {
            let bytes = bytemuck::cast_slice(words);
            if bytes.len() > 256 {
                return Err(js_error(format!(
                    "presentation {label} record exceeds 256 bytes"
                )));
            }
            if !bytes.is_empty() {
                queue.write_buffer(buffer, 0, bytes);
            }
            Ok(words.len() as u32)
        };
        self.queue.write_buffer(
            &self.presentation_camera,
            0,
            bytemuck::cast_slice(&camera[..40]),
        );
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera[..16]));
        let environment_words = write(
            &self.queue,
            &self.presentation_environment,
            environment,
            "environment",
        )?;
        self.shared_fog_mask
            .write_look(&self.queue, bytemuck::cast_slice(fog))
            .map_err(js_error)?;
        let overlay_words = write(&self.queue, &self.presentation_overlay, overlay, "overlay")?;
        self.stats.presentation_uploads = self.stats.presentation_uploads.saturating_add(1);
        self.stats.presentation_bound = true;
        self.stats.presentation_camera_words = 40;
        self.stats.presentation_environment_words = environment_words;
        self.stats.presentation_fog_words = fog.len() as u32;
        self.stats.presentation_overlay_words = overlay_words;
        self.presentation_snapshot = Some(PresentationSnapshot {
            camera: camera[..40].to_vec(),
            environment: environment.to_vec(),
            fog: fog.to_vec(),
            overlay: overlay.to_vec(),
            shadow: Vec::new(),
            casters: Vec::new(),
            batches: Vec::new(),
        });
        Ok(())
    }

    /// Upload presentation terrain as xyz/normal/rgba records.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_shared_ground(&mut self, mesh: &[f32]) -> Result<(), JsValue> {
        if !mesh.len().is_multiple_of(10) {
            return Err(js_error("shared ground mesh needs ten floats per vertex"));
        }
        let mut positions = Vec::with_capacity(mesh.len() / 10 * 3);
        let mut colours = Vec::with_capacity(mesh.len() / 10 * 4);
        let mut indices = Vec::with_capacity(mesh.len() / 10);
        for (index, vertex) in mesh.chunks_exact(10).enumerate() {
            positions.extend_from_slice(&vertex[..3]);
            colours.extend_from_slice(&[
                (vertex[6].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex[7].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex[8].clamp(0.0, 1.0) * 255.0) as u8,
                (vertex[9].clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            indices.push(index as u32);
        }
        self.shared_ground = Some(
            Model::from_indexed_fitted(
                &self.device,
                &self.queue,
                "shared-presentation-ground",
                &positions,
                &colours,
                &indices,
            )
            .map_err(js_error)?,
        );
        self.ground_snapshot = Some(mesh.to_vec());
        Ok(())
    }
}
