//! The shared runtime consumes only drawn publications, independently of shell pacing.
use native_client::{bundle::Library, capture::Replay, effects, gpu};
use presentation::assets::RUNTIME;
use presentation::records::NONE;
use renderer_core::runtime::{Runtime, RuntimeConfig, RuntimeWorld, WorldGeometry};
use std::{path::Path, sync::Arc};

fn scene(gpu: &gpu::Gpu) -> (Replay, Runtime) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let mut replay = Replay::open(
        &root.join("fixtures/parity/capture/run/paris-corner"),
        &library,
    )
    .unwrap();
    let geometry = Arc::new(WorldGeometry::from_triangles(
        &replay.terrain.0,
        &replay.terrain.1,
        &replay.presentation().ground().unwrap().water_cells(),
        replay.backdrop_reach_m,
    ));
    let config = RuntimeConfig {
        size: [320, 180],
        light: replay.light.clone(),
        fog_look: replay.fog_look,
        grass_density: Some(0.01),
        xray_min_hidden_fraction: None,
        paint: None,
        separate_high_air: false,
        water_detail: Default::default(),
        controls: Default::default(),
    };
    let effect_sheets = effects::load_sheets(
        &root.join("assets/third-party/effects"),
        renderer_core::pipelines::builtin_sizes().effect_sheet,
    )
    .unwrap();
    let mut runtime = Runtime::new(
        &gpu.device,
        &gpu.queue,
        config,
        RuntimeWorld {
            presentation: replay.presentation_mut(),
            assets: &library,
            geometry,
            effect_sheets,
        },
    )
    .unwrap();
    pollster::block_on(runtime.prepare_assets(&gpu.device, &gpu.queue)).unwrap();
    let first = replay.records[0].clone();
    replay
        .frame(&first, runtime.static_generation(), |_| {})
        .unwrap();
    runtime
        .prepare(&gpu.device, &gpu.queue, replay.presentation_mut())
        .unwrap();
    (replay, runtime)
}

#[test]
fn hidden_grass_does_not_request_scar_uploads_for_its_last_regrow_window() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a real GPU");
    let (mut replay, mut runtime) = scene(&gpu);
    assert!(
        runtime.ground.material.scars.grass_upload().is_some(),
        "control must regrow grass"
    );
    let mut controls = runtime.config().controls;
    controls.grass = false;
    runtime
        .set_controls(&gpu.queue, replay.presentation(), controls)
        .unwrap();
    for _ in 0..3 {
        runtime
            .prepare(&gpu.device, &gpu.queue, replay.presentation_mut())
            .unwrap();
        assert!(
            runtime.ground.material.scars.grass_upload().is_none(),
            "hidden grass requested a stale scar upload"
        );
        assert!(
            runtime.ground.material.scars.view_upload().is_some(),
            "hiding grass must still maintain ground scars"
        );
    }
    controls.grass = true;
    runtime
        .set_controls(&gpu.queue, replay.presentation(), controls)
        .unwrap();
    runtime
        .prepare(&gpu.device, &gpu.queue, replay.presentation_mut())
        .unwrap();
    assert!(
        runtime.ground.material.scars.grass_upload().is_some(),
        "restoring grass must load current scars for its regrow"
    );
}

#[test]
fn skipped_publications_refresh_current_static_instances_without_shell_invalidation() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a real GPU");
    let (mut replay, mut runtime) = scene(&gpu);
    let previous: std::collections::HashMap<_, _> = runtime
        .models
        .statics()
        .listed()
        .map(|held| {
            (
                held.key,
                bytemuck::cast_slice::<_, u8>(runtime.models.statics().records(held.key)).to_vec(),
            )
        })
        .collect();
    let mut current = replay.records[replay.records.len() / 2].clone();
    current.canvas = [320, 180];
    // Reframe, then publish the same view again before preparing it, as the lab does.
    replay
        .frame(&current, runtime.static_generation(), |_| {})
        .unwrap();
    let frame = replay.last_frame();
    assert!(frame.batches.iter().any(|b| runtime.models.statics().holds(b.appearance) && !previous.contains_key(&b.key)), "control needs a newly named static batch");
    assert!(
        frame.batches.iter().filter(|b| b.first != NONE).any(|b| {
            previous.get(&b.key).is_some_and(|old| {
                old.as_slice()
                    != bytemuck::cast_slice::<_, u8>(
                        &frame.instances[b.first as usize..(b.first + b.count) as usize],
                    )
            })
        }),
        "control needs changed instances under an existing static key"
    );
    replay
        .frame(&current, runtime.static_generation(), |_| {})
        .unwrap();
    assert!(
        replay.last_frame().batches.iter().any(|b| b.first == NONE),
        "control needs held batches"
    );
    runtime
        .prepare(&gpu.device, &gpu.queue, replay.presentation_mut())
        .unwrap();
    replay
        .presentation_mut()
        .resend_static_frame(runtime.static_generation());
    let frame = replay.last_frame();
    let statics = runtime.models.statics();
    let mut checked = 0;
    for batch in frame.batches.iter().filter(|b| statics.holds(b.appearance)) {
        assert_ne!(batch.first, NONE);
        let records = &frame.instances[batch.first as usize..(batch.first + batch.count) as usize];
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(statics.records(batch.key)),
            bytemuck::cast_slice::<_, u8>(records),
            "held static instances must match current canonical population for key {}",
            batch.key
        );
        checked += 1;
    }
    assert!(checked > 0, "control needs static instances");
}
