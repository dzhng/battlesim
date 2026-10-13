//! The uniforms Rust hands a pipeline are the structs its shader declares:
//! every field of a struct record sits at the offset of the exported WGSL's
//! member of the same name, and the record spans the struct. The struct
//! records' schemas are generated (`records.gen.ts`); the records carried
//! inside a struct the browser composes are checked here by hand. A renderer
//! uploads the record's bytes as they are.
use naga::{Module, TypeInner};
use presentation::records::{camel, Binding, Layout, Sun, View, LAYOUTS};
use std::fs;

fn module(file: &str) -> Module {
    let source = fs::read_to_string(format!("{}/{file}", pipelines::DIR)).unwrap();
    naga::front::wgsl::parse_str(&source).unwrap()
}

/// `(member, offset)` of the struct `name`, and its span.
fn members(module: &Module, name: &str) -> (Vec<(String, u32)>, u32) {
    let ty = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no struct {name}"))
        .1;
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("{name} is not a struct")
    };
    (
        members
            .iter()
            .map(|m| (m.name.clone().unwrap(), m.offset))
            .collect(),
        *span,
    )
}

/// The record's fields from the first named `prefix…` on, by WGSL name, at
/// offsets from it.
fn fields(layout: &Layout, prefix: &str) -> Vec<(String, u32)> {
    let base = layout
        .fields
        .iter()
        .find(|f| f.name.starts_with(prefix))
        .unwrap()
        .offset;
    layout
        .fields
        .iter()
        .filter(|f| f.name.starts_with(prefix) && !f.name.starts_with("pad"))
        .map(|f| (camel(&f.name[prefix.len()..]), (f.offset - base) as u32))
        .collect()
}

#[test]
fn every_struct_record_is_the_shaders_struct() {
    let modules: Vec<(String, Module)> = fs::read_dir(pipelines::DIR)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|f| f.ends_with(".wgsl"))
        .map(|f| (f.clone(), module(&f)))
        .collect();
    for layout in LAYOUTS.iter().filter(|l| l.binding == Binding::Struct) {
        let mut found = 0;
        for (file, module) in &modules {
            if !module
                .types
                .iter()
                .any(|(_, t)| t.name.as_deref() == Some(layout.name))
            {
                continue;
            }
            let (want, span) = members(module, layout.name);
            assert_eq!(fields(layout, ""), want, "{file} {}", layout.name);
            assert_eq!(layout.size as u32, span, "{file} {} size", layout.name);
            found += 1;
        }
        assert!(found > 0, "no shader declares {}", layout.name);
    }
}

#[test]
fn the_view_carries_the_sky_s_rays() {
    let (want, span) = members(&module("background.wgsl"), "Rays");
    assert_eq!(fields(&View::LAYOUT, "sky_"), want);
    assert_eq!(span, 48, "three vec3f, as the view's three rays");
}

#[test]
fn the_sun_is_the_environment_s_light_after_its_view() {
    let (members, _) = members(&module("worldOpaque.wgsl"), "Environment");
    let at = |name: &str| members.iter().find(|(n, _)| n == name).unwrap().1;
    let base = at("sunDirection");
    let want: Vec<(String, u32)> = ["sunDirection", "sunRadiance", "settings", "fill"]
        .iter()
        .map(|n| (n.to_string(), at(n) - base))
        .collect();
    assert_eq!(fields(&Sun::LAYOUT, ""), want);
    assert_eq!(
        at("aerial") - base,
        Sun::LAYOUT.size as u32,
        "the haze follows the sun"
    );
}
