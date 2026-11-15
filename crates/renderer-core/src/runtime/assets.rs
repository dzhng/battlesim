//! One complete replaceable asset generation, independent of the live frame.
use super::*;
use crate::grass::GrassData;
use presentation::presentation::catalog::{CatalogDraft, CatalogView};
use presentation::presentation::lab::ModelInput;
use presentation::static_world::ground::grass::Quality;

#[derive(Clone)]
pub(super) struct StaticIdentity {
    ground: usize,
    revision: u32,
    placed: bool,
    scenery: Vec<(String, BundleIdentity)>,
    grass: Vec<(String, BundleIdentity)>,
    density: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BundleIdentity {
    Content([u8; 32]),
    Pointer(usize),
}

impl StaticIdentity {
    pub(super) fn new<S: BundleSource>(
        presentation: &Presentation,
        library: &S,
        density: Option<f64>,
    ) -> Result<Self, String> {
        let ground = presentation
            .ground()
            .ok_or("renderer world has no presentation ground")?;
        let bundles = |names: Vec<String>| {
            names
                .into_iter()
                .filter_map(|name| {
                    let bundle = library.bundle(&name).ok()?;
                    let identity = library.bundle_identity(&name).ok().flatten().map_or(
                        BundleIdentity::Pointer(Arc::as_ptr(&bundle) as usize),
                        BundleIdentity::Content,
                    );
                    Some((name, identity))
                })
                .collect::<Vec<_>>()
        };
        Ok(Self {
            ground: ground as *const _ as usize,
            revision: presentation.ground_revision(),
            placed: presentation.scenery_is_placed(),
            scenery: bundles(ground.scenery_kinds()),
            grass: bundles(ground.grass_kinds()),
            density: density.map(f64::to_bits),
        })
    }

    fn same_bundles(a: &[(String, BundleIdentity)], b: &[(String, BundleIdentity)]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|((an, ab), (bn, bb))| an == bn && ab == bb)
    }

    fn scenery_same(&self, other: &Self) -> bool {
        self.ground == other.ground
            && self.revision == other.revision
            && self.placed == other.placed
            && Self::same_bundles(&self.scenery, &other.scenery)
    }

    fn grass_same(&self, other: &Self) -> bool {
        self.ground == other.ground
            && self.revision == other.revision
            && self.density == other.density
            && Self::same_bundles(&self.grass, &other.grass)
    }
}

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
    statics: StaticIdentity,
    work: AdmissionWork,
}

/// An opaque complete generation. Dropping stale work never changes active art.
pub struct ReadyAssets(PendingAssets);

impl PendingAssets {
    pub fn work(&self) -> &AdmissionWork {
        &self.work
    }
}

impl ReadyAssets {
    pub fn work(&self) -> &AdmissionWork {
        &self.0.work
    }
}

impl PendingAssets {
    pub async fn prepare(mut self) -> Result<ReadyAssets, String> {
        self.assets
            .models
            .bake_cards(&self.device, &self.queue, &self.pipelines, &mut self.work)
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
        mut starting_work: AdmissionWork,
    ) -> Result<PendingAssets, String> {
        starting_work.rebuild_reason = Some(RebuildReason::Replacement);
        let mut work = starting_work;
        let statics = StaticIdentity::new(presentation, library, self.config.grass_density)?;
        let scenery_changed = !self.statics_identity.scenery_same(&statics);
        let grass_changed = !self.statics_identity.grass_same(&statics);
        if scenery_changed && self.statics_identity.placed {
            let finest = crate::scenery::finest_meshes(
                presentation.ground().ok_or("scenery requires ground")?,
                library,
            )?;
            let refs: Vec<_> = finest.iter().map(Vec::as_slice).collect();
            catalog.stage_scenery(presentation, &refs)?;
            work.scenery_placements += 1;
        }
        let view = catalog.view(presentation)?;
        let models = Models::new(device, queue, &self.pipelines, library, view, &mut work)?;
        let scenery = if scenery_changed {
            Scenery::new(device, queue, &self.pipelines, library, view, &mut work)?
        } else {
            self.scenery.reuse(view)?
        };
        let grass = if grass_changed {
            grass(
                device,
                queue,
                &self.pipelines,
                library,
                view,
                &self.ground.heights,
                self.config.grass_density,
                &mut work,
            )?
        } else {
            self.grass.clone()
        };
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
            statics,
            work,
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
        self.statics_identity = pending.statics;
        self.admission_work = pending.work;
        self.apply_asset_controls(queue, self.config.controls);
        self.invalidate_statics();
        self.prepared_frame = None;
        self.pending_catalog_frame = Some(presentation.last_frame().index);
        Ok(true)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn grass(
    device: &Device,
    queue: &Queue,
    pipelines: &Pipelines,
    library: &impl BundleSource,
    catalog: CatalogView<'_>,
    heights: &Buffer,
    density: Option<f64>,
    work: &mut AdmissionWork,
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
        work,
    )))
}
