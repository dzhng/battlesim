use super::*;
use naga::{AddressSpace, ImageClass, ImageDimension, ScalarKind, StorageAccess, TypeInner};

fn binding_matches(module: &naga::Module, global: &naga::GlobalVariable, ty: BindingType) -> bool {
    match (global.space, &module.types[global.ty].inner, ty) {
        (AddressSpace::Uniform, _, BindingType::Buffer { ty, .. }) => {
            ty == BufferBindingType::Uniform
        }
        (AddressSpace::Storage { access }, _, BindingType::Buffer { ty, .. }) => {
            ty == BufferBindingType::Storage {
                read_only: !access.contains(StorageAccess::STORE),
            }
        }
        (AddressSpace::Handle, TypeInner::Sampler { comparison }, BindingType::Sampler(ty)) => {
            *comparison == (ty == SamplerBindingType::Comparison)
        }
        (
            AddressSpace::Handle,
            TypeInner::Image {
                dim,
                arrayed,
                class,
            },
            BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled,
            },
        ) => {
            let dimension = match (dim, arrayed) {
                (ImageDimension::D1, false) => TextureViewDimension::D1,
                (ImageDimension::D2, false) => TextureViewDimension::D2,
                (ImageDimension::D2, true) => TextureViewDimension::D2Array,
                (ImageDimension::D3, false) => TextureViewDimension::D3,
                (ImageDimension::Cube, false) => TextureViewDimension::Cube,
                (ImageDimension::Cube, true) => TextureViewDimension::CubeArray,
                _ => return false,
            };
            let sample_matches = match (class, sample_type) {
                (ImageClass::Sampled { kind, multi }, sample_type) => {
                    *multi == multisampled
                        && matches!(
                            (kind, sample_type),
                            (ScalarKind::Float, TextureSampleType::Float { .. })
                                | (ScalarKind::Sint, TextureSampleType::Sint)
                                | (ScalarKind::Uint, TextureSampleType::Uint)
                        )
                }
                (ImageClass::Depth { multi }, TextureSampleType::Depth) => *multi == multisampled,
                _ => false,
            };
            dimension == view_dimension && sample_matches
        }
        _ => false,
    }
}

#[test]
fn shader_bindings_match_registry_layouts() {
    let layouts = layouts();
    for pipeline in pipelines() {
        let shaders: &[&str] = match pipeline.name {
            "modelPrepassCutout" => &["modelPrepassCutout.wgsl", "modelPrepassCutoutDiscard.wgsl"],
            "modelCasterCutout" => &["modelCasterCutout.wgsl", "modelCasterCutoutDiscard.wgsl"],
            _ => std::slice::from_ref(&pipeline.shader),
        };
        for shader in shaders {
            let source = crate::pipelines::builtin_source(shader).unwrap();
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|error| panic!("{} ({shader}): {error}", pipeline.name));
            for (_, global) in module.global_variables.iter() {
                let Some(binding) = &global.binding else {
                    continue;
                };
                let context = format!(
                    "{} ({}) global {:?} at group {} binding {}",
                    pipeline.name, shader, global.name, binding.group, binding.binding
                );
                let layout_name = pipeline
                    .layouts
                    .get(binding.group as usize)
                    .unwrap_or_else(|| panic!("{context}: missing registry group"));
                let layout = layouts
                    .iter()
                    .find(|layout| layout.name == *layout_name)
                    .unwrap_or_else(|| panic!("{context}: missing layout {layout_name}"));
                let entry = layout
                    .entries
                    .iter()
                    .find(|entry| entry.binding == binding.binding)
                    .unwrap_or_else(|| panic!("{context}: missing binding in {layout_name}"));
                assert!(
                    binding_matches(&module, global, entry.ty),
                    "{context}: shader {:?} {:?} does not match {layout_name} {:?}",
                    global.space,
                    module.types[global.ty].inner,
                    entry.ty,
                );
            }
        }
    }
}
