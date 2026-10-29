//! Atomic source, canonical facts and GPU appearance-generation admission.
use super::*;
use presentation::presentation::catalog::Placement;
use presentation::presentation::lab::{Asset, ModelInput};
use presentation::world::Detail;
use renderer_core::runtime::ReadyAssets;
use std::collections::{BTreeMap, HashSet};

/// Owned source draft; adding or dropping it cannot change a live renderer.
#[wasm_bindgen]
pub struct AppearanceDraft {
    library: Library,
    facts: BTreeMap<String, Asset>,
    detail: Detail,
    placements: BTreeMap<String, Placement>,
}

#[wasm_bindgen]
impl AppearanceDraft {
    /// Placement metadata accompanies mesh bytes in the authored catalog.
    pub fn set_placement(&mut self, name: String, json: &str) -> Result<(), JsValue> {
        self.placements
            .insert(name, serde_json::from_str(json).map_err(js_error)?);
        Ok(())
    }

    pub fn add_bundle(
        &mut self,
        name: String,
        bundle: &[u8],
        texture_ids: js_sys::Array,
        textures: js_sys::Array,
        skeleton: Option<Vec<u8>>,
    ) -> Result<(), JsValue> {
        let payloads = crate::asset::texture_payloads(texture_ids, textures)?;
        let refs: Vec<_> = payloads
            .iter()
            .map(|(id, data)| (id.as_str(), data.as_slice()))
            .collect();
        // Both decoders finish before either draft owner changes.
        let facts = Asset::from_bundle(bundle, skeleton.as_deref()).map_err(js_error)?;
        self.library
            .insert_bundle(&name, bundle, &refs)
            .map_err(js_error)?;
        self.facts.insert(name, facts);
        Ok(())
    }
}

/// Complete GPU generation plus retained source bytes for device recreation.
#[wasm_bindgen]
pub struct PreparedAppearances {
    ready: ReadyAssets,
    library: Arc<Library>,
}

#[wasm_bindgen]
impl Renderer {
    /// Share immutable source textures with the active generation while a new
    /// catalog is decoded. The draft cannot change live bundle ownership.
    pub fn appearance_draft(&self, detail: &str) -> Result<AppearanceDraft, JsValue> {
        Ok(AppearanceDraft {
            library: self.model_library.empty_generation(),
            facts: BTreeMap::new(),
            placements: BTreeMap::new(),
            detail: serde_json::from_str(detail).map_err(js_error)?,
        })
    }

    /// Creates a future with owned resources synchronously. Browser frames,
    /// controls, resize and disposal may proceed while real atlas reads await.
    #[wasm_bindgen(unchecked_return_type = "Promise<PreparedAppearances>")]
    pub fn prepare_appearances(
        &self,
        presentation: &Presentation,
        draft: AppearanceDraft,
    ) -> Result<js_sys::Promise, JsValue> {
        self.gpu.ensure_live()?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before preparing appearances"))?;
        let catalog = presentation
            .core()
            .stage_catalog(draft.facts, draft.detail, draft.placements)
            .map_err(js_error)?;
        let view = catalog.view(presentation.core()).map_err(js_error)?;
        let world = view.world();
        let mut names: HashSet<String> = world
            .appearances
            .iter()
            .map(|r| world.strings[r.name as usize..(r.name + r.name_len) as usize].into())
            .collect();
        if let Some(ground) = view.ground() {
            names.extend(ground.grass_kinds());
        }
        // Preserve canonical battle/static sources, remove retired external art,
        // and replace only names carried by the desired generation.
        let mut library = (*self.model_library).clone();
        library.replace_with(draft.library);
        library.retain(|name| names.contains(name));
        let pending = runtime
            .stage_assets(
                &self.gpu.device,
                &self.gpu.queue,
                presentation.core(),
                catalog,
                &library,
            )
            .map_err(js_error)?;
        let gpu = self.gpu.clone();
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            let ready = pending.prepare().await.map_err(js_error)?;
            gpu.ensure_live()?;
            Ok(PreparedAppearances {
                ready,
                library: Arc::new(library),
            }
            .into())
        }))
    }

    /// Commit latest model requests and matching GPU/catalog owners in one
    /// synchronous call. Zero means the world, renderer or device was replaced.
    pub fn commit_appearances(
        &mut self,
        presentation: &mut Presentation,
        prepared: PreparedAppearances,
        models_json: &str,
    ) -> Result<u32, JsValue> {
        self.gpu.ensure_live()?;
        let models: Vec<ModelInput> = serde_json::from_str(models_json).map_err(js_error)?;
        let Some(runtime) = &mut self.runtime else {
            return Ok(0);
        };
        let committed = runtime
            .commit_assets(
                &self.gpu.device,
                &self.gpu.queue,
                presentation.core_mut(),
                prepared.ready,
                models,
            )
            .map_err(js_error)?;
        if committed {
            self.model_library = prepared.library;
            return Ok(presentation.refresh_world_header());
        }
        Ok(0)
    }
}
