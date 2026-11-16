//! An own mount's readiness row, decoded by the writer's field names.
use super::cells::target;
use super::{at, present, Mount, Reader, Result};
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
        Ok(Mount {
            mount: v!(p, "mount"),
            loaded: present(v!(p, "loaded")),
            ammo,
            aim: v!(p, "aim"),
            reload: v!(p, "reload"),
            target: target!(self, p)?,
            reason: self.name("actionReasons", v!(p, "reason"))?,
            guiding: v!(p, "guiding") == 1.0,
            reloading: present(v!(p, "reloadKind")),
        })
    }
}
