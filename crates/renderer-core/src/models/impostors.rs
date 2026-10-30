//! Model rest poses feed both runtime cards and configurable review atlases.
use super::*;
use crate::impostor::{self, BakeRequest, Spec};

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
    ) -> Result<(), String> {
        let rows: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                row.as_ref()
                    .filter(|row| {
                        row.rest.is_some() && matches!(row.kind, Kind::Soldier | Kind::Corpse)
                    })
                    .map(|_| index)
            })
            .collect();
        if rows.is_empty() {
            return Ok(());
        }
        let mut atlases = Vec::new();
        for &row in &rows {
            atlases.push(
                self.impostor_request(device, queue, pipelines, row as u32, &Spec::runtime())?
                    .read()
                    .await?,
            );
        }
        let cards = Cards::new(device, queue, pipelines, &atlases)?;
        for (layer, row) in rows.into_iter().enumerate() {
            self.rows[row].as_mut().expect("baked row").card = Some(layer as u32);
        }
        self.cards = Some(cards);
        Ok(())
    }

    pub fn impostor_request(
        &self,
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        row: u32,
        spec: &Spec,
    ) -> Result<BakeRequest, String> {
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
        let materials = crate::buffer(
            device,
            queue,
            "impostor-authored-materials",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&self.authored_materials),
        );
        let models = crate::model_material::ModelMaterialBindings::new(
            device,
            &self.layout,
            &identity,
            &materials,
            &self.albedo,
            &self.surface,
            &self.sampler,
        );
        let part = drawable.part(Surface::Opaque);
        BakeRequest::new(
            device,
            queue,
            pipelines,
            impostor::BakeInput {
                appearance: rest.name.clone(),
                bounds: rest.bounds,
                vertices: drawable.vertices.clone(),
                indices: drawable.indices.clone(),
                first: part.first,
                count: part.count,
                models: models.bind_group().clone(),
                spec: spec.clone(),
            },
        )
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
