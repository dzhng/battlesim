//! Synthetic admission work and generation-swap contracts.
#![allow(clippy::too_many_arguments)]

#[path = "support/mod.rs"]
mod support;

use contract::identity::ContentId;
use presentation::presentation::{lab::Asset, Presentation};
use presentation::records::{Appearance, Joint, NONE};
use renderer_core::bundle::Library;
use renderer_core::effects::EffectSheets;
use renderer_core::models::VERTEX_BYTES;
use renderer_core::models::{ArtStore, Models};
use renderer_core::runtime::WorldGeometry;
use renderer_core::runtime::{AdmissionWork, Runtime, RuntimeConfig, RuntimeWorld};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[path = "support/asset_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn geometry_growth_packs_only_the_new_content() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes = [
        bundle_bytes("static", 1, "default", 1),
        bundle_bytes("static", 2, "default", 1),
        bundle_bytes("static", 3, "default", 1),
    ];
    let xy: Art<'_> = &[("x", &bytes[0]), ("y", &bytes[1])];
    let axy: Art<'_> = &[("a", &bytes[2]), ("x", &bytes[0]), ("y", &bytes[1])];
    let mut stage = Stage::new(&gpu, xy, runtime_config(), None);
    let ready = stage.stage(&gpu, axy);
    let bundle = renderer_core::bundle::decode("a", &bytes[2], &[]).unwrap();
    let renderer_core::bundle::Shape::Static(states) = &bundle.shape else {
        panic!("static fixture");
    };
    let vertex_bytes = states[0]
        .tiers
        .iter()
        .map(|m| m.vertices() * VERTEX_BYTES)
        .sum::<usize>();
    let index_bytes = states[0]
        .tiers
        .iter()
        .map(|m| m.indices.len() * size_of::<u32>())
        .sum::<usize>();
    // All three contents' material rows are uploaded again. Geometry uploads
    // contain only a's vertex and u32 index buffers; row/instance buffers are
    // prepared later, and do not enter this admission count.
    let material_bytes = axy.len() * bundle.materials.len() * 5 * 4 * size_of::<f32>();
    assert_eq!(ready.work().vertex_bytes_packed, vertex_bytes as u64);
    assert_eq!(
        ready.work().buffer_bytes_uploaded,
        (vertex_bytes + index_bytes + material_bytes) as u64
    );
    assert_eq!(ready.work().rebuild_reason, None);
    stage.commit(&gpu, axy, ready);
    let pixels = stage.pixels(&gpu);
    let mut clean = Stage::new(&gpu, axy, runtime_config(), None);
    assert_eq!(pixels, clean.pixels(&gpu));
    let mut no_models = runtime_config().controls;
    no_models.models = false;
    no_models.surfaces = [false; 4];
    clean
        .runtime
        .set_controls(&gpu.queue, &clean.presentation, no_models)
        .unwrap();
    assert_ne!(
        pixels,
        clean.pixels(&gpu),
        "the comparison must contain visible art"
    );
}

#[test]
fn material_slots_keep_reservations_when_arrival_order_changes_and_refuse_overflow() {
    use renderer_core::models::MaterialSlots;
    let bytes = [
        bundle_bytes("static", 1, "default", 1),
        bundle_bytes("static", 2, "default", 1),
    ];
    let ids = bytes.each_ref().map(|b| content_id(b));
    let counts = [2, 3];
    let mut slots = MaterialSlots::default();
    let assigned = [
        slots.reserve(ids[0].clone(), counts[0]).unwrap(),
        slots.reserve(ids[1].clone(), counts[1]).unwrap(),
    ];
    for i in [1, 0] {
        assert_eq!(
            slots.reserve(ids[i].clone(), counts[i]).unwrap(),
            assigned[i]
        );
    }
    assert_eq!(assigned, [0, counts[0]]);
    let mut exhausted = MaterialSlots::default();
    exhausted
        .reserve(ids[0].clone(), u16::MAX as usize + 1)
        .unwrap();
    assert!(exhausted
        .reserve(ids[1].clone(), counts[1])
        .unwrap_err()
        .contains("slots exhausted"));
    assert!(exhausted.base(&ids[1]).is_none());
}

#[test]
fn changed_corpse_pose_does_not_reuse_geometry() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let bytes = bundle_bytes("skinned", 1, "default", 1);
    let library = Library::from_bundle("x", &bytes, &[]).unwrap();
    let rows = [Appearance {
        name_len: 1,
        lies: 1,
        ..Default::default()
    }];
    let store = ArtStore::shared();
    let mut held = Vec::new();
    let mut counts = Vec::new();
    for offset in [0., 4., 4.] {
        let mut work = AdmissionWork::default();
        let pose = Joint {
            x: [1., 0., 0., offset],
            ..identity()
        };
        held.push(
            Models::from_world(
                &gpu.device,
                &gpu.queue,
                &pipelines,
                &library,
                &rows,
                "x",
                |_| Some(vec![pose]),
                |_, _| None,
                store.clone(),
                &mut work,
            )
            .unwrap(),
        );
        counts.push(work.vertex_bytes_packed);
    }
    assert!(counts[0] > 0);
    assert_eq!(
        counts[1], counts[0],
        "same row and content with a changed baked palette is a miss"
    );
    assert_eq!(counts[2], 0, "the same baked palette is reused");
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
        ArtStore::shared(),
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
            card_read_waits: 0,
            statics_bytes_uploaded: 0,
            scenery_placements: 0,
            rebuild_reason: None,
        }
    );
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
    ground(&mut presentation, None);
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

#[test]
fn dropped_pending_slots_force_fresh_admission_without_changing_pixels() {
    use renderer_core::runtime::RebuildReason;
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let x = bundle_bytes("static", 1, "default", 1);
    let y = bundle_bytes("static", 2, "default", 1);
    let a = bundle_bytes("static", 3, "default", 1);
    let xy: Art<'_> = &[("x", &x), ("y", &y)];
    let mut config = runtime_config();
    config.art_store.dead_material_slots = 0;
    let mut stage = Stage::new(&gpu, xy, config, None);
    let before = stage.pixels(&gpu);
    let dropped = stage.stage(&gpu, &[("a", &a), ("x", &x), ("y", &y)]);
    drop(dropped);
    assert_eq!(
        stage.pixels(&gpu),
        before,
        "dropped work cannot change the live frame"
    );
    let ready = stage.stage(&gpu, xy);
    assert_eq!(
        ready.work().rebuild_reason,
        Some(RebuildReason::DeadMaterialSlots)
    );
    let packed = stage.runtime.admission_work().vertex_bytes_packed;
    assert_eq!(ready.work().vertex_bytes_packed, packed);
    stage.commit(&gpu, xy, ready);
    assert_eq!(stage.pixels(&gpu), before);
    let mut clean = Stage::new(&gpu, xy, runtime_config(), None);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
}

#[test]
fn replacement_content_draws_new_geometry_and_releases_retired_buffers() {
    use renderer_core::memory::ResourceMemory;
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let old = bundle_bytes("static", 1, "default", 1);
    let new = bundle_bytes("static", 3, "default", 1);
    let art: Art<'_> = &[("x", &old)];
    let mut stage = Stage::new(&gpu, art, runtime_config(), None);
    let before = stage.pixels(&gpu);
    let ready = stage.stage(&gpu, &[("x", &new)]);
    assert_eq!(
        ready.work().vertex_bytes_packed,
        stage.runtime.admission_work().vertex_bytes_packed
    );
    assert_eq!(
        stage.pixels(&gpu),
        before,
        "preparation leaves the live model drawable"
    );
    stage.commit(&gpu, &[("x", &new)], ready);
    let after = stage.pixels(&gpu);
    assert_ne!(after, before);
    let mut clean = Stage::new(&gpu, &[("x", &new)], runtime_config(), None);
    assert_eq!(after, clean.pixels(&gpu));

    // Observe the store through the production memory visitor. Keeping the
    // store after all model owners are dropped must keep no GPU buffers.
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let store = ArtStore::shared();
    let mut library = Library::from_bundle("x", &old, &[]).unwrap();
    let rows = [Appearance {
        name_len: 1,
        lies: NONE,
        ..Default::default()
    }];
    let old_models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        &rows,
        "x",
        |_| None,
        |_, _| None,
        store.clone(),
        &mut AdmissionWork::default(),
    )
    .unwrap();
    let memory = || {
        let mut memory = ResourceMemory::default();
        store.lock().unwrap().record_memory(&mut memory);
        memory.counts().buffer_bytes
    };
    let old_bytes = memory();
    assert!(old_bytes > 0);
    library
        .insert_bundle("x", content_id(&new), &new, &[])
        .unwrap();
    let new_models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        &rows,
        "x",
        |_| None,
        |_, _| None,
        store.clone(),
        &mut AdmissionWork::default(),
    )
    .unwrap();
    assert_eq!(memory(), old_bytes * 2);
    drop(old_models);
    assert_eq!(
        memory(),
        old_bytes,
        "retired geometry has no store-owned strong reference"
    );
    drop(new_models);
    assert_eq!(memory(), 0);
}

#[test]
fn slot_exhaustion_reclaims_dead_ranges_and_refuses_an_oversized_catalog() {
    use renderer_core::runtime::RebuildReason;
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let x = bundle_bytes("static", 1, "default", 1);
    let y = bundle_bytes("static", 2, "default", 1);
    let capacity = u16::MAX as usize + 1;
    let full = material_bundle(capacity);
    let mut config = runtime_config();
    config.art_store.dead_material_slots = capacity;
    let mut stage = Stage::new(&gpu, &[("old", &full)], config, None);
    let before = stage.pixels(&gpu);
    let art: Art<'_> = &[("x", &x), ("y", &y)];
    let ready = stage.stage(&gpu, art);
    assert_eq!(
        ready.work().rebuild_reason,
        Some(RebuildReason::MaterialSlotsExhausted)
    );
    assert_eq!(stage.pixels(&gpu), before);
    stage.commit(&gpu, art, ready);
    let mut clean = Stage::new(&gpu, art, runtime_config(), None);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));

    let too_many = material_bundle(capacity + 1);
    let library = Library::from_bundle("oversized", &too_many, &[]).unwrap();
    let draft = stage
        .presentation
        .stage_catalog(
            catalog(&[("oversized", &too_many)]),
            detail(),
            Default::default(),
        )
        .unwrap();
    let live = stage.pixels(&gpu);
    let failed = stage.runtime.stage_assets(
        &gpu.device,
        &gpu.queue,
        &stage.presentation,
        draft,
        &library,
        AdmissionWork::default(),
    );
    let Err(error) = failed else {
        panic!("an oversized active catalog must be refused");
    };
    assert!(error.contains("material slots exhausted"), "{error}");
    assert_eq!(stage.pixels(&gpu), live);

    let bundle = library.bundle("oversized").unwrap();
    let renderer_core::bundle::Shape::Static(states) = &bundle.shape else {
        panic!("static fixture");
    };
    let mesh = &states[0].tiers[0];
    assert!(
        renderer_core::models::pack_vertices(mesh, capacity, Some(0), 0).is_err(),
        "packing must never truncate a material address"
    );
    assert!(renderer_core::models::pack_vertices(mesh, capacity - 1, Some(0), 0).is_ok());
}

#[test]
fn stale_and_dropped_geometry_preparations_leave_live_pixels_unchanged() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let x = bundle_bytes("static", 1, "default", 1);
    let y = bundle_bytes("static", 2, "default", 1);
    let z = bundle_bytes("static", 3, "default", 1);
    let mut stage = Stage::new(&gpu, &[("x", &x)], runtime_config(), None);
    let initial = stage.pixels(&gpu);
    let dropped = stage.stage(&gpu, &[("y", &y), ("x", &x)]);
    assert_eq!(stage.pixels(&gpu), initial);
    drop(dropped);
    assert_eq!(stage.pixels(&gpu), initial);
    let stale = stage.stage(&gpu, &[("y", &y), ("x", &x)]);
    let art: Art<'_> = &[("z", &z), ("x", &x)];
    let ready = stage.stage(&gpu, art);
    stage.commit(&gpu, art, ready);
    let committed = stage.pixels(&gpu);
    assert!(!stage
        .runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut stage.presentation,
            stale,
            models(&[("y", &y), ("x", &x)])
        )
        .unwrap());
    assert_eq!(stage.pixels(&gpu), committed);
}

#[test]
fn card_bake_reports_one_read_wait_per_memory_batch() {
    let gpu = support::gpu().expect("a GPU is required for card bake tests");
    let card_bytes = renderer_core::impostor::Spec::runtime()
        .bake_memory_bytes()
        .unwrap();
    for bound in [card_bytes * 8, card_bytes * 2 + card_bytes / 2] {
        let mut config = runtime_config();
        config.art_store.card_bake_memory_bytes = bound;
        let Stage {
            mut presentation,
            mut runtime,
        } = Stage::new(&gpu, &[], config, None);
        assert_eq!(
            runtime.admission_work().card_read_waits,
            0,
            "no cards means no read wait"
        );
        let (mut library, assets, models) = card_catalog();
        let catalog = presentation
            .stage_catalog(assets, detail(), Default::default())
            .unwrap();
        let expected_cards = catalog
            .view(&presentation)
            .unwrap()
            .world()
            .appearances
            .len();
        assert!(expected_cards > 2, "fixture must split at the narrow bound");
        let work = library.take_admission_work();
        let pending = runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &presentation,
                catalog,
                &library,
                work,
            )
            .unwrap();
        let ready = pollster::block_on(pending.prepare()).unwrap();
        let work = ready.work().clone();
        assert_eq!(work.cards_baked as usize, expected_cards);
        assert_eq!(
            work.card_read_waits as usize,
            expected_cards.div_ceil((bound / card_bytes) as usize)
        );
        assert!(runtime
            .commit_assets(&gpu.device, &gpu.queue, &mut presentation, ready, models)
            .unwrap());
        assert_eq!(
            runtime.admission_work(),
            work,
            "the committed report carries the bake waits"
        );
    }
}

#[test]
fn cards_baked_in_one_batch_match_each_card_baked_alone() {
    let gpu = support::gpu().expect("a GPU is required for card bake tests");
    let spec = renderer_core::impostor::Spec::runtime();
    let mut config = runtime_config();
    config.art_store.card_bake_memory_bytes = spec.bake_memory_bytes().unwrap() * 8;
    let Stage {
        mut presentation,
        mut runtime,
    } = Stage::new(&gpu, &[], config, None);
    assert_eq!(
        runtime.admission_work().card_read_waits,
        0,
        "no cards means no read wait"
    );
    let (mut library, assets, models) = card_catalog();
    let catalog = presentation
        .stage_catalog(assets, detail(), Default::default())
        .unwrap();
    let rows: Vec<_> = (0..catalog
        .view(&presentation)
        .unwrap()
        .world()
        .appearances
        .len() as u32)
        .collect();
    let work = library.take_admission_work();
    let ready = pollster::block_on(
        runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &presentation,
                catalog,
                &library,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert!(runtime
        .commit_assets(&gpu.device, &gpu.queue, &mut presentation, ready, models)
        .unwrap());
    let batched = pollster::block_on(
        runtime
            .models
            .impostor_request(
                &gpu.device,
                &gpu.queue,
                &runtime.pipelines,
                &rows,
                &spec,
                &mut AdmissionWork::default(),
            )
            .unwrap()
            .read(),
    )
    .unwrap();
    assert_eq!(batched.len(), rows.len());
    for (&row, batch) in rows.iter().zip(&batched) {
        let alone = pollster::block_on(
            runtime
                .models
                .impostor_request(
                    &gpu.device,
                    &gpu.queue,
                    &runtime.pipelines,
                    &[row],
                    &spec,
                    &mut AdmissionWork::default(),
                )
                .unwrap()
                .read(),
        )
        .unwrap()
        .remove(0);
        assert!(
            batch
                .albedo
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] > 0),
            "the fixture must draw geometry"
        );
        assert_eq!(batch.hash, alone.hash);
        assert_eq!(batch.albedo, alone.albedo);
        assert_eq!(batch.normal, alone.normal);
        assert_eq!(batch.mask, alone.mask);
    }
    assert!(
        batched
            .iter()
            .any(|atlas| atlas.albedo != batched[0].albedo),
        "the fixture must draw distinct authored materials"
    );
    assert!(
        batched
            .iter()
            .any(|atlas| atlas.center != batched[0].center),
        "the fixture must distinguish its authored poses"
    );
    assert_ne!(
        batched.first().unwrap().hash,
        batched.last().unwrap().hash,
        "the fixture must distinguish its poses/materials"
    );
}

#[test]
fn card_bake_uploads_the_authored_materials_once_per_stage() {
    let gpu = support::gpu().expect("a GPU is required for card bake tests");
    let spec = renderer_core::impostor::Spec::runtime();
    let mut config = runtime_config();
    config.art_store.card_bake_memory_bytes =
        spec.bake_memory_bytes().unwrap() * 2 + spec.bake_memory_bytes().unwrap() / 2;
    let Stage {
        presentation,
        runtime,
    } = Stage::new(&gpu, &[], config, None);
    assert_eq!(
        runtime.admission_work().card_read_waits,
        0,
        "no cards means no read wait"
    );
    let (mut library, assets, _) = card_catalog();
    let catalog = presentation
        .stage_catalog(assets, detail(), Default::default())
        .unwrap();
    let material_bytes = ["cloth-a", "cloth-b"]
        .into_iter()
        .map(|name| library.bundle(name).unwrap().materials.len())
        .sum::<usize>()
        * 5
        * 4
        * std::mem::size_of::<f32>();
    let work = library.take_admission_work();
    let pending = runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &presentation,
            catalog,
            &library,
            work,
        )
        .unwrap();
    let staged_bytes = pending.work().buffer_bytes_uploaded;
    let ready = pollster::block_on(pending.prepare()).unwrap();
    assert!(
        ready.work().cards_baked > 1,
        "fixture must bake several cards"
    );
    assert_eq!(
        ready.work().buffer_bytes_uploaded - staged_bytes,
        material_bytes as u64,
        "the authored table is uploaded once, independent of the number of cards or batches"
    );
}

#[test]
fn card_batch_split_uses_the_policy_sink_once_per_runtime() {
    static REPORTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    fn report(message: &str) {
        REPORTS.lock().unwrap().push(message.into());
    }
    REPORTS.lock().unwrap().clear();
    let gpu = support::gpu().expect("a GPU is required for card bake tests");
    let card_bytes = renderer_core::impostor::Spec::runtime()
        .bake_memory_bytes()
        .unwrap();
    let bound = card_bytes * 2;
    for runtime_count in 1..=2 {
        let mut config = runtime_config();
        config.art_store.card_bake_memory_bytes = bound;
        config.art_store.report = report;
        let stage = Stage::new(&gpu, &[], config, None);
        for _ in 0..2 {
            let (mut library, assets, _) = card_catalog();
            let catalog = stage
                .presentation
                .stage_catalog(assets, detail(), Default::default())
                .unwrap();
            let cards = catalog
                .view(&stage.presentation)
                .unwrap()
                .world()
                .appearances
                .len();
            let work = library.take_admission_work();
            let pending = stage
                .runtime
                .stage_assets(
                    &gpu.device,
                    &gpu.queue,
                    &stage.presentation,
                    catalog,
                    &library,
                    work,
                )
                .unwrap();
            let ready = pollster::block_on(pending.prepare()).unwrap();
            assert_eq!(
                ready.work().card_read_waits as usize,
                cards.div_ceil((bound / card_bytes) as usize)
            );
            drop(ready);
            let reports = REPORTS.lock().unwrap();
            assert_eq!(
                reports.len(),
                runtime_count,
                "dropped pending bakes share the runtime's split-report lifetime"
            );
            assert!(
                reports
                    .last()
                    .unwrap()
                    .contains(&format!("{bound} byte bound")),
                "the existing sink receives the selected byte bound"
            );
        }
    }
}

#[path = "asset_generations/statics.rs"]
mod statics;
