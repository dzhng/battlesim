use super::*;
#[wasm_bindgen]
impl Renderer {
    pub fn frame_stats(&self) -> Result<String, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before inspecting frame stats"))?;
        let mut stats = runtime.frame_stats();
        stats["frames"] = self.stats.frames.into();
        stats["memory"] = serde_json::to_value(self.memory_snapshot().gpu).map_err(js_error)?;
        stats["gpu"] = serde_json::to_value(self.gpu_timer.as_ref().map(|timer| timer.stats()))
            .map_err(js_error)?;
        serde_json::to_string(&stats).map_err(js_error)
    }

    pub fn drawn_instances(&self) -> Result<String, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before inspecting drawn instances"))?;
        serde_json::to_string(&runtime.drawn_instances()).map_err(js_error)
    }

    pub fn stats(&self) -> String {
        let mut stats = serde_json::to_value(self.stats).expect("renderer counters serialize");
        let fields = stats
            .as_object_mut()
            .expect("renderer counters are an object");
        fields.extend(serde_json::json!({
            "playerGeometryVertices": self.player_meshes.vertex_counts(),
            "xrayMeshes": self.runtime.as_ref().is_some_and(|r| r.models.has_xray_meshes()),
            "separateHighAir": self.high_surface.is_some(),
            "preparation": self.runtime.as_ref().map(|r| r.preparation_stats()),
            "fogSightUploads": self.stats.fog_sight_uploads + self.runtime.as_ref().map_or(0, |r| r.fog.upload_count() as u32),
            "fogTablesReady": self.runtime.as_ref().is_some_and(|r| r.fog.has_complete_tables()),
            "deviceLost": self.previous_device_losses + self.gpu.losses(),
            "width": self.width,
            "height": self.height,
            "sharedTargets": self.runtime.is_some(),
            "sharedDepth": self.runtime.is_some(),
            "sharedWorld": self.runtime.is_some(),
            "sharedFogCompute": self.runtime.is_some(),
            "asset": self.asset_report.as_ref().map(|r| serde_json::from_str::<serde_json::Value>(r).expect("stored asset report is JSON")),
        }).as_object().expect("diagnostic fields are an object").clone());
        serde_json::to_string(&stats).expect("renderer counters serialize")
    }
}

impl Renderer {
    pub(super) fn update_stats(&mut self, presentation: &presentation::presentation::Presentation) {
        let runtime = self.runtime.as_ref().expect("installed runtime");
        let f = presentation.last_frame();
        let s = &mut self.stats;
        s.model_vertices = runtime.models.resident_vertices();
        s.presentation_material_words = runtime.models.material_words();
        s.shared_scenery = true;
        s.scenery_vertices = runtime.scenery.resident_vertices();
        s.scenery_runs = runtime.scenery.run_count();
        s.shadow_scenery_batches = runtime.scenery.caster_run_counts();
        s.presentation_model_runs = runtime.models.run_count();
        s.shadow_caster_batches = runtime.models.caster_run_counts();
        s.presentation_model_words = (size_of_val(f.palettes.as_slice())
            + size_of_val(f.instances.as_slice())
            + size_of_val(f.batches.as_slice())) as u32
            / 4;
        s.presentation_effect_words = size_of_val(presentation.effects()) as u32 / 4;
        s.fog_height_words = (runtime.ground.heights.size() / 4) as u32;
        s.presentation_preparations += 1;
        s.presentation_bound = true;
        s.presentation_camera_words = size_of_val(&f.camera) as u32 / 4;
        s.presentation_environment_words = (runtime.environment.data.size() / 4) as u32;
        s.presentation_fog_words = size_of_val(&self.config.fog_look) as u32 / 4;
        s.presentation_overlay_words = self.overlay_glow.len() as u32;
        s.shared_terrain = true;
        s.shared_materials = true;
        s.shared_shadow = true;
        s.shared_environment = true;
        s.shared_effects = true;
        s.shared_fog_sight = true;
        s.shared_model_prepass = true;
        s.target_generation = runtime.targets.generation();
        s.shadow_passes += runtime.shadow.active() as u32;
        s.shadow_model_passes += s
            .shadow_caster_batches
            .iter()
            .take(runtime.shadow.active())
            .filter(|&&n| n > 0)
            .count() as u32;
    }
}
