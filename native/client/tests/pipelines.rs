//! Every pipeline the browser exports builds on this machine's device, from
//! the export alone. Needs a GPU: run under the GPU lock.
use native_client::gpu;
use native_client::pipelines::Pipelines;

#[test]
fn every_exported_pipeline_builds_on_the_device() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a device");
    let built = Pipelines::build(&gpu.device, pipelines::DIR).unwrap_or_else(|e| panic!("{e}"));
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
