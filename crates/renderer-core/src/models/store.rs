//! Scene/device ownership of stable material slots and weak geometry reuse.
use super::Drawable;
use crate::runtime::RebuildReason;
use contract::identity::ContentId;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, Weak};

const MATERIAL_SLOT_LIMIT: usize = u16::MAX as usize + 1;

/// Append-only ranges in the vertex ABI's 16-bit material address space.
/// Reservations survive dropped preparations; identities never change base.
#[derive(Default)]
pub struct MaterialSlots {
    ranges: BTreeMap<ContentId, std::ops::Range<usize>>,
    end: usize,
}

impl MaterialSlots {
    pub fn reserve(&mut self, content: ContentId, count: usize) -> Result<usize, String> {
        if let Some(range) = self.ranges.get(&content) {
            return Ok(range.start);
        }
        let end = self
            .end
            .checked_add(count)
            .filter(|&end| end <= MATERIAL_SLOT_LIMIT)
            .ok_or("model material slots exhausted: vertex ABI allows 65536 slots")?;
        let base = self.end;
        self.ranges.insert(content, base..end);
        self.end = end;
        Ok(base)
    }

    pub fn base(&self, content: &ContentId) -> Option<usize> {
        self.ranges.get(content).map(|range| range.start)
    }

    pub(super) fn len(&self) -> usize {
        self.end
    }
}

/// Scene art tripwires and the platform sink that reports when they fire.
#[derive(Clone, Copy)]
pub struct ArtStorePolicy {
    pub dead_material_slots: usize,
    /// Target and readback byte bound for each card-bake batch.
    pub card_bake_memory_bytes: u64,
    pub report: fn(&str),
}

impl Default for ArtStorePolicy {
    fn default() -> Self {
        Self {
            dead_material_slots: 8192,
            card_bake_memory_bytes: crate::impostor::DEFAULT_BAKE_MEMORY_BYTES,
            report: |message| eprintln!("{message}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct GeometryKey {
    pub content: ContentId,
    pub state: String,
    pub pose: Option<Vec<u8>>,
    pub tracks: Vec<u8>,
}

/// Routing and placement rows share immutable geometry across generations.
/// Corpse palettes and running-gear flags are inputs to the packed vertices.
/// Only generations own buffers; the index cannot keep retired geometry alive.
#[derive(Default)]
pub struct ArtStore {
    pub(super) slots: MaterialSlots,
    geometry: HashMap<GeometryKey, Weak<[Drawable]>>,
    policy: ArtStorePolicy,
}

pub type SharedArtStore = Arc<Mutex<ArtStore>>;

impl ArtStore {
    pub fn new(policy: ArtStorePolicy) -> Self {
        Self {
            policy,
            ..Default::default()
        }
    }

    pub fn shared() -> SharedArtStore {
        Arc::new(Mutex::new(Self::default()))
    }

    /// Include geometry held by any live or pending generation in a scene's
    /// memory snapshot. The visitor deduplicates buffers shared by generations.
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for geometry in self.geometry.values().filter_map(Weak::upgrade) {
            for tier in geometry.iter() {
                memory.buffer(&tier.vertices);
                memory.buffer(&tier.indices);
            }
        }
    }

    /// Preflight all wanted content before packing anything. A fresh store uses
    /// this same admission; an oversized wanted catalog fails without wrapping.
    pub(super) fn admit(
        store: SharedArtStore,
        wanted: &BTreeMap<ContentId, usize>,
    ) -> Result<(SharedArtStore, Option<RebuildReason>), String> {
        let mut held = store.lock().expect("art store lock");
        let active: usize = wanted.values().sum();
        if active > MATERIAL_SLOT_LIMIT {
            return Err(format!(
                "model material slots exhausted: {active} wanted, limit {MATERIAL_SLOT_LIMIT}"
            ));
        }
        let live: usize = wanted
            .keys()
            .filter_map(|id| held.slots.ranges.get(id))
            .map(|r| r.len())
            .sum();
        let added: usize = wanted
            .iter()
            .filter(|(id, _)| held.slots.base(id).is_none())
            .map(|(_, count)| count)
            .sum();
        let dead = held.slots.len() - live;
        let reason = if dead > held.policy.dead_material_slots {
            Some((
                RebuildReason::DeadMaterialSlots,
                dead,
                held.policy.dead_material_slots,
            ))
        } else if held.slots.len() + added > MATERIAL_SLOT_LIMIT {
            Some((
                RebuildReason::MaterialSlotsExhausted,
                held.slots.len() + added,
                MATERIAL_SLOT_LIMIT,
            ))
        } else {
            None
        };
        if let Some((kind, value, bound)) = reason {
            (held.policy.report)(&format!("art-store tripwire kind={kind:?} value={value} bound={bound} owner=Runtime.art_store outcome=fresh-store"));
            let fresh = Arc::new(Mutex::new(Self::new(held.policy)));
            drop(held);
            let (fresh, _) = Self::admit(fresh, wanted)?;
            return Ok((fresh, Some(kind)));
        }
        for (id, &count) in wanted {
            held.slots.reserve(id.clone(), count)?;
        }
        held.geometry
            .retain(|_, geometry| geometry.strong_count() > 0);
        drop(held);
        Ok((store, None))
    }

    pub(super) fn geometry(
        store: &SharedArtStore,
        key: GeometryKey,
        build: impl FnOnce() -> Result<Vec<Drawable>, String>,
    ) -> Result<Arc<[Drawable]>, String> {
        let mut held = store.lock().expect("art store lock");
        if let Some(geometry) = held.geometry.get(&key).and_then(Weak::upgrade) {
            return Ok(geometry);
        }
        let geometry: Arc<[Drawable]> = Arc::from(build()?);
        held.geometry.insert(key, Arc::downgrade(&geometry));
        Ok(geometry)
    }
}
