//! Browser byte transport into the shared authored-model owner.
use super::*;
use presentation::records::{Appearance, Batch, Instance as ModelInstance, Joint, NONE};

pub(super) struct ModelWorldSnapshot {
    pub(super) appearances: Vec<Appearance>,
    pub(super) strings: String,
    offsets: Vec<u32>,
    rest: Vec<Joint>,
}

pub(super) struct ModelFrameSnapshot {
    palettes: Vec<Joint>,
    instances: Vec<ModelInstance>,
    batches: Vec<Batch>,
}

fn records<Source: bytemuck::Pod, Record: bytemuck::Pod>(
    words: &[Source],
) -> Result<&[Record], JsValue> {
    bytemuck::try_cast_slice(words).map_err(js_error)
}

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
    ) -> Result<String, JsValue> {
        if texture_ids.length() != textures.length() {
            return Err(js_error("bundle texture ids and payloads differ"));
        }
        let payloads = texture_ids
            .iter()
            .zip(textures.iter())
            .map(|(id, data)| {
                let id = id
                    .as_string()
                    .ok_or_else(|| js_error("texture id must be a content hash"))?;
                Ok((id, js_sys::Uint8Array::new(&data).to_vec()))
            })
            .collect::<Result<Vec<_>, JsValue>>()?;
        let refs = payloads
            .iter()
            .map(|(id, data)| (id.as_str(), data.as_slice()))
            .collect::<Vec<_>>();
        Arc::make_mut(&mut self.model_library)
            .insert_bundle(&name, bundle, &refs)
            .map_err(js_error)?;
        let report = format!(
            "{{\"name\":{}}}",
            js_sys::JSON::stringify(&JsValue::from_str(&name))?
                .as_string()
                .unwrap_or_default()
        );
        self.asset_report = Some(report.clone());
        Ok(report)
    }

    /// Install the exact presentation appearance ABI and its rest palettes.
    /// Rest offsets index float words; each palette contains 12-word joints.
    #[wasm_bindgen]
    pub fn load_model_world(
        &mut self,
        appearance_words: &[u32],
        strings: String,
        offsets: &[u32],
        rest: &[f32],
    ) -> Result<(), JsValue> {
        let appearances = records::<_, Appearance>(appearance_words)?;
        if offsets.len() != appearances.len() + 1
            || offsets.first() != Some(&0)
            || offsets.last().copied() != Some(rest.len() as u32)
            || offsets.windows(2).any(|pair| {
                pair[0] > pair[1] || !pair[0].is_multiple_of(12) || !pair[1].is_multiple_of(12)
            })
        {
            return Err(js_error("appearance rest palette offsets are invalid"));
        }
        let world = Arc::new(ModelWorldSnapshot {
            appearances: appearances.to_vec(),
            strings,
            offsets: offsets.iter().map(|offset| offset / 12).collect(),
            rest: records::<_, Joint>(rest)?.to_vec(),
        });
        self.install_model_world(world)
    }

    /// Upload a frame through the same indexed-model owner as native.
    #[wasm_bindgen]
    pub fn upload_model_frame(
        &mut self,
        palettes: &[f32],
        instances: &[f32],
        batches: &[u32],
    ) -> Result<(), JsValue> {
        let palettes = records::<_, Joint>(palettes)?;
        let instances = records::<_, ModelInstance>(instances)?;
        let batches = records::<_, Batch>(batches)?;
        self.prepare_model_frame(palettes, instances, batches)?;
        // Materialize unchanged static batches for a fresh device. Their CPU
        // records have the same lifetime owner as their retained GPU buffers.
        let models = self.models.as_ref().expect("prepared models exist");
        let mut instances = instances.to_vec();
        let mut batches = batches.to_vec();
        for batch in &mut batches {
            if batch.first == NONE {
                batch.first = instances.len() as u32;
                instances.extend_from_slice(models.statics().records(batch.key));
            }
        }
        self.model_frame = Some(Arc::new(ModelFrameSnapshot {
            palettes: palettes.to_vec(),
            instances,
            batches,
        }));
        Ok(())
    }
}

impl Renderer {
    pub(super) fn install_model_world(
        &mut self,
        world: Arc<ModelWorldSnapshot>,
    ) -> Result<(), JsValue> {
        let models = Models::from_world(
            &self.device,
            &self.queue,
            &self.production_pipelines,
            self.model_library.as_ref(),
            &world.appearances,
            &world.strings,
            |row| {
                let start = world.offsets[row as usize] as usize;
                let end = world.offsets[row as usize + 1] as usize;
                (start != end).then(|| world.rest[start..end].to_vec())
            },
        )
        .map_err(js_error)?;
        self.stats.model_vertices = models.resident_vertices();
        self.stats.presentation_material_words = models.material_words();
        self.stats.shared_materials = true;
        self.models = Some(models);
        self.model_world = Some(world);
        self.model_frame = None;
        self.scenery = None;
        self.scenery_world = None;
        self.stats.shared_scenery = false;
        self.stats.scenery_vertices = 0;
        self.stats.scenery_runs = 0;
        self.stats.shadow_scenery_batches = [0; 4];
        Ok(())
    }

    pub(super) fn prepare_model_frame(
        &mut self,
        palettes: &[Joint],
        instances: &[ModelInstance],
        batches: &[Batch],
    ) -> Result<(), JsValue> {
        let models = self
            .models
            .as_mut()
            .ok_or_else(|| js_error("load model world before uploading a model frame"))?;
        models
            .prepare_records(&self.device, &self.queue, palettes, instances, batches)
            .map_err(js_error)?;
        if let Some(scenery) = &mut self.scenery {
            scenery.prepare(models.statics());
        }
        self.stats.presentation_model_runs = models.run_count();
        self.stats.shadow_caster_batches = models.caster_run_counts();
        self.stats.presentation_model_words = (std::mem::size_of_val(palettes)
            + std::mem::size_of_val(instances)
            + std::mem::size_of_val(batches)) as u32
            / 4;
        self.update_scenery_stats();
        Ok(())
    }

    pub(super) fn restore_model_frame(
        &mut self,
        frame: Arc<ModelFrameSnapshot>,
    ) -> Result<(), JsValue> {
        self.prepare_model_frame(&frame.palettes, &frame.instances, &frame.batches)?;
        self.model_frame = Some(frame);
        Ok(())
    }
}
