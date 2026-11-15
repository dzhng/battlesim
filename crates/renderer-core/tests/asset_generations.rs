//! Synthetic admission work and generation-swap contracts.
#![allow(clippy::too_many_arguments)]

#[path = "support/readback.rs"]
mod readback;
#[path = "support/mod.rs"]
mod support;

use contract::identity::ContentId;
use presentation::presentation::{lab::Asset, Presentation, Ui};
use presentation::records::{Appearance, Joint, NONE};
use renderer_core::bundle::Library;
use renderer_core::effects::EffectSheets;
use renderer_core::models::Models;
use renderer_core::models::VERTEX_BYTES;
use renderer_core::runtime::PlayerGeometry;
use renderer_core::runtime::WorldGeometry;
use renderer_core::runtime::{AdmissionWork, Runtime, RuntimeConfig, RuntimeWorld};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

fn identity() -> Joint {
    Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    }
}

fn bundle_bytes(kind: &str, max_z: u8, state: &str, tier_count: usize) -> Vec<u8> {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: Vec<u8>, length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let mesh = json!({
        "positions": array("f32", bytemuck::cast_slice(&[0.5f32, 0., 0., 0.5, 0., max_z as f32, 0., 0.5, max_z as f32]).to_vec(), 9),
        "normals": array("i16", vec![0; 24], 12),
        "uvs": array("f32", vec![0; 24], 6),
        "colors": array("u8", vec![255; 12], 12),
        "indices": array("u16", vec![0, 0, 1, 0, 2, 0], 3),
        "draws": [{"material": 0, "first": 0, "count": 3}],
    });
    let content = json!({"kind": kind, "materials": [{
        "name": "synthetic", "base_color": [1,1,1,1], "metallic": 0,
        "roughness": 1, "tint": 0, "coverage": {"kind":"opaque"}
    }], "states": [{"name":state, "bounds":{"min":[0,0,0],"max":[1,1,max_z]}, "tiers":vec![mesh; tier_count]}]});
    let mut header = serde_json::to_vec(&json!({"content": content, "views": views})).unwrap();
    header.resize(header.len().next_multiple_of(4), b' ');
    let mut bytes = Vec::new();
    for word in [0x5441_4742_u32, 4, header.len() as u32, body.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(header);
    bytes.extend(body);
    bytes
}

fn detail() -> presentation::world::Detail {
    presentation::world::Detail {
        lod_px: [80., 40., 20.],
        impostor_px: 4.,
        shadow_caster_px: 0.,
    }
}

fn runtime_config() -> RuntimeConfig {
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let settings = presentation::settings::PresentationSettings::from_value(&game).unwrap();
    RuntimeConfig {
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

fn effects() -> Arc<EffectSheets> {
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

fn world_names(presentation: &Presentation) -> Vec<String> {
    let world = presentation.world();
    world
        .appearances
        .iter()
        .map(|row| world.strings[row.name as usize..(row.name + row.name_len) as usize].to_owned())
        .collect()
}

fn content_id(bytes: &[u8]) -> ContentId {
    ContentId::bundle(contract::identity::bytes_hash(bytes)).unwrap()
}

fn static_presentation(scenery_bytes: &[u8]) -> Presentation {
    let mut p = Presentation::new(None, 30.).unwrap();
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let settings = presentation::settings::PresentationSettings::from_value(&game).unwrap();
    let mut biome = presentation::static_world::ground::biome::Biome::load(
        include_str!("../../../fixtures/biomes/summer.json"),
        None,
    )
    .unwrap();
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
    p.set_ground(
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
    let bundle = renderer_core::bundle::decode("synthetic-scenery", scenery_bytes, &[]).unwrap();
    let mesh = renderer_core::scenery::tier_mesh("synthetic-scenery", &bundle, 0).unwrap();
    p.place_scenery(&[mesh.as_slice()]).unwrap();
    p
}

fn render_pixels(
    gpu: &support::Gpu,
    runtime: &mut Runtime,
    presentation: &mut Presentation,
) -> Vec<[u8; 4]> {
    let camera = presentation::view::Camera {
        eye: [32., -48., 32.],
        target: [32., 32., 0.],
        up: [0., 0., 1.],
        fov_y: 1.,
        near: 0.1,
        size: [64, 64],
    };
    presentation
        .frame(0., &camera, &Ui::default(), runtime.static_generation())
        .unwrap();
    runtime
        .prepare(&gpu.device, &gpu.queue, presentation)
        .unwrap();
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    runtime.encode(
        &gpu.device,
        &gpu.queue,
        &presentation.last_frame().view,
        &mut encoder,
        PlayerGeometry::default(),
    );
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("statics-carry-pixels"),
        size: 64 * 64 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        runtime.targets.display_texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(64 * 4),
                rows_per_image: None,
            },
        },
        runtime.targets.display_texture.size(),
    );
    readback::pixels(&gpu.device, &gpu.queue, encoder, &buffer)
}

#[test]
fn same_name_with_new_content_replaces_the_admitted_art() {
    let bytes_a = bundle_bytes("static", 1, "default", 1);
    let bytes_b = bundle_bytes("static", 2, "default", 1);

    let mut library = Library::default();
    library
        .insert_bundle("synthetic", content_id(&bytes_a), &bytes_a, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 1);
    let old = library.bundle("synthetic").unwrap();
    let old_top = match &old.shape {
        renderer_core::bundle::Shape::Static(states) => states[0].top,
        _ => panic!("synthetic bundle is static"),
    };

    library
        .insert_bundle("synthetic", content_id(&bytes_b), &bytes_b, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 1);
    assert_eq!(Arc::strong_count(&old), 1);
    let new_top = match &library.bundle("synthetic").unwrap().shape {
        renderer_core::bundle::Shape::Static(states) => states[0].top,
        _ => panic!("synthetic bundle is static"),
    };
    assert_ne!(old_top, new_top);
}

#[test]
fn identical_content_under_a_second_name_decodes_once() {
    let bytes = bundle_bytes("static", 1, "default", 1);
    let id = content_id(&bytes);
    let mut library = Library::default();
    library
        .insert_bundle("synthetic-a", id.clone(), &bytes, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 1);
    library
        .insert_bundle("synthetic-b", id, &bytes, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 0);
    assert!(Arc::ptr_eq(
        &library.bundle("synthetic-a").unwrap(),
        &library.bundle("synthetic-b").unwrap()
    ));
    let old = library.bundle("synthetic-a").unwrap();
    let replacement = bundle_bytes("static", 2, "default", 1);
    library
        .insert_bundle("synthetic-a", content_id(&replacement), &replacement, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 1);
    assert!(Arc::ptr_eq(&old, &library.bundle("synthetic-b").unwrap()));
}

#[test]
fn first_admission_reports_the_synthetic_fixture_work() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let mut library =
        Library::from_bundle("synthetic", &bundle_bytes("static", 1, "default", 1), &[]).unwrap();
    let mut work = library.take_admission_work();
    let _models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        &[Appearance {
            name_len: 9,
            lies: NONE,
            ..Default::default()
        }],
        "synthetic",
        |_| Some(vec![identity()]),
        |_, _| None,
        &mut work,
    )
    .unwrap();

    let vertices = 3;
    let indices = 3;
    let materials = 1;
    let expected_vertex_bytes = vertices * VERTEX_BYTES;
    let expected_material_bytes = materials * 5 * 4 * std::mem::size_of::<f32>();
    let expected_index_bytes = indices * std::mem::size_of::<u32>();
    assert_eq!(
        work,
        AdmissionWork {
            bundles_decoded: 1,
            vertex_bytes_packed: expected_vertex_bytes as u64,
            buffer_bytes_uploaded: (expected_vertex_bytes
                + expected_index_bytes
                + expected_material_bytes) as u64,
            texture_layers_written: 0,
            texture_bytes_uploaded: 0,
            cards_baked: 0,
            statics_bytes_uploaded: 0,
            scenery_placements: 0,
            rebuild_reason: None,
        }
    );
}

#[test]
fn unchanged_statics_are_carried_without_uploads_or_replacement_placement() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let scenery_bytes = bundle_bytes("static", 1, "summer", 4);
    let unit_bytes = bundle_bytes("static", 1, "default", 1);
    let mut presentation = static_presentation(&scenery_bytes);
    let geometry = Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[]));
    let mut library = Library::from_bundle("synthetic-scenery", &scenery_bytes, &[]).unwrap();
    library
        .insert_bundle("new-unit", content_id(&unit_bytes), &unit_bytes, &[])
        .unwrap();
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config.clone(),
        library.take_admission_work(),
        RuntimeWorld {
            presentation: &mut presentation,
            assets: &library,
            geometry: geometry.clone(),
            effect_sheets: effects(),
        },
    )
    .unwrap();
    pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let candidate = presentation
        .stage_catalog(
            [(
                "new-unit".into(),
                Asset::from_bundle(&unit_bytes, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut next_library = library.empty_generation();
    next_library
        .insert_bundle(
            "synthetic-scenery",
            content_id(&scenery_bytes),
            &scenery_bytes,
            &[],
        )
        .unwrap();
    next_library
        .insert_bundle("new-unit", content_id(&unit_bytes), &unit_bytes, &[])
        .unwrap();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &next_library,
            next_library.clone().take_admission_work(),
        )
        .unwrap();
    let ready = pollster::block_on(pending.prepare()).unwrap();
    assert_eq!(ready.work().scenery_placements, 0);
    assert_eq!(ready.work().statics_bytes_uploaded, 0);
    assert!(runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut presentation,
            ready,
            Vec::new()
        )
        .unwrap());
    let carried = render_pixels(&gpu, &mut runtime, &mut presentation);

    let mut clean_presentation = static_presentation(&scenery_bytes);
    let clean_catalog = clean_presentation
        .stage_catalog(
            [(
                "new-unit".into(),
                Asset::from_bundle(&unit_bytes, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    assert!(clean_presentation
        .commit_catalog(clean_catalog, Vec::new())
        .unwrap());
    let mut clean_runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config,
        next_library.take_admission_work(),
        RuntimeWorld {
            presentation: &mut clean_presentation,
            assets: &next_library,
            geometry,
            effect_sheets: effects(),
        },
    )
    .unwrap();
    pollster::block_on(clean_runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let clean = render_pixels(&gpu, &mut clean_runtime, &mut clean_presentation);
    assert_eq!(
        carried, clean,
        "carried statics differ from a clean final-catalog build"
    );
}

#[test]
fn changed_scenery_content_places_and_uploads_again() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let original = bundle_bytes("static", 1, "summer", 4);
    let changed = bundle_bytes("static", 2, "summer", 4);
    let mut presentation = static_presentation(&original);
    let geometry = Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[]));
    let mut library = Library::from_bundle("synthetic-scenery", &original, &[]).unwrap();
    let unit = bundle_bytes("static", 1, "default", 1);
    library
        .insert_bundle("new-unit", content_id(&unit), &unit, &[])
        .unwrap();
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config,
        library.take_admission_work(),
        RuntimeWorld {
            presentation: &mut presentation,
            assets: &library,
            geometry,
            effect_sheets: effects(),
        },
    )
    .unwrap();
    let candidate = presentation
        .stage_catalog(
            [
                (
                    "synthetic-scenery".into(),
                    Asset::from_bundle(&changed, None).unwrap(),
                ),
                ("new-unit".into(), Asset::from_bundle(&unit, None).unwrap()),
            ]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut next = library.clone();
    next.insert_bundle("synthetic-scenery", content_id(&changed), &changed, &[])
        .unwrap();
    next.insert_bundle("new-unit", content_id(&unit), &unit, &[])
        .unwrap();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &next,
            next.clone().take_admission_work(),
        )
        .unwrap();
    let ready = pollster::block_on(pending.prepare()).unwrap();
    assert_eq!(ready.work().scenery_placements, 1);
    assert!(ready.work().statics_bytes_uploaded > 0);
}

#[test]
fn real_generation_swap_keeps_live_work_for_dropped_stale_and_failed_candidates() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes_a = bundle_bytes("static", 1, "default", 1);
    let bytes_b = bundle_bytes("static", 1, "default", 1);
    let asset_a = Asset::from_bundle(&bytes_a, None).unwrap();
    let asset_b = Asset::from_bundle(&bytes_b, None).unwrap();
    let mut presentation = Presentation::new(None, 30.).unwrap();
    presentation.install_lab_assets(BTreeMap::from([("synthetic-a".into(), asset_a)]), detail());
    presentation
        .set_lab_models(
            serde_json::from_value(json!([{
                "appearance":"synthetic-a", "x":0, "y":0, "z":0, "yaw":0,
                "tier":0, "pose":{"kind":"static", "state":"default"}
            }]))
            .unwrap(),
        )
        .unwrap();
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let settings = presentation::settings::PresentationSettings::from_value(&game).unwrap();
    let biome = presentation::static_world::ground::biome::Biome::load(
        include_str!("../../../fixtures/biomes/summer.json"),
        None,
    )
    .unwrap();
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
    let geometry = Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[]));
    let mut library_a = Library::from_bundle("synthetic-a", &bytes_a, &[]).unwrap();
    let work_a = library_a.take_admission_work();
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        runtime_config(),
        work_a,
        RuntimeWorld {
            presentation: &mut presentation,
            assets: &library_a,
            geometry: geometry.clone(),
            effect_sheets: effects(),
        },
    )
    .unwrap();
    pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let live = runtime.admission_work();

    let candidate = presentation
        .stage_catalog(
            [("synthetic-b".into(), asset_b)].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut library_b = Library::from_bundle("synthetic-b", &bytes_b, &[]).unwrap();
    let library_b_work = library_b.take_admission_work();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &library_b,
            library_b_work,
        )
        .unwrap();
    let ready = pollster::block_on(pending.prepare()).unwrap();
    let candidate_work = ready.work().clone();
    assert_ne!(candidate_work, live);
    assert!(runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut presentation,
            ready,
            Vec::new(),
        )
        .unwrap());
    assert_eq!(runtime.admission_work(), candidate_work);
    let live_names = world_names(&presentation);
    let live_after_commit = runtime.admission_work();

    let dropped_catalog = presentation
        .stage_catalog(
            [(
                "synthetic-a".into(),
                Asset::from_bundle(&bytes_a, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let dropped_work = library_a.take_admission_work();
    let dropped = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            dropped_catalog,
            &library_a,
            dropped_work,
        )
        .unwrap();
    drop(dropped);
    assert_eq!(runtime.admission_work(), live_after_commit);
    assert_eq!(world_names(&presentation), live_names);

    let stale_catalog = presentation
        .stage_catalog(
            [(
                "synthetic-a".into(),
                Asset::from_bundle(&bytes_a, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let stale_work = library_a.take_admission_work();
    let stale = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            stale_catalog,
            &library_a,
            stale_work,
        )
        .unwrap();
    let stale = pollster::block_on(stale.prepare()).unwrap();
    let renewed_catalog = presentation
        .stage_catalog(
            [(
                "synthetic-c".into(),
                Asset::from_bundle(&bytes_b, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut library_c = Library::from_bundle("synthetic-c", &bytes_b, &[]).unwrap();
    let library_c_work = library_c.take_admission_work();
    let renewed = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            renewed_catalog,
            &library_c,
            library_c_work,
        )
        .unwrap();
    let renewed = pollster::block_on(renewed.prepare()).unwrap();
    assert!(runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut presentation,
            renewed,
            Vec::new(),
        )
        .unwrap());
    let live_after_renewed = runtime.admission_work();
    let renewed_names = world_names(&presentation);
    assert!(!runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut presentation,
            stale,
            Vec::new(),
        )
        .unwrap());
    assert_eq!(runtime.admission_work(), live_after_renewed);
    assert_eq!(world_names(&presentation), renewed_names);

    let failed_catalog = presentation
        .stage_catalog(
            [(
                "missing".into(),
                Asset::from_bundle(&bytes_a, None).unwrap(),
            )]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    assert!(runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            failed_catalog,
            &Library::default(),
            AdmissionWork::default(),
        )
        .is_err());
    assert_eq!(runtime.admission_work(), live_after_renewed);
    assert_eq!(world_names(&presentation), renewed_names);
}
