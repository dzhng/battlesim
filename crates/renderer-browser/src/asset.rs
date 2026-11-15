//! Fetched bundle bytes admitted by the common model library.
use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Return identities lacking decoded source or retained presentation facts.
    /// The list is identity keyed so names may be replaced or shared freely.
    pub fn missing_content_ids(
        &self,
        presentation: &Presentation,
        content_ids_json: &str,
    ) -> Result<String, JsValue> {
        let contents: Vec<contract::identity::ContentId> =
            serde_json::from_str(content_ids_json).map_err(js_error)?;
        serde_json::to_string(
            &self
                .model_library
                .missing_content(presentation.core(), &contents),
        )
        .map_err(js_error)
    }

    /// Decode fetched assets without constructing GPU tables per appearance.
    #[wasm_bindgen]
    pub fn load_bundle(
        &mut self,
        name: String,
        content_id: String,
        bundle: &[u8],
        texture_ids: js_sys::Array,
        textures: js_sys::Array,
    ) -> Result<(), JsValue> {
        let payloads = texture_payloads(texture_ids, textures)?;
        let refs = payloads
            .iter()
            .map(|(id, data)| (id.as_str(), data.as_slice()))
            .collect::<Vec<_>>();
        let content: contract::identity::ContentId =
            serde_json::from_str(&content_id).map_err(js_error)?;
        Arc::make_mut(&mut self.model_library)
            .insert_bundle(&name, content, bundle, &refs)
            .map_err(js_error)?;
        Ok(())
    }
}

pub(super) fn texture_payloads(
    texture_ids: js_sys::Array,
    textures: js_sys::Array,
) -> Result<Vec<(String, Vec<u8>)>, JsValue> {
    if texture_ids.length() != textures.length() {
        return Err(js_error("bundle texture ids and payloads differ"));
    }
    texture_ids
        .iter()
        .zip(textures.iter())
        .map(|(id, data)| {
            let id = id
                .as_string()
                .ok_or_else(|| js_error("texture id must be a content hash"))?;
            Ok((id, js_sys::Uint8Array::new(&data).to_vec()))
        })
        .collect::<Result<Vec<_>, JsValue>>()
}
