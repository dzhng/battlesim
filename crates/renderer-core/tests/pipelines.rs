//! Every built-in pipeline builds on this machine's device and retains the
//! size facts the shared catalog owns. Needs a GPU: run under the GPU lock.
mod support;

use renderer_core::pipelines;

#[test]
fn every_builtin_pipeline_builds_on_the_device() {
    let support::Gpu {
        device,
        queue: _queue,
    } = support::gpu().expect("a device");
    let built = pipelines::build_builtin(&device).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        built.names().count(),
        renderer_core::pipelines::builtin_pipeline_count()
    );
}

#[test]
fn every_bloom_level_has_a_blur_pipeline() {
    let support::Gpu {
        device,
        queue: _queue,
    } = support::gpu().expect("a device");
    let built = pipelines::build_builtin(&device).unwrap_or_else(|e| panic!("{e}"));
    let blurs = built.names().filter(|n| n.starts_with("bloomBlur")).count();
    assert_eq!(built.sizes.bloom_levels, blurs);
}
