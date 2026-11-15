//! Synthetic art and complete runtime admissions for generation contracts.
use super::{support, *};
pub fn identity() -> Joint {
    Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    }
}

pub fn bundle_bytes(
    kind: &str,
    max_z: u8,
    state: &str,
    tier_count: usize,
    textures: &[(&str, usize)],
) -> Vec<u8> {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: Vec<u8>, length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let mut mesh = json!({
        "positions": array("f32", [0_f32,0.,0., 4.,0.,0., 0.,4.,f32::from(max_z)].into_iter().flat_map(f32::to_le_bytes).collect(), 9),
        "normals": array("i16", [0_i16,0,32767,0].repeat(3).into_iter().flat_map(i16::to_le_bytes).collect(), 12),
        "uvs": array("f32", [0_f32,0.,1.,0.,0.,1.].into_iter().flat_map(f32::to_le_bytes).collect(), 6),
        "colors": array("u8", vec![255; 12], 12),
        "indices": array("u16", vec![0, 0, 1, 0, 2, 0], 3),
        "draws": [{"material": 0, "first": 0, "count": 3}],
    });
    if kind == "skinned" {
        mesh["joints"] = array("u8", vec![0; 12], 12);
        mesh["weights"] = array(
            "u16",
            [65535_u16, 0, 0, 0]
                .repeat(3)
                .into_iter()
                .flat_map(u16::to_le_bytes)
                .collect(),
            12,
        );
    }
    let mut content = json!({"kind": kind, "materials": [{
        "name": "synthetic", "base_color": [f32::from(max_z)/4., 0.25, 0.5, 1.], "metallic": 0,
        "roughness": 1, "tint": 0, "coverage": {"kind":"opaque"},
        "textures": textures.iter().copied().collect::<BTreeMap<_,_>>()
    }]});
    if kind == "skinned" {
        content["tiers"] = json!(vec![mesh; tier_count]);
    } else {
        content["states"] = json!([{"name":state, "bounds":{"min":[0,0,0],"max":[1,1,max_z]}, "tiers":vec![mesh; tier_count]}]);
    }
    container(content, json!(views), &body)
}

pub fn detail() -> presentation::world::Detail {
    presentation::world::Detail {
        lod_px: [80., 40., 20.],
        impostor_px: 4.,
        shadow_caster_px: 0.,
    }
}

pub fn runtime_config() -> RuntimeConfig {
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/presentation.json")).unwrap();
    let settings = presentation::settings::PresentationSettings::from_value(&game).unwrap();
    RuntimeConfig {
        art_store: Default::default(),
        size: [64, 64],
        light: settings.light,
        fog_look: settings.fog.selected().unwrap().look().unwrap(),
        grass_density: None,
        xray_min_hidden_fraction: None,
        paint: None,
        separate_high_air: false,
        hdr_output: false,
        water_detail: Default::default(),
        controls: Default::default(),
    }
}

pub fn effects() -> Arc<EffectSheets> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, 4, 4);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    {
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255; 64]).unwrap();
    }
    Arc::new(EffectSheets::decode(4, &[&png, &png]).unwrap())
}

pub fn world_names(presentation: &Presentation) -> Vec<String> {
    let world = presentation.world();
    world
        .appearances
        .iter()
        .map(|row| world.strings[row.name as usize..(row.name + row.name_len) as usize].to_owned())
        .collect()
}

pub fn content_id(bytes: &[u8]) -> ContentId {
    ContentId::bundle(contract::identity::bytes_hash(bytes)).unwrap()
}

pub type Art<'a> = &'a [(&'a str, &'a [u8])];

pub fn catalog(
    presentation: &Presentation,
    art: Art<'_>,
) -> presentation::presentation::catalog::CatalogDraft {
    let wanted = art
        .iter()
        .map(|(name, bytes)| (name.to_string(), content_id(bytes)))
        .collect();
    let facts = art
        .iter()
        .map(|(_, bytes)| (content_id(bytes), Asset::from_bundle(bytes, None).unwrap()))
        .collect();
    presentation
        .stage_catalog(wanted, facts, detail(), Default::default())
        .unwrap()
}

pub fn library(art: Art<'_>, textures: &[(&str, &[u8])]) -> Library {
    let mut library = Library::default();
    for (name, bytes) in art {
        library
            .insert_bundle(name, content_id(bytes), bytes, textures)
            .unwrap();
    }
    library
}

pub fn models(art: Art<'_>) -> Vec<presentation::presentation::lab::ModelInput> {
    serde_json::from_value(json!(art
        .iter()
        .enumerate()
        .map(|(i, (name, bytes))| {
            let bundle = presentation::assets::Container::<serde_json::Value>::parse(
                bytes,
                presentation::assets::BUNDLE,
                name,
            )
            .unwrap();
            json!({
                "appearance":name, "x":(i % 2) as f32 * 6., "y":(i / 2) as f32 * 6., "z":0, "yaw":0,
                "tier":0, "pose":{"kind":"static", "state":bundle.content["states"][0]["name"]}
            })
        })
        .collect::<Vec<_>>()))
    .unwrap()
}

pub fn ground(presentation: &mut Presentation, scenery: Option<&[u8]>) {
    let game =
        serde_json::from_str(include_str!("../../../../fixtures/presentation.json")).unwrap();
    let settings = presentation::settings::PresentationSettings::from_value(&game).unwrap();
    let mut biome = presentation::static_world::ground::biome::Biome::load(
        include_str!("../../../../fixtures/biomes/summer.json"),
        None,
    )
    .unwrap();
    if scenery.is_some() {
        biome.trees.species = vec![presentation::static_world::ground::biome::TreeSpecies {
            appearance: "synthetic-scenery".into(),
            weight: 1.,
            tint: [1.; 3],
            family: None,
            interior_m: None,
        }];
        biome.trees.understorey.appearance = "synthetic-scenery".into();
        biome.trees.hedgerows.appearance = "synthetic-scenery".into();
        biome.forest_floor.dressing.kinds.clear();
        biome.grass.growth.clear();
    }
    presentation
        .set_ground(
            presentation::static_world::ground::Source {
                map: [0., 0., 64., 64.],
                playable: [0., 0., 64., 64.],
                grid: presentation::static_world::ground::tables::Grid {
                    nx: 17,
                    ny: 17,
                    spacing: 4.,
                    page_size: 16,
                    ..Default::default()
                },
                ..Default::default()
            },
            biome,
            settings.tuning(),
        )
        .unwrap();
    if let Some(bytes) = scenery {
        let bundle = renderer_core::bundle::decode("synthetic-scenery", bytes, &[]).unwrap();
        let mesh = renderer_core::scenery::tier_mesh("synthetic-scenery", &bundle, 0).unwrap();
        presentation.place_scenery(&[mesh.as_slice()]).unwrap();
    }
}

pub struct Stage {
    pub presentation: Presentation,
    pub runtime: Runtime,
}

impl Stage {
    pub fn new(
        gpu: &support::Gpu,
        art: Art<'_>,
        config: RuntimeConfig,
        scenery: Option<&[u8]>,
        textures: &[(&str, &[u8])],
    ) -> Self {
        let mut presentation = Presentation::new(None, 30.).unwrap();
        presentation.install_lab_assets(
            art.iter()
                .map(|(name, bytes)| {
                    (
                        name.to_string(),
                        (content_id(bytes), Asset::from_bundle(bytes, None).unwrap()),
                    )
                })
                .collect(),
            detail(),
        );
        presentation.set_lab_models(models(art)).unwrap();
        ground(&mut presentation, scenery);
        let mut library = library(art, textures);
        let mut runtime = Runtime::new(
            &gpu.device,
            &gpu.queue,
            config,
            library.take_admission_work(),
            RuntimeWorld {
                presentation: &mut presentation,
                assets: &library,
                geometry: Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[])),
                effect_sheets: effects(),
            },
        )
        .unwrap();
        pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
        Self {
            presentation,
            runtime,
        }
    }

    pub fn stage(
        &mut self,
        gpu: &support::Gpu,
        art: Art<'_>,
        textures: &[(&str, &[u8])],
    ) -> renderer_core::runtime::ReadyAssets {
        let draft = catalog(&self.presentation, art);
        let mut library = library(art, textures);
        let work = library.take_admission_work();
        let pending = self
            .runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &self.presentation,
                draft,
                &library,
                work,
            )
            .unwrap();
        pollster::block_on(pending.prepare()).unwrap()
    }

    pub fn commit(
        &mut self,
        gpu: &support::Gpu,
        art: Art<'_>,
        ready: renderer_core::runtime::ReadyAssets,
    ) {
        assert!(self
            .runtime
            .commit_assets(
                &gpu.device,
                &gpu.queue,
                &mut self.presentation,
                ready,
                models(art)
            )
            .unwrap());
    }

    pub fn pixels(&mut self, gpu: &support::Gpu) -> Vec<u8> {
        let camera = presentation::view::Camera {
            eye: [12., -16., 16.],
            target: [4., 4., 0.],
            up: [0., 0., 1.],
            fov_y: 0.8,
            near: 0.1,
            size: self.runtime.targets.size,
        };
        self.presentation
            .frame(
                0.,
                &camera,
                &Default::default(),
                self.runtime.static_generation(),
            )
            .unwrap();
        self.runtime
            .prepare(&gpu.device, &gpu.queue, &mut self.presentation)
            .unwrap();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        self.runtime.encode(
            &gpu.device,
            &gpu.queue,
            &self.presentation.last_frame().view,
            &mut encoder,
            Default::default(),
        );
        let read = renderer_core::readback::RgbaReadback::encode(
            &gpu.device,
            &mut encoder,
            &self.runtime.targets.display_texture,
        )
        .unwrap();
        gpu.queue.submit([encoder.finish()]);
        pollster::block_on(read.read(&gpu.device)).unwrap()
    }
}

/// The same small mesh with enough material rows to exercise ABI capacity.
pub fn material_bundle(count: usize) -> Vec<u8> {
    let bytes = bundle_bytes("static", 1, "default", 1, &[]);
    let header_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut header: serde_json::Value =
        serde_json::from_slice(&bytes[16..16 + header_len]).unwrap();
    let material = header["content"]["materials"][0].clone();
    header["content"]["materials"] = json!(vec![material; count]);
    container(
        header["content"].take(),
        header["views"].take(),
        &bytes[16 + header_len..],
    )
}

pub fn container(content: serde_json::Value, views: serde_json::Value, body: &[u8]) -> Vec<u8> {
    let mut header = serde_json::to_vec(&json!({"content": content, "views": views})).unwrap();
    header.resize(header.len().next_multiple_of(4), b' ');
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, header.len() as u32, body.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(header);
    bytes.extend(body);
    bytes
}

pub fn card_bundle(color: [f32; 3]) -> (Vec<u8>, Vec<u8>) {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: &[u8], length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let positions = [
        -0.35f32, -0.2, 0., 0.3, -0.2, 0., -0.1, 0.25, 0., -0.1, 0., 0.8,
    ];
    let mesh = json!({
        "positions": array("f32", bytemuck::cast_slice(&positions), positions.len()),
        "normals": array("i16", bytemuck::cast_slice(&[0i16, 0, 32767, 0].repeat(4)), 16),
        "uvs": array("f32", bytemuck::cast_slice(&[0f32; 8]), 8),
        "colors": array("u8", &[255; 16], 16),
        "joints": array("u8", &[0; 16], 16),
        "weights": array("u16", bytemuck::cast_slice(&[65535u16, 0, 0, 0].repeat(4)), 16),
        "indices": array("u16", bytemuck::cast_slice(&[0u16, 1, 2, 0, 1, 3, 1, 2, 3, 2, 0, 3]), 12),
        "draws": [{"material": 0, "first": 0, "count": 12}],
    });
    let bundle = container(
        json!({
            "kind": "skinned", "skeleton": "synthetic",
            "materials": [{"name":"cloth", "base_color":[color[0],color[1],color[2],1],
                "metallic":0, "roughness":1, "tint":1, "coverage":{"kind":"opaque"}}],
            "joints": [{"name":"root", "parent":-1,
                "bind":{"t":[0,0,0],"r":[0,0,0,1],"s":[1,1,1]},
                "inverse_bind":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}],
            "tiers":[mesh], "far_pose":{"clip":"rise","phase":0},
            "corpse_poses":[{"clip":"rise","phase":1}], "sockets":[]
        }),
        json!(views),
        &body,
    );
    let mut samples = vec![0, 2];
    samples.extend(bytemuck::cast_slice::<f32, u8>(&[0., 0., 0., 0.4, 0., 0.2]));
    let skeleton = container(
        json!({"sample_hz":1,"clips":[{
            "name":"rise","frames":2,"duration":1,"loop":false,
            "rotation_modes":{"$view":0},"translation_modes":{"$view":1},
            "rotations":{"$view":2},"translations":{"$view":3}
        }]}),
        json!([["u8", 0, 1], ["u8", 1, 1], ["i16", 2, 0], ["f32", 2, 6]]),
        &samples,
    );
    (bundle, skeleton)
}

pub fn card_catalog(
    presentation: &Presentation,
) -> (
    Library,
    presentation::presentation::catalog::CatalogDraft,
    Vec<presentation::presentation::lab::ModelInput>,
) {
    let mut library = Library::default();
    let mut wanted = BTreeMap::new();
    let mut facts = BTreeMap::new();
    let mut models = Vec::new();
    for (name, color) in [("cloth-a", [0.3, 0.6, 0.9]), ("cloth-b", [0.8, 0.4, 0.2])] {
        let (bundle, skeleton) = card_bundle(color);
        let content = ContentId::new(
            contract::identity::bytes_hash(&bundle),
            Some(contract::identity::bytes_hash(&skeleton)),
        )
        .unwrap();
        library
            .insert_bundle(name, content.clone(), &bundle, &[])
            .unwrap();
        wanted.insert(name.into(), content.clone());
        facts.insert(
            content,
            Asset::from_bundle(&bundle, Some(&skeleton)).unwrap(),
        );
        for pose in [json!({"kind":"rest"}), json!({"kind":"corpse","lies":0})] {
            models.push(
                serde_json::from_value(json!({
                    "appearance":name, "x":0,"y":0,"z":0,"yaw":0,"pose":pose
                }))
                .unwrap(),
            );
        }
    }
    let catalog = presentation
        .stage_catalog(wanted, facts, detail(), Default::default())
        .unwrap();
    (library, catalog, models)
}

/// Deterministic nonuniform RGBA mips in the runtime texture wire format.
pub fn texture_bytes(size: u32, srgb: bool, value: u8) -> Vec<u8> {
    let format = if srgb {
        "rgba8unorm-srgb"
    } else {
        "rgba8unorm"
    };
    let mut bytes = format!("{format} {size}x{size}\n").into_bytes();
    for level in 0..=size.ilog2() {
        let edge = size >> level;
        for y in 0..edge {
            for x in 0..edge {
                bytes.extend([
                    value.wrapping_add((x * 11) as u8),
                    value.wrapping_add((y * 17) as u8),
                    value,
                    255,
                ]);
            }
        }
    }
    bytes
}

pub fn admit_models(
    gpu: &support::Gpu,
    pipelines: &renderer_core::pipelines::Pipelines,
    art: Art<'_>,
    textures: &[(&str, &[u8])],
    store: renderer_core::models::SharedArtStore,
) -> (Models, AdmissionWork) {
    let mut library = library(art, textures);
    let mut names = String::new();
    let mut rows = Vec::new();
    for (name, _) in art {
        rows.push(Appearance {
            name: names.len() as u32,
            name_len: name.len() as u32,
            lies: NONE,
            ..Default::default()
        });
        names.push_str(name);
    }
    let mut work = library.take_admission_work();
    let models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        pipelines,
        &library,
        &rows,
        &names,
        |_| Some(vec![identity()]),
        |_, _| None,
        store,
        &mut work,
    )
    .unwrap();
    (models, work)
}

pub fn model_memory(
    models: &[&Models],
    store: &renderer_core::models::SharedArtStore,
) -> renderer_core::memory::AllocationCounts {
    let mut memory = renderer_core::memory::ResourceMemory::default();
    store.lock().unwrap().record_memory(&mut memory);
    for model in models {
        model.record_memory(&mut memory);
    }
    memory.counts()
}
