//! Every exported shader is one `wgpu` compiles: naga parses and validates it
//! and writes it as Metal Shading Language, as `wgpu`'s Metal backend does,
//! and each pipeline's entry points are in its module at the right stage.

use naga::back::msl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use serde_json::Value;
use std::fs;

#[test]
fn every_exported_pipeline_compiles_for_wgpu() {
    let listing: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}/pipelines.json", pipelines::DIR)).unwrap(),
    )
    .unwrap();
    let pipelines = listing["pipelines"].as_array().unwrap();
    assert!(!pipelines.is_empty());
    let mut failures = Vec::new();
    for pipeline in pipelines {
        let name = pipeline["name"].as_str().unwrap();
        let source = fs::read_to_string(format!(
            "{}/{}",
            pipelines::DIR,
            pipeline["shader"].as_str().unwrap()
        ))
        .unwrap();
        if let Err(error) = compile(pipeline, &source) {
            failures.push(format!("{name}: {error}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} pipelines fail:\n{}",
        failures.len(),
        pipelines.len(),
        failures.join("\n")
    );
}

fn compile(pipeline: &Value, source: &str) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    // WebGPU's `pack2x16float` and `unpack2x16float` are core: `wgpu` grants
    // this capability on every WebGPU-compliant adapter (Metal, D3D12, Vulkan).
    let capabilities = Capabilities::default() | Capabilities::SHADER_FLOAT16_IN_FLOAT32;
    let info = Validator::new(ValidationFlags::all(), capabilities)
        .validate(&module)
        .map_err(|e| e.emit_to_string(source))?;
    for (stage, shader_stage) in [
        ("vertex", naga::ShaderStage::Vertex),
        ("fragment", naga::ShaderStage::Fragment),
        ("compute", naga::ShaderStage::Compute),
    ] {
        let Some(entry) = pipeline.get(stage) else {
            continue;
        };
        let found = module
            .entry_points
            .iter()
            .filter(|e| e.stage == shader_stage);
        match entry["entryPoint"].as_str() {
            Some(point) if !found.clone().any(|e| e.name == point) => {
                return Err(format!("no {stage} entry point {point}"))
            }
            None if found.count() != 1 => return Err(format!("no single {stage} entry point")),
            _ => {}
        }
    }
    let options = msl::Options {
        lang_version: (2, 4),
        ..Default::default()
    };
    msl::write_string(&module, &info, &options, &msl::PipelineOptions::default())
        .map_err(|e| format!("Metal: {e}"))?;
    Ok(())
}
