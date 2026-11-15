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
        let mut wanted_textures = std::collections::BTreeMap::new();
        for (bundle, _) in bundles.values() {
            for material in &bundle.materials {
                for &index in material.textures.values() {
                    let texture = &bundle.textures[index];
                    wanted_textures.insert(texture.id.clone(), texture.clone());
                }
            }
        }
        let plans = super::plan::rows(
            appearances,
            strings,
            &bundles,
            &wanted_textures,
            &rest_palette,
            &track_side,
        )?;
        let wanted_cards = plans
            .iter()
            .flatten()
            .filter_map(|plan| plan.card.clone())
            .collect();
        let (store, reason) = ArtStore::admit(
            store,
            &wanted,
            &wanted_textures,
            &wanted_cards,
            device.limits().max_texture_array_layers,
        )?;
        if reason.is_some() {
            work.rebuild_reason = reason;
        }
        let mut materials =
            vec![0.; store.lock().expect("art store lock").slots.len().max(1) * MATERIAL_ROWS * 4];
        let mut held = store.lock().expect("art store lock");
        let policy = held.policy;
        let [albedo, surface] = held.textures.admit(
            device,
            queue,
            &wanted_textures,
            policy.texture_spare_layers,
            policy.report,
            work,
        );
        let mut written = std::collections::BTreeSet::new();
        for (bundle, content) in bundles.values() {
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
        let mut rows = Vec::new();
        for (r, a) in appearances.iter().enumerate() {
            // The scenery's rows are the scenery layer's (`scenery.rs`).
            let prop = a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND;
            if a.draws != 0 && !prop {
                rows.push(None);
                continue;
            }
            let Some(plan) = &plans[r] else {
                rows.push(None);
                continue;
            };
            let name = text(a.name, a.name_len);
            let (bundle, content) = &bundles[name];
            let base = store
                .lock()
                .expect("art store lock")
                .slots
                .base(content)
                .expect("reserved content");
            let label = format!("{name}-{r}");
            let single =
                |mesh: &Mesh, rigid, work: &mut AdmissionWork| -> Result<Drawable, String> {
                    Ok(drawable(
                        device,
                        queue,
                        &label,
                        bundle,
                        &[(mesh, pack_vertices(mesh, base, rigid, 0)?)],
                        work,
                    ))
                };
            let rest = || {
                plan.palette
                    .clone()
                    .ok_or(format!("{name}: no rest pose for row {r}"))
            };
            let row = match &bundle.shape {
                Shape::Skinned(tiers) if a.lies != NONE => {
                    let palette = rest()?;
                    let geometry =
                        ArtStore::geometry(&store, plan.geometry.clone(), work, |work| {
                            let finest = tiers.first().map(|mesh| posed_mesh(mesh, &palette));
                            let tiers = tiers
                                .iter()
                                .enumerate()
                                .map(|(tier, mesh)| {
                                    if tier == 0 {
                                        single(finest.as_ref().expect("finest tier"), Some(0), work)
                                    } else {
                                        single(&posed_mesh(mesh, &palette), Some(0), work)
                                    }
                                })
                                .collect::<Result<_, _>>()?;
                            Ok(ModelGeometry {
                                tiers,
                                rests: Default::default(),
                            })
                        })?;
                    let rest = tiers.first().map(|mesh| {
                        geometry.rest_pose(&palette, work, || {
                            RestPose::new(&posed_mesh(mesh, &palette), RestPose::identity())
                        })
                    });
                    Row {
                        rest,
                        kind: Kind::Corpse,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        geometry,
                        card: None,
                        card_key: plan.card.clone(),
                    }
                }
                Shape::Skinned(tiers) => {
                    let palette = rest()?;
                    let geometry =
                        ArtStore::geometry(&store, plan.geometry.clone(), work, |work| {
                            let tiers = tiers
                                .iter()
                                .map(|m| single(m, None, work))
                                .collect::<Result<_, _>>()?;
                            Ok(ModelGeometry {
                                tiers,
                                rests: Default::default(),
                            })
                        })?;
                    let rest = tiers.first().map(|mesh| {
                        geometry.rest_pose(&palette, work, || {
                            RestPose::new(&posed_mesh(mesh, &palette), palette.clone())
                        })
                    });
                    Row {
                        rest,
                        kind: Kind::Soldier,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        card: None,
                        card_key: plan.card.clone(),
                        geometry,
                    }
                }
                Shape::Articulated(nodes) => {
                    let count = nodes.first().map_or(0, |n| n.tiers.len());
                    let palette = plan.palette.clone();
                    let geometry =
                        ArtStore::geometry(&store, plan.geometry.clone(), work, |work| {
                            let tiers = (0..count)
                                .map(|t| {
                                    let meshes = articulated_meshes(nodes, t, base, |node| {
                                        track_side(r as u32, node)
                                    })?;
                                    Ok(drawable(device, queue, &label, bundle, &meshes, work))
                                })
                                .collect::<Result<_, String>>()?;
                            Ok(ModelGeometry {
                                tiers,
                                rests: Default::default(),
                            })
                        })?;
                    let rest = palette.as_ref().map(|palette| {
                        geometry.rest_pose(palette, work, || {
                            RestPose::articulated(nodes, palette.clone())
                        })
                    });
                    Row {
                        rest,
                        kind: Kind::Vehicle,
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        geometry,
                        card: None,
                        card_key: plan.card.clone(),
                    }
                }
                Shape::Static(states) => {
                    let s = states
                        .iter()
                        .find(|s| s.name == plan.geometry.state)
                        .expect("planned authored state");
                    let geometry =
                        ArtStore::geometry(&store, plan.geometry.clone(), work, |work| {
                            let tiers = s
                                .tiers
                                .iter()
                                .map(|m| single(m, Some(0), work))
                                .collect::<Result<_, _>>()?;
                            Ok(ModelGeometry {
                                tiers,
                                rests: Default::default(),
                            })
                        })?;
                    let palette = RestPose::identity();
                    let rest = s.tiers.first().map(|mesh| {
                        geometry.rest_pose(&palette, work, || RestPose::new(mesh, palette.clone()))
                    });
                    Row {
                        rest,
                        kind: if prop { Kind::Prop } else { Kind::Piece },
                        building: a.draws == 0 && a.placed != 0,
                        prop,
                        geometry,
                        card: None,
                        card_key: plan.card.clone(),
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
