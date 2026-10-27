use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = root.join("../../packages/battle-renderer/pipelines");
    println!("cargo:rerun-if-changed={}", dir.display());
    let manifest = fs::read_to_string(dir.join("pipelines.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let entries = value["pipelines"].as_array().unwrap();
    let pipeline_dir = dir.canonicalize().unwrap();
    let manifest_path = pipeline_dir.join("pipelines.json");
    let mut generated = format!(
        "pub const MANIFEST: &str = include_str!({:?});\n",
        manifest_path
    );
    generated.push_str(&format!("pub const COUNT: usize = {};\n", entries.len()));
    generated.push_str("pub fn source(name: &str) -> Option<&'static str> { match name {\n");
    for entry in entries {
        let name = entry["name"].as_str().unwrap();
        let shader = entry["shader"].as_str().unwrap();
        generated.push_str(&format!(
            "{name:?} => Some(include_str!({:?})),\n",
            pipeline_dir.join(shader).canonicalize().unwrap()
        ));
        generated.push_str(&format!(
            "{shader:?} => Some(include_str!({:?})),\n",
            pipeline_dir.join(shader).canonicalize().unwrap()
        ));
    }
    generated.push_str("_ => None, } }\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("pipeline_export.rs"),
        generated,
    )
    .unwrap();
}
