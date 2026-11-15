//! Atomic source, canonical facts and GPU appearance-generation admission.
use super::*;
use presentation::presentation::catalog::Placement;
use presentation::presentation::lab::ModelInput;
use presentation::world::Detail;
use renderer_core::bundle::SourceDraft;
use renderer_core::runtime::{PendingAssets, ReadyAssets};
use std::collections::BTreeMap;

/// Owned source draft; adding or dropping it cannot change a live renderer.
#[wasm_bindgen]
pub struct AppearanceDraft {
    sources: SourceDraft,
    detail: Detail,
    placements: BTreeMap<String, Placement>,
}

#[wasm_bindgen]
impl AppearanceDraft {
    /// Declare every wanted route. Bytes are supplied separately only for
    /// identities the renderer reports missing.
    pub fn declare(&mut self, name: String, content_id: String) -> Result<(), JsValue> {
        let content = serde_json::from_str(&content_id).map_err(js_error)?;
        self.sources.declare(name, content);
        Ok(())
    }

    /// Placement metadata accompanies mesh bytes in the authored catalog.
    pub fn set_placement(&mut self, name: String, json: &str) -> Result<(), JsValue> {
        self.placements
            .insert(name, serde_json::from_str(json).map_err(js_error)?);
        Ok(())
    }

    pub fn add_bundle(
        &mut self,
        content_id: String,
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
        let content = serde_json::from_str(&content_id).map_err(js_error)?;
        self.sources
            .add_bundle(content, bundle, &refs, skeleton.as_deref())
            .map_err(js_error)?;
        Ok(())
    }
}

/// One prospective generation, warming geometry between presented frames.
#[wasm_bindgen]
pub struct WarmingAppearances {
    pending: PendingAssets,
    library: Arc<Library>,
    gpu: std::rc::Rc<crate::gpu::GpuInner>,
}
#[wasm_bindgen]
impl WarmingAppearances {
    pub fn step(&mut self) -> Result<bool, JsValue> {
        self.gpu.ensure_live()?;
        self.pending.step().map_err(js_error)
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
    pub fn appearance_draft(
        &self,
        presentation: &Presentation,
        detail: &str,
    ) -> Result<AppearanceDraft, JsValue> {
        Ok(AppearanceDraft {
            sources: SourceDraft::new(&self.model_library, presentation.core()),
            placements: BTreeMap::new(),
            detail: serde_json::from_str(detail).map_err(js_error)?,
        })
    }

    /// Resolve the prospective catalog before warming its missing geometry.
    pub fn begin_appearances(
        &self,
        presentation: &Presentation,
        draft: AppearanceDraft,
    ) -> Result<WarmingAppearances, JsValue> {
        self.gpu.ensure_live()?;
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before preparing appearances"))?;
        let (library, catalog, work) = draft
            .sources
            .stage(
                &self.model_library,
                presentation.core(),
                draft.detail,
                draft.placements,
            )
            .map_err(js_error)?;
        let pending = runtime
            .stage_assets(
                &self.gpu.device,
                &self.gpu.queue,
                presentation.core(),
                catalog,
                &library,
                work,
            )
            .map_err(js_error)?;
        Ok(WarmingAppearances {
            pending,
            library: Arc::new(library),
            gpu: self.gpu.clone(),
        })
    }

    /// Finish the existing admission and await its cards after geometry warming.
    #[wasm_bindgen(unchecked_return_type = "Promise<PreparedAppearances>")]
    pub fn prepare_appearances(
        &self,
        warming: WarmingAppearances,
    ) -> Result<js_sys::Promise, JsValue> {
        let WarmingAppearances {
            pending,
            library,
            gpu,
        } = warming;
        gpu.ensure_live()?;
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            let ready = pending.prepare().await.map_err(js_error)?;
            gpu.ensure_live()?;
            Ok(PreparedAppearances { ready, library }.into())
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
