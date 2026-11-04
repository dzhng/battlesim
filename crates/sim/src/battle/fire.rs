//! Each tick's shooting: guided missiles steered from the launchers that
//! support them, then every mount acting.

use super::*;

impl Battle {
    /// Where each soldier stands, not where he leans out to: leaning is not
    /// moving.
    pub(super) fn standing(&self) -> BTreeMap<u32, V2> {
        self.units
            .iter()
            .flat_map(|u| u.members.iter().map(|s| (s.id, s.position.xy())))
            .collect()
    }

    /// What changed place on the map this tick, units by their base and
    /// soldiers by where they stand. Moving is changing place on the map: an
    /// aircraft climbing or sinking over one spot stands still, and a ground
    /// unit cannot change height without changing place.
    pub(super) fn moved(
        &self,
        before: &Poses,
        after: &Poses,
        stood: &BTreeMap<u32, V2>,
    ) -> weapons::tick::Moved {
        weapons::tick::Moved {
            units: before
                .units
                .iter()
                .zip(&after.units)
                .map(|(a, b)| (a.base - b.base).xy().length() > 1e-9)
                .collect(),
            soldiers: self
                .units
                .iter()
                .flat_map(|u| &u.members)
                .filter(|s| {
                    stood
                        .get(&s.id)
                        .is_some_and(|p| (*p - s.position.xy()).length() > 1e-9)
                })
                .map(|s| s.id)
                .collect(),
        }
    }

    /// Guided missiles (P05, P06): a launcher supports its missile while it
    /// stands still, has a living operator (including a survivor who takes
    /// over the launcher), its side identifies the target (a round: while it
    /// flies and one of the side's units sees it), and its operator has physical line of sight independent of
    /// spotting range. It steers it at the target's observed position. Losing any of these (or a
    /// Stop, which drops the mount's support) releases it at once and for good:
    /// the missile coasts straight on for `guided.release_coast_s`, then goes
    /// to ground, so a far one misses a target it can no longer see.
    /// Flares are dispensed first, so a spoofed round is steered no more.
    pub(super) fn guide(&mut self, moved: &weapons::tick::Moved) {
        self.dispense_flares();
        let mut supported = BTreeSet::new();
        let alive: Vec<bool> = self.units.iter().map(|u| u.alive()).collect();
        // Guidance and gun fire share the one aim rule.
        let aim_z: Vec<f64> = self
            .units
            .iter()
            .map(|u| u.aim_height(&self.rules))
            .collect();
        // A round guided at is seen as a unit is: by one of the side's
        // units, where it flies now.
        let rounds_seen: BTreeSet<(Side, ProjectileId)> = self
            .units
            .iter()
            .flat_map(|u| u.mounts.iter().map(move |m| (u.side, m.support)))
            .filter_map(|(side, support)| match support?.target {
                Target::Round(round) => Some((side, round)),
                _ => None,
            })
            .filter(|&(side, round)| {
                self.projectiles.get(round.id).is_some_and(|p| {
                    crate::sensing::side_sees_round(
                        &self.world,
                        &self.rules,
                        &self.units,
                        side,
                        p.position,
                        round.layer,
                    )
                })
            })
            .map(|(side, round)| (side, round.id))
            .collect();
        for (i, unit) in self.units.iter_mut().enumerate() {
            weapons::assign_operators(&self.arsenal, unit);
            let knowledge = &self.knowledge[unit.side.index()];
            for m in 0..unit.mounts.len() {
                let mount = &unit.mounts[m];
                let Some(s) = mount.support else { continue };
                let sighting = match s.target {
                    // Guidance precedes this tick's sensing and reads the
                    // previous completed observation, as before.
                    Target::Unit(t) if alive[t.0 as usize] => knowledge
                        .track(t)
                        .filter(|tr| tr.last_seen + 1 == self.tick)
                        .map(|tr| tr.position + v3(0.0, 0.0, aim_z[t.0 as usize])),
                    // A round is followed where it flies, while it flies and
                    // its side sees it, as the interceptor lock found it.
                    Target::Round(round) if rounds_seen.contains(&(unit.side, round.id)) => {
                        self.projectiles.get(round.id).map(|p| p.position)
                    }
                    _ => None,
                };
                // Entering or leaving a building is a transition that releases too.
                let settled = unit
                    .hold
                    .as_ref()
                    .is_none_or(|g| matches!(g.phase, crate::hold::Phase::Inside));
                // A launcher that guides only standing still lets go as it moves.
                let on_the_move = self.rounds.get(&s.projectile).is_some_and(|r| {
                    self.arsenal.weapons[r.weapon].def.ballistics.guidance
                        == Some(contract::ballistics::Guidance::OnTheMove)
                });
                let keep = (!moved.carrier(i, unit, mount) || on_the_move)
                    && alive[i]
                    && settled
                    && (unit.hull.is_some() || mount.operator.is_some())
                    && self.projectiles.get(s.projectile).is_some();
                match sighting.filter(|&at| {
                    keep && weapons::guidance_clear(&self.world, &self.rules, unit, mount, at)
                }) {
                    Some(at) => {
                        self.projectiles.steer(s.projectile, at);
                        supported.insert(s.projectile);
                    }
                    None => unit.mounts[m].support = None,
                }
            }
        }
        // Fire-and-forget seekers update from the last authoritative target
        // pose, without requiring the launcher to stand still or retain LOS.
        let seekers: Vec<_> = self
            .rounds
            .iter()
            .filter_map(|(id, round)| round.seeker_target.map(|target| (*id, round.side, target)))
            .collect();
        for (projectile, side, target) in seekers {
            // Where the target is and how it moves (a ground unit's pose
            // here is its position alone).
            let seen = match target {
                Target::Unit(id) if self.units.get(id.0 as usize).is_some_and(Unit::alive) => {
                    let unit = &self.units[id.0 as usize];
                    let aim = unit.position + v3(0.0, 0.0, unit.aim_height(&self.rules));
                    Some((aim, unit.flight_velocity()))
                }
                // A contact id is its firing side's own.
                Target::Contact(id) => self.knowledge[side.index()]
                    .contact(id)
                    .map(|c| (c.center, v3(0.0, 0.0, 0.0))),
                Target::Round(round) => self
                    .projectiles
                    .get(round.id)
                    .map(|p| (p.position, p.velocity)),
                _ => None,
            };
            if let (Some((at, velocity)), Some(seeker)) = (seen, self.projectiles.get(projectile)) {
                let point = crate::flight::lead(seeker.position, seeker.velocity, at, velocity);
                self.projectiles.steer_seeker(projectile, point);
            }
        }
        let released: Vec<ProjectileId> = self
            .projectiles
            .active()
            .iter()
            .filter(|p| p.guidance.is_some_and(|g| g.supported) && !supported.contains(&p.id))
            .map(|p| p.id)
            .collect();
        for id in released {
            self.projectiles
                .release(id, self.rules.guided.release_coast_s, &self.world);
        }
    }

    /// Every mount acts; shots become rounds in flight and firing evidence.
    /// Returns the units that launched this tick.
    pub(super) fn fire(&mut self, moved: &weapons::tick::Moved) -> BTreeSet<UnitId> {
        // Flight may have killed an operator after this tick's guidance.
        for unit in &mut self.units {
            weapons::assign_operators(&self.arsenal, unit);
        }
        let rounds: Vec<weapons::IncomingRound> = self
            .projectiles
            .active()
            .iter()
            .filter_map(|p| {
                let round = self.rounds.get(&p.id)?;
                self.arsenal.weapons[round.weapon]
                    .def
                    .interceptable
                    .then_some(weapons::IncomingRound {
                        id: p.id,
                        side: round.side,
                        position: p.position,
                        velocity: p.velocity,
                        layer: round.layer,
                        ends_in_s: p.lifetime_s - p.age_s,
                        seen: Side::ALL.map(|side| {
                            crate::sensing::side_sees_round(
                                &self.world,
                                &self.rules,
                                &self.units,
                                side,
                                p.position,
                                round.layer,
                            )
                        }),
                    })
            })
            .collect();
        let ctx = FireContext {
            world: &self.world,
            structures: &self.structures,
            ground: &self.ground,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
            knowledge: &self.knowledge,
            rounds: &rounds,
        };
        let aims = weapons::garrison_aims(&ctx, &self.units);
        garrison::allocate_slots(&mut self.units, &aims, &self.rules, self.tick);
        let shots = weapons::tick::advance(&ctx, &mut self.units, moved, &mut self.combat_rng);
        let fired = shots.iter().map(|s| s.unit).collect();
        for shot in shots {
            let side = self.units[shot.unit.0 as usize].side;
            for launch in shot.launches {
                let seeker = self.arsenal.weapons[shot.weapon].def.seeker.is_some();
                let guided = launch.guidance.is_some() && !seeker;
                let mut launch = launch;
                if seeker {
                    if let Some(guidance) = launch.guidance.as_mut() {
                        guidance.supported = false;
                    }
                }
                let id = self.projectiles.launch(launch);
                self.rounds.insert(
                    id,
                    Round {
                        weapon: shot.weapon,
                        unit: shot.unit,
                        side,
                        seeker_target: seeker.then_some(shot.target),
                        fused_target: match shot.target {
                            Target::Unit(target) => Some(target),
                            _ => None,
                        },
                        layer: self.units[shot.unit.0 as usize].layer(),
                    },
                );
                // A guided round is supported by the mount that launched it.
                if guided {
                    let unit = &mut self.units[shot.unit.0 as usize];
                    unit.mounts[shot.mount].support = Some(Support {
                        projectile: id,
                        target: shot.target,
                    });
                }
            }
            if self.arsenal.weapons[shot.weapon].def.intercepts()
                && matches!(shot.target, Target::Ground(_) | Target::Round(_))
            {
                self.units[shot.unit.0 as usize].mounts[shot.mount].lock = None;
            }
            // The attacked unit may answer this attacker under Return fire only.
            if let Target::Unit(t) = shot.target {
                self.units[t.0 as usize].attackers.insert(shot.unit);
                crate::air::dogfight::on_fire(&mut self.units, shot.unit, t);
            }
            let shooter = &self.units[shot.unit.0 as usize];
            let heard = self
                .arsenal
                .heard(shooter.kind, shooter.mounts[shot.mount].spec);
            self.record_fire(shot.unit, heard);
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An AT team's rifleman walks to a new post mid-aim: the gunner, who
    /// stands still, keeps aiming and launches, though the squad's centre
    /// moves. A stationary weapon reads the body that carries it.
    #[test]
    fn a_gunner_standing_still_fires_while_a_teammate_moves() {
        let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
            "map": { "size": [1000, 1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": crate::fixtures::test_rules(),
            "units": [
                { "side": "blue", "kind": "test_at", "position": [100, 300] },
                { "side": "red", "kind": "test_tank", "position": [600, 300] }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let mut battle = Battle::new(&setup, 3);
        for _ in 0..battle.rules.tick_hz {
            battle.step();
        }
        let launcher = battle.units[0]
            .mounts
            .iter()
            .position(|m| m.operator.is_some())
            .expect("the gunner works the launcher");
        let gunner = battle.units[0].mounts[launcher].operator.unwrap();
        let rifleman = battle.units[0]
            .members
            .iter()
            .position(|s| s.id != gunner)
            .unwrap();
        let start = battle.units[0].members[rifleman].position.xy();
        let mut launched = false;
        for _ in 0..8 * battle.rules.tick_hz {
            battle.units[0].members[rifleman].post = Some(v2(100.0, 260.0));
            battle.step();
            assert_ne!(
                battle.units[0].mounts[launcher].reason,
                contract::observation::ActionReason::MovingStationaryWeapon,
                "tick {}: the gunner stands still",
                battle.tick
            );
            launched |= battle
                .rounds
                .values()
                .any(|r| r.unit == UnitId(0) && battle.arsenal.weapons[r.weapon].id == "atgm");
        }
        let walked = (battle.units[0].members[rifleman].position.xy() - start).length();
        assert!(walked > 5.0, "the rifleman walked {walked:.1} m");
        assert!(launched, "the missile left while he walked");
    }
}
