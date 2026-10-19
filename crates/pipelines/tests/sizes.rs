//! The sizes the export lists that a client allocates a struct array by are
//! the strides of the structs the shaders declare. A client sizing a buffer
//! by a stale stride leaves its tail unwritten and reads rows at the wrong
//! offsets.
use naga::TypeInner;
use std::fs;

/// The span (array stride) of the struct `name` in the exported `file`.
fn span(file: &str, name: &str) -> u32 {
    let source = fs::read_to_string(format!("{}/{file}", pipelines::DIR)).unwrap();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    let ty = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{file} declares no struct {name}"))
        .1;
    let TypeInner::Struct { span, .. } = ty.inner else {
        panic!("{name} is not a struct")
    };
    span
}

#[test]
fn a_grass_clump_is_allocated_by_the_shaders_stride() {
    let listing: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(format!("{}/pipelines.json", pipelines::DIR)).unwrap(),
    )
    .unwrap();
    let listed = listing["sizes"]["grassClump"].as_u64().unwrap() as u32;
    for file in ["grassBuild.wgsl", "grassDraw.wgsl"] {
        assert_eq!(listed, span(file, "GrassClump"), "{file} GrassClump stride");
    }
}
