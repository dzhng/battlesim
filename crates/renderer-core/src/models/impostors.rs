//! Model rest poses feed both runtime cards and configurable review atlases.
use super::*;
use crate::impostor::{self, AtlasRequest, Spec};
use std::sync::atomic::{AtomicBool, Ordering};

impl Models {
    /// Bake every authored far/corpse pose before admitting a world for drawing.
    /// No simulation or presentation borrow survives these asynchronous reads.
    pub async fn bake_cards(
        &mut self,
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        policy: ArtStorePolicy,
        split_reported: &AtomicBool,
        work: &mut crate::runtime::AdmissionWork,
    ) -> Result<(), String> {
        let mut representatives = std::collections::BTreeMap::new();
        for (index, row) in self.rows.iter().enumerate() {
            if let Some(key) = row.as_ref().and_then(|row| row.card_key.as_ref()) {
                representatives.entry(key.clone()).or_insert(index as u32);
            }
        }
        if representatives.is_empty() {
            return Ok(());
        }
        let spec = Spec::runtime();
        let wanted = representatives.keys().cloned().collect();
        let (array, missing) = {
            let mut held = self.store.lock().expect("art store lock");
            let array = held
                .cards
                .admit(device, queue, &wanted, &spec, policy, work);
            let missing = representatives
                .iter()
                .filter(|(key, _)| !held.cards.written(&array, key))
                .map(|(key, &row)| (key.clone(), row))
                .collect::<Vec<_>>();
            (array, missing)
        };
        let rows = missing.iter().map(|(_, row)| *row).collect::<Vec<_>>();
        let card_bytes = spec.bake_memory_bytes()?;
        let batch_memory_bytes = policy.card_bake_memory_bytes;
        if !rows.is_empty() && batch_memory_bytes < card_bytes {
            return Err(format!(
                "impostor bake bound {batch_memory_bytes} bytes cannot hold one card ({card_bytes} bytes)"
            ));
        }
        let cards_per_batch = (batch_memory_bytes / card_bytes)
            .min(rows.len() as u64)
            .max(1) as usize;
        if rows.len() > cards_per_batch && !split_reported.swap(true, Ordering::Relaxed) {
            let message = format!(
                "impostor bake split: {} cards, {} batches, {batch_memory_bytes} byte bound ({card_bytes} bytes/card)",
                rows.len(), rows.len().div_ceil(cards_per_batch)
            );
            (policy.report)(&message);
        }
        for batch in missing.chunks(cards_per_batch) {
            let rows = batch.iter().map(|(_, row)| *row).collect::<Vec<_>>();
            let request = self.impostor_request(device, queue, pipelines, &rows, &spec, work)?;
            work.cards_baked += batch.len() as u32;
            work.card_read_waits += 1;
            let atlases = request.read().await?;
            let mut held = self.store.lock().expect("art store lock");
            for ((key, _), atlas) in batch.iter().zip(&atlases) {
                held.cards.complete(queue, &array, key, atlas, work);
            }
        }
        let held = self.store.lock().expect("art store lock");
        let cards = Cards::new(device, queue, pipelines, array, held.cards.frames(&wanted))?;
        for row in self.rows.iter_mut().flatten() {
            if let Some(key) = &row.card_key {
                row.card = Some(held.cards.layer(key));
            }
        }
        self.cards = Some(cards);
        Ok(())
    }

    /// Submit review or runtime rows in order, sharing the generation's
    /// immutable authored materials while keeping each pose snapshot owned.
    pub fn impostor_request(
        &self,
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        rows: &[u32],
        spec: &Spec,
        work: &mut crate::runtime::AdmissionWork,
    ) -> Result<AtlasRequest, String> {
        if rows.is_empty() {
            return Err("impostor request needs at least one appearance row".into());
        }
        let inputs = rows
            .iter()
            .map(|&row_index| {
                let row = self
                    .rows
                    .get(row_index as usize)
                    .and_then(Option::as_ref)
                    .ok_or("impostor appearance row is not installed")?;
                let rest = row
                    .rest
                    .as_ref()
                    .ok_or("appearance has no authored far or corpse pose")?;
                let materials = self.impostor_materials.get_or_init(|| {
                    let bytes = bytemuck::cast_slice(&self.authored_materials);
                    work.buffer_bytes_uploaded += bytes.len() as u64;
                    crate::buffer(
                        device,
                        queue,
                        "impostor-authored-materials",
                        BufferUsages::STORAGE,
                        bytes,
                    )
                });
                let drawable = row
                    .geometry
                    .tiers
                    .first()
                    .ok_or("impostor appearance has no finest mesh")?;
                let palette = crate::buffer(
                    device,
                    queue,
                    "impostor-palette",
                    BufferUsages::STORAGE,
                    bytemuck::cast_slice(&rest.palette),
                );
                let models = crate::model_material::ModelMaterialBindings::new(
                    device,
                    &self.layout,
                    &palette,
                    materials,
                    &self.albedo,
                    &self.surface,
                    &self.sampler,
                );
                let part = drawable.part(Surface::Opaque);
                Ok(impostor::BakeInput {
                    appearance: self.row_names[row_index as usize].clone(),
                    bounds: rest.bounds,
                    vertices: drawable.vertices.clone(),
                    indices: drawable.indices.clone(),
                    first: part.first,
                    count: part.count,
                    models: models.bind_group().clone(),
                    spec: spec.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        AtlasRequest::new(device, queue, pipelines, inputs)
    }

    pub fn card_group(&self) -> Option<(&'static str, &BindGroup)> {
        self.cards
            .as_ref()
            .map(|cards| ("cardLayout", &cards.group))
    }
    pub fn draw_cards(&self, pass: &mut RenderPass, layer: Layer) {
        if !self.models_shown || !self.surfaces[OPAQUE] {
            return;
        }
        if let Some(cards) = &self.cards {
            let work = cards.draw(pass, layer, |pass| self.in_world(pass));
            self.submissions
                .record(crate::submitted::Pass::Color, |stats| {
                    stats.models_including_props.add(work);
                });
        }
    }
    pub fn draw_high_air_cards(&self, pass: &mut RenderPass, layer: Layer) {
        if !self.models_shown || !self.surfaces[OPAQUE] {
            return;
        }
        if let Some(cards) = &self.cards {
            let work = cards.draw(pass, layer, |pass| pass == HIGH_PASS);
            self.submissions
                .record(crate::submitted::Pass::HighColor, |stats| {
                    stats.models_including_props.add(work);
                });
        }
    }
}
