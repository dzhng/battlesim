use naga::ShaderStage;
use naga::{front::wgsl, TypeInner};
use presentation::records::{camel, Binding, Layout, LAYOUTS};

fn assert_layout(
    module: &naga::Module,
    handle: naga::Handle<naga::Type>,
    layout: &Layout,
    path: &str,
) {
    let TypeInner::Struct { members, span } = &module.types[handle].inner else {
        panic!("{path} is not a struct")
    };
    let fields: Vec<_> = layout
        .fields
        .iter()
        .filter(|field| !field.name.starts_with("pad"))
        .collect();
    let expected: Vec<_> = fields
        .iter()
        .map(|field| (camel(field.name), field.offset as u32))
        .collect();
    let members: Vec<_> = members
        .iter()
        .filter(|member| !member.name.as_deref().unwrap().starts_with("pad"))
        .collect();
    let actual: Vec<_> = members
        .iter()
        .map(|member| (member.name.clone().unwrap(), member.offset))
        .collect();
    assert_eq!(actual, expected, "{path}");
    assert_eq!(*span as usize, layout.size, "{path} size");
    for (member, field) in members.iter().zip(fields) {
        let path = format!("{path}.{}", field.name);
        let nested = match module.types[member.ty].inner {
            TypeInner::Array { base, size, .. } => {
                let naga::ArraySize::Constant(count) = size else {
                    panic!("{path} must have a constant array count")
                };
                assert_eq!(count.get() as usize, field.count, "{path} array count");
                base
            }
            _ => {
                if field.record.is_some() {
                    assert_eq!(field.count, 1, "{path} record count");
                }
                member.ty
            }
        };
        if let Some(record) = field.record {
            assert_layout(module, nested, record, &path);
        }
    }
}

fn module(shader: &str) -> naga::Module {
    wgsl::parse_str(&renderer_core::pipelines::builtin_source(shader).unwrap())
        .unwrap_or_else(|error| panic!("{shader}: {error}"))
}

fn members(shader: &str, name: &str) -> (Vec<(String, u32)>, u32) {
    let module = module(shader);
    let ty = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{shader} declares no struct {name}"))
        .1;
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("{name} is not a struct")
    };
    (
        members
            .iter()
            .map(|member| (member.name.clone().unwrap(), member.offset))
            .collect(),
        *span,
    )
}

#[test]
fn builtin_shaders_match_record_layouts() {
    for layout in LAYOUTS
        .iter()
        .chain(renderer_core::records::LAYOUTS)
        .filter(|layout| layout.binding == Binding::Struct)
    {
        let mut found = false;
        for shader in renderer_core::pipelines::builtin_shaders() {
            let source = renderer_core::pipelines::builtin_source(shader).unwrap();
            let module =
                wgsl::parse_str(&source).unwrap_or_else(|error| panic!("{shader}: {error}"));
            let Some((handle, _)) = module
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(layout.name))
            else {
                continue;
            };
            assert_layout(
                &module,
                handle,
                layout,
                &format!("{shader}: {}", layout.name),
            );
            found = true;
        }
        assert!(found, "no built-in shader declares {}", layout.name);
    }
}

#[test]
fn grass_stride_matches_the_builtin_shader_sources() {
    let expected = renderer_core::pipelines::builtin_sizes().grass_clump as u32;
    for shader in ["grassBuild.wgsl", "grassDraw.wgsl"] {
        let (_, span) = members(shader, "GrassClump");
        assert_eq!(span, expected, "{shader} GrassClump stride");
    }
}

#[test]
fn every_builtin_shader_naga_validates_and_has_declared_entries() {
    for shader in renderer_core::pipelines::builtin_shaders() {
        let module = module(shader);
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default()
                | naga::valid::Capabilities::SHADER_FLOAT16_IN_FLOAT32,
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{shader}: {error}"));
        assert!(
            !module.entry_points.is_empty(),
            "{shader} has no entry point"
        );
        for entry in &module.entry_points {
            assert!(
                matches!(
                    entry.stage,
                    ShaderStage::Vertex | ShaderStage::Fragment | ShaderStage::Compute
                ),
                "{shader}: unsupported entry stage {:?}",
                entry.stage
            );
        }
    }
}

#[test]
fn every_buffer_struct_has_a_checked_layout() {
    fn visit(
        module: &naga::Module,
        ty: naga::Handle<naga::Type>,
        names: &mut std::collections::BTreeSet<String>,
    ) {
        match &module.types[ty].inner {
            TypeInner::Array { base, .. } => visit(module, *base, names),
            TypeInner::Struct { members, .. } => {
                names.insert(
                    module.types[ty]
                        .name
                        .clone()
                        .expect("buffer struct has a name"),
                );
                for member in members {
                    visit(module, member.ty, names);
                }
            }
            _ => {}
        }
    }
    let mut unchecked = std::collections::BTreeSet::new();
    for shader in renderer_core::pipelines::builtin_shaders() {
        let module = module(shader);
        let mut names = std::collections::BTreeSet::new();
        for (_, global) in module.global_variables.iter() {
            if matches!(
                global.space,
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. }
            ) {
                visit(&module, global.ty, &mut names);
            }
        }
        for name in names {
            let reflected = LAYOUTS
                .iter()
                .chain(renderer_core::records::LAYOUTS)
                .any(|layout| layout.binding == Binding::Struct && layout.name == name);
            let allowed = BUFFER_LAYOUT_EXCEPTIONS
                .iter()
                .any(|(allowed, _)| *allowed == name);
            if !reflected && !allowed {
                unchecked.insert(name);
            }
        }
    }
    assert!(
        unchecked.is_empty(),
        "buffer structs without checked layouts: {unchecked:?}"
    );
}

// These formats have owners outside the reflected frame-upload records.
const BUFFER_LAYOUT_EXCEPTIONS: &[(&str, &str)] = &[
    ("Count", "GPU atomic counters, cleared and read as words by probes::grass"),
    ("GrassClump", "GPU-generated storage record; grass_stride_matches_the_builtin_shader_sources checks its allocation stride"),
    ("FogMaskView", "single u32 diagnostic enum, uploaded as a word by fog_mask"),
];
