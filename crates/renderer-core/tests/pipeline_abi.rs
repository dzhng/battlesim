use naga::ShaderStage;
use naga::{front::wgsl, TypeInner};
use presentation::records::{camel, Binding, Layout, Sun, View, LAYOUTS};

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

fn fields(layout: &presentation::records::Layout, prefix: &str) -> Vec<(String, u32)> {
    let base = layout
        .fields
        .iter()
        .find(|field| field.name.starts_with(prefix))
        .unwrap()
        .offset;
    layout
        .fields
        .iter()
        .filter(|field| field.name.starts_with(prefix) && !field.name.starts_with("pad"))
        .map(|field| {
            (
                camel(&field.name[prefix.len()..]),
                (field.offset - base) as u32,
            )
        })
        .collect()
}

#[test]
fn builtin_shaders_match_presentation_record_layouts() {
    for layout in LAYOUTS
        .iter()
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
fn sky_rays_and_environment_light_keep_nested_uniform_offsets() {
    let (rays, span) = members("background.wgsl", "Rays");
    assert_eq!(fields(&View::LAYOUT, "sky_"), rays);
    assert_eq!(span, 48);

    let (environment, _) = members("worldOpaque.wgsl", "Environment");
    let at = |name: &str| {
        environment
            .iter()
            .find(|(field, _)| field == name)
            .unwrap()
            .1
    };
    let base = at("sunDirection");
    let expected = ["sunDirection", "sunRadiance", "settings", "fill"]
        .into_iter()
        .map(|name| (name.to_owned(), at(name) - base))
        .collect::<Vec<_>>();
    assert_eq!(fields(&Sun::LAYOUT, ""), expected);
    assert_eq!(at("aerial") - base, Sun::LAYOUT.size as u32);
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
