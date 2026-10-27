//! Shared directional-shadow resource ownership.
//!
//! The native and browser shells fit cascades in the presentation layer and
//! draw different caster resources, but the GPU resource contract is one:
//! one depth-array layer per cascade, one receiver uniform, one camera buffer
//! per cascade, and a greater-equal comparison sampler. Keeping this owner in
//! `renderer-core` prevents the native shell from becoming a second resource
//! implementation while leaving pass-specific caster routing to each shell.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::{bind, buffer};

pub const CASCADES: usize = 4;

pub struct ShadowResources {
    layers: Vec<TextureView>,
    /// The complete depth array bound by lit receivers.
    pub depth: TextureView,
    pub receiver: Buffer,
    pub compare: Sampler,
    cameras: Vec<Buffer>,
    groups: Vec<BindGroup>,
    active: usize,
}

impl ShadowResources {
    /// Allocate the shared shadow target and receiver/caster bindings.
    /// `receiver_bytes` and `camera_bytes` are presentation record bytes; the
    /// core deliberately does not depend on the presentation crate.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        map_size: u32,
        receiver_bytes: &[u8],
        camera_bytes: usize,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("sun-depth"),
            size: Extent3d {
                width: map_size,
                height: map_size,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let layers = (0..CASCADES as u32)
            .map(|layer| {
                texture.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let depth = texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let receiver = buffer(device, "sun-shadow", uniform, receiver_bytes);
        let cameras: Vec<Buffer> = (0..CASCADES)
            .map(|_| buffer(device, "sun-caster", uniform, &vec![0; camera_bytes]))
            .collect();
        let groups = cameras
            .iter()
            .map(|camera| {
                bind(
                    device,
                    pipelines.layout("typegpuCameraLayout"),
                    "sun-caster",
                    &[camera.as_entire_binding()],
                )
            })
            .collect();
        let compare = device.create_sampler(&SamplerDescriptor {
            label: Some("sun-compare"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            compare: Some(CompareFunction::GreaterEqual),
            ..Default::default()
        });
        Self {
            layers,
            depth,
            receiver,
            compare,
            cameras,
            groups,
            active: 0,
        }
    }

    pub fn prepare(&mut self, queue: &Queue, receiver_bytes: &[u8], cameras: &[&[u8]]) {
        queue.write_buffer(&self.receiver, 0, receiver_bytes);
        for (buffer, bytes) in self.cameras.iter().zip(cameras.iter().copied()) {
            queue.write_buffer(buffer, 0, bytes);
        }
        self.active = cameras.len().min(CASCADES);
    }

    pub fn layers(&self) -> &[TextureView] {
        &self.layers
    }
    pub fn groups(&self) -> &[BindGroup] {
        &self.groups
    }
    pub fn active(&self) -> usize {
        self.active
    }
}
