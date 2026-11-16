//! A generation warmed over frames is the same atomic generation as a cold stage.
use super::*;

#[test]
fn geometry_steps_pack_one_key_and_final_admission_packs_none() {
    let gpu = support::gpu().expect("a real GPU");
    let bytes = [
        bundle_bytes("static", 1, "default", 2, &[]),
        bundle_bytes("static", 2, "default", 2, &[]),
        bundle_bytes("static", 3, "default", 2, &[]),
        bundle_bytes("static", 4, "default", 2, &[]),
    ];
    let initial: Art<'_> = &[("x", &bytes[0]), ("y", &bytes[1])];
    let wanted: Art<'_> = &[
        ("a", &bytes[2]),
        ("b", &bytes[3]),
        ("x", &bytes[0]),
        ("y", &bytes[1]),
    ];
    let stage = Stage::new(&gpu, initial, runtime_config(), None, &[]);
    let mut source = library(wanted, &[]);
    let work = source.take_admission_work();
    let mut pending = stage
        .runtime
        .stage_assets(
            &gpu.device,
            &gpu.queue,
            &stage.presentation,
            catalog(&stage.presentation, wanted),
            &source,
            work,
        )
        .unwrap();
    assert_eq!(
        pending.work().vertex_bytes_packed,
        0,
        "creating the candidate must not pack geometry"
    );
    let renderer_core::bundle::Shape::Static(states) = &source.bundle("a").unwrap().shape else {
        panic!("static fixture")
    };
    let one_key = states[0]
        .tiers
        .iter()
        .map(|m| m.vertices() * VERTEX_BYTES)
        .sum::<usize>() as u64;
    let mut previous = pending.work().vertex_bytes_packed;
    let mut steps = Vec::new();
    while pending.step().unwrap() {
        let current = pending.work().vertex_bytes_packed;
        steps.push(current - previous);
        previous = current;
    }
    assert_eq!(
        steps,
        vec![one_key, one_key],
        "each step packs exactly one missing key"
    );
    let ready = pollster::block_on(pending.prepare()).unwrap();
    assert_eq!(
        ready.work().vertex_bytes_packed,
        previous,
        "final admission must find every packed key"
    );
    let mut source = library(wanted, &[]);
    let cold = Stage::new(&gpu, initial, runtime_config(), None, &[]);
    let work = source.take_admission_work();
    let at_once = pollster::block_on(
        cold.runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &cold.presentation,
                catalog(&cold.presentation, wanted),
                &source,
                work,
            )
            .unwrap()
            .prepare(),
    )
    .unwrap();
    assert_eq!(
        ready.work(),
        at_once.work(),
        "stepping preserves the complete admission report"
    );
}

#[test]
fn dropped_warm_geometry_leaves_live_pixels_and_committed_steps_equal_a_clean_runtime() {
    let gpu = support::gpu().expect("a real GPU");
    let bytes = [
        bundle_bytes("static", 1, "default", 1, &[]),
        bundle_bytes("static", 2, "default", 1, &[]),
        bundle_bytes("static", 3, "default", 1, &[]),
    ];
    let initial: Art<'_> = &[("x", &bytes[0])];
    let wanted: Art<'_> = &[("a", &bytes[1]), ("b", &bytes[2]), ("x", &bytes[0])];
    let mut stage = Stage::new(&gpu, initial, runtime_config(), None, &[]);
    let before = stage.pixels(&gpu);
    let make = |stage: &Stage| {
        let mut source = library(wanted, &[]);
        let work = source.take_admission_work();
        stage
            .runtime
            .stage_assets(
                &gpu.device,
                &gpu.queue,
                &stage.presentation,
                catalog(&stage.presentation, wanted),
                &source,
                work,
            )
            .unwrap()
    };
    let mut dropped = make(&stage);
    assert!(dropped.step().unwrap());
    let packed = dropped.work().vertex_bytes_packed;
    assert!(packed > 0);
    assert_eq!(
        stage.pixels(&gpu),
        before,
        "partial warming cannot alter committed pixels"
    );
    drop(dropped);
    let mut next = make(&stage);
    assert!(next.step().unwrap());
    assert_eq!(
        next.work().vertex_bytes_packed,
        packed,
        "the dropped draft released its only geometry holder"
    );
    assert_eq!(stage.pixels(&gpu), before);
    while next.step().unwrap() {}
    let ready = pollster::block_on(next.prepare()).unwrap();
    stage.commit(&gpu, wanted, ready);
    let pixels = stage.pixels(&gpu);
    let mut cold = Stage::new(&gpu, wanted, runtime_config(), None, &[]);
    assert_eq!(pixels, cold.pixels(&gpu));
    cold.presentation.set_lab_models(vec![]).unwrap();
    assert_ne!(
        pixels,
        cold.pixels(&gpu),
        "the pixel oracle must draw the warmed models"
    );
}
