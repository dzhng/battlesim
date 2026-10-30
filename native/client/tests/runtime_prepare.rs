//! The shared runtime consumes only drawn publications, independently of shell pacing.
use native_client::{bundle::Library, capture::Replay, effects, gpu};
use presentation::assets::RUNTIME;
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
