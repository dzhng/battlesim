//! Cells many rows pack the same way (`publication/cells.rs` packs the
//! shared ones): an absent value, an exact integer in two limbs, and, by
//! the writer's field names under the caller's `FIELDS`, a target
//! (`target!`) and a host (`host!`).
use super::{MountTarget, Name, Reader, Result, LIMB_BITS};

pub(super) const LIMB: f64 = (1u32 << LIMB_BITS) as f64;

/// An exact integer from its limbs; none when absent.
pub(super) fn limbs(low: f64, high: f64) -> Option<f64> {
    (low >= 0.0).then_some(low + high * LIMB)
}

/// A value, or none where the writer marks absence with a negative.
pub(super) fn present(v: f64) -> Option<f64> {
    (v >= 0.0).then_some(v)
}

/// The target `$row`'s `targetKind`..`targetZ` cells name.
macro_rules! target {
    ($reader:expr, $row:expr) => {
        $reader.target([
            v!($row, "targetKind"),
            v!($row, "targetId"),
            v!($row, "targetX"),
            v!($row, "targetY"),
            v!($row, "targetZ"),
        ])
    };
}

/// The host `$row`'s `hostKind`, `hostLo` and `hostHi` cells name.
macro_rules! host {
    ($reader:expr, $row:expr) => {
        $reader.host([v!($row, "hostKind"), v!($row, "hostLo"), v!($row, "hostHi")])
    };
}

pub(super) use {host, target};

impl Reader {
    /// A target from its kind (indexing targetKinds), id and point cells:
    /// none, an id for a unit or contact, a point for ground.
    pub(super) fn target(&self, [kind, id, x, y, z]: [f64; 5]) -> Result<Option<MountTarget>> {
        let kind = self.name("targetKinds", kind)?;
        Ok(match &*kind {
            "none" => None,
            "ground" => Some(MountTarget {
                kind,
                id: None,
                point: Some([x, y, z]),
            }),
            _ => Some(MountTarget {
                kind,
                id: Some(id),
                point: None,
            }),
        })
    }

    /// A host's kind (indexing hostKinds) and id from its two limbs; none
    /// for a negative kind.
    pub(super) fn host(&self, [kind, lo, hi]: [f64; 3]) -> Result<Option<(Name, f64)>> {
        let kind = self.optional("hostKinds", kind)?;
        Ok(kind.map(|kind| (kind, limbs(lo, hi).unwrap_or(0.0))))
    }
}
