//! Every built-in pipeline builds on this machine's device and retains the
//! size facts the shared catalog owns. Needs a GPU: run under the GPU lock.
use native_client::gpu;
use native_client::pipelines;

#[test]
fn every_builtin_pipeline_builds_on_the_device() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a device");
    let built = pipelines::build(&gpu.device).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        built.names().count(),
        renderer_core::pipelines::builtin_pipeline_count()
    );
}

#[test]
fn every_bloom_level_has_a_blur_pipeline() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a device");
    let built = pipelines::build(&gpu.device).unwrap_or_else(|e| panic!("{e}"));
    let blurs = built.names().filter(|n| n.starts_with("bloomBlur")).count();
    assert_eq!(built.sizes.bloom_levels, blurs);
}
