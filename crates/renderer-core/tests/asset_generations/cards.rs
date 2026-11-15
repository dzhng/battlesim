//! Bounded card baking through real runtime admissions and readbacks.
use super::*;

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
        } = Stage::new(&gpu, &[], config, None, &[]);
        assert_eq!(
            runtime.admission_work().card_read_waits,
            0,
            "no cards means no read wait"
        );
        let (mut library, catalog, models) = card_catalog(&presentation, CARD_ART);
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
    } = Stage::new(&gpu, &[], config, None, &[]);
    assert_eq!(
        runtime.admission_work().card_read_waits,
        0,
        "no cards means no read wait"
    );
    let (mut library, catalog, models) = card_catalog(&presentation, CARD_ART);
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
    } = Stage::new(&gpu, &[], config, None, &[]);
    assert_eq!(
        runtime.admission_work().card_read_waits,
        0,
        "no cards means no read wait"
    );
    let (mut library, catalog, _) = card_catalog(&presentation, CARD_ART);
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
        let stage = Stage::new(&gpu, &[], config, None, &[]);
        for _ in 0..2 {
            let (mut library, catalog, _) = card_catalog(&stage.presentation, CARD_ART);
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
