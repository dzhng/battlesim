//! Every pipeline the browser exports builds on this machine's device, from
//! the export alone, and the sizes it exports name the pipelines the passes
//! look up. Needs a GPU: run under the GPU lock.
use native_client::gpu;
use native_client::pipelines;

#[test]
fn every_exported_pipeline_builds_on_the_device() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a device");
    let built = pipelines::build(&gpu.device, pipelines::DIR).unwrap_or_else(|e| panic!("{e}"));
    let listed = std::fs::read_to_string(format!("{}/pipelines.json", pipelines::DIR)).unwrap();
    let listed: serde_json::Value = serde_json::from_str(&listed).unwrap();
    let mut built: Vec<&str> = built.names().collect();
    let mut listed: Vec<&str> = listed["pipelines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    built.sort();
    listed.sort();
    assert_eq!(built, listed);
}

#[test]
fn every_bloom_level_the_export_sizes_has_its_blur_pipeline() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a device");
    let built = pipelines::build(&gpu.device, pipelines::DIR).unwrap_or_else(|e| panic!("{e}"));
    let blurs = built.names().filter(|n| n.starts_with("bloomBlur")).count();
    assert!(blurs > 0, "the export has no bloom blur");
    assert_eq!(built.sizes.bloom_levels, blurs);
}
