//! Model rest poses feed both runtime cards and configurable review atlases.
use super::*;
use crate::impostor::{self, BakeRequest, Spec};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct RestPose {
    name: String,
    bounds: [[f64; 3]; 2],
    palette: Vec<Joint>,
}
impl RestPose {
    pub fn identity() -> Vec<Joint> {
        vec![Joint {
            x: [1., 0., 0., 0.],
            y: [0., 1., 0., 0.],
            z: [0., 0., 1., 0.],
        }]
    }
    pub fn articulated(name: &str, nodes: &[crate::bundle::Node], palette: Vec<Joint>) -> Self {
        let mut positions = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if let Some(mesh) = node.tiers.first() {
                let joint = &palette[index];
                for point in mesh.positions.as_chunks::<3>().0 {
                    for row in [joint.x, joint.y, joint.z] {
                        positions.push(
                            row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3],
                        );
                    }
                }
            }
        }
        Self::new(
            name,
            &Mesh {
                positions,
                ..Default::default()
            },
            palette,
        )
    }

    pub fn new(name: &str, mesh: &Mesh, palette: Vec<Joint>) -> Self {
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for point in mesh.positions.as_chunks::<3>().0.iter() {
            for c in 0..3 {
                bounds[0][c] = bounds[0][c].min(f64::from(point[c]));
                bounds[1][c] = bounds[1][c].max(f64::from(point[c]));
            }
        }
        Self {
            name: name.into(),
            bounds,
            palette,
        }
    }
}
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
        let rows: Vec<u32> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                row.as_ref()
                    .filter(|row| {
                        row.rest.is_some() && matches!(row.kind, Kind::Soldier | Kind::Corpse)
                    })
                    .map(|_| index as u32)
            })
            .collect();
        if rows.is_empty() {
            return Ok(());
        }
        let spec = Spec::runtime();
        let card_bytes = spec.bake_memory_bytes()?;
        let batch_memory_bytes = policy.card_bake_memory_bytes;
        if batch_memory_bytes < card_bytes {
            return Err(format!(
                "impostor bake bound {batch_memory_bytes} bytes cannot hold one card ({card_bytes} bytes)"
            ));
        }
        let cards_per_batch = (batch_memory_bytes / card_bytes).min(rows.len() as u64) as usize;
        if rows.len() > cards_per_batch && !split_reported.swap(true, Ordering::Relaxed) {
            let message = format!(
                "impostor bake split: {} cards, {} batches, {batch_memory_bytes} byte bound ({card_bytes} bytes/card)",
                rows.len(), rows.len().div_ceil(cards_per_batch)
            );
            (policy.report)(&message);
        }
        let mut atlases = Vec::with_capacity(rows.len());
        for batch in rows.chunks(cards_per_batch) {
            let request = self.impostor_request(device, queue, pipelines, batch, &spec, work)?;
            work.cards_baked += batch.len() as u32;
            work.card_read_waits += 1;
            atlases.extend(request.read().await?);
        }
        let cards = Cards::new(device, queue, pipelines, &atlases)?;
        for (layer, row) in rows.into_iter().enumerate() {
            self.rows[row as usize].as_mut().expect("baked row").card = Some(layer as u32);
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
    ) -> Result<BakeRequest, String> {
        if rows.is_empty() {
            return Err("impostor request needs at least one appearance row".into());
        }
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
        let inputs = rows
            .iter()
            .map(|&row| {
                let row = self
                    .rows
                    .get(row as usize)
                    .and_then(Option::as_ref)
                    .ok_or("impostor appearance row is not installed")?;
                let rest = row
                    .rest
                    .as_ref()
                    .ok_or("appearance has no authored far or corpse pose")?;
                let drawable = row
                    .tiers
                    .first()
                    .ok_or("impostor appearance has no finest mesh")?;
                let identity = crate::buffer(
                    device,
                    queue,
                    "impostor-palette",
                    BufferUsages::STORAGE,
                    bytemuck::cast_slice(&rest.palette),
                );
                let models = crate::model_material::ModelMaterialBindings::new(
                    device,
                    &self.layout,
                    &identity,
                    materials,
                    &self.albedo,
                    &self.surface,
                    &self.sampler,
                );
                let part = drawable.part(Surface::Opaque);
                Ok(impostor::BakeInput {
                    appearance: rest.name.clone(),
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
        BakeRequest::new(device, queue, pipelines, inputs)
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
