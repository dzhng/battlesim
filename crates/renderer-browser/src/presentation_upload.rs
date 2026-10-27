use super::*;

const ENVIRONMENT_MAX_MIP: f32 = 6.0;

#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_fog_heights(&mut self, heights: &[u32]) -> Result<(), JsValue> {
        self.shared_fog_sight
            .upload_heights(&self.device, &self.queue, bytemuck::cast_slice(heights))
            .map_err(js_error)?;
        self.fog_height_snapshot = Some(heights.to_vec());
        self.stats.fog_height_words = heights.len() as u32;
        Ok(())
    }

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

    /// Upload the packed presentation camera/environment/fog/overlay records.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_frame(
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
        let mut environment_data = environment.to_vec();
        if environment_data.len() >= 29 {
            environment_data[28] = ENVIRONMENT_MAX_MIP;
        }
        let environment_words = write(
            &self.queue,
            &self.presentation_environment,
            &environment_data,
            "environment",
        )?;
        self.shared_fog_mask
            .write_look(&self.queue, bytemuck::cast_slice(fog))
            .map_err(js_error)?;
        let overlay_words = write(&self.queue, &self.presentation_overlay, overlay, "overlay")?;
        let glow = if overlay.len() >= 2 {
            [overlay[0], overlay[0] / 2.5, overlay[1], 0.0]
        } else {
            [0.0; 4]
        };
        self.shared_overlay_resources
            .write_glow(&self.queue, bytemuck::cast_slice(&glow));
        self.stats.presentation_uploads = self.stats.presentation_uploads.saturating_add(1);
        self.stats.presentation_bound = true;
        self.stats.presentation_camera_words = 40;
        self.stats.presentation_environment_words = environment_words;
        self.stats.presentation_fog_words = fog.len() as u32;
        self.stats.presentation_overlay_words = overlay_words;
        self.presentation_snapshot = Some(PresentationSnapshot {
            camera: camera[..40].to_vec(),
            sky: Vec::new(),
            environment: environment.to_vec(),
            fog: fog.to_vec(),
            overlay: overlay.to_vec(),
            shadow: Vec::new(),
            casters: Vec::new(),
            batches: Vec::new(),
        });
        Ok(())
    }

    /// Bake the resolved presentation sky into the shared environment LUT.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_sky(&mut self, sky: &[f32]) -> Result<(), JsValue> {
        if sky.len() < 40 {
            return Err(js_error("presentation sky record needs 40 words"));
        }
        self.environment
            .bake_sky(&self.device, &self.queue, bytemuck::cast_slice(&sky[..40]));
        if let Some(snapshot) = &mut self.presentation_snapshot {
            snapshot.sky = sky[..40].to_vec();
        }
        Ok(())
    }

    /// Upload presentation terrain as xyz/normal/rgba records.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn upload_ground(&mut self, mesh: &[f32]) -> Result<(), JsValue> {
        if !mesh.len().is_multiple_of(10) {
            return Err(js_error("shared ground mesh needs ten floats per vertex"));
        }
        let mut positions = Vec::with_capacity(mesh.len() / 10 * 3);
        let mut colours = Vec::with_capacity(mesh.len() / 10 * 4);
        let mut indices = Vec::with_capacity(mesh.len() / 10);
        for (index, vertex) in mesh.as_chunks::<10>().0.iter().enumerate() {
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
        let mut shadow = Vec::with_capacity(mesh.len());
        for vertex in mesh.as_chunks::<10>().0 {
            shadow.extend_from_slice(&vertex[..3]);
            shadow.extend_from_slice(&vertex[3..6]);
            shadow.extend_from_slice(&vertex[6..10]);
        }
        let shadow_bytes = if shadow.is_empty() {
            vec![0; 40]
        } else {
            bytemuck::cast_slice(&shadow).to_vec()
        };
        self.shadow_ground_vertices = renderer_core::buffer(
            &self.device,
            "shared-shadow-ground-vertices",
            BufferUsages::VERTEX,
            &shadow_bytes,
        );
        self.shadow_ground_count = (mesh.len() / 10) as u32;
        self.ground_snapshot = Some(mesh.to_vec());
        Ok(())
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
    #[allow(clippy::too_many_arguments)] // The WASM ABI mirrors the packed fog record groups.
    pub fn upload_fog(
        &mut self,
        params: &[f32],
        eyes: &[f32],
        foliage: &[f32],
        occluders: &[f32],
        rebuild: &[u32],
        rebuild_dispatch: &[u32],
        map_words: &[u32],
        tiles: &[u32],
        wholes: &[u32],
        whole_items: &[u32],
        whole_uniform: &[f32],
        whole_dispatch: &[u32],
    ) -> Result<(), JsValue> {
        if params.len() < 64 || map_words.len() < 2 || tiles.len() < 3 {
            return Err(js_error("invalid fog sight record"));
        }
        self.shared_fog_sight
            .upload(
                &self.device,
                &self.queue,
                &renderer_core::fog_sight::FogSightUpload {
                    params: bytemuck::cast_slice(&params[..64]),
                    eyes: bytemuck::cast_slice(eyes),
                    foliage: bytemuck::cast_slice(foliage),
                    occluders: bytemuck::cast_slice(occluders),
                    rebuild: bytemuck::cast_slice(rebuild),
                    map_words: [map_words[0], map_words[1]],
                    tiles: [tiles[0], tiles[1], tiles[2]],
                    wholes,
                    whole_items: bytemuck::cast_slice(whole_items),
                    whole_uniform: bytemuck::cast_slice(whole_uniform),
                },
            )
            .map_err(js_error)?;
        self.fog_rebuild_dispatch =
            (rebuild_dispatch.len() >= 2).then(|| [rebuild_dispatch[0], rebuild_dispatch[1]]);
        self.fog_whole_dispatch =
            (whole_dispatch.len() >= 3).then(|| [whole_dispatch[1], whole_dispatch[2]]);
        self.fog_dispatch_active = true;
        self.stats.fog_sight_uploads = self.stats.fog_sight_uploads.saturating_add(1);
        Ok(())
    }
}
