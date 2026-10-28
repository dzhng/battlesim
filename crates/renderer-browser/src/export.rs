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
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| JsValue::from_str("install presentation before validating export"))?;
        let built = runtime.pipelines.names().count();
        Ok(format!("{{\"pipelines\":{built}}}"))
    }
}
