use super::*;

pub(super) struct BrowserEffects {
    resources: renderer_core::effects::EffectsResources,
    atlas_view: TextureView,
    sampler: Sampler,
}

impl BrowserEffects {
    pub(super) fn new(
        device: &Device,
        targets: &TargetOwner,
        pipelines: &Pipelines,
        camera: &Buffer,
        environment: &Buffer,
        pmrem: &TextureView,
        cast_lights: &Buffer,
    ) -> Self {
        let atlas = device.create_texture(&TextureDescriptor {
            label: Some("shared-effect-atlas"),
            size: Extent3d {
                width: pipelines.sizes.effect_sheet,
                height: pipelines.sizes.effect_sheet,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("shared-effect-sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });
        let resources = renderer_core::effects::EffectsResources::new(
            device,
            pipelines,
            &[
                BindingResource::Buffer(camera.as_entire_buffer_binding()),
                BindingResource::TextureView(&targets.depth),
                BindingResource::TextureView(&atlas_view),
                BindingResource::Sampler(&sampler),
                BindingResource::Buffer(environment.as_entire_buffer_binding()),
                BindingResource::TextureView(pmrem),
                BindingResource::Buffer(cast_lights.as_entire_buffer_binding()),
            ],
            64,
            64 * 1024,
        );
        Self {
            resources,
            atlas_view,
            sampler,
        }
    }

    pub(super) fn rebuild(
        &mut self,
        device: &Device,
        queue: &Queue,
        targets: &TargetOwner,
        pipelines: &Pipelines,
        camera: &Buffer,
        environment: &Buffer,
        pmrem: &TextureView,
        cast_lights: &Buffer,
        effects: &[f32],
    ) {
        self.resources = renderer_core::effects::EffectsResources::new(
            device,
            pipelines,
            &[
                BindingResource::Buffer(camera.as_entire_buffer_binding()),
                BindingResource::TextureView(&targets.depth),
                BindingResource::TextureView(&self.atlas_view),
                BindingResource::Sampler(&self.sampler),
                BindingResource::Buffer(environment.as_entire_buffer_binding()),
                BindingResource::TextureView(pmrem),
                BindingResource::Buffer(cast_lights.as_entire_buffer_binding()),
            ],
            64,
            64 * 1024,
        );
        self.prepare(device, queue, effects);
    }

    pub(super) fn prepare(&mut self, device: &Device, queue: &Queue, effects: &[f32]) {
        self.resources
            .prepare(device, queue, bytemuck::cast_slice(effects));
    }

    pub(super) fn encode(&self, encoder: &mut CommandEncoder, targets: &TargetOwner) {
        self.resources.encode(encoder, targets);
    }
}
