//! External asset/geometry transport and synchronous canonical world admission.
use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Update external world faces on the existing runtime and retain the same
    /// source for recovery. Canonical terrain and appearance owners stay live.
    pub fn set_world_props(&mut self, words: Vec<f32>) -> Result<(), JsValue> {
        if !words.len().is_multiple_of(10) {
            return Err(js_error("world geometry needs ten floats per vertex"));
        }
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| js_error("install presentation before replacing world props"))?;
        self.geometry = None;
        runtime.set_world_props(&self.gpu.device, &self.gpu.queue, words);
        self.geometry = Some(runtime.geometry());
        Ok(())
    }

    /// Only external meshes cross this boundary; ground tables stay in presentation.
    pub fn set_world_geometry(
        &mut self,
        terrain: Vec<f32>,
        skirt: Vec<f32>,
        backdrop: Vec<f32>,
        water: Vec<f32>,
    ) -> Result<(), JsValue> {
        if [&terrain, &skirt, &backdrop, &water]
            .into_iter()
            .any(|mesh| !mesh.len().is_multiple_of(10))
        {
            return Err(js_error("world geometry needs ten floats per vertex"));
        }
        self.geometry = Some(Arc::new(WorldGeometry {
            terrain,
            skirt,
            backdrop,
            water,
        }));
        self.runtime = None;
        self.display_group = None;
        Ok(())
    }

    pub fn load_effect_atlas(&mut self, pngs: js_sys::Array) -> Result<(), JsValue> {
        let bytes: Vec<_> = pngs
            .iter()
            .map(|value| js_sys::Uint8Array::new(&value).to_vec())
            .collect();
        let refs: Vec<_> = bytes.iter().map(Vec::as_slice).collect();
        // The authored atlas side is exported with the production pipeline sizes.
        let side = renderer_core::pipelines::builtin_sizes().effect_sheet as u32;
        self.effect_sheets = Some(Arc::new(
            EffectSheets::decode(side, &refs).map_err(js_error)?,
        ));
        self.runtime = None;
        self.display_group = None;
        Ok(())
    }

    pub fn install_presentation(&mut self, presentation: &mut Presentation) -> Result<(), JsValue> {
        let mut runtime = Runtime::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.config.clone(),
            RuntimeWorld {
                presentation: presentation.core_mut(),
                assets: self.model_library.as_ref(),
                geometry: self
                    .geometry
                    .clone()
                    .ok_or_else(|| js_error("load external world geometry first"))?,
                effect_sheets: self
                    .effect_sheets
                    .clone()
                    .ok_or_else(|| js_error("load effect sheets first"))?,
            },
        )
        .map_err(js_error)?;
        runtime.set_overlay_glow(&self.gpu.queue, self.overlay_glow);
        self.runtime = Some(runtime);
        self.rebind_display();
        presentation.refresh_frame_header();
        Ok(())
    }

    /// Async GPU asset work is separate from synchronous presentation admission.
    pub async fn prepare_assets(&mut self) -> Result<(), JsValue> {
        self.runtime
            .as_mut()
            .ok_or_else(|| js_error("install presentation before preparing assets"))?
            .prepare_assets(&self.gpu.device, &self.gpu.queue)
            .await
            .map_err(js_error)
    }

    pub fn set_controls(&mut self, presentation: &Presentation, json: &str) -> Result<(), JsValue> {
        let controls = serde_json::from_str(json).map_err(js_error)?;
        if let Some(runtime) = &mut self.runtime {
            runtime
                .set_controls(&self.gpu.queue, presentation.core(), controls)
                .map_err(js_error)?;
        }
        self.config.controls = controls;
        Ok(())
    }

    /// Canonical static-batch generation for the next presentation frame.
    pub fn static_generation(&self) -> Result<u32, JsValue> {
        self.runtime
            .as_ref()
            .map(Runtime::static_generation)
            .ok_or_else(|| js_error("install presentation before framing"))
    }

    pub fn set_fog_style(&mut self, style: String) -> Result<(), JsValue> {
        let style: presentation::fog::FogStyle = serde_json::from_str(&style).map_err(js_error)?;
        let look = style.look().map_err(js_error)?;
        self.config.fog_look = look;
        if let Some(runtime) = &mut self.runtime {
            runtime.set_fog_look(&self.gpu.queue, look);
        }
        Ok(())
    }

    pub fn set_overlay_glow(&mut self, radius_px: f32, strength: f32) {
        self.overlay_glow = [radius_px, radius_px / 2.5, strength, 0.0];
        if let Some(runtime) = &mut self.runtime {
            runtime.set_overlay_glow(&self.gpu.queue, self.overlay_glow);
        }
    }
}
