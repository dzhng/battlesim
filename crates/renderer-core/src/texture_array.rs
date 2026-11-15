//! Append-only layer addresses and final-holder ownership, shared by art arrays.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::sync::{Arc, Weak};
use wgpu::*;

/// The planes of one allocation share dimensions, capacity and lifetime.
/// Model arrays have one plane; runtime cards have albedo and packed normals.
pub(crate) struct SharedArray(Vec<TextureView>);

impl SharedArray {
    pub fn view(&self, plane: usize) -> &TextureView {
        &self.0[plane]
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for view in &self.0 {
            memory.view(view);
        }
    }
}

impl Deref for SharedArray {
    type Target = TextureView;
    fn deref(&self) -> &TextureView {
        &self.0[0]
    }
}

impl Drop for SharedArray {
    fn drop(&mut self) {
        for view in &self.0 {
            view.texture().destroy();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArrayShape {
    pub size: [u32; 2],
    pub levels: u32,
    pub formats: Vec<TextureFormat>,
}

impl ArrayShape {
    fn matches(&self, array: &SharedArray) -> bool {
        self.formats.len() == array.0.len()
            && self.formats.iter().enumerate().all(|(plane, &format)| {
                let texture = array.view(plane).texture();
                self.size == [texture.width(), texture.height()]
                    && self.levels == texture.mip_level_count()
                    && format == texture.format()
            })
    }
}

struct Allocation<K> {
    owner: Weak<SharedArray>,
    written: BTreeSet<K>,
}

pub(crate) struct LayerArray<K> {
    /// Names the array's texture, and its owner in a tripwire report.
    label: &'static str,
    slots: BTreeMap<K, u32>,
    allocations: Vec<Allocation<K>>,
}

pub(crate) struct LayerPressure {
    pub active: usize,
    pub dead: usize,
    pub total: usize,
}

pub(crate) struct ArrayAdmission {
    pub owner: Arc<SharedArray>,
    pub copied_bytes: u64,
    pub rebuilt: bool,
}

impl<K: Clone + Ord> LayerArray<K> {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            slots: BTreeMap::new(),
            allocations: Vec::new(),
        }
    }
    pub fn pressure(&self, wanted: &BTreeSet<K>) -> LayerPressure {
        let dead = self.slots.keys().filter(|id| !wanted.contains(*id)).count();
        let added = wanted
            .iter()
            .filter(|id| !self.slots.contains_key(*id))
            .count();
        LayerPressure {
            active: wanted.len(),
            dead,
            total: self.slots.len() + added,
        }
    }
    pub fn reserve(&mut self, wanted: &BTreeSet<K>) {
        for id in wanted {
            let next = self.slots.len() as u32;
            self.slots.entry(id.clone()).or_insert(next);
        }
    }
    pub fn layer(&self, key: &K) -> u32 {
        self.slots[key]
    }
    pub fn written(&self, owner: &Arc<SharedArray>, key: &K) -> bool {
        let weak = Arc::downgrade(owner);
        self.allocations
            .iter()
            .find(|a| a.owner.ptr_eq(&weak))
            .is_some_and(|a| a.written.contains(key))
    }
    pub fn mark_written(&mut self, owner: &Arc<SharedArray>, key: K) {
        let weak = Arc::downgrade(owner);
        self.allocations
            .iter_mut()
            .find(|a| a.owner.ptr_eq(&weak))
            .expect("held array allocation")
            .written
            .insert(key);
    }
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for allocation in &self.allocations {
            if let Some(owner) = allocation.owner.upgrade() {
                owner.record_memory(memory);
            }
        }
    }

    pub fn admit(
        &mut self,
        device: &Device,
        queue: &Queue,
        wanted: &BTreeSet<K>,
        shape: &ArrayShape,
        spare: u32,
        report: fn(&str),
    ) -> ArrayAdmission {
        let label = self.label;
        self.reserve(wanted);
        self.allocations.retain(|a| a.owner.strong_count() > 0);
        // Reservations absent from this table need no backing address in this
        // allocation. They stay reserved for future admissions without forcing
        // a third array just because another pending generation names them.
        let needed = wanted
            .iter()
            .map(|key| self.slots[key] + 1)
            .max()
            .unwrap_or(1);
        // Shape affects filtered pixels. Prefer a surviving exact-shape array
        // over a differently sized pending version.
        let matching = self
            .allocations
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| {
                allocation.owner.upgrade().map(|owner| (index, owner))
            })
            .filter(|(_, owner)| shape.matches(owner))
            .max_by_key(|(_, owner)| owner.texture().depth_or_array_layers())
            .map(|(index, _)| index);
        if let Some(index) = matching {
            let last = self.allocations.len() - 1;
            self.allocations.swap(index, last);
        }
        let previous = self.allocations.last().and_then(|a| a.owner.upgrade());
        if let Some(previous) = &previous {
            if shape.matches(previous) && previous.texture().depth_or_array_layers() >= needed {
                return ArrayAdmission {
                    owner: previous.clone(),
                    copied_bytes: 0,
                    rebuilt: false,
                };
            }
        }
        let capacity = if wanted.is_empty() {
            needed
        } else {
            needed
                .saturating_add(spare)
                .min(device.limits().max_texture_array_layers)
        };
        let owner = Arc::new(SharedArray(
            shape
                .formats
                .iter()
                .map(|&format| {
                    device
                        .create_texture(&TextureDescriptor {
                            label: Some(label),
                            size: Extent3d {
                                width: shape.size[0],
                                height: shape.size[1],
                                depth_or_array_layers: capacity,
                            },
                            mip_level_count: shape.levels,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format,
                            usage: TextureUsages::TEXTURE_BINDING
                                | TextureUsages::COPY_DST
                                | TextureUsages::COPY_SRC,
                            view_formats: &[],
                        })
                        .create_view(&TextureViewDescriptor {
                            dimension: Some(TextureViewDimension::D2Array),
                            ..Default::default()
                        })
                })
                .collect(),
        ));
        let mut written = BTreeSet::new();
        let mut copied_bytes = 0;
        let mut rebuilt = false;
        if let Some(previous) = &previous {
            if shape.matches(previous) {
                let mut encoder = device.create_command_encoder(&Default::default());
                for key in &self
                    .allocations
                    .last()
                    .expect("previous allocation")
                    .written
                {
                    let layer = self.slots[key];
                    for plane in 0..shape.formats.len() {
                        for level in 0..shape.levels {
                            let extent = Extent3d {
                                width: (shape.size[0] >> level).max(1),
                                height: (shape.size[1] >> level).max(1),
                                depth_or_array_layers: 1,
                            };
                            let info = |texture| TexelCopyTextureInfo {
                                texture,
                                mip_level: level,
                                origin: Origin3d {
                                    x: 0,
                                    y: 0,
                                    z: layer,
                                },
                                aspect: TextureAspect::All,
                            };
                            encoder.copy_texture_to_texture(
                                info(previous.view(plane).texture()),
                                info(owner.view(plane).texture()),
                                extent,
                            );
                            copied_bytes += u64::from(extent.width) * u64::from(extent.height) * 4;
                        }
                    }
                    written.insert(key.clone());
                }
                queue.submit([encoder.finish()]);
                report(&format!("art-store tripwire kind=TextureCapacity value={needed} bound={} owner=Runtime.art_store.{label} outcome=gpu-copy capacity={capacity}", previous.texture().depth_or_array_layers()));
            } else {
                rebuilt = true;
                report(&format!("art-store tripwire kind=TextureShape value={:?} bound={:?} owner=Runtime.art_store.{label} outcome=array-rebuild", shape.size, [previous.texture().width(), previous.texture().height()]));
            }
        }
        self.allocations.push(Allocation {
            owner: Arc::downgrade(&owner),
            written,
        });
        ArrayAdmission {
            owner,
            copied_bytes,
            rebuilt,
        }
    }
}
