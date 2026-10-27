use super::{pipeline_export, Renderer};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Renderer {
    /// Diagnostics for the exact production pipeline export embedded in this
    /// browser artifact.
    #[wasm_bindgen]
    pub fn exported_pipeline_count() -> usize {
        pipeline_export::COUNT
    }

    #[wasm_bindgen]
    pub fn exported_pipeline_manifest() -> String {
        pipeline_export::MANIFEST.to_owned()
    }

    #[wasm_bindgen]
    pub fn exported_pipeline_source(name: String) -> Option<String> {
        pipeline_export::source(&name).map(str::to_owned)
    }

    /// Validate the embedded export while proving the shared environment,
    /// shadow, and fog owners are all constructed on this device.
    pub async fn validate_export(&self) -> Result<String, JsValue> {
        let built = self.production_pipelines.names().count();
        let _resources = (
            &self.shared_post,
            &self.shared_fog_mask,
            self.environment.bindings.group(),
            &self.environment.sky,
            &self.environment.pmrem,
            &self.environment.dfg,
            &self.environment.sampler,
            &self.environment.cast_lights,
            self.shared_shadow.active(),
            self.shadow_routing.counts(),
            &self.shared_fog_compute,
        );
        Ok(format!("{{\"pipelines\":{built}}}"))
    }
}
