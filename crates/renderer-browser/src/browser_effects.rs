use super::*;

pub(super) struct BrowserEffects {
    resources: renderer_core::effects::EffectsResources,
    atlas: Texture,
    atlas_view: TextureView,
    sampler: Sampler,
    layers: Vec<Vec<u8>>,
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
                depth_or_array_layers: 2,
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
            atlas,
            atlas_view,
            sampler,
            layers: vec![Vec::new(), Vec::new()],
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

    pub(super) fn upload_layer(
        &mut self,
        queue: &Queue,
        layer: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        if layer >= 2 || rgba.len() != 1024 * 1024 * 4 {
            return Err("effect atlas layer must be 1024² RGBA8".into());
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.atlas,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: TextureAspect::All,
            },
            rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024 * 4),
                rows_per_image: Some(1024),
            },
            Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
        );
        self.layers[layer as usize] = rgba.to_vec();
        Ok(())
    }

    pub(super) fn layers(&self) -> &[Vec<u8>] {
        &self.layers
    }

    pub(super) fn prepare(&mut self, device: &Device, queue: &Queue, effects: &[f32]) {
        self.resources
            .prepare(device, queue, bytemuck::cast_slice(effects));
    }

    pub(super) fn encode(&self, encoder: &mut CommandEncoder, targets: &TargetOwner) {
        self.resources.encode(encoder, targets);
    }
}
