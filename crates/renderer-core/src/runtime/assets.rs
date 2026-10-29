//! One complete replaceable asset generation, independent of the live frame.
use super::*;
use crate::grass::GrassData;
use presentation::presentation::catalog::{CatalogDraft, CatalogView};
use presentation::presentation::lab::ModelInput;
use presentation::static_world::ground::grass::Quality;

struct Assets {
    models: Models,
    scenery: Scenery,
    grass: Option<Grass>,
}

/// Owns all resources needed by asynchronous card baking. No Runtime or
/// Presentation borrow survives synchronous admission.
pub struct PendingAssets {
    device: Device,
    queue: Queue,
    pipelines: Arc<Pipelines>,
    catalog: CatalogDraft,
    assets: Assets,
}

/// An opaque complete generation. Dropping stale work never changes active art.
pub struct ReadyAssets(PendingAssets);

impl PendingAssets {
    pub async fn prepare(mut self) -> Result<ReadyAssets, String> {
        self.assets
            .models
            .bake_cards(&self.device, &self.queue, &self.pipelines)
            .await?;
        Ok(ReadyAssets(self))
    }
}

impl Runtime {
    /// Stage only owners whose lifetime follows appearance art. Targets, sky,
    /// environment, fog, scars and frame orchestration stay live throughout.
    pub fn stage_assets(
        &self,
        device: &Device,
        queue: &Queue,
        presentation: &Presentation,
        mut catalog: CatalogDraft,
        library: &impl BundleSource,
    ) -> Result<PendingAssets, String> {
        if catalog.needs_scenery() {
            let names = presentation
                .ground()
                .ok_or("scenery requires ground")?
                .scenery_kinds();
            let finest = names
                .iter()
                .map(|name| crate::scenery::tier_mesh(name, library.bundle(name)?.as_ref(), 0))
                .collect::<Result<Vec<_>, String>>()?;
            let refs: Vec<_> = finest.iter().map(Vec::as_slice).collect();
            catalog.stage_scenery(presentation, &refs)?;
        }
        let view = catalog.view(presentation)?;
        let models = Models::new(device, queue, &self.pipelines, library, view)?;
        let scenery = Scenery::new(device, queue, &self.pipelines, library, view)?;
        let grass = grass(
            device,
            queue,
            &self.pipelines,
            library,
            view,
            &self.ground.heights,
            self.config.grass_density,
        )?;
        Ok(PendingAssets {
            device: device.clone(),
            queue: queue.clone(),
            pipelines: self.pipelines.clone(),
            catalog,
            assets: Assets {
                models,
                scenery,
                grass,
            },
        })
    }

    /// The shell calls this synchronously before publishing its next frame.
    /// A different device or canonical world rejects the candidate unchanged.
    pub fn commit_assets(
        &mut self,
        device: &Device,
        queue: &Queue,
        presentation: &mut Presentation,
        ready: ReadyAssets,
        models: Vec<ModelInput>,
    ) -> Result<bool, String> {
        let pending = ready.0;
        if &pending.device != device
            || !Arc::ptr_eq(&pending.pipelines, &self.pipelines)
            || !pending.catalog.is_current(presentation)
        {
            return Ok(false);
        }
        if !presentation.commit_catalog(pending.catalog, models)? {
            return Ok(false);
        }
        self.models = pending.assets.models;
        self.scenery = pending.assets.scenery;
        self.grass = pending.assets.grass;
        self.apply_asset_controls(queue, self.config.controls);
        self.invalidate_statics();
        self.prepared_frame = None;
        self.pending_catalog_frame = Some(presentation.last_frame().index);
        Ok(true)
    }
}

pub(super) fn grass(
    device: &Device,
    queue: &Queue,
    pipelines: &Pipelines,
    library: &impl BundleSource,
    catalog: CatalogView<'_>,
    heights: &Buffer,
    density: Option<f64>,
) -> Result<Option<Grass>, String> {
    let Some(density) = density else {
        return Ok(None);
    };
    let resolved = catalog
        .ground()
        .ok_or("renderer world has no presentation ground")?;
    let packed = crate::grass::pack_shapes(library, &resolved.grass_kinds())?;
    let params = resolved.grass_params(
        &packed.facts,
        &presentation::static_world::ground::grass::LIMITS,
        &Quality { density },
    )?;
    Ok(Some(Grass::from_data(
        device,
        queue,
        pipelines,
        heights,
        GrassData {
            params: &params,
            props: &resolved.grass_props(),
            shapes: &packed.shapes,
            max_blades: packed.max_blades,
            capacity: resolved.biome.grass.capacity,
        },
    )))
}
