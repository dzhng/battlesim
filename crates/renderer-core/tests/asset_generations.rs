//! Synthetic admission work and generation-swap contracts.
#![allow(clippy::too_many_arguments)]

#[path = "support/readback.rs"]
mod readback;
#[path = "support/mod.rs"]
mod support;

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

fn bundle_bytes(kind: &str) -> Vec<u8> {
    bundle_bytes_with_state(kind, "default", 1)
}

fn bundle_bytes_with_state(kind: &str, state: &str, tier_count: usize) -> Vec<u8> {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: Vec<u8>, length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let positions: Vec<u8> = [0.0f32, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.5, 1.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    let mesh = json!({
        "positions": array("f32", positions, 9),
        "normals": array("i16", vec![0; 24], 12),
        "uvs": array("f32", vec![0; 24], 6),
        "colors": array("u8", vec![255; 12], 12),
        "indices": array("u16", vec![0, 0, 1, 0, 2, 0], 3),
        "draws": [{"material": 0, "first": 0, "count": 3}],
    });
    let tiers = vec![mesh; tier_count];
    let content = json!({"kind": kind, "materials": [{
        "name": "synthetic", "base_color": [1,1,1,1], "metallic": 0,
        "roughness": 1, "tint": 0, "coverage": {"kind":"opaque"}
    }], "states": [{"name":state,"bounds":{"min":[0,0,0],"max":[1,1,1]}, "tiers":tiers}]});
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

fn static_presentation() -> Presentation {
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
        tint: [1., 1., 1.],
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
    let mesh = scenery_mesh();
    p.place_scenery(&[mesh.as_slice()]).unwrap();
    p
}

fn scenery_mesh() -> Vec<f32> {
    let mut mesh = Vec::new();
    for z in [0.0, 1.0] {
        for side in 0..6 {
            let angle = side as f64 / 6.0 * std::f64::consts::TAU;
            mesh.extend(
                [
                    0.5 * libm::cos(angle),
                    0.5 * libm::sin(angle),
                    z,
                    0.0,
                    0.0,
                    1.0,
                    0.2,
                    0.1,
                    0.05,
                    0.0,
                ]
                .map(|v| v as f32),
            );
        }
    }
    mesh.extend([2.0, 0.0, 1.2, 0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 1.0]);
    mesh
}

fn changed_scenery_bundle() -> Vec<u8> {
    let mut bytes = bundle_bytes_with_state("static", "summer", 4);
    let header = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    bytes[16 + header..20 + header].copy_from_slice(&1.0f32.to_le_bytes());
    bytes
}

fn render_pixels(
    gpu: &support::Gpu,
    runtime: &mut Runtime,
    presentation: &mut Presentation,
    camera: &presentation::view::Camera,
) -> Vec<[u8; 4]> {
    presentation
        .frame(0.0, camera, &Ui::default(), runtime.static_generation())
        .unwrap();
    runtime
        .prepare(&gpu.device, &gpu.queue, presentation)
        .unwrap();
    let view = presentation.last_frame().view;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    runtime.encode(
        &gpu.device,
        &gpu.queue,
        &view,
        &mut encoder,
        PlayerGeometry::default(),
    );
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("statics-carry-pixels"),
        size: u64::from(runtime.targets.size[0] * runtime.targets.size[1] * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        runtime.targets.display_texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(runtime.targets.size[0] * 4),
                rows_per_image: None,
            },
        },
        runtime.targets.display_texture.size(),
    );
    readback::pixels(&gpu.device, &gpu.queue, encoder, &buffer)
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

#[test]
fn first_admission_reports_the_synthetic_fixture_work() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let library = Library::from_bundle("synthetic", &bundle_bytes("static"), &[]).unwrap();
    let mut work = library.admission_work();
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
    let bytes = bundle_bytes_with_state("static", "summer", 4);
    let asset = Asset::from_bundle(&bytes, None).unwrap();
    let mut presentation = static_presentation();
    let geometry = Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[]));
    let library = Library::from_bundle("synthetic-scenery", &bytes, &[]).unwrap();
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config,
        library.clone().admission_work(),
        RuntimeWorld {
            presentation: &mut presentation,
            assets: &library,
            geometry: geometry.clone(),
            effect_sheets: effects(),
        },
    )
    .unwrap();
    pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let camera = presentation::view::Camera {
        eye: [32.0, -48.0, 32.0],
        target: [32.0, 32.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [64, 64],
    };
    let before = render_pixels(&gpu, &mut runtime, &mut presentation, &camera);
    let candidate = presentation
        .stage_catalog(
            [("new-unit".into(), asset)].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut next_library = library.clone();
    next_library
        .insert_bundle("synthetic-scenery", &bytes, &[])
        .unwrap();
    next_library
        .insert_bundle("new-unit", &bundle_bytes("static"), &[])
        .unwrap();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &next_library,
            next_library.admission_work(),
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
            Vec::new(),
        )
        .unwrap());
    let after = render_pixels(&gpu, &mut runtime, &mut presentation, &camera);
    assert_eq!(after, before, "carried statics changed the committed frame");
}

#[test]
fn changed_scenery_content_places_and_uploads_again() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes = bundle_bytes_with_state("static", "summer", 4);
    let changed = changed_scenery_bundle();
    let asset = Asset::from_bundle(&bundle_bytes("static"), None).unwrap();
    let scenery_asset = Asset::from_bundle(&changed, None).unwrap();
    let mut presentation = static_presentation();
    let geometry = Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[]));
    let library = Library::from_bundle("synthetic-scenery", &bytes, &[]).unwrap();
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config,
        library.clone().admission_work(),
        RuntimeWorld {
            presentation: &mut presentation,
            assets: &library,
            geometry: geometry.clone(),
            effect_sheets: effects(),
        },
    )
    .unwrap();
    let candidate = presentation
        .stage_catalog(
            [
                ("synthetic-scenery".into(), scenery_asset),
                ("new-unit".into(), asset),
            ]
            .into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut next_library = library.clone();
    next_library
        .insert_bundle("synthetic-scenery", &changed, &[])
        .unwrap();
    next_library
        .insert_bundle("new-unit", &bundle_bytes("static"), &[])
        .unwrap();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &next_library,
            next_library.admission_work(),
        )
        .unwrap();
    let ready = pollster::block_on(pending.prepare()).unwrap();
    assert_eq!(ready.work().scenery_placements, 1);
    assert!(ready.work().statics_bytes_uploaded > 0);
}

#[test]
fn real_generation_swap_keeps_live_work_for_dropped_stale_and_failed_candidates() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes_a = bundle_bytes("static");
    let bytes_b = bundle_bytes("static");
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
    let library_a = Library::from_bundle("synthetic-a", &bytes_a, &[]).unwrap();
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        runtime_config(),
        library_a.clone().admission_work(),
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
    let library_b = Library::from_bundle("synthetic-b", &bytes_b, &[]).unwrap();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            candidate,
            &library_b,
            library_b.admission_work(),
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
    let dropped = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            dropped_catalog,
            &library_a,
            library_a.admission_work(),
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
    let stale = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            stale_catalog,
            &library_a,
            library_a.admission_work(),
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
    let library_c = Library::from_bundle("synthetic-c", &bytes_b, &[]).unwrap();
    let renewed = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            renewed_catalog,
            &library_c,
            library_c.admission_work(),
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
