use super::*;

#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_post(&mut self, bloom: &[f32], grade: &[f32]) -> Result<(), JsValue> {
        self.shared_post
            .write(
                &self.queue,
                bytemuck::cast_slice(bloom),
                bytemuck::cast_slice(grade),
            )
            .map_err(js_error)?;
        self.post_bloom = bloom.to_vec();
        self.post_grade = grade.to_vec();
        Ok(())
    }

    /// Upload canonical camera/view records, cast lights and screen treatments.
    #[wasm_bindgen]
    pub fn upload_frame(
        &mut self,
        camera: &[f32],
        view: &[f32],
        fog: &[f32],
        overlay: &[f32],
        lights: &[f32],
        light_wrap: f32,
    ) -> Result<(), JsValue> {
        let camera_record = record(camera, "camera")?;
        let view_record = record(view, "view")?;
        let cast_lights = bytemuck::try_cast_slice(lights).map_err(js_error)?;
        self.environment
            .prepare(&self.queue, &view_record, &camera_record);
        self.environment
            .set_cast_lights(&self.queue, cast_lights, light_wrap);
        self.sky.prepare(&self.queue, &view_record);
        self.shared_fog_mask
            .write_look(&self.queue, bytemuck::cast_slice(fog))
            .map_err(js_error)?;
        let glow = if overlay.len() >= 2 {
            [overlay[0], overlay[0] / 2.5, overlay[1], 0.0]
        } else {
            [0.0; 4]
        };
        self.shared_overlay_resources
            .write_glow(&self.queue, bytemuck::cast_slice(&glow));
        self.stats.presentation_uploads = self.stats.presentation_uploads.saturating_add(1);
        self.stats.presentation_bound = true;
        self.stats.presentation_camera_words = camera.len() as u32;
        self.stats.presentation_environment_words = self.environment.data.size() as u32 / 4;
        self.stats.presentation_fog_words = fog.len() as u32;
        self.stats.presentation_overlay_words = overlay.len() as u32;
        self.presentation_snapshot = Some(PresentationSnapshot {
            camera: camera.to_vec(),
            view: view.to_vec(),
            lights: lights.to_vec(),
            light_wrap,
            fog: fog.to_vec(),
            overlay: overlay.to_vec(),
            effects: Vec::new(),
            shadow: Vec::new(),
            casters: Vec::new(),
            batches: Vec::new(),
        });
        self.prepare_ground()?;
        Ok(())
    }

    /// Upload the packed presentation effect instances used by the shared
    /// lit-world effects pass.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_effects(&mut self, effects: &[f32]) -> Result<(), JsValue> {
        if !effects.len().is_multiple_of(16) {
            return Err(js_error("effect instances need sixteen floats each"));
        }
        self.shared_effects
            .prepare(&self.device, &self.queue, bytemuck::cast_slice(effects));
        if let Some(snapshot) = &mut self.presentation_snapshot {
            snapshot.effects = effects.to_vec();
        }
        self.stats.presentation_effect_words = effects.len() as u32;
        Ok(())
    }

    /// Upload one premultiplied RGBA8 flipbook layer into the shared effects
    /// atlas. The browser shell decodes the authored PNG; Rust owns the GPU
    /// texture and its recovery snapshot.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_effect_atlas(&mut self, layer: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.shared_effects
            .upload_layer(&self.queue, layer, rgba)
            .map_err(js_error)
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn overlay(&mut self, mesh: &[f32]) -> Result<(), JsValue> {
        if !mesh.len().is_multiple_of(10) {
            return Err(js_error("shared overlay mesh needs ten floats per vertex"));
        }
        let mut vertices: Vec<f32> = Vec::with_capacity(mesh.len());
        for vertex in mesh.as_chunks::<10>().0 {
            vertices.extend_from_slice(bytemuck::cast_slice(&vertex[..3]));
            vertices.extend_from_slice(bytemuck::cast_slice(&vertex[3..6]));
            vertices.extend_from_slice(bytemuck::cast_slice(&vertex[6..10]));
        }
        let vertex_bytes = if vertices.is_empty() {
            vec![0; 40]
        } else {
            bytemuck::cast_slice(&vertices).to_vec()
        };
        self.overlay_vertices = renderer_core::buffer(
            &self.device,
            "shared-presentation-overlay",
            BufferUsages::VERTEX,
            &vertex_bytes,
        );
        self.overlay_vertex_count = (mesh.len() / 10) as u32;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    /// Upload the canonical presentation fog record across separate WASM memories.
    pub fn upload_fog(&mut self, snapshot: JsValue) -> Result<(), JsValue> {
        let upload: renderer_core::fog_sight::FogUpload =
            serde_wasm_bindgen::from_value(snapshot).map_err(js_error)?;
        self.shared_fog_sight
            .upload(&self.device, &self.queue, &upload);
        self.stats.fog_sight_uploads = self.stats.fog_sight_uploads.saturating_add(1);
        Ok(())
    }
}
