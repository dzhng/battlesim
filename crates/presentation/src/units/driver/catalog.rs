//! Replacement art must still be able to draw the history the driver retains.
use super::*;

impl PoseDriver {
    /// Checks only current clip/blend/corpse references, not an old catalog's
    /// complete vocabulary. Run again at commit after asynchronous preparation.
    pub fn validate_catalog(&self, units: &UnitsSection) -> Result<(), String> {
        for state in self.soldiers.values() {
            let pose = &state.pose;
            let Some(name) = units.resolve(
                &pose.kind,
                &pose.side,
                pose.soldier,
                pose.slot,
                pose.operator_mount,
                pose.active_mount,
            ) else {
                continue;
            };
            let Some(Appearance::Skinned { clips, .. }) = units.appearances.get(name) else {
                continue;
            };
            for clip in
                std::iter::once(&pose.clip).chain(pose.blend.iter().map(|blend| &blend.clip))
            {
                if !clips.contains_key(clip.as_ref()) {
                    return Err(format!(
                        "{name}: battle replacement is missing retained clip {clip}"
                    ));
                }
            }
        }
        for corpse in self
            .corpses
            .lying
            .values()
            .chain(self.out.fading.iter().map(|f| &f.corpse))
        {
            let Some(name) = units.resolve(
                &corpse.kind,
                &corpse.side,
                corpse.soldier,
                corpse.slot,
                corpse.operator_mount,
                None,
            ) else {
                continue;
            };
            let Some(Appearance::Skinned { corpse_poses, .. }) = units.appearances.get(name) else {
                continue;
            };
            if corpse_poses.get(corpse.lies).is_none() {
                return Err(format!(
                    "{name}: battle replacement is missing retained corpse pose {}",
                    corpse.lies
                ));
            }
        }
        Ok(())
    }
}
