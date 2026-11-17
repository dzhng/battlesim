//! Physical flare diversion before the next guidance command.

use super::*;

impl Battle {
    /// A decoyable round closing on a unit with a flare ready draws one, in
    /// round order, one per flare reload. A seeker follows the flare; an
    /// operator-guided round loses its guidance, as when its operator loses
    /// sight of the target, and flies on to the ground.
    pub(super) fn dispense_flares(&mut self) {
        // What each round closes on: its seeker's target, or what its
        // launcher steers it at.
        let mut closing: BTreeMap<ProjectileId, UnitId> = self
            .units
            .iter()
            .flat_map(|u| &u.mounts)
            .filter_map(|m| match m.support {
                Some(weapons::Support {
                    projectile,
                    target: Target::Unit(target),
                }) => Some((projectile, target)),
                _ => None,
            })
            .collect();
        closing.extend(
            self.rounds
                .iter()
                .filter_map(|(id, round)| match round.seeker_target {
                    Some(Target::Unit(target)) => Some((*id, target)),
                    _ => None,
                }),
        );
        for (projectile, target_id) in closing {
            let Some(round) = self.rounds.get(&projectile) else {
                continue;
            };
            if !self.arsenal.weapons[round.weapon].def.decoyable {
                continue;
            }
            let target = &self.units[target_id.0 as usize];
            // A wreck or a withdrawn airframe dispenses nothing.
            if !target.alive() {
                continue;
            }
            let Some(flare) = target.unit_type(&self.rules).capabilities.flares else {
                continue;
            };
            if !target
                .flares
                .is_some_and(|f| f.charges > 0 && f.ready_tick <= self.tick)
            {
                continue;
            }
            // Ejected off its side, from its hull's top.
            let half = target.hull.expect("flares are carried by a hull (catalog)");
            let lateral = v2(-libm::sin(target.yaw), libm::cos(target.yaw));
            let position = (target.position.xy() + lateral * (half.y + 0.5))
                .with_z(target.position.z + half.z);
            let base = target.flight_velocity();
            let velocity = v3(
                base.x + lateral.x * 18.0,
                base.y + lateral.y * 18.0,
                base.z - 2.0,
            );
            if let Some(f) = self.units[target_id.0 as usize].flares.as_mut() {
                f.charges -= 1;
                f.ready_tick =
                    self.tick + (flare.reload_s * self.rules.tick_hz as f64).round() as u64;
            }
            // A spoofed round no longer flies at the unit.
            let round = self.rounds.get_mut(&projectile).expect("checked above");
            round.aimed_at = None;
            if round.seeker_target.take().is_some() {
                self.projectiles.decoy_seeker(
                    projectile,
                    crate::flight::Decoy {
                        position,
                        velocity,
                        meet_m: self.rules.guided.flare_meet_m,
                    },
                );
            } else {
                // Its launcher lets go; guidance releases it, as for a lost
                // sight line.
                for mount in self.units.iter_mut().flat_map(|u| &mut u.mounts) {
                    if mount.support.is_some_and(|s| s.projectile == projectile) {
                        mount.support = None;
                    }
                }
            }
        }
    }
}
