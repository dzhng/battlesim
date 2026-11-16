//! An own mount's readiness row, decoded by the writer's field names, and
//! the target cells it shares with an order step's row.
use super::{at, present, Mount, MountTarget, Reader, Result};
use crate::publication::MOUNT_FIELDS;

impl Reader {
    pub(super) fn mount(&self, p: &[f32; MOUNT_FIELDS.len()]) -> Result<Mount> {
        const FIELDS: &[&str] = &MOUNT_FIELDS;
        // Ammo per kind: -1 unlimited, -2 no such kind on this mount.
        let ammo = [v!(p, "ammo0"), v!(p, "ammo1")]
            .into_iter()
            .take(v!(p, "kinds").max(0.0) as usize)
            .map(|n| (n != -1.0).then_some(n))
            .collect();
        let cells = [
            v!(p, "targetKind"),
            v!(p, "targetId"),
            v!(p, "targetX"),
            v!(p, "targetY"),
            v!(p, "targetZ"),
        ];
        Ok(Mount {
            mount: v!(p, "mount"),
            loaded: present(v!(p, "loaded")),
            ammo,
            aim: v!(p, "aim"),
            reload: v!(p, "reload"),
            target: self.target(cells)?,
            reason: self.name("actionReasons", v!(p, "reason"))?,
            guiding: v!(p, "guiding") == 1.0,
            reloading: present(v!(p, "reloadKind")),
        })
    }

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
}
