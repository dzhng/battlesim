use serde::{Deserialize, Serialize};

/// Why model admission started with a fresh scene/device art store.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RebuildReason {
    /// The first generation of a runtime: there was none to reuse.
    Initial,
    /// Current art was admitted again on a recreated GPU device.
    DeviceRecreated,
    /// Dead material slots passed the store's tripwire bound.
    DeadMaterialSlots,
    /// The store could not assign a 16-bit material slot.
    MaterialSlotsExhausted,
    /// Dead texture layers passed the store's tripwire bound.
    DeadTextureLayers,
    /// Historical texture layer addresses exceeded the device limit.
    TextureLayersExhausted,
}

/// What admitting one complete appearance generation cost, counted where the
/// work happens. Counts and payload bytes, never times, so the report is the
/// same on every machine and both shells.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionWork {
    pub bundles_decoded: u32,
    pub vertex_bytes_packed: u64,
    pub buffer_bytes_uploaded: u64,
    pub texture_layers_written: u32,
    pub texture_bytes_uploaded: u64,
    /// Existing mip payload copied GPU-side when array capacity grows.
    pub texture_bytes_copied: u64,
    /// Existing arrays rebuilt at a different edge; their CPU writes are
    /// also included in texture_layers_written and texture_bytes_uploaded.
    pub texture_arrays_rebuilt: u32,
    pub cards_baked: u32,
    /// Concurrent readback groups awaited by the card bake.
    pub card_read_waits: u32,
    /// New packed albedo/normal card layers written from completed bakes.
    pub card_bytes_uploaded: u64,
    /// Existing packed card layers copied GPU-side on capacity growth.
    pub card_bytes_copied: u64,
    pub statics_bytes_uploaded: u64,
    pub scenery_placements: u32,
    pub rebuild_reason: Option<RebuildReason>,
}
