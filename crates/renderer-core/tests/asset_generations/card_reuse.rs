//! Shared card keys, real GPU pixels and generation lifetimes.
use super::*;

#[test]
fn growth_bakes_only_the_new_contents_card_keys() {
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let spec = renderer_core::impostor::Spec::runtime();
    let bound = spec.bake_memory_bytes().unwrap();
    let mut config = runtime_config();
    config.art_store.card_bake_memory_bytes = bound;
    let mut stage = Stage::new(&gpu, &[], config, None, &[]);
    let xy = [("x", [0.3, 0.6, 0.9]), ("y", [0.8, 0.4, 0.2])];
    let a = ("a", [0.4, 0.8, 0.3]);
    let (mut library, catalog, models) = card_catalog(&stage.presentation, &xy);
    let work = library.take_admission_work();
    let ready = pollster::block_on(
        stage
            .runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &stage.presentation,
                catalog,
                &library,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert!(stage
        .runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut stage.presentation,
            ready,
            models
        )
        .unwrap());
    let (_, alone, _) = card_catalog(&stage.presentation, &[a]);
    let expected_keys = alone
        .view(&stage.presentation)
        .unwrap()
        .world()
        .appearances
        .len();
    let (mut library, catalog, models) = card_catalog(&stage.presentation, &[a, xy[0], xy[1]]);
    let work = library.take_admission_work();
    let ready = pollster::block_on(
        stage
            .runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &stage.presentation,
                catalog,
                &library,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert_eq!(
        ready.work().cards_baked as usize,
        expected_keys,
        "only a's distinct rest and corpse keys need baking"
    );
    assert_eq!(
        ready.work().card_read_waits as usize,
        expected_keys.div_ceil((bound / spec.bake_memory_bytes().unwrap()) as usize)
    );
    assert!(stage
        .runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut stage.presentation,
            ready,
            models
        )
        .unwrap());
}

fn candidate(
    gpu: &support::Gpu,
    stage: &Stage,
    art: &[(&str, [f32; 3])],
) -> (
    renderer_core::runtime::ReadyAssets,
    Vec<presentation::presentation::lab::ModelInput>,
) {
    let (mut library, catalog, mut models) = card_catalog(&stage.presentation, art);
    for (index, model) in models.iter_mut().enumerate() {
        model.tier = Some(presentation::view::IMPOSTOR as u32);
        model.x = 1. + index as f64 * 1.2;
        model.y = 4.;
        model.scale = [3.; 3];
    }
    let work = library.take_admission_work();
    let ready = pollster::block_on(
        stage
            .runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &stage.presentation,
                catalog,
                &library,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    (ready, models)
}

fn commit_cards(gpu: &support::Gpu, stage: &mut Stage, art: &[(&str, [f32; 3])]) -> AdmissionWork {
    let (ready, models) = candidate(gpu, stage, art);
    let work = ready.work().clone();
    assert!(stage
        .runtime
        .commit_assets(
            &gpu.device,
            &gpu.queue,
            &mut stage.presentation,
            ready,
            models,
        )
        .unwrap());
    work
}

#[test]
fn a_frame_drawn_from_reused_cards_equals_a_clean_bakes() {
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let all = [
        ("a", [0.4, 0.8, 0.3]),
        ("x", [0.3, 0.6, 0.9]),
        ("y", [0.8, 0.4, 0.2]),
    ];
    let mut stage = Stage::new(&gpu, &[], runtime_config(), None, &[]);
    commit_cards(&gpu, &mut stage, &all[1..]);
    commit_cards(&gpu, &mut stage, &all);
    let mut clean = Stage::new(&gpu, &[], runtime_config(), None, &[]);
    commit_cards(&gpu, &mut clean, &all);
    let pixels = stage.pixels(&gpu);
    assert_eq!(pixels, clean.pixels(&gpu));
    assert!(
        stage.runtime.frame_stats()["models"]["cards"]
            .as_u64()
            .unwrap()
            > 0
    );
    let mut hidden = runtime_config().controls;
    hidden.models = false;
    hidden.surfaces = [false; 4];
    clean
        .runtime
        .set_controls(&gpu.queue, &clean.presentation, hidden)
        .unwrap();
    assert_ne!(pixels, clean.pixels(&gpu), "the comparison must draw cards");
}

#[test]
fn names_with_identical_content_bake_one_card_per_pose() {
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let art = [("first", [0.3, 0.6, 0.9]), ("second", [0.3, 0.6, 0.9])];
    let mut stage = Stage::new(&gpu, &[], runtime_config(), None, &[]);
    let (_, alone, _) = card_catalog(&stage.presentation, &art[..1]);
    let distinct = alone
        .view(&stage.presentation)
        .unwrap()
        .world()
        .appearances
        .len();
    let work = commit_cards(&gpu, &mut stage, &art);
    assert_eq!(
        work.cards_baked as usize, distinct,
        "the second name bakes nothing; rest and corpse remain different cards"
    );
}

#[test]
fn dropping_pending_card_bakes_preserves_the_live_picture() {
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let all = [
        ("x", [0.3, 0.6, 0.9]),
        ("a", [0.4, 0.8, 0.3]),
        ("b", [0.8, 0.4, 0.2]),
    ];
    let mut config = runtime_config();
    // One arriving content has rest and corpse keys. Both fit this spare pair.
    config.art_store.texture_spare_layers = 2;
    let mut stage = Stage::new(&gpu, &[], config, None, &[]);
    commit_cards(&gpu, &mut stage, &all[..1]);
    let before = stage.pixels(&gpu);
    let (fitting, _) = candidate(&gpu, &stage, &all[..2]);
    assert!(fitting.work().cards_baked > 0);
    assert_eq!(fitting.work().card_bytes_copied, 0);
    drop(fitting);
    assert_eq!(stage.pixels(&gpu), before);
    let (retry, _) = candidate(&gpu, &stage, &all[..2]);
    assert_eq!(
        retry.work().cards_baked,
        0,
        "live holder retains unused immutable layers"
    );
    drop(retry);
    let (growing, _) = candidate(&gpu, &stage, &all);
    assert!(growing.work().cards_baked > 0);
    assert!(growing.work().card_bytes_copied > 0);
    assert_eq!(stage.pixels(&gpu), before);
    drop(growing);
    assert_eq!(stage.pixels(&gpu), before);
    let (retry, _) = candidate(&gpu, &stage, &all);
    assert!(
        retry.work().cards_baked > 0,
        "a dropped last holder releases its new array"
    );
    drop(retry);
    assert_eq!(stage.pixels(&gpu), before);
}

fn admit_card_models(
    gpu: &support::Gpu,
    pipelines: &renderer_core::pipelines::Pipelines,
    art: &[(&str, [f32; 3])],
    store: renderer_core::models::SharedArtStore,
    policy: renderer_core::models::ArtStorePolicy,
) -> (Models, AdmissionWork) {
    let presentation = Presentation::new(None, 30.).unwrap();
    let (mut library, catalog, _) = card_catalog(&presentation, art);
    let mut work = library.take_admission_work();
    let mut models = Models::new(
        &gpu.device,
        &gpu.queue,
        pipelines,
        &library,
        catalog.view(&presentation).unwrap(),
        store,
        &mut work,
    )
    .unwrap();
    pollster::block_on(models.bake_cards(
        &gpu.device,
        &gpu.queue,
        pipelines,
        policy,
        &std::sync::atomic::AtomicBool::new(false),
        &mut work,
    ))
    .unwrap();
    (models, work)
}

#[test]
fn card_memory_shares_spares_grows_with_two_arrays_and_releases_the_last_holder() {
    use renderer_core::models::ArtStorePolicy;
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let art = [
        ("x", [0.3, 0.6, 0.9]),
        ("a", [0.4, 0.8, 0.3]),
        ("b", [0.8, 0.4, 0.2]),
    ];
    let policy = ArtStorePolicy {
        texture_spare_layers: 2,
        ..Default::default()
    };
    let store = Arc::new(std::sync::Mutex::new(ArtStore::new(policy)));
    let baseline = model_memory(&[], &store);
    let (live, _) = admit_card_models(&gpu, &pipelines, &art[..1], store.clone(), policy);
    let one = model_memory(&[&live], &store);
    let (fitting, fit_work) = admit_card_models(&gpu, &pipelines, &art[..2], store.clone(), policy);
    let fit = model_memory(&[&live, &fitting], &store);
    assert_eq!(
        fit.texture_bytes, one.texture_bytes,
        "spares allocate no card array"
    );
    assert_eq!(fit.textures, one.textures);
    assert_eq!(fit_work.card_bytes_copied, 0);
    let (pending, work) = admit_card_models(&gpu, &pipelines, &art, store.clone(), policy);
    assert!(work.card_bytes_copied > 0);
    let overlap = model_memory(&[&live, &fitting, &pending], &store);
    let clean_store = Arc::new(std::sync::Mutex::new(ArtStore::new(policy)));
    let (clean, _) = admit_card_models(&gpu, &pipelines, &art, clean_store.clone(), policy);
    let destination = model_memory(&[&clean], &clean_store);
    assert!(
        overlap.texture_bytes <= one.texture_bytes + destination.texture_bytes,
        "growth holds at most source and destination, including the shared model arrays"
    );
    assert!(overlap.textures <= one.textures + destination.textures);
    drop((live, fitting));
    let committed = model_memory(&[&pending], &store);
    assert_eq!(
        committed.texture_bytes, destination.texture_bytes,
        "retirement releases the source"
    );
    assert_eq!(committed.textures, destination.textures);
    // Surviving generations can still read shared textures after retirement.
    let rows = [0];
    let request = pending
        .impostor_request(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &rows,
            &renderer_core::impostor::Spec::runtime(),
            &mut AdmissionWork::default(),
        )
        .unwrap();
    drop(pending);
    let released = model_memory(&[], &store);
    assert_eq!(released.buffer_bytes, baseline.buffer_bytes);
    assert_eq!(released.texture_bytes, baseline.texture_bytes);
    assert_eq!(released.buffers, baseline.buffers);
    assert_eq!(released.textures, baseline.textures);
    let read = pollster::block_on(request.read()).unwrap();
    assert!(read[0]
        .albedo
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[3] > 0));
    println!("card memory live={one:?} fits={fit:?} overlap={overlap:?} committed={committed:?} baseline={baseline:?}");
}

#[test]
fn a_changed_rest_palette_is_a_new_card_over_the_same_vertices() {
    let gpu = support::gpu().expect("a GPU is required for card admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let (bundle, skeleton) = card_bundle([0.4, 0.8, 0.3], 0.);
    let mut library = Library::default();
    library
        .insert_bundle(
            "x",
            ContentId::new(
                contract::identity::bytes_hash(&bundle),
                Some(contract::identity::bytes_hash(&skeleton)),
            )
            .unwrap(),
            &bundle,
            &[],
        )
        .unwrap();
    let store = ArtStore::shared();
    let mut held = Vec::new();
    for translation in [0., 0.4] {
        let mut palette = identity();
        palette.x[3] = translation;
        let mut work = AdmissionWork::default();
        let mut models = Models::from_world(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &library,
            &[Appearance {
                name_len: 1,
                lies: NONE,
                ..Default::default()
            }],
            "x",
            |_| Some(vec![palette]),
            |_, _| None,
            store.clone(),
            &mut work,
        )
        .unwrap();
        if !held.is_empty() {
            assert_eq!(
                work.vertex_bytes_packed, 0,
                "living mesh vertices are unposed"
            );
        }
        pollster::block_on(models.bake_cards(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            Default::default(),
            &std::sync::atomic::AtomicBool::new(false),
            &mut work,
        ))
        .unwrap();
        assert_eq!(
            work.cards_baked, 1,
            "the rendered palette is a card-key input"
        );
        held.push(models);
    }
}
