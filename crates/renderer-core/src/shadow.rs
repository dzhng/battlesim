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
/// `presentation::records::Camera`'s packed size.  The core intentionally
/// keeps the record boundary byte based so it stays independent of the
/// presentation crate, while shells can use the generated record constants.
pub const CAMERA_BYTES: usize = 160;
/// `presentation::records::SunShadow`'s packed size.
pub const RECEIVER_BYTES: usize = 400;

/// The per-cascade caster admission derived from presentation batch records.
///
/// Presentation owns culling and writes one `cascade_mask` on every caster
/// batch.  The renderer must preserve that decision for every shadow layer;
/// this small value object lets native and browser shells consume the same
/// routing contract without importing presentation types into the core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CasterRouting {
    batches: [u32; CASCADES],
}

impl CasterRouting {
    /// Read packed `Batch` words (`stride` words per record).  The offsets are
    /// supplied by the shell because the core does not own presentation's
    /// generated schema.  `pass` identifies `CAST_PASS`; `caster` rejects
    /// view-only batches, and `cascade_mask` admits a batch to each layer.
    pub fn from_batch_words(
        words: &[u32],
        stride: usize,
        pass_offset: usize,
        caster_offset: usize,
        cascade_mask_offset: usize,
        pass: u32,
        none: u32,
    ) -> Self {
        let mut routing = Self::default();
        if stride == 0 {
            return routing;
        }
        for batch in words.chunks_exact(stride) {
            let (Some(&batch_pass), Some(&caster), Some(&mask)) = (
                batch.get(pass_offset),
                batch.get(caster_offset),
                batch.get(cascade_mask_offset),
            ) else {
                continue;
            };
            if batch_pass != pass || caster == none {
                continue;
            }
            for cascade in 0..CASCADES {
                if mask & (1 << cascade) != 0 {
                    routing.batches[cascade] = routing.batches[cascade].saturating_add(1);
                }
            }
        }
        routing
    }

    /// Number of admitted caster batches for one cascade.
    pub fn batches(&self, cascade: usize) -> u32 {
        self.batches.get(cascade).copied().unwrap_or(0)
    }

    /// All four cascade counts, for telemetry and shell scheduling.
    pub fn counts(&self) -> [u32; CASCADES] {
        self.batches
    }
}

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

#[cfg(test)]
mod tests {
    use super::{CasterRouting, CASCADES};

    #[test]
    fn routes_only_cast_batches_to_their_admitted_cascades() {
        // Batch words in the presentation layout are 12 words wide:
        // pass=3, caster=4, cascade_mask=9.
        let words = [
            0,
            0,
            0,
            1,
            2,
            0,
            0,
            0,
            0,
            0b0101,
            0,
            0, // casts 0 and 2
            0,
            0,
            0,
            1,
            u32::MAX,
            0,
            0,
            0,
            0,
            0b1111,
            0,
            0, // no caster
            0,
            0,
            0,
            0,
            2,
            0,
            0,
            0,
            0,
            0b1111,
            0,
            0, // view pass
            0,
            0,
            0,
            1,
            3,
            0,
            0,
            0,
            0,
            0b0010,
            0,
            0, // casts 1
        ];
        let routing = CasterRouting::from_batch_words(&words, 12, 3, 4, 9, 1, u32::MAX);
        assert_eq!(routing.counts(), [1, 1, 1, 0]);
        assert_eq!(routing.batches(CASCADES), 0);
    }

    #[test]
    fn malformed_stride_and_offsets_are_ignored() {
        let routing = CasterRouting::from_batch_words(&[1, 2, 3], 0, 0, 1, 2, 1, u32::MAX);
        assert_eq!(routing, CasterRouting::default());
        let routing = CasterRouting::from_batch_words(&[1, 2, 3], 4, 3, 4, 9, 1, u32::MAX);
        assert_eq!(routing, CasterRouting::default());
    }
}
