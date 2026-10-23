//! The battle-state digest: everything a replay must reproduce, folded in a
//! fixed order. Equal digests mean equal battles.

use super::*;

/// The `PlayerIndex` of `unit`'s owner: its id's place in the player list.
pub(super) fn owner_index(player_ids: &[String], unit: &Unit) -> Option<usize> {
    let owner = unit.owner.as_deref()?;
    player_ids.iter().position(|id| id == owner)
}

impl Battle {
    /// Digest of the complete authoritative state at the current tick.
    pub fn digest(&self) -> u64 {
        let mut d = Digest::default();
        d.u64(self.tick);
        if let Some(skirmish) = &self.skirmish {
            skirmish.digest(&mut d);
        }
        d.u64(self.units.len() as u64);
        for u in &self.units {
            u.digest(&mut d, owner_index(&self.player_ids, u));
        }
        d.u64(self.last_soldier as u64);
        for knowledge in &self.knowledge {
            knowledge.digest(&mut d);
        }
        for sightings in &self.sightings {
            d.u64(sightings.len() as u64);
            for s in sightings {
                d.u64(s.observer.0 as u64)
                    .u64(s.target.0 as u64)
                    .u64(s.members.len() as u64);
                for &k in &s.members {
                    d.u64(k as u64);
                }
            }
        }
        d.u64(self.fired.len() as u64);
        for id in &self.fired {
            d.u64(id.0 as u64);
        }
        for side in &self.sides {
            side.digest(&mut d);
        }
        self.planner.digest(&mut d);
        d.u64(self.world.obstacle_revision());
        // Every body's pose (L3): shoves move them.
        self.world.digest_props(&mut d);
        for (id, t) in self.world.moved() {
            d.u64(id as u64).u64(t);
        }
        self.world.digest_cleared(&mut d);
        self.world.digest_buildings(&mut d);
        d.u64(self.expiries.len() as u64);
        for (id, t) in &self.expiries {
            d.u64(*id as u64).u64(*t);
        }
        self.structures.digest(&mut d);
        // Only a battle with an aircraft going down folds crashes in, so
        // ground battles keep their digests.
        if !self.crashes.is_empty() {
            d.u64(self.crashes.len() as u64);
            for crash in &self.crashes {
                crash.digest(&mut d);
            }
        }
        // An empty fall log folds nothing: a battle where nothing topples
        // digests as if there were no log.
        if !self.fallen.is_empty() {
            d.u64(self.fallen.len() as u64);
            for f in self.fallen.values() {
                d.u64(u64::from(f.prop))
                    .f64(f.at[0])
                    .f64(f.at[1])
                    .f64(f.toward[0])
                    .f64(f.toward[1])
                    .u64(f.tick);
            }
        }
        self.ground.digest(&mut d);
        self.projectiles.digest(&mut d);
        d.u64(self.rounds.len() as u64);
        for (id, r) in &self.rounds {
            d.u64(id.0)
                .u64(r.weapon as u64)
                .u64(r.unit.0 as u64)
                .u64(r.side.index() as u64);
            if let Some(target) = r.seeker_target {
                d.u64(1);
                target.digest(&mut d);
            } else {
                d.u64(0);
            }
            if let Some(target) = r.fused_target {
                d.u64(1).u64(target.0 as u64);
            } else {
                d.u64(0);
            }
            d.u64(r.layer as u64);
        }
        d.u64(self.suppressed.len() as u64);
        for ((id, i), v) in &self.suppressed {
            d.u64(id.0).u64(*i as u64).f64(*v);
        }
        // Command sequencing (next_seq, accepted, pending) is input
        // bookkeeping, left out: live play accepts a command a tick before a
        // replay admits it, and its effect is the applied orders above.
        d.u64(self.combat_rng.state())
            .u64(self.damage_rng.state())
            .u64(self.ricochet_rng.state());
        // The defender's memory is a player's, not the battle's: its effect is
        // its accepted commands, and a replay runs without it.

        d.u64(self.script_victory.map_or(0, u64::from));
        if let Some(error) = &self.script_error {
            d.bytes(error.as_bytes());
        }
        if let Some(tick) = self.script_terminal_tick {
            d.u64(tick);
        }
        d.u64(self.script_events.len() as u64);
        for event in &self.script_events {
            d.u64(event.side.index() as u64)
                .bytes(event.name.as_bytes());
        }
        if let Some((_, referee)) = &self.referee {
            referee.digest(&mut d);
        }
        d.finish()
    }
}
