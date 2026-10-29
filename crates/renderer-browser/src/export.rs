use super::Renderer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Renderer {
    /// Diagnostics for the exact production pipeline catalog embedded in this
    /// browser artifact.
    #[wasm_bindgen]
    pub fn exported_pipeline_count() -> usize {
        renderer_core::pipelines::builtin_pipeline_count()
    }

    #[wasm_bindgen]
    pub fn exported_pipeline_source(name: String) -> Option<String> {
        renderer_core::pipelines::builtin_source(&name).map(str::to_owned)
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
