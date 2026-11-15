//! Synthetic admission work and generation-swap contracts.
#![allow(clippy::too_many_arguments)]

#[path = "support/mod.rs"]
mod support;

use contract::identity::ContentId;
use presentation::presentation::{lab::Asset, Presentation};
use presentation::records::{Appearance, Joint, NONE};
use renderer_core::bundle::{Library, SourceDraft};
use renderer_core::effects::EffectSheets;
use renderer_core::models::Models;
use renderer_core::models::VERTEX_BYTES;
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

fn bundle_bytes(kind: &str, max_z: u8) -> Vec<u8> {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: Vec<u8>, length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let mesh = json!({
        "positions": array("f32", vec![0; 36], 9),
        "normals": array("i16", vec![0; 24], 12),
        "uvs": array("f32", vec![0; 24], 6),
        "colors": array("u8", vec![255; 12], 12),
        "indices": array("u16", vec![0, 0, 1, 0, 2, 0], 3),
        "draws": [{"material": 0, "first": 0, "count": 3}],
    });
    let content = json!({"kind": kind, "materials": [{
        "name": "synthetic", "base_color": [1,1,1,1], "metallic": 0,
        "roughness": 1, "tint": 0, "coverage": {"kind":"opaque"}
    }], "states": [{"name":"default", "bounds":{"min":[0,0,0],"max":[1,1,max_z]}, "tiers":[mesh]}]});
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

#[test]
fn same_name_with_new_content_replaces_the_admitted_art() {
    let bytes_a = bundle_bytes("static", 1);
    let bytes_b = bundle_bytes("static", 2);

    let mut presentation = Presentation::new(None, 30.).unwrap();
    presentation.install_lab_assets(
        [(
            "synthetic".into(),
            (
                content_id(&bytes_a),
                Asset::from_bundle(&bytes_a, None).unwrap(),
            ),
        )]
        .into(),
        detail(),
    );
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
    assert_eq!(
        library.missing_content(&presentation, &[content_id(&bytes_a)]),
        vec![content_id(&bytes_a)]
    );
    assert_eq!(Arc::strong_count(&old), 1);
    let new_top = match &library.bundle("synthetic").unwrap().shape {
        renderer_core::bundle::Shape::Static(states) => states[0].top,
        _ => panic!("synthetic bundle is static"),
    };
    assert_ne!(old_top, new_top);
}

#[test]
fn identical_content_under_a_second_name_decodes_once() {
    let bytes = bundle_bytes("static", 1);
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
    let replacement = bundle_bytes("static", 2);
    library
        .insert_bundle("synthetic-a", content_id(&replacement), &replacement, &[])
        .unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 1);
    assert!(Arc::ptr_eq(&old, &library.bundle("synthetic-b").unwrap()));
}

#[test]
fn missing_content_is_exact_and_held_sources_decode_nothing() {
    let held_bytes = bundle_bytes("static", 1);
    let missing_x = bundle_bytes("static", 2);
    let missing_y = bundle_bytes("static", 3);
    let held = content_id(&held_bytes);
    let x = content_id(&missing_x);
    let y = content_id(&missing_y);
    let mut library = Library::default();
    let mut presentation = Presentation::new(None, 30.).unwrap();
    presentation.install_lab_assets(
        [(
            "held".into(),
            (held.clone(), Asset::from_bundle(&held_bytes, None).unwrap()),
        )]
        .into(),
        detail(),
    );
    library
        .insert_bundle("held", held.clone(), &held_bytes, &[])
        .unwrap();
    assert_eq!(
        library.missing_content(&presentation, &[held.clone(), x.clone(), y.clone()]),
        vec![x.clone(), y.clone()]
    );
    assert_eq!(
        library.missing_content(&presentation, &[y.clone(), held.clone(), x.clone()]),
        vec![y.clone(), x.clone()]
    );
    library.insert_content(x.clone(), &missing_x, &[]).unwrap();
    library.insert_content(y.clone(), &missing_y, &[]).unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 3);
    library.insert_content(held, &held_bytes, &[]).unwrap();
    assert_eq!(library.take_admission_work().bundles_decoded, 0);

    let mut undeclared = library.empty_generation();
    undeclared
        .insert_content(y.clone(), &missing_y, &[])
        .unwrap();
    library.replace_with(undeclared);
    library.retain(|name| name == "held");
    assert!(
        !library.has_content(&y),
        "undeclared draft bytes cannot revive art"
    );
}

#[test]
fn held_requires_both_decoded_content_and_retained_facts() {
    let bytes = bundle_bytes("static", 7);
    let id = content_id(&bytes);
    let mut library = Library::default();
    let mut presentation = Presentation::new(None, 30.).unwrap();
    library
        .insert_bundle("first", id.clone(), &bytes, &[])
        .unwrap();
    assert_eq!(
        library.missing_content(&presentation, std::slice::from_ref(&id)),
        vec![id.clone()]
    );
    let mut draft = SourceDraft::new(&library, &presentation);
    draft.declare("first".into(), id.clone());
    draft.add_bundle(id.clone(), &bytes, &[], None).unwrap();
    let (library, catalog, work) = draft
        .stage(&library, &presentation, detail(), Default::default())
        .unwrap();
    assert_eq!(
        work.bundles_decoded, 0,
        "only missing facts are read for decoded content"
    );
    assert!(presentation.commit_catalog(catalog, Vec::new()).unwrap());
    assert!(library.missing_content(&presentation, &[id]).is_empty());
}

#[test]
fn first_admission_reports_the_synthetic_fixture_work() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let mut library = Library::from_bundle("synthetic", &bundle_bytes("static", 1), &[]).unwrap();
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

fn set_synthetic_ground(presentation: &mut Presentation) {
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
}

#[test]
fn real_generation_swap_keeps_live_work_for_dropped_stale_and_failed_candidates() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes_a = bundle_bytes("static", 1);
    let bytes_b = bundle_bytes("static", 1);
    let asset_a = Asset::from_bundle(&bytes_a, None).unwrap();
    let asset_b = Asset::from_bundle(&bytes_b, None).unwrap();
    let mut presentation = Presentation::new(None, 30.).unwrap();
    presentation.install_lab_assets(
        BTreeMap::from([("synthetic-a".into(), (content_id(&bytes_a), asset_a))]),
        detail(),
    );
    presentation
        .set_lab_models(
            serde_json::from_value(json!([{
                "appearance":"synthetic-a", "x":0, "y":0, "z":0, "yaw":0,
                "tier":0, "pose":{"kind":"static", "state":"default"}
            }]))
            .unwrap(),
        )
        .unwrap();
    set_synthetic_ground(&mut presentation);
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
            [("synthetic-b".into(), content_id(&bytes_b))].into(),
            [(content_id(&bytes_b), asset_b)].into(),
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
            [("synthetic-a".into(), content_id(&bytes_a))].into(),
            [(
                content_id(&bytes_a),
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
            [("synthetic-a".into(), content_id(&bytes_a))].into(),
            [(
                content_id(&bytes_a),
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
            [("synthetic-c".into(), content_id(&bytes_b))].into(),
            [(
                content_id(&bytes_b),
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
            [("missing".into(), content_id(&bytes_a))].into(),
            [(
                content_id(&bytes_a),
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

#[test]
fn first_scene_sources_admit_delta_and_retired_drafts_leave_live_art_intact() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let fixtures = [
        ("x", bundle_bytes("static", 2)),
        ("y", bundle_bytes("static", 3)),
    ];
    let fresh = bundle_bytes("static", 4);
    let stray = bundle_bytes("static", 5);
    let mut p = Presentation::new(None, 30.).unwrap();
    set_synthetic_ground(&mut p);
    // The browser's initial order: presentation installs all named facts, then
    // load_bundle fills the decoded library, then Runtime admits that pair.
    p.install_lab_assets(
        fixtures
            .iter()
            .map(|(name, bytes)| {
                (
                    (*name).into(),
                    (content_id(bytes), Asset::from_bundle(bytes, None).unwrap()),
                )
            })
            .collect(),
        detail(),
    );
    let mut library = Library::default();
    for (name, bytes) in &fixtures {
        library
            .insert_bundle(name, content_id(bytes), bytes, &[])
            .unwrap();
    }
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        runtime_config(),
        library.take_admission_work(),
        RuntimeWorld {
            presentation: &mut p,
            assets: &library,
            geometry: Arc::new(WorldGeometry::from_triangles(&[], &[], &[], 64., None, &[])),
            effect_sheets: effects(),
        },
    )
    .unwrap();
    pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let mut draft = SourceDraft::new(&library, &p);
    for (name, bytes) in &fixtures {
        draft.declare((*name).into(), content_id(bytes));
    }
    draft.declare("a".into(), content_id(&fresh));
    draft.declare("alias-a".into(), content_id(&fresh));
    let wanted = fixtures
        .iter()
        .map(|(_, bytes)| content_id(bytes))
        .chain([content_id(&fresh)])
        .collect::<Vec<_>>();
    let missing = library.missing_content(&p, &wanted);
    assert_eq!(missing, vec![content_id(&fresh)]);
    draft
        .add_bundle(content_id(&fresh), &fresh, &[], None)
        .unwrap();
    // The old page may still supply bytes for a hit; neither decoder runs.
    draft
        .add_bundle(content_id(&fixtures[0].1), &fixtures[0].1, &[], None)
        .unwrap();
    let (grown_library, catalog, work) = draft
        .stage(&library, &p, detail(), Default::default())
        .unwrap();
    assert_eq!(work.bundles_decoded as usize, missing.len());
    let ready = pollster::block_on(
        runtime
            .stage_assets(&gpu.device, &gpu.queue, &p, catalog, &grown_library, work)
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert_eq!(ready.work().bundles_decoded as usize, missing.len());
    let models = |names: &[&str]| {
        names.iter().enumerate().map(|(i, name)|
        serde_json::from_value(json!({"appearance":name,"x":i,"y":0,"z":0,"yaw":0,"tier":0,"pose":{"kind":"static","state":"default"}})).unwrap()
    ).collect()
    };
    assert!(runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut p,
            ready,
            models(&["a", "alias-a", "x", "y"])
        )
        .unwrap());
    library = grown_library;
    assert!(library.missing_content(&p, &wanted).is_empty());
    assert!(Arc::ptr_eq(
        &library.bundle("a").unwrap(),
        &library.bundle("alias-a").unwrap()
    ));
    let camera = presentation::view::Camera {
        eye: [0., -20., 15.],
        target: [0.; 3],
        up: [0., 0., 1.],
        fov_y: 1.,
        near: 0.1,
        size: [64, 64],
    };
    assert_eq!(
        p.frame(0., &camera, &Default::default(), 0)
            .unwrap()
            .instances
            .len(),
        4
    );

    let mut late = SourceDraft::new(&library, &p);
    late.declare("x".into(), content_id(&fixtures[0].1));
    let mut replacement = SourceDraft::new(&library, &p);
    replacement.declare("x".into(), content_id(&fresh));
    replacement
        .add_bundle(content_id(&stray), &stray, &[], None)
        .unwrap();
    let (replaced_library, catalog, work) = replacement
        .stage(&library, &p, detail(), Default::default())
        .unwrap();
    let ready = pollster::block_on(
        runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &p,
                catalog,
                &replaced_library,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert!(runtime
        .commit_assets(&gpu.device, &gpu.queue, &mut p, ready, models(&["x"]))
        .unwrap());
    library = replaced_library;
    assert!(
        !library.has_content(&content_id(&stray)),
        "undeclared bytes must not survive commit"
    );
    assert!(!p.retains_catalog_facts(&content_id(&stray)));
    for (_, bytes) in &fixtures {
        assert!(
            !library.has_content(&content_id(bytes)),
            "unrouted old content is retired"
        );
        assert!(!p.retains_catalog_facts(&content_id(bytes)));
    }
    let world = p.world().clone();
    let work = runtime.admission_work();
    let before = p
        .frame(0., &camera, &Default::default(), 0)
        .unwrap()
        .instances
        .clone();
    let error = late
        .stage(&library, &p, detail(), Default::default())
        .err()
        .expect("retired held content must refuse preparation");
    assert!(
        error.contains("source content retired or not supplied"),
        "{error}"
    );
    assert_eq!(p.world(), &world);
    assert_eq!(runtime.admission_work(), work);
    assert_eq!(
        p.frame(0., &camera, &Default::default(), 0)
            .unwrap()
            .instances,
        before
    );
}
