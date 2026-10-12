//! The uniforms Rust hands a pipeline are the structs its shader declares:
//! every field of the presentation's light records sits at the offset of the
//! exported WGSL's member of the same name, and the records span the struct.
//! A renderer uploads the record's bytes as they are.
use naga::{Module, TypeInner};
use presentation::records::{Aerial, Bloom, Grade, Layout, SkyLight, Sun, View};
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

fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in snake.chars() {
        match c {
            '_' => upper = true,
            c if upper => {
                out.push(c.to_ascii_uppercase());
                upper = false;
            }
            c => out.push(c),
        }
    }
    out
}

/// The record's fields from `first` on, by WGSL name, at offsets from it.
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
fn the_light_records_are_the_shaders_structs() {
    for (file, name, layout) in [
        ("bake.wgsl", "SkyLight", &SkyLight::LAYOUT),
        ("background.wgsl", "SkyLight", &SkyLight::LAYOUT),
        ("bloomHighpass.wgsl", "Bloom", &Bloom::LAYOUT),
        ("bloomComposite.wgsl", "Bloom", &Bloom::LAYOUT),
        ("postLook.wgsl", "Grade", &Grade::LAYOUT),
        ("worldOpaque.wgsl", "Aerial", &Aerial::LAYOUT),
        ("effects.wgsl", "Aerial", &Aerial::LAYOUT),
    ] {
        let (want, span) = members(&module(file), name);
        assert_eq!(fields(layout, ""), want, "{file} {name}");
        assert_eq!(layout.size as u32, span, "{file} {name} size");
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
