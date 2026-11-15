//! Atomic catalog publication across candidate outcomes.
use super::*;

#[test]
fn real_generation_swap_keeps_live_work_for_dropped_stale_and_failed_candidates() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let bytes_a = bundle_bytes("static", 1, "default", 1, &[]);
    let bytes_b = bundle_bytes("static", 1, "default", 1, &[]);
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
