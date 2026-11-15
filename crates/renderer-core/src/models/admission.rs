//! Authored model rows and shared material admission.
use super::*;
use crate::models::store::{GeometryKey, SharedArtStore};
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
        let mut bundles = std::collections::BTreeMap::new();
        let mut wanted = std::collections::BTreeMap::new();
        for a in appearances {
            if a.draws != 0 && a.draws != DRAWS_PROP && a.draws != DRAWS_PROP_GROUND {
                continue;
            }
            let name = text(a.name, a.name_len);
            if bundles.contains_key(name) {
                continue;
            }
            let content = library.content_id(name)?;
            let bundle = library.bundle(name)?;
            wanted.insert(content.clone(), bundle.materials.len());
            bundles.insert(name.to_owned(), (bundle, content));
        }
        let (store, reason) = ArtStore::admit(store, &wanted)?;
        if reason.is_some() {
            work.rebuild_reason = reason;
        }
        let mut materials =
            vec![0.; store.lock().expect("art store lock").slots.len().max(1) * MATERIAL_ROWS * 4];
        let mut layers = Layers::default();
        let mut written = std::collections::BTreeSet::new();
        for (bundle, content) in bundles.values() {
            if !written.insert(content) {
                continue;
            }
            let base = store
                .lock()
                .expect("art store lock")
                .slots
                .base(content)
                .expect("reserved content");
            for (i, m) in bundle.materials.iter().enumerate() {
                let mut layer =
                    |c: &str| layers.layer(m.textures.get(c).map(|&i| &bundle.textures[i]));
                let at = (base + i) * MATERIAL_ROWS * 4;
                materials[at..at + MATERIAL_ROWS * 4]
                    .copy_from_slice(&material_rows(m, &mut layer));
            }
        }
        let mut rows = Vec::new();
        for (r, a) in appearances.iter().enumerate() {
            // The scenery's rows are the scenery layer's (`scenery.rs`).
            let prop = a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND;
            if a.draws != 0 && !prop {
                rows.push(None);
                continue;
            }
            let (name, state) = (text(a.name, a.name_len), text(a.state, a.state_len));
            let (bundle, content) = &bundles[name];
            let base = store
                .lock()
                .expect("art store lock")
                .slots
                .base(content)
                .expect("reserved content");
            let label = format!("{name}-{r}");
            let mut single = |mesh: &Mesh, rigid| {
                Ok(drawable(
                    device,
                    queue,
                    &label,
                    bundle,
                    &[(mesh, pack_vertices(mesh, base, rigid, 0)?)],
                    work,
                ))
            };
            let rest = || rest_palette(r as u32).ok_or(format!("{name}: no rest pose for row {r}"));
            let row = match &bundle.shape {
                Shape::Skinned(tiers) if a.lies != NONE => {
                    let palette = rest()?;
                    let rest = tiers.first().map(|mesh| {
                        RestPose::new(name, &posed_mesh(mesh, &palette), RestPose::identity())
                    });
                    let key = GeometryKey {
                        content: content.clone(),
                        state: String::new(),
                        pose: Some(bytemuck::cast_slice(&palette).to_vec()),
                        tracks: Vec::new(),
                    };
                    let tiers = ArtStore::geometry(&store, key, || {
                        tiers
                            .iter()
                            .map(|m| single(&posed_mesh(m, &palette), Some(0)))
                            .collect()
                    })?;
                    Row {
                        kind: Kind::Corpse,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        tiers,
                        card: None,
                        rest,
                    }
                }
                Shape::Skinned(tiers) => {
                    let palette = rest()?;
                    let rest = tiers.first().map(|mesh| {
                        RestPose::new(name, &posed_mesh(mesh, &palette), palette.clone())
                    });
                    Row {
                        kind: Kind::Soldier,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        card: None,
                        rest,
                        tiers: ArtStore::geometry(
                            &store,
                            GeometryKey {
                                content: content.clone(),
                                state: String::new(),
                                pose: None,
                                tracks: Vec::new(),
                            },
                            || tiers.iter().map(|m| single(m, None)).collect(),
                        )?,
                    }
                }
                Shape::Articulated(nodes) => {
                    let count = nodes.first().map_or(0, |n| n.tiers.len());
                    let tracks = nodes
                        .iter()
                        .enumerate()
                        .map(|(node, _)| match track_side(r as u32, node) {
                            Some(presentation::world::TrackSide::Left) => 1,
                            Some(presentation::world::TrackSide::Right) => 2,
                            None => 0,
                        })
                        .collect();
                    let tiers = ArtStore::geometry(
                        &store,
                        GeometryKey {
                            content: content.clone(),
                            state: String::new(),
                            pose: None,
                            tracks,
                        },
                        || {
                            (0..count)
                                .map(|t| {
                                    let meshes = articulated_meshes(nodes, t, base, |node| {
                                        track_side(r as u32, node)
                                    })?;
                                    Ok(drawable(device, queue, &label, bundle, &meshes, work))
                                })
                                .collect()
                        },
                    )?;
                    Row {
                        kind: Kind::Vehicle,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        tiers,
                        card: None,
                        rest: rest_palette(r as u32)
                            .map(|palette| RestPose::articulated(name, nodes, palette)),
                    }
                }
                Shape::Static(states) => {
                    // A piece the bundle lacks (a wreck with no turret to
                    // throw) is a row nothing draws.
                    let found = states
                        .iter()
                        .find(|s| s.name == state)
                        .or_else(|| state.is_empty().then(|| states.first()).flatten());
                    let Some(s) = found else {
                        rows.push(None);
                        continue;
                    };
                    Row {
                        kind: if prop { Kind::Prop } else { Kind::Piece },
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        tiers: ArtStore::geometry(
                            &store,
                            GeometryKey {
                                content: content.clone(),
                                state: s.name.clone(),
                                pose: None,
                                tracks: Vec::new(),
                            },
                            || s.tiers.iter().map(|m| single(m, Some(0))).collect(),
                        )?,
                        card: None,
                        rest: s
                            .tiers
                            .first()
                            .map(|mesh| RestPose::new(name, mesh, RestPose::identity())),
                    }
                }
            };
            rows.push(Some(row));
        }
        work.buffer_bytes_uploaded += bytemuck::cast_slice::<f32, u8>(&materials).len() as u64;
        let authored_materials = materials.clone();
        let materials = crate::buffer(
            device,
            queue,
            "model-materials",
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
            bytemuck::cast_slice(&materials),
        );
        let albedo = texture_array(
            device,
            queue,
            "model-albedo",
            TextureFormat::Rgba8UnormSrgb,
            &layers.albedo,
            work,
        );
        let surface = texture_array(
            device,
            queue,
            "model-surface",
            TextureFormat::Rgba8Unorm,
            &layers.surface,
            work,
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
        let mut installed: Vec<_> = bundles.into_keys().collect();
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
