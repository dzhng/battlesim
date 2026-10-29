//! A publication taken for the effects and sound: the air transits it
//! shows, what the effects read of it, and the hulls the side watched brew
//! up; and forgotten when the records in flight go stale.
use super::*;

impl Loaded {
    /// One publication noted for effects (and sound), with the hulls the
    /// side watched brew up and who it sees wading on `ground`.
    pub(super) fn note(&mut self, o: &Observation, ground: Option<&Ground>) {
        self.transits.note(
            &o.air_transits,
            o.tick,
            self.input.tick_hz,
            &self.input.units.transit,
        );
        let water = ground.map(Ground::standing_water);
        let mut publication =
            self.effect_feed
                .publication(o, &self.input.units, &self.input.effects, water.as_ref());
        let effects = &self.input.effects;
        publication.cook_offs =
            self.cook_offs
                .note(o, &self.input.units, effects, &effects.wreck_looks);
        let launches = self.effects.note(&publication);
        self.sound = Some((publication, launches));
    }

    /// Records still in flight are stale (`Presentation::invalidate`): the
    /// air transits seen so far go, and the wave field starts over, so no
    /// wave one side saw made lingers for the next.
    pub(super) fn forget(&mut self) {
        self.transits.clear();
        self.effects.restart_waves();
    }
}
