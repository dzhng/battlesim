//! Grass regrowth and capacity selection through the production GPU pipeline.
mod support;
use std::path::Path;

#[test]
fn grass_regrowth_preserves_draw_order_and_selects_a_stable_prefix_when_full() {
    use presentation::static_world::ground::grass::{frame_params, Quality, Window, LIMITS};
    use presentation::static_world::ground::{
        biome::Biome, grass::KindFacts, tables::Grid, Ground, Source,
    };
    use renderer_core::grass::{Grass, GrassData};
    use renderer_core::probes::grass::{queue_clumps, queue_counts};
    use renderer_core::terrain::{TerrainResources, TerrainUpload};

    let gpu = support::gpu().expect("a real GPU");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut biome = Biome::load(
        &std::fs::read_to_string(root.join("fixtures/biomes/summer.json")).unwrap(),
        None,
    )
    .unwrap();
    biome.grass.max_clumps_m2 = 16.0;
    biome.grass.pixels_per_clump = 4.0;
    biome.grass.fade_m_per_px = [0.05, 0.1];
    biome.grass.near_tier_px = 10.0;
    // One fake grass in every plot kind, with no roads, props, forests or water.
    let mut growth = biome.grass.growth.values().next().unwrap().clone();
    growth.density = 1.0;
    growth.height = 1.0;
    growth.rows = 0.0;
    growth.patches.height = [1.0, 1.0];
    growth.patches.thin = 0.0;
    growth.mix.truncate(1);
    biome.grass.growth = biome
        .plots
        .iter()
        .map(|p| (p.name.clone(), growth.clone()))
        .chain([("verge".into(), growth.clone())])
        .collect();
    let tuning = presentation::settings::PresentationSettings::from_value(
        &serde_json::from_slice(&std::fs::read(root.join("fixtures/presentation.json")).unwrap())
            .unwrap(),
    )
    .unwrap()
    .tuning();
    let ground = Ground::new(
        Source {
            map: [0.0, 0.0, 64.0, 64.0],
            playable: [0.0, 0.0, 64.0, 64.0],
            grid: Grid {
                nx: 33,
                ny: 33,
                spacing: 2.0,
                page_size: 16,
                ..Default::default()
            },
            ..Default::default()
        },
        biome,
        tuning,
    )
    .unwrap();
    let facts = [KindFacts {
        mesh_height: 0.4,
        top: 0.4,
        blades: 1.0,
        root_width: 0.1,
        bases: [0, 0],
    }];
    let params = ground
        .grass_params(&facts, &LIMITS, &Quality { density: 1.0 })
        .unwrap();
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let tables = ground.terrain_tables(ground.terrain_field().unwrap());
    let terrain = TerrainResources::from_upload(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        TerrainUpload {
            params: bytemuck::cast_slice(&tables.params),
            nodes: bytemuck::cast_slice(&tables.nodes),
            plots: bytemuck::cast_slice(&ground.plot_records(true)),
            surfaces: bytemuck::cast_slice(&tables.surfaces),
            surface_index: bytemuck::cast_slice(&tables.surface_index),
            scars: bytemuck::cast_slice(&tables.scars),
        },
    );
    let heights = renderer_core::buffer(
        &gpu.device,
        &gpu.queue,
        "flat-grass-ground",
        wgpu::BufferUsages::STORAGE,
        bytemuck::cast_slice(&ground.source.grid.pages()),
    );
    let window = Window {
        x0: 0.0,
        y0: 0.0,
        tiles_x: 16,
        tiles_y: 16,
        reach: 64.0,
    };
    let grow = |capacity: [u32; 2]| {
        let mut params = params.clone();
        params[26..28].copy_from_slice(&capacity);
        let mut grass = Grass::from_data(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &heights,
            GrassData {
                params: &params,
                props: &ground.grass_props(),
                shapes: &[0.0; 16],
                max_blades: 1,
                capacity,
            },
        );
        let mut results = Vec::new();
        for _ in 0..3 {
            grass.regrow();
            grass.prepare_records(
                &gpu.device,
                &gpu.queue,
                Some((window, 1.0)),
                &Default::default(),
                |words, window| {
                    // A fixed field view without frustum cuts; the density,
                    // ground, scars and placement still use the real pipeline.
                    frame_params(words, &[0.0; 16], [32.0, 32.0, 4.0], 0.001, window);
                },
            );
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            grass.encode_build(&mut encoder, ("terrainLayout", terrain.bind_group()));
            gpu.queue.submit([encoder.finish()]);
            let counts = queue_counts(&gpu.device, &gpu.queue, &grass).unwrap();
            let clumps = queue_clumps(&gpu.device, &gpu.queue, &grass, &[]).unwrap();
            let counts = pollster::block_on(counts.read(&gpu.device)).unwrap();
            let clumps = pollster::block_on(clumps.read(&gpu.device)).unwrap();
            results.push((counts, serde_json::to_value(clumps).unwrap()));
        }
        assert!(
            results.windows(2).all(|pair| pair[0] == pair[1]),
            "regrowing a frozen field must preserve every clump and its draw order"
        );
        results.remove(0)
    };
    let (full, clumps) = grow([100_000, 100_000]);
    assert_eq!((full.near, full.far), (full.near_found, full.far_found));
    let (capped, selected) = grow([3, 5]);
    assert_eq!((capped.near_found, capped.far_found), (full.near, full.far));
    assert!(
        full.near > 3 || full.far > 5,
        "the field must exercise overflow"
    );
    let expected: Vec<_> = clumps
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["tier"] == 0)
        .take(3)
        .chain(
            clumps
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["tier"] == 1)
                .take(5),
        )
        .cloned()
        .collect();
    assert_eq!(
        selected,
        serde_json::json!(expected),
        "full tiers must retain a deterministic prefix, rather than the first racing invocations"
    );
}
