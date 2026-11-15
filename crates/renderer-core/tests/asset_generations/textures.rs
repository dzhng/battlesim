//! Shared model texture admission, pixels and lifetime.
use super::*;

#[test]
fn texture_growth_writes_only_new_content_and_shares_existing_channels() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let payloads = [
        texture_bytes(4, true, 37),
        texture_bytes(4, false, 63),
        texture_bytes(4, false, 91),
    ];
    let ids = payloads
        .each_ref()
        .map(|bytes| contract::identity::bytes_hash(bytes));
    let textures: Vec<_> = ids
        .iter()
        .zip(&payloads)
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect();
    let x = bundle_bytes("static", 1, "default", 1, &[("albedo", 0), ("orm", 1)]);
    let y = bundle_bytes("static", 2, "default", 1, &[("albedo", 0), ("orm", 1)]);
    let a = bundle_bytes("static", 3, "default", 1, &[("albedo", 0), ("orm", 2)]);
    let xy: Art<'_> = &[("x", &x), ("y", &y)];
    let axy: Art<'_> = &[("a", &a), ("x", &x), ("y", &y)];
    let mut stage = Stage::new(&gpu, xy, runtime_config(), None, &textures);
    let before = stage.pixels(&gpu);
    let ready = stage.stage(&gpu, axy, &textures);
    let bundle = renderer_core::bundle::decode("a", &a, &textures).unwrap();
    let new = [&bundle.textures[2]];
    assert_eq!(
        ready.work().texture_layers_written,
        new.len() as u32,
        "a shares x's albedo; only its ORM is new"
    );
    assert_eq!(
        ready.work().texture_bytes_uploaded,
        new.iter()
            .flat_map(|texture| &texture.levels)
            .map(|level| level.len() as u64)
            .sum::<u64>()
    );
    assert_eq!(
        stage.pixels(&gpu),
        before,
        "writing unused layers must leave committed materials intact"
    );
    stage.commit(&gpu, axy, ready);
    let mut clean = Stage::new(&gpu, axy, runtime_config(), None, &textures);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
}

#[test]
fn shared_texture_survives_retirement_and_last_holder_returns_memory_to_baseline() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let payload = texture_bytes(4, true, 57);
    let id = contract::identity::bytes_hash(&payload);
    let textures = [(id.as_str(), payload.as_slice())];
    let bytes = bundle_bytes("static", 1, "default", 1, &[("albedo", 0)]);
    let art: Art<'_> = &[("x", &bytes)];
    let store = ArtStore::shared();
    let baseline = model_memory(&[], &store);
    let (old, _) = admit_models(&gpu, &pipelines, art, &textures, store.clone());
    let one = model_memory(&[&old], &store);
    let (new, work) = admit_models(&gpu, &pipelines, art, &textures, store.clone());
    assert_eq!(work.texture_layers_written, 0);
    assert_eq!(
        model_memory(&[&old, &new], &store).texture_bytes,
        one.texture_bytes
    );
    let spec = renderer_core::impostor::Spec {
        yaws: 1,
        pitches: vec![0.8],
        cell: 16,
        supersample: 1,
    };
    let read = || {
        pollster::block_on(
            new.impostor_request(
                &gpu.device,
                &gpu.queue,
                &pipelines,
                &[0],
                &spec,
                &mut AdmissionWork::default(),
            )
            .unwrap()
            .read(),
        )
        .unwrap()
        .remove(0)
        .albedo
    };
    let before = read();
    assert!(
        before.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0),
        "the readback must contain the textured model"
    );
    drop(old);
    assert_eq!(
        read(),
        before,
        "the remaining holder can still sample and read back its shared texture"
    );
    drop(new);
    assert_eq!(
        model_memory(&[], &store).texture_bytes,
        baseline.texture_bytes
    );
}

#[test]
fn texture_array_growth_and_larger_edges_draw_the_clean_build_pixels() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let payloads = [
        texture_bytes(4, true, 47),
        texture_bytes(4, false, 63),
        texture_bytes(4, false, 91),
        texture_bytes(4, false, 127),
        texture_bytes(8, false, 157),
    ];
    let ids = payloads
        .each_ref()
        .map(|bytes| contract::identity::bytes_hash(bytes));
    let textures: Vec<_> = ids
        .iter()
        .zip(&payloads)
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect();
    let x = bundle_bytes("static", 1, "default", 1, &[("albedo", 0), ("orm", 1)]);
    let y = bundle_bytes("static", 2, "default", 1, &[("albedo", 0), ("orm", 1)]);
    let a = bundle_bytes(
        "static",
        3,
        "default",
        1,
        &[("albedo", 0), ("normal", 2), ("orm", 3)],
    );
    let b = bundle_bytes("static", 4, "default", 1, &[("albedo", 0), ("orm", 4)]);
    let xy: Art<'_> = &[("x", &x), ("y", &y)];
    let axy: Art<'_> = &[("a", &a), ("x", &x), ("y", &y)];
    let all: Art<'_> = &[("a", &a), ("b", &b), ("x", &x), ("y", &y)];
    let mut config = runtime_config();
    config.art_store.texture_spare_layers = 1;
    let mut stage = Stage::new(&gpu, xy, config, None, &textures);
    let initial = stage.pixels(&gpu);
    let grown = stage.stage(&gpu, axy, &textures);
    let decoded = renderer_core::bundle::decode("a", &a, &textures).unwrap();
    let surface_bytes = decoded.textures[1]
        .levels
        .iter()
        .map(|l| l.len() as u64)
        .sum::<u64>();
    assert_eq!(
        grown.work().texture_bytes_copied,
        surface_bytes,
        "the old ORM's mips are copied GPU-side"
    );
    assert_eq!(grown.work().texture_arrays_rebuilt, 0);
    assert_eq!(stage.pixels(&gpu), initial);
    stage.commit(&gpu, axy, grown);
    let mut clean = Stage::new(&gpu, axy, runtime_config(), None, &textures);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
    let before_edge = stage.pixels(&gpu);
    let enlarged = stage.stage(&gpu, all, &textures);
    assert_eq!(
        enlarged.work().texture_arrays_rebuilt,
        1,
        "only the linear array's edge changed"
    );
    assert_eq!(stage.pixels(&gpu), before_edge);
    stage.commit(&gpu, all, enlarged);
    let mut clean = Stage::new(&gpu, all, runtime_config(), None, &textures);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
    let mut channels = runtime_config().controls;
    channels.texture_channels = [false; 3];
    stage
        .runtime
        .set_controls(&gpu.queue, &stage.presentation, channels)
        .unwrap();
    clean
        .runtime
        .set_controls(&gpu.queue, &clean.presentation, channels)
        .unwrap();
    assert_eq!(
        stage.pixels(&gpu),
        clean.pixels(&gpu),
        "texture-channel diagnostics use this generation's layer rows"
    );
}

#[test]
fn dropped_texture_layers_leave_live_pixels_unchanged_and_are_reused() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let payloads = [
        texture_bytes(4, true, 57),
        texture_bytes(4, true, 97),
        texture_bytes(8, true, 143),
    ];
    let ids = payloads
        .each_ref()
        .map(|bytes| contract::identity::bytes_hash(bytes));
    let textures: Vec<_> = ids
        .iter()
        .zip(&payloads)
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect();
    let x = bundle_bytes("static", 1, "default", 1, &[("albedo", 0)]);
    let a = bundle_bytes("static", 3, "default", 1, &[("albedo", 1)]);
    let art: Art<'_> = &[("x", &x)];
    let next: Art<'_> = &[("a", &a), ("x", &x)];
    let mut stage = Stage::new(&gpu, art, runtime_config(), None, &textures);
    let before = stage.pixels(&gpu);
    let dropped = stage.stage(&gpu, next, &textures);
    assert!(dropped.work().texture_layers_written > 0);
    assert_eq!(stage.pixels(&gpu), before);
    drop(dropped);
    assert_eq!(stage.pixels(&gpu), before);
    let ready = stage.stage(&gpu, next, &textures);
    assert_eq!(
        ready.work().texture_layers_written,
        0,
        "the unused immutable layer is still held by the live array"
    );
    assert_eq!(ready.work().texture_bytes_uploaded, 0);
    stage.commit(&gpu, next, ready);
    let mut clean = Stage::new(&gpu, next, runtime_config(), None, &textures);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
    let current = stage.pixels(&gpu);
    let b = bundle_bytes("static", 4, "default", 1, &[("albedo", 2)]);
    let larger = stage.stage(&gpu, &[("a", &a), ("b", &b), ("x", &x)], &textures);
    assert_eq!(stage.pixels(&gpu), current);
    let unchanged = stage.stage(&gpu, next, &textures);
    assert_eq!(unchanged.work().texture_layers_written, 0);
    assert_eq!(
        unchanged.work().texture_bytes_copied,
        0,
        "the surviving exact-edge array already holds every requested layer"
    );
    stage.commit(&gpu, next, unchanged);
    assert!(
        stage.pixels(&gpu) == current,
        "a held larger pending array must not change the committed small-edge picture"
    );
    drop(larger);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
}

#[test]
fn texture_overlap_uses_spares_then_grows_within_the_preplan_total_payload() {
    use renderer_core::models::ArtStorePolicy;
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let pipelines = renderer_core::pipelines::build_builtin(&gpu.device).unwrap();
    let payloads = [
        texture_bytes(4, false, 63),
        texture_bytes(4, false, 91),
        texture_bytes(4, false, 127),
    ];
    let ids = payloads
        .each_ref()
        .map(|bytes| contract::identity::bytes_hash(bytes));
    let textures: Vec<_> = ids
        .iter()
        .zip(&payloads)
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect();
    let x = bundle_bytes("static", 1, "default", 1, &[("orm", 0)]);
    let y = bundle_bytes("static", 2, "default", 1, &[("orm", 0)]);
    let fits = bundle_bytes("static", 3, "default", 1, &[("orm", 1)]);
    let grows = bundle_bytes("static", 3, "default", 1, &[("normal", 1), ("orm", 2)]);
    let old_art: Art<'_> = &[("x", &x), ("y", &y)];
    let fit_art: Art<'_> = &[("a", &fits), ("x", &x), ("y", &y)];
    let grow_art: Art<'_> = &[("a", &grows), ("x", &x), ("y", &y)];
    let new_store = |spare| {
        Arc::new(std::sync::Mutex::new(ArtStore::new(ArtStorePolicy {
            texture_spare_layers: spare,
            ..Default::default()
        })))
    };
    // Independent exact-fit arrays and independent geometry reproduce the
    // two-generation payload before this plan, without a second renderer path.
    let baseline_old_store = new_store(0);
    let baseline_new_store = new_store(0);
    let (baseline_old, _) = admit_models(
        &gpu,
        &pipelines,
        old_art,
        &textures,
        baseline_old_store.clone(),
    );
    let (baseline_new, _) = admit_models(
        &gpu,
        &pipelines,
        grow_art,
        &textures,
        baseline_new_store.clone(),
    );
    let mut memory = renderer_core::memory::ResourceMemory::default();
    baseline_old.record_memory(&mut memory);
    baseline_new.record_memory(&mut memory);
    let preplan = memory.counts();
    drop(memory);
    drop((baseline_old, baseline_new));

    let store = new_store(1);
    let (live, _) = admit_models(&gpu, &pipelines, old_art, &textures, store.clone());
    let live_memory = model_memory(&[&live], &store);
    let (fitting, _) = admit_models(&gpu, &pipelines, fit_art, &textures, store.clone());
    let fit_memory = model_memory(&[&live, &fitting], &store);
    assert_eq!(
        fit_memory.texture_bytes, live_memory.texture_bytes,
        "using spare layers allocates no texture"
    );
    assert_eq!(fit_memory.textures, live_memory.textures);
    drop((fitting, live));

    // Measure growth independently of the fitting candidate's cancelled
    // material-slot reservation; texture and material slot lifetime differ.
    let store = new_store(1);
    let (live, _) = admit_models(&gpu, &pipelines, old_art, &textures, store.clone());
    let clean_store = new_store(1);
    let (clean, _) = admit_models(&gpu, &pipelines, grow_art, &textures, clean_store.clone());
    let destination = model_memory(&[&clean], &clean_store);
    let shared_geometry = renderer_core::bundle::decode("x", &x, &textures).unwrap();
    let renderer_core::bundle::Shape::Static(states) = &shared_geometry.shape else {
        panic!("static fixture");
    };
    let one_mesh_bytes = states[0]
        .tiers
        .iter()
        .map(|mesh| mesh.vertices() * VERTEX_BYTES + mesh.indices.len() * size_of::<u32>())
        .sum::<usize>() as u64;
    drop(clean);
    let (pending, work) = admit_models(&gpu, &pipelines, grow_art, &textures, store.clone());
    let overlap = model_memory(&[&live, &pending], &store);
    assert!(work.texture_bytes_copied > 0);
    assert!(
        overlap.texture_bytes <= live_memory.texture_bytes + destination.texture_bytes,
        "growth holds at most source plus destination, with no intermediate copy"
    );
    assert!(overlap.textures <= live_memory.textures + destination.textures);
    assert_eq!(
        overlap.buffer_bytes,
        live_memory.buffer_bytes + destination.buffer_bytes - old_art.len() as u64 * one_mesh_bytes,
        "only new geometry and the candidate's generation-local tables/buffers add payload"
    );
    let total = |m: renderer_core::memory::AllocationCounts| m.buffer_bytes + m.texture_bytes;
    assert!(
        total(overlap) <= total(preplan),
        "shared overlap {} must not exceed pre-plan {}",
        total(overlap),
        total(preplan)
    );
    drop(live);
    let committed = model_memory(&[&pending], &store);
    assert_eq!(
        committed.texture_bytes, destination.texture_bytes,
        "commit releases the old array"
    );
    assert_eq!(committed.textures, destination.textures);
    println!("slice04 memory live={live_memory:?} fits={fit_memory:?} grows={overlap:?} committed={committed:?} preplan={preplan:?}");
}

#[test]
fn dead_texture_layers_use_the_same_fresh_store_admission() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let payloads = [texture_bytes(4, true, 57), texture_bytes(4, true, 97)];
    let ids = payloads
        .each_ref()
        .map(|bytes| contract::identity::bytes_hash(bytes));
    let textures: Vec<_> = ids
        .iter()
        .zip(&payloads)
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect();
    let x = bundle_bytes("static", 1, "default", 1, &[("albedo", 0)]);
    let a = bundle_bytes("static", 3, "default", 1, &[("albedo", 1)]);
    let art: Art<'_> = &[("x", &x)];
    let mut config = runtime_config();
    config.art_store.dead_texture_layers = 0;
    let mut stage = Stage::new(&gpu, art, config, None, &textures);
    let before = stage.pixels(&gpu);
    drop(stage.stage(&gpu, &[("a", &a), ("x", &x)], &textures));
    let ready = stage.stage(&gpu, art, &textures);
    assert_eq!(
        ready.work().rebuild_reason,
        Some(renderer_core::runtime::RebuildReason::DeadTextureLayers)
    );
    assert_eq!(stage.pixels(&gpu), before);
    stage.commit(&gpu, art, ready);
    assert_eq!(stage.pixels(&gpu), before);
    let mut clean = Stage::new(&gpu, art, runtime_config(), None, &textures);
    assert_eq!(stage.pixels(&gpu), clean.pixels(&gpu));
}
