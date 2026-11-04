//! Fetched bundle bytes admitted by the common model library.
use super::*;

#[wasm_bindgen]
impl Renderer {
    /// Decode fetched assets without constructing GPU tables per appearance.
    #[wasm_bindgen]
    pub fn load_bundle(
        &mut self,
        name: String,
        bundle: &[u8],
        texture_ids: js_sys::Array,
        textures: js_sys::Array,
    ) -> Result<(), JsValue> {
        let payloads = texture_payloads(texture_ids, textures)?;
        let refs = payloads
            .iter()
            .map(|(id, data)| (id.as_str(), data.as_slice()))
            .collect::<Vec<_>>();
        Arc::make_mut(&mut self.model_library)
            .insert_bundle(&name, bundle, &refs)
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
