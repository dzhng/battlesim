//! Stable content-addressed layers and final-holder ownership of model arrays.
use crate::bundle::Texture as SourceTexture;
use crate::runtime::{AdmissionWork, RebuildReason};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::sync::{Arc, Weak};
use wgpu::*;

pub(super) struct SharedTexture(TextureView);

impl Deref for SharedTexture {
    type Target = TextureView;
    fn deref(&self) -> &TextureView {
        &self.0
    }
}

impl Drop for SharedTexture {
    fn drop(&mut self) {
        self.0.texture().destroy();
    }
}

struct Allocation {
    owner: Weak<SharedTexture>,
    written: BTreeSet<String>,
}

#[derive(Default)]
struct LayerArray {
    slots: BTreeMap<String, u32>,
    allocations: Vec<Allocation>,
}

/// The installed textures, one layer each in two arrays:
/// albedo (sRGB), and surface (normal and ORM, linear).
#[derive(Default)]
pub(super) struct TextureStore {
    albedo: LayerArray,
    surface: LayerArray,
}

impl TextureStore {
    pub fn pressure(
        &self,
        wanted: &BTreeMap<String, Arc<SourceTexture>>,
        limit: u32,
        dead_bound: usize,
    ) -> Result<Option<(RebuildReason, usize, usize)>, String> {
        let mut dead = 0;
        let mut exhausted = None;
        for (srgb, array) in [(true, &self.albedo), (false, &self.surface)] {
            let active = wanted.values().filter(|t| t.srgb == srgb).count();
            if active > limit as usize {
                return Err(format!(
                    "model texture layers exhausted: {active} wanted, limit {limit}"
                ));
            }
            dead += array
                .slots
                .keys()
                .filter(|id| !wanted.contains_key(*id))
                .count();
            let added = wanted
                .values()
                .filter(|t| t.srgb == srgb && !array.slots.contains_key(&t.id))
                .count();
            let total = array.slots.len() + added;
            if total > limit as usize {
                exhausted = Some((RebuildReason::TextureLayersExhausted, total, limit as usize));
            }
        }
        Ok(if dead > dead_bound {
            Some((RebuildReason::DeadTextureLayers, dead, dead_bound))
        } else {
            exhausted
        })
    }

    pub fn admit(
        &mut self,
        device: &Device,
        queue: &Queue,
        wanted: &BTreeMap<String, Arc<SourceTexture>>,
        spare: u32,
        report: fn(&str),
        work: &mut AdmissionWork,
    ) -> [Arc<SharedTexture>; 2] {
        [
            self.albedo
                .admit(device, queue, wanted, true, spare, report, work),
            self.surface
                .admit(device, queue, wanted, false, spare, report, work),
        ]
    }

    pub fn layer(&self, texture: Option<&Arc<SourceTexture>>) -> f32 {
        let Some(texture) = texture else { return -1.0 };
        let array = if texture.srgb {
            &self.albedo
        } else {
            &self.surface
        };
        array.slots[&texture.id] as f32
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for array in [&self.albedo, &self.surface] {
            for allocation in &array.allocations {
                if let Some(owner) = allocation.owner.upgrade() {
                    memory.view(&owner);
                }
            }
        }
    }
}

impl LayerArray {
    #[allow(clippy::too_many_arguments)]
    fn admit(
        &mut self,
        device: &Device,
        queue: &Queue,
        wanted: &BTreeMap<String, Arc<SourceTexture>>,
        srgb: bool,
        spare: u32,
        report: fn(&str),
        work: &mut AdmissionWork,
    ) -> Arc<SharedTexture> {
        let textures: Vec<_> = wanted.values().filter(|t| t.srgb == srgb).collect();
        for texture in &textures {
            let next = self.slots.len() as u32;
            self.slots.entry(texture.id.clone()).or_insert(next);
        }
        self.allocations.retain(|a| a.owner.strong_count() > 0);
        let edge = textures.iter().map(|t| t.size).max().unwrap_or(1);
        // Reservations absent from this table need no backing address in this
        // allocation. They stay reserved for future admissions without forcing
        // a third array just because another pending generation names them.
        let needed = textures
            .iter()
            .map(|texture| self.slots[&texture.id] + 1)
            .max()
            .unwrap_or(1);
        // Array edge affects bilinear sampling, even for unchanged source art.
        // Prefer a surviving exact-edge array over a larger pending version.
        let matching = self
            .allocations
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| {
                allocation.owner.upgrade().map(|owner| (index, owner))
            })
            .filter(|(_, owner)| owner.texture().width() == edge)
            .max_by_key(|(_, owner)| owner.texture().depth_or_array_layers())
            .map(|(index, _)| index);
        if let Some(index) = matching {
            let last = self.allocations.len() - 1;
            self.allocations.swap(index, last);
        }
        let previous = self.allocations.last().and_then(|a| a.owner.upgrade());
        let fits = previous.as_ref().is_some_and(|a| {
            a.texture().width() == edge && a.texture().depth_or_array_layers() >= needed
        });
        if !fits {
            let size = edge;
            let capacity = if textures.is_empty() {
                needed
            } else {
                needed
                    .saturating_add(spare)
                    .min(device.limits().max_texture_array_layers)
            };
            let label = if srgb {
                "model-albedo"
            } else {
                "model-surface"
            };
            let texture = device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: capacity,
                },
                mip_level_count: size.ilog2() + 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let owner = Arc::new(SharedTexture(texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            })));
            let mut written = BTreeSet::new();
            if let Some(previous) = &previous {
                if previous.texture().width() == size {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    for id in &self
                        .allocations
                        .last()
                        .expect("previous allocation")
                        .written
                    {
                        let layer = self.slots[id];
                        for level in 0..texture.mip_level_count() {
                            let extent = Extent3d {
                                width: size >> level,
                                height: size >> level,
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
                                info(previous.texture()),
                                info(&texture),
                                extent,
                            );
                            work.texture_bytes_copied +=
                                u64::from(extent.width) * u64::from(extent.height) * 4;
                        }
                        written.insert(id.clone());
                    }
                    queue.submit([encoder.finish()]);
                    report(&format!("art-store tripwire kind=TextureCapacity value={needed} bound={} owner=Runtime.art_store.{label} outcome=gpu-copy capacity={capacity}", previous.texture().depth_or_array_layers()));
                } else {
                    work.texture_arrays_rebuilt += 1;
                    report(&format!("art-store tripwire kind=TextureEdge value={size} bound={} owner=Runtime.art_store.{label} outcome=array-rebuild", previous.texture().width()));
                }
            }
            self.allocations.push(Allocation {
                owner: Arc::downgrade(&owner),
                written,
            });
            for texture in textures {
                self.write(queue, &owner, texture, work);
            }
            return owner;
        }
        let owner = previous.expect("fitting allocation");
        for texture in textures {
            self.write(queue, &owner, texture, work);
        }
        owner
    }

    fn write(
        &mut self,
        queue: &Queue,
        owner: &SharedTexture,
        source: &SourceTexture,
        work: &mut AdmissionWork,
    ) {
        if !self
            .allocations
            .last_mut()
            .expect("array allocation")
            .written
            .insert(source.id.clone())
        {
            return;
        }
        write_layer(queue, owner.texture(), self.slots[&source.id], source, work);
    }
}

/// One layer of the array texture, holding its source with every level:
/// the edge is the largest's, a smaller one upsampled into the finer levels.
fn write_layer(
    queue: &Queue,
    texture: &Texture,
    layer: u32,
    t: &SourceTexture,
    work: &mut AdmissionWork,
) {
    let size = texture.width();
    let levels = texture.mip_level_count();
    work.texture_layers_written += 1;
    let shift = (size / t.size).ilog2();
    for level in 0..levels {
        let edge = size >> level;
        let upsampled;
        let data = if level >= shift {
            &t.levels[(level - shift) as usize]
        } else {
            let (from, k) = (t.size as usize, t.size as f64 / f64::from(edge));
            let src = &t.levels[0];
            upsampled = (0..edge as usize * edge as usize)
                .flat_map(|i| {
                    let (y, x) = (i / edge as usize, i % edge as usize);
                    let s = ((y as f64 * k) as usize * from + (x as f64 * k) as usize) * 4;
                    src[s..s + 4].to_vec()
                })
                .collect::<Vec<u8>>();
            &upsampled
        };
        queue.write_texture(
            TexelCopyTextureInfo {
                texture,
                mip_level: level,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(edge * 4),
                rows_per_image: Some(edge),
            },
            Extent3d {
                width: edge,
                height: edge,
                depth_or_array_layers: 1,
            },
        );
        work.texture_bytes_uploaded += data.len() as u64;
    }
}
