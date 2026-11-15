//! Content/pose-derived facts share the lifetime of the cached model geometry.
use super::*;

#[test]
fn rest_pose_growth_derives_only_the_new_contents_rows() {
    let gpu = support::gpu().expect("a GPU is required for rest-pose admission tests");
    let bytes = [
        bundle_bytes("static", 1, "default", 2, &[]),
        bundle_bytes("static", 2, "default", 2, &[]),
        bundle_bytes("articulated", 3, "default", 2, &[]),
    ];
    let xy: Art<'_> = &[("x", &bytes[0]), ("y", &bytes[1])];
    let axy: Art<'_> = &[("a", &bytes[2]), ("x", &bytes[0]), ("y", &bytes[1])];
    let mut stage = Stage::new(&gpu, xy, runtime_config(), None, &[]);
    let draft = catalog(&stage.presentation, axy);
    let world = draft.view(&stage.presentation).unwrap().world();
    let added_rows = world
        .appearances
        .iter()
        .filter(|row| world.name(row) == "a")
        .count();
    assert!(added_rows > 0, "the fixture must add a drawable row");
    let ready = stage.stage(&gpu, axy, &[]);
    assert_eq!(ready.work().rest_poses_derived as usize, added_rows);
    stage.commit(&gpu, axy, ready);
    let pixels = stage.pixels(&gpu);
    let mut clean = Stage::new(&gpu, axy, runtime_config(), None, &[]);
    assert_eq!(pixels, clean.pixels(&gpu));
    let mut no_models = runtime_config().controls;
    no_models.models = false;
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
fn shared_rest_poses_bake_identically_to_fresh_poses_under_new_names() {
    let gpu = support::gpu().expect("a GPU is required for rest-pose card tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let presentation = Presentation::new(None, 30.).unwrap();
    let (library, catalog, _) = card_catalog(&presentation, CARD_ART);
    let store = ArtStore::shared();
    let held = Models::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        catalog.view(&presentation).unwrap(),
        store.clone(),
        &mut AdmissionWork::default(),
    )
    .unwrap();
    let aliases: Vec<_> = CARD_ART
        .iter()
        .enumerate()
        .map(|(i, (_, color))| (format!("alias-{i}"), *color))
        .collect();
    let aliases: Vec<_> = aliases
        .iter()
        .map(|(name, color)| (name.as_str(), *color))
        .collect();
    let (library, catalog, _) = card_catalog(&presentation, &aliases);
    let view = catalog.view(&presentation).unwrap();
    let rows: Vec<_> = (0..view.world().appearances.len() as u32).collect();
    let mut work = AdmissionWork::default();
    let shared = Models::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        view,
        store,
        &mut work,
    )
    .unwrap();
    assert_eq!(
        work.rest_poses_derived, 0,
        "names cannot prevent content/pose reuse"
    );
    let fresh = Models::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &library,
        view,
        ArtStore::shared(),
        &mut AdmissionWork::default(),
    )
    .unwrap();
    let spec = renderer_core::impostor::Spec {
        yaws: 2,
        pitches: vec![0., 0.3],
        cell: 32,
        supersample: 1,
    };
    let bake = |models: &Models| {
        pollster::block_on(
            models
                .impostor_request(
                    &gpu.device,
                    &gpu.queue,
                    &pipelines,
                    &rows,
                    &spec,
                    &mut AdmissionWork::default(),
                )
                .unwrap()
                .read(),
        )
        .unwrap()
    };
    let shared = bake(&shared);
    let fresh = bake(&fresh);
    assert_eq!(shared.len(), rows.len());
    assert_eq!(fresh.len(), rows.len());
    for (shared, fresh) in shared.iter().zip(&fresh) {
        assert!(
            shared.albedo.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "the fixture must draw"
        );
        assert_eq!(
            (
                &shared.appearance,
                &shared.spec,
                shared.width,
                shared.height,
                shared.center,
                shared.radius,
                &shared.albedo,
                &shared.normal,
                &shared.mask,
                &shared.hash
            ),
            (
                &fresh.appearance,
                &fresh.spec,
                fresh.width,
                fresh.height,
                fresh.center,
                fresh.radius,
                &fresh.albedo,
                &fresh.normal,
                &fresh.mask,
                &fresh.hash
            )
        );
    }
    drop(held);
}
