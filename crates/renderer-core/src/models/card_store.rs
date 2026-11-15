//! Card content keys, immutable layer writes and small frame metadata.
use super::store::GeometryKey;
use crate::impostor::{Atlas, CardFrame, Spec};
use crate::runtime::AdmissionWork;
use crate::texture_array::{ArrayShape, LayerArray, LayerPressure, SharedArray};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use wgpu::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CardKey {
    geometry: GeometryKey,
    palette: Vec<u8>,
    spec: Vec<u8>,
    texture_shapes: [[u32; 3]; 2],
}

impl CardKey {
    pub fn new(
        geometry: &GeometryKey,
        palette: &[presentation::records::Joint],
        spec: &Spec,
        texture_shapes: [[u32; 3]; 2],
    ) -> Self {
        Self {
            geometry: geometry.clone(),
            palette: bytemuck::cast_slice(palette).to_vec(),
            spec: serde_json::to_vec(spec).expect("serializable bake spec"),
            texture_shapes,
        }
    }
}

pub(super) struct CardStore {
    layers: LayerArray<CardKey>,
    frames: BTreeMap<CardKey, CardFrame>,
}

impl Default for CardStore {
    fn default() -> Self {
        Self {
            layers: LayerArray::new("cards"),
            frames: BTreeMap::new(),
        }
    }
}

impl CardStore {
    pub fn pressure(&self, wanted: &BTreeSet<CardKey>) -> LayerPressure {
        self.layers.pressure(wanted)
    }
    pub fn reserve(&mut self, wanted: &BTreeSet<CardKey>) {
        self.layers.reserve(wanted);
    }
    pub fn admit(
        &mut self,
        device: &Device,
        queue: &Queue,
        wanted: &BTreeSet<CardKey>,
        spec: &Spec,
        policy: super::ArtStorePolicy,
        work: &mut AdmissionWork,
    ) -> Arc<SharedArray> {
        let shape = ArrayShape {
            size: [spec.yaws * spec.cell, spec.pitches.len() as u32 * spec.cell],
            levels: 1,
            formats: vec![TextureFormat::Rgba8Unorm; 2],
        };
        let admitted = self.layers.admit(
            device,
            queue,
            wanted,
            &shape,
            policy.texture_spare_layers,
            policy.report,
        );
        work.card_bytes_copied += admitted.copied_bytes;
        admitted.owner
    }
    pub fn written(&self, array: &Arc<SharedArray>, key: &CardKey) -> bool {
        self.layers.written(array, key)
    }
    pub fn layer(&self, key: &CardKey) -> u32 {
        self.layers.layer(key)
    }
    pub fn frames(&self, wanted: &BTreeSet<CardKey>) -> BTreeMap<u32, CardFrame> {
        wanted
            .iter()
            .map(|key| (self.layer(key), self.frames[key].clone()))
            .collect()
    }
    pub fn complete(
        &mut self,
        queue: &Queue,
        array: &Arc<SharedArray>,
        key: &CardKey,
        atlas: &Atlas,
        work: &mut AdmissionWork,
    ) {
        if self.written(array, key) {
            return;
        }
        let layer = self.layer(key);
        let mut packed = atlas.normal.clone();
        for (normal, mask) in packed
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(atlas.mask.as_chunks::<4>().0.iter())
        {
            normal[3] = mask[0];
        }
        for (plane, bytes) in [&atlas.albedo, &packed].into_iter().enumerate() {
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: array.view(plane).texture(),
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: TextureAspect::All,
                },
                bytes,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(atlas.width * 4),
                    rows_per_image: Some(atlas.height),
                },
                Extent3d {
                    width: atlas.width,
                    height: atlas.height,
                    depth_or_array_layers: 1,
                },
            );
            work.card_bytes_uploaded += bytes.len() as u64;
        }
        self.frames.insert(
            key.clone(),
            CardFrame {
                center: atlas.center,
                radius: atlas.radius,
            },
        );
        self.layers.mark_written(array, key.clone());
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        self.layers.record_memory(memory);
    }
}
