//! Copied presentation inputs for the shared scenery owner.
use super::*;
use presentation::static_world::ground::mesh::VERTEX_FLOATS;
use renderer_core::scenery::Scenery;

pub(super) struct SceneryWorldSnapshot {
    kinds: Vec<String>,
    offsets: Vec<u32>,
    stumps: Vec<f32>,
}

#[wasm_bindgen]
impl Renderer {
    /// Stump offsets are float-word offsets per exact appearance row plus a
    /// terminal offset. Empty ranges mean that row has no stump mesh.
    #[wasm_bindgen]
    pub fn load_scenery_world(
        &mut self,
        kinds: js_sys::Array,
        stump_offsets: &[u32],
        stump_words: &[f32],
    ) -> Result<(), JsValue> {
        let world = self
            .model_world
            .as_ref()
            .ok_or_else(|| js_error("load model world before installing scenery"))?;
        if stump_offsets.len() != world.appearances.len() + 1
            || stump_offsets.first() != Some(&0)
            || stump_offsets.last().copied() != Some(stump_words.len() as u32)
            || stump_offsets.windows(2).any(|pair| {
                pair[0] > pair[1]
                    || !pair[0].is_multiple_of(VERTEX_FLOATS as u32)
                    || !pair[1].is_multiple_of(VERTEX_FLOATS as u32)
            })
        {
            return Err(js_error("scenery stump offsets are invalid"));
        }
        let kinds = kinds
            .iter()
            .map(|kind| {
                kind.as_string()
                    .ok_or_else(|| js_error("scenery kinds must be appearance names"))
            })
            .collect::<Result<Vec<_>, JsValue>>()?;
        self.install_scenery_world(Arc::new(SceneryWorldSnapshot {
            kinds,
            offsets: stump_offsets.to_vec(),
            stumps: stump_words.to_vec(),
        }))
    }
}

impl Renderer {
    pub(super) fn install_scenery_world(
        &mut self,
        snapshot: Arc<SceneryWorldSnapshot>,
    ) -> Result<(), JsValue> {
        let world = self
            .model_world
            .as_ref()
            .ok_or_else(|| js_error("load model world before installing scenery"))?;
        let mut scenery = Scenery::from_world(
            &self.device,
            &self.production_pipelines,
            self.model_library.as_ref(),
            &world.appearances,
            &world.strings,
            &snapshot.kinds,
            |row| {
                let start = snapshot.offsets[row as usize] as usize;
                let end = snapshot.offsets[row as usize + 1] as usize;
                (start != end).then(|| &snapshot.stumps[start..end])
            },
        )
        .map_err(js_error)?;
        if let Some(models) = &self.models {
            scenery.prepare(models.statics());
        }
        self.stats.shared_scenery = true;
        self.stats.scenery_vertices = scenery.resident_vertices();
        self.scenery = Some(scenery);
        self.scenery_world = Some(snapshot);
        self.update_scenery_stats();
        Ok(())
    }

    pub(super) fn update_scenery_stats(&mut self) {
        if let Some(scenery) = &self.scenery {
            self.stats.scenery_runs = scenery.run_count();
            self.stats.shadow_scenery_batches = scenery.caster_run_counts();
        }
    }
}
