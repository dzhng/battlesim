//! A pending generation holds one geometry key's work per step, never live rows.
use super::admission_plan::AdmissionPlan;
use super::store::GeometryKey;
use super::*;
use crate::runtime::AdmissionWork;
use contract::identity::ContentId;
use std::collections::{BTreeMap, VecDeque};

pub(crate) struct GeometryDraft {
    device: Device,
    queue: Queue,
    plan: AdmissionPlan,
    store: SharedArtStore,
    keys: VecDeque<Vec<usize>>,
    held: Vec<Row>,
    appearances: Vec<presentation::records::Appearance>,
    strings: String,
}
impl GeometryDraft {
    pub fn new(
        device: &Device,
        queue: &Queue,
        library: &impl BundleSource,
        catalog: presentation::presentation::catalog::CatalogView<'_>,
        store: SharedArtStore,
        work: &mut AdmissionWork,
    ) -> Result<Self, String> {
        let world = catalog.world();
        let plan = AdmissionPlan::new(
            library,
            &world.appearances,
            &world.strings,
            |r| catalog.rest_palette(r),
            |r, n| catalog.track_side(r, n),
        )?;
        let store = plan.store(store, device.limits().max_texture_array_layers, work)?;
        let mut groups = BTreeMap::<GeometryKey, Vec<usize>>::new();
        for (i, row) in plan.rows.iter().enumerate() {
            if let Some(row) = row {
                groups.entry(row.plan.geometry.clone()).or_default().push(i);
            }
        }
        let keys = groups
            .into_iter()
            .filter_map(|(key, rows)| {
                let missing = rows.iter().any(|&i| {
                    let row = plan.rows[i].as_ref().expect("planned row");
                    let palette = row.plan.palette.clone().unwrap_or_else(RestPose::identity);
                    !ArtStore::has_rest(&store, &key, &palette)
                });
                missing.then_some(rows)
            })
            .collect();
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            plan,
            store,
            keys,
            held: Vec::new(),
            appearances: world.appearances.clone(),
            strings: world.strings.clone(),
        })
    }

    pub fn step(&mut self, work: &mut AdmissionWork) -> Result<bool, String> {
        let Some(rows) = self.keys.pop_front() else {
            return Ok(false);
        };
        for i in rows {
            self.held
                .push(self.plan.rows[i].as_ref().expect("planned row").admit(
                    &self.device,
                    &self.queue,
                    &self.store,
                    work,
                )?);
        }
        Ok(true)
    }

    pub fn admit(
        mut self,
        pipelines: &Pipelines,
        work: &mut AdmissionWork,
    ) -> Result<Models, String> {
        // Native/cold callers can finish at once; the viewport calls step once
        // per presented frame before reaching the same admission below.
        while self.step(work)? {}
        Models::from_world(
            &self.device,
            &self.queue,
            pipelines,
            &self,
            &self.appearances,
            &self.strings,
            |r| self.plan.rows[r as usize].as_ref()?.plan.palette.clone(),
            |r, n| match *self.plan.rows[r as usize]
                .as_ref()?
                .plan
                .geometry
                .tracks
                .get(n)?
            {
                1 => Some(presentation::world::TrackSide::Left),
                2 => Some(presentation::world::TrackSide::Right),
                _ => None,
            },
            self.store.clone(),
            work,
        )
    }
}
impl BundleSource for GeometryDraft {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        self.plan
            .bundles
            .get(name)
            .map(|b| b.0.clone())
            .ok_or_else(|| format!("{name}: draft source missing"))
    }
    fn content_id(&self, name: &str) -> Result<ContentId, String> {
        self.plan
            .bundles
            .get(name)
            .map(|b| b.1.clone())
            .ok_or_else(|| format!("{name}: draft identity missing"))
    }
}
