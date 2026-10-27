//! Shared environment bind-group construction.
//!
//! A shell owns the presentation record buffer and resolves its sky, PMREM,
//! DFG, cast-light, and sun-shadow resources. This owner consumes those real
//! resources in the exported `environment` layout used by lit model pipelines;
//! it never creates fallback textures or a second material graph.

use wgpu::*;

pub struct EnvironmentBindings {
    group: BindGroup,
}

impl EnvironmentBindings {
    pub fn new(
        device: &Device,
        layout: &BindGroupLayout,
        resources: &[BindingResource<'_>],
    ) -> Self {
        assert_eq!(
            resources.len(),
            9,
            "environment layout requires nine real resources"
        );
        Self {
            group: device.create_bind_group(&BindGroupDescriptor {
                label: Some("environment"),
                layout,
                entries: &resources
                    .iter()
                    .enumerate()
                    .map(|(binding, resource)| BindGroupEntry {
                        binding: binding as u32,
                        resource: resource.clone(),
                    })
                    .collect::<Vec<_>>(),
            }),
        }
    }

    pub fn group(&self) -> &BindGroup {
        &self.group
    }
}
