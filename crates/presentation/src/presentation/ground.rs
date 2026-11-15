//! Resolved ground admission and its renderer resource identity.
use super::*;

impl Presentation {
    /// The map's ground, resolved from `source` under `biome` (its region's
    /// finish) and the fixture's tuning; its scenery is placed by
    /// `place_scenery` and drawn from the next `load_world`.
    pub fn set_ground(
        &mut self,
        source: Source,
        biome: Biome,
        tuning: Tuning,
    ) -> Result<(), String> {
        let ground = Ground::new(source, biome, tuning)?;
        // A region's scars load as far past it as a crater's bowl moves them.
        self.scars.set_reach(ground.scar_reach_m());
        // Sight is cut by the new ground.
        if let Some(fog) = &mut self.fog {
            fog.sight = FogSight::new(ground.tuning.fog_geometry.clone())?;
            fog.sight.set_world(
                SightWorld::of(&ground.source, fog.sensors),
                ground.source.foliage.clone(),
            )?;
            fog.from = None;
            fog.known_from = None;
            fog.dirty = true;
        }
        self.catalog_state.renew_ground();
        self.ground = Some(ground);
        self.scenery = None;
        self.props = None;
        self.statics.invalidate();
        Ok(())
    }

    pub fn ground(&self) -> Option<&Ground> {
        self.ground.as_ref()
    }

    /// Ground identity changes on `set_ground` and `ground_mut`, including a
    /// mutable lab retune. Ordinary catalog commits leave it unchanged.
    pub fn ground_revision(&self) -> u32 {
        self.catalog_state.ground_revision()
    }

    /// The ground's mutably (a lab growing grass by other rules).
    pub fn ground_mut(&mut self) -> Option<&mut Ground> {
        self.catalog_state.renew_ground();
        self.ground.as_mut()
    }
}
