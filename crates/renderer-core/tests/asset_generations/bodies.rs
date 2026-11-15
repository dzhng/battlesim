//! Corpse rows draw the authored living geometry with their own rest palette.
use super::*;

#[test]
fn corpse_rows_pack_only_the_living_tiers_and_share_their_buffers() {
    let gpu = support::gpu().expect("a GPU is required for corpse admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let presentation = Presentation::new(None, 30.).unwrap();
    let art = &CARD_ART[..1];
    let (library, catalog, _) = card_catalog(&presentation, art);
    let view = catalog.view(&presentation).unwrap();
    let world = view.world();
    let rows: Vec<_> = world
        .appearances
        .iter()
        .copied()
        .filter(|r| r.lies == NONE)
        .collect();
    let admit = |rows: &[Appearance]| {
        let store = ArtStore::shared();
        let mut work = AdmissionWork::default();
        let models = Models::from_world(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &library,
            rows,
            &world.strings,
            |row| view.rest_palette(row),
            |_, _| None,
            store.clone(),
            &mut work,
        )
        .unwrap();
        (models, store, work)
    };
    let (living, living_store, living_work) = admit(&rows);
    let (all, all_store, all_work) = admit(&world.appearances);
    let bundle = library.bundle(art[0].0).unwrap();
    let renderer_core::bundle::Shape::Skinned(tiers) = &bundle.shape else {
        panic!("skinned fixture")
    };
    let expected = tiers
        .iter()
        .map(|m| m.vertices() * VERTEX_BYTES)
        .sum::<usize>() as u64;
    assert!(expected > 0, "the fixture must pack real vertices");
    assert_eq!(living_work.vertex_bytes_packed, expected);
    assert_eq!(
        all_work.vertex_bytes_packed, expected,
        "poses must never pack geometry"
    );
    let living_memory = model_memory(&[&living], &living_store);
    let all_memory = model_memory(&[&all], &all_store);
    assert_eq!(
        (all_memory.buffers, all_memory.buffer_bytes),
        (living_memory.buffers, living_memory.buffer_bytes),
        "corpse routing cannot own another mesh buffer"
    );
}

#[test]
fn corpse_frame_and_card_equal_the_living_mesh_frozen_in_its_rest_palette() {
    let gpu = support::gpu().expect("a GPU is required for corpse pixel tests");
    let Stage {
        mut presentation,
        mut runtime,
    } = Stage::new(&gpu, &[], runtime_config(), None, &[]);
    let (mut library, catalog, _) = card_catalog(&presentation, &CARD_ART[..1]);
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
        .commit_assets(&gpu.device, &gpu.queue, &mut presentation, ready, vec![])
        .unwrap());
    let world = presentation.world();
    let corpse = world.appearances.iter().position(|r| r.lies == 0).unwrap() as u32;
    let rest = presentation.rest_palette(corpse).unwrap();
    let palette: Vec<f32> = rest
        .iter()
        .flat_map(|j| j.x.into_iter().chain(j.y).chain(j.z))
        .collect();
    let name = CARD_ART[0].0;
    let mut stage = Stage {
        presentation,
        runtime,
    };
    let model = |pose, tier| {
        serde_json::from_value(
            json!([{"appearance":name,"x":3,"y":3,"z":0,"yaw":0.4,"tier":tier,"pose":pose}]),
        )
        .unwrap()
    };
    stage
        .presentation
        .set_lab_models(model(json!({"kind":"corpse","lies":0}), 0))
        .unwrap();
    let corpse_pixels = stage.pixels(&gpu);
    stage
        .presentation
        .set_lab_models(model(json!({"kind":"posed","palette":palette}), 0))
        .unwrap();
    assert_eq!(
        corpse_pixels,
        stage.pixels(&gpu),
        "near corpse equals a frozen living body"
    );
    stage.presentation.set_lab_models(vec![]).unwrap();
    assert_ne!(
        corpse_pixels,
        stage.pixels(&gpu),
        "the oracle must draw a body"
    );

    // Give the living row the corpse's rest palette, so both independent bakes
    // describe the same frozen body rather than the living row's far pose.
    let world = stage.presentation.world();
    let mut models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &stage.runtime.pipelines,
        &library,
        &world.appearances,
        &world.strings,
        |_| Some(rest.clone()),
        |_, _| None,
        ArtStore::shared(),
        &mut AdmissionWork::default(),
    )
    .unwrap();
    let rows: Vec<_> = (0..world.appearances.len() as u32).collect();
    let cards = pollster::block_on(
        models
            .impostor_request(
                &gpu.device,
                &gpu.queue,
                &stage.runtime.pipelines,
                &rows,
                &renderer_core::impostor::Spec::runtime(),
                &mut AdmissionWork::default(),
            )
            .unwrap()
            .read(),
    )
    .unwrap();
    assert_eq!(cards.len(), 2);
    for card in &cards {
        assert!(
            card.albedo.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "the card oracle must draw"
        );
    }
    assert_eq!(cards[0].center, cards[1].center);
    assert_eq!(cards[0].radius, cards[1].radius);
    assert_eq!(
        cards[0].albedo, cards[1].albedo,
        "corpse card equals frozen living card"
    );
    assert_eq!(cards[0].normal, cards[1].normal);
    assert_eq!(cards[0].mask, cards[1].mask);
    pollster::block_on(models.bake_cards(
        &gpu.device,
        &gpu.queue,
        &stage.runtime.pipelines,
        Default::default(),
        &std::sync::atomic::AtomicBool::new(false),
        &mut AdmissionWork::default(),
    ))
    .unwrap();
    stage.runtime.models = models;
    let card_tier = presentation::view::IMPOSTOR as u32;
    stage
        .presentation
        .set_lab_models(model(json!({"kind":"corpse","lies":0}), card_tier))
        .unwrap();
    let corpse_card_frame = stage.pixels(&gpu);
    assert_eq!(stage.runtime.models.drawn().impostors, 1);
    stage
        .presentation
        .set_lab_models(model(json!({"kind":"posed","palette":palette}), card_tier))
        .unwrap();
    assert_eq!(
        corpse_card_frame,
        stage.pixels(&gpu),
        "far corpse equals the frozen living card in the frame"
    );
    stage.presentation.set_lab_models(vec![]).unwrap();
    assert_ne!(
        corpse_card_frame,
        stage.pixels(&gpu),
        "the far oracle must draw its card"
    );
}
