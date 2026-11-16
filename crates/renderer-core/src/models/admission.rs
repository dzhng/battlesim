//! Authored model rows and shared material admission.
use super::*;
use crate::models::store::SharedArtStore;
use crate::runtime::AdmissionWork;

impl Models {
    /// Installs every row of the presentation's world from `library`.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        library: &impl BundleSource,
        catalog: presentation::presentation::catalog::CatalogView<'_>,
        store: SharedArtStore,
        work: &mut AdmissionWork,
    ) -> Result<Self, String> {
        let world = catalog.world();
        Self::from_world(
            device,
            queue,
            pipelines,
            library,
            &world.appearances,
            &world.strings,
            |row| catalog.rest_palette(row),
            |row, node| catalog.track_side(row, node),
            store,
            work,
        )
    }

    /// Install the exact world appearance publication. A shell supplies each
    /// body's presentation-owned far or corpse pose through `rest_palette`,
    /// and each node's running-gear side through `track_side`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_world(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        library: &impl BundleSource,
        appearances: &[presentation::records::Appearance],
        strings: &str,
        rest_palette: impl Fn(u32) -> Option<Vec<Joint>>,
        track_side: impl Fn(u32, usize) -> Option<presentation::world::TrackSide>,
        store: SharedArtStore,
        work: &mut AdmissionWork,
    ) -> Result<Self, String> {
        let text = |at: u32, len: u32| &strings[at as usize..(at + len) as usize];
        let plan = super::admission_plan::AdmissionPlan::new(
            library,
            appearances,
            strings,
            rest_palette,
            track_side,
        )?;
        let store = plan.store(store, device.limits().max_texture_array_layers, work)?;
        let mut materials =
            vec![0.; store.lock().expect("art store lock").slots.len().max(1) * MATERIAL_ROWS * 4];
        let mut held = store.lock().expect("art store lock");
        let policy = held.policy;
        let [albedo, surface] = held.textures.admit(
            device,
            queue,
            &plan.textures,
            policy.texture_spare_layers,
            policy.report,
            work,
        );
        let mut written = std::collections::BTreeSet::new();
        for (bundle, content) in plan.bundles.values() {
            if !written.insert(content) {
                continue;
            }
            let base = held.slots.base(content).expect("reserved content");
            for (i, m) in bundle.materials.iter().enumerate() {
                let mut layer = |c: &str| {
                    held.textures
                        .layer(m.textures.get(c).map(|&i| &bundle.textures[i]))
                };
                let at = (base + i) * MATERIAL_ROWS * 4;
                materials[at..at + MATERIAL_ROWS * 4]
                    .copy_from_slice(&material_rows(m, &mut layer));
            }
        }
        drop(held);
        let rows = plan
            .rows
            .iter()
            .map(|row| {
                row.as_ref()
                    .map(|row| row.admit(device, queue, &store, work))
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        work.buffer_bytes_uploaded += bytemuck::cast_slice::<f32, u8>(&materials).len() as u64;
        let authored_materials = materials.clone();
        let materials = crate::buffer(
            device,
            queue,
            "model-materials",
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
            bytemuck::cast_slice(&materials),
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("model-textures"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let palette = growable(device, "model-palette", BufferUsages::STORAGE, 0);
        let instances = growable(device, "model-instances", BufferUsages::VERTEX, 0);
        let layout = pipelines.layout("modelLayout").clone();
        let material_bindings = crate::model_material::ModelMaterialBindings::new(
            device, &layout, &palette, &materials, &albedo, &surface, &sampler,
        );
        let mut installed: Vec<_> = plan.bundles.into_keys().collect();
        installed.sort();
        Ok(Self {
            installed,
            row_names: appearances
                .iter()
                .map(|a| text(a.name, a.name_len).to_owned())
                .collect(),
            submissions: Default::default(),
            material_bindings,
            rows,
            layout,
            materials,
            authored_materials,
            impostor_materials: Default::default(),
            surfaces: [true; 4],
            buildings_shown: true,
            models_shown: true,
            albedo,
            surface,
            sampler,
            palette,
            instances,
            statics: Statics::from_rows(appearances),
            runs: Vec::new(),
            drawn: Drawn::default(),
            xray_instance_count: 0,
            has_xray_meshes: false,
            high_apart: false,
            cards: None,
            store,
        })
    }
}

/// The exact articulated vertices uploaded at admission, before GPU allocation.
pub(super) fn articulated_meshes(
    nodes: &[crate::bundle::Node],
    tier: usize,
    material_base: usize,
    track_side: impl Fn(usize) -> Option<presentation::world::TrackSide>,
) -> Result<Vec<(&Mesh, Vec<u8>)>, String> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let track = match track_side(i) {
                Some(presentation::world::TrackSide::Left) => 1,
                Some(presentation::world::TrackSide::Right) => 2,
                None => 0,
            };
            let mesh = &node.tiers[tier];
            Ok((
                mesh,
                pack_vertices(mesh, material_base, Some(i as u8), track)?,
            ))
        })
        .collect()
}
