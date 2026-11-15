use serde::{Deserialize, Serialize};

/// Why an admission reused nothing from a live generation.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RebuildReason {
    /// The first generation of a runtime: there was none to reuse.
    Initial,
    /// A whole generation replaced the live one.
    Replacement,
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
    pub cards_baked: u32,
    /// Concurrent readback groups awaited by the card bake.
    pub card_read_waits: u32,
    pub statics_bytes_uploaded: u64,
    pub scenery_placements: u32,
    pub rebuild_reason: Option<RebuildReason>,
}
