//! Authored model rows and shared material admission.
use super::*;

impl Models {
    /// Installs every row of the presentation's world from `library`.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        library: &impl BundleSource,
        catalog: presentation::presentation::catalog::CatalogView<'_>,
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
    ) -> Result<Self, String> {
        let text = |at: u32, len: u32| &strings[at as usize..(at + len) as usize];
        let mut bundles: HashMap<String, (Arc<Bundle>, usize)> = HashMap::new();
        let mut materials: Vec<f32> = Vec::new();
        let mut layers = Layers::default();
        let mut rows = Vec::new();
        // Routing and placement rows share immutable geometry within this
        // material generation. Corpse poses remain distinct authored meshes.
        let mut geometry: HashMap<(String, String, u32), Arc<[Drawable]>> = HashMap::new();
        for (r, a) in appearances.iter().enumerate() {
            // The scenery's rows are the scenery layer's (`scenery.rs`).
            let prop = a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND;
            if a.draws != 0 && !prop {
                rows.push(None);
                continue;
            }
            let (name, state) = (text(a.name, a.name_len), text(a.state, a.state_len));
            if !bundles.contains_key(name) {
                let bundle = library.bundle(name)?;
                let base = materials.len() / (MATERIAL_ROWS * 4);
                for m in &bundle.materials {
                    let mut layer =
                        |c: &str| layers.layer(m.textures.get(c).map(|&i| &bundle.textures[i]));
                    materials.extend(material_rows(m, &mut layer));
                }
                bundles.insert(name.to_owned(), (bundle, base));
            }
            let (bundle, base) = &bundles[name];
            let label = format!("{name}-{r}");
            let single = |mesh: &Mesh, rigid| {
                drawable(
                    device,
                    queue,
                    &label,
                    bundle,
                    &[(mesh, pack_vertices(mesh, *base, rigid, 0))],
                )
            };
            let rest = || rest_palette(r as u32).ok_or(format!("{name}: no rest pose for row {r}"));
            let row = match &bundle.shape {
                Shape::Skinned(tiers) if a.lies != NONE => {
                    let palette = rest()?;
                    let rest = tiers.first().map(|mesh| {
                        RestPose::new(name, &posed_mesh(mesh, &palette), RestPose::identity())
                    });
                    let tiers = geometry
                        .entry((name.to_owned(), String::new(), a.lies))
                        .or_insert_with(|| {
                            tiers
                                .iter()
                                .map(|m| single(&posed_mesh(m, &palette), Some(0)))
                                .collect()
                        })
                        .clone();
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
                        tiers: geometry
                            .entry((name.to_owned(), String::new(), NONE))
                            .or_insert_with(|| tiers.iter().map(|m| single(m, None)).collect())
                            .clone(),
                    }
                }
                Shape::Articulated(nodes) => {
                    let count = nodes.first().map_or(0, |n| n.tiers.len());
                    let tiers = geometry
                        .entry((name.to_owned(), String::new(), NONE))
                        .or_insert_with(|| {
                            (0..count)
                                .map(|t| {
                                    let meshes = articulated_meshes(nodes, t, *base, |node| {
                                        track_side(r as u32, node)
                                    });
                                    drawable(device, queue, &label, bundle, &meshes)
                                })
                                .collect()
                        })
                        .clone();
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
                        tiers: geometry
                            .entry((name.to_owned(), s.name.clone(), NONE))
                            .or_insert_with(|| s.tiers.iter().map(|m| single(m, Some(0))).collect())
                            .clone(),
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
        if materials.is_empty() {
            materials.resize(MATERIAL_ROWS * 4, 0.0);
        }
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
        );
        let surface = texture_array(
            device,
            queue,
            "model-surface",
            TextureFormat::Rgba8Unorm,
            &layers.surface,
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
        })
    }
}

/// The exact articulated vertices uploaded at admission, before GPU allocation.
pub(super) fn articulated_meshes(
    nodes: &[crate::bundle::Node],
    tier: usize,
    material_base: usize,
    track_side: impl Fn(usize) -> Option<presentation::world::TrackSide>,
) -> Vec<(&Mesh, Vec<u8>)> {
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
            (
                mesh,
                pack_vertices(mesh, material_base, Some(i as u8), track),
            )
        })
        .collect()
}
