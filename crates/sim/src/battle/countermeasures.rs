//! Physical flare diversion before the next guidance command.

use super::*;

impl Battle {
    /// A decoyable round closing on a unit with a flare ready draws one, in
    /// round order, one per flare reload. Every diverted missile follows the
    /// physical flare independently of its launcher.
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
            // Ejected sideways and aft, from its hull's top.
            let layer = target.layer();
            let half = target.hull.expect("flares are carried by a hull (catalog)");
            let lateral = v2(-libm::sin(target.yaw), libm::cos(target.yaw));
            let aft = v2(-libm::cos(target.yaw), -libm::sin(target.yaw));
            let position = (target.position.xy() + lateral * (half.y + 0.5))
                .with_z(target.position.z + half.z);
            let base = target.flight_velocity();
            let velocity = v3(
                base.x + lateral.x * 18.0 + aft.x * 10.0,
                base.y + lateral.y * 18.0 + aft.y * 10.0,
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
            round.seeker_target = None;
            for mount in self.units.iter_mut().flat_map(|u| &mut u.mounts) {
                if mount.support.is_some_and(|s| s.projectile == projectile) {
                    mount.support = None;
                }
            }
            self.projectiles.divert(
                projectile,
                crate::flight::Decoy {
                    position,
                    velocity,
                    meet_m: self.rules.guided.flare_meet_m,
                    drag_s: self.rules.guided.flare_drag_s,
                    fall_mps: self.rules.guided.flare_fall_mps,
                    burn_s: self.rules.guided.flare_burn_s,
                },
                layer,
            );
        }
    }
}
