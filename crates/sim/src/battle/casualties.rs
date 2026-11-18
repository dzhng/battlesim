//! Physical deaths, crashes and forced exits from destroyed hosts.

use super::*;

impl Battle {
    /// Downed aircraft fall; each that meets the ground bursts there and
    /// leaves its wreck.
    pub(super) fn fall_crashes(&mut self) {
        if self.crashes.is_empty() {
            return;
        }
        let dt = 1.0 / self.rules.tick_hz as f64;
        let gravity = self.projectiles.gravity();
        let mut landed = Vec::new();
        let mut left_map = Vec::new();
        for mut crash in std::mem::take(&mut self.crashes) {
            let half = self.units[crash.unit.0 as usize]
                .hull
                .expect("an airframe's hull");
            let fall = crash.fall(&self.world, gravity, half, dt);
            // Passengers stay aboard while it falls, so they follow the
            // crash rather than the airframe's last live pose.
            let passengers = crate::carriage::passengers(&self.units, crash.unit);
            for &index in &passengers {
                let unit = &mut self.units[index];
                for member in unit.members.iter_mut().filter(|member| member.alive()) {
                    member.position = crash.position;
                }
                unit.position = crash.position;
            }
            match fall {
                crate::crash::Fall::Landed => landed.push(crash),
                crate::crash::Fall::Falling => self.crashes.push(crash),
                // It takes them with it, credited to whoever brought it down.
                crate::crash::Fall::LeftMap => {
                    for index in passengers {
                        let unit = &mut self.units[index];
                        crate::bail_out::lost_with_host(unit, crash.position);
                        unit.hold = None;
                        unit.orders.clear();
                        unit.route = None;
                        left_map.push(damage::UnitDeath {
                            victim: unit.id,
                            source: crash.source,
                        });
                    }
                }
            }
        }
        for crash in landed {
            self.land(crash);
        }
        if !left_map.is_empty() {
            self.consequences(damage::Outcome {
                destroyed: left_map,
                ..Default::default()
            });
        }
    }

    /// A downed aircraft meets the ground (D3, D31): it bursts as the crash
    /// row, credited to whoever brought it down, then its wreck comes to rest
    /// on the nearest ground clear of live hulls and buildings.
    pub(super) fn land(&mut self, crash: crate::crash::Crash) {
        let weapon = (self.arsenal.weapons.iter())
            .position(|w| w.id == crate::crash::CRASH_WEAPON)
            .expect("the rules' crash row");
        let def = self.arsenal.weapons[weapon].def.clone();
        let at = crate::crash::burst_point(crash.position);
        let ctx = DamageContext {
            world: &self.world,
            windows: &self.windows,
            ground: &self.ground,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
        };
        let outcome = damage::detonate(
            &ctx,
            &def,
            at,
            crash.source,
            &mut self.units,
            &mut self.damage_rng,
        );
        let side = self.units[crash.unit.0 as usize].side;
        self.blasts.push(Burst {
            side,
            layer: AltitudeLayer::Ground,
            sample: at,
            blast: Blast {
                point: xyz(at),
                radius: def.blast_radius_m,
                kind: weapon,
            },
        });
        let fallen_before: BTreeSet<PropId> = self.fallen.keys().copied().collect();
        self.consequences(outcome);
        self.ground
            .burst(&self.world, at, def.blast_radius_m, &self.rules.ground);
        // The sides that saw it go down see it come down: the trees its
        // crash felled fall in their sight, as its wreck lands there.
        let felled: Vec<PropId> = (self.fallen.keys())
            .filter(|id| !fallen_before.contains(id))
            .copied()
            .collect();
        for &(side, _) in &crash.knowing {
            for &prop in &felled {
                self.sides[side.index()].saw_fallen(prop);
                self.knowledge[side.index()].learn_fallen(prop);
            }
        }
        let unit = &self.units[crash.unit.0 as usize];
        let half = unit.hull.expect("an airframe's hull");
        let kind = (unit.unit_type(&self.rules).hull())
            .expect("an airframe's hull row")
            .wreck
            .clone();
        let wreck_of = self.rules.catalog.id(unit.kind).to_string();
        let hulls: Vec<Obb2> = (self.units.iter())
            .filter(|o| o.on_field())
            .filter_map(|o| o.ground_footprint())
            .collect();
        let rest = crate::crash::resting_place(
            &self.world,
            crash.position.xy(),
            crash.yaw,
            half.xy(),
            &hulls,
        );
        let wreck = self.world.add_prop(&PropDefinition {
            kind,
            center: [rest.x, rest.y],
            yaw: crash.yaw,
            half_extents: [half.x, half.y, half.z],
            base_z: None,
            wreck_of: Some(wreck_of),
        });
        let mut passenger_deaths = Vec::new();
        self.bail_out_carrier(
            crash.unit,
            rest,
            wreck,
            crash.source,
            &mut passenger_deaths,
            contract::scenario::BailOutHost::Air,
        );
        if !passenger_deaths.is_empty() {
            self.consequences(damage::Outcome {
                destroyed: passenger_deaths,
                ..Default::default()
            });
        }
        if let Some(prop) = self.world.prop(wreck) {
            for (side, _) in crash.knowing {
                self.sides[side.index()].learn(
                    prop,
                    self.authored_props,
                    self.rules.pushing.relearn_m,
                    true,
                );
            }
        }
    }

    /// Hostile damage or suppression grants return fire; a destroyed vehicle
    /// leaves a permanent wreck (M06, M07) and carries its passenger deaths
    /// into the same causal stream. A death a side was watching ends its
    /// track, while an unseen death discloses nothing. The vehicle's own side
    /// knows where it died, and a side that watched it die saw the wreck
    /// appear: both learn the wreck at once, so a vehicle never vanishes from
    /// the picture of a side that knew where it stood.
    pub(super) fn consequences(&mut self, outcome: damage::Outcome) {
        for (victim, shooter) in outcome.attacked {
            self.units[victim.0 as usize].attackers.insert(shooter);
        }
        let mut destroyed = outcome.destroyed;
        // Structural damage in event order; a prop is destroyed once (L10, Q17).
        for hit in outcome.structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                destroyed.extend(self.destroy_prop(hit.prop, hit.toward).into_iter().map(
                    |victim| damage::UnitDeath {
                        victim,
                        source: hit.source,
                    },
                ));
            }
        }
        let mut death_index = 0;
        while death_index < destroyed.len() {
            let death = destroyed[death_index];
            death_index += 1;
            let id = death.victim;
            let unit = &self.units[id.0 as usize];
            let own = unit.side;
            // Each side that saw it die, with the id it knew it by.
            let mut knowing = vec![(own, id.0)];
            for side in Side::ALL.into_iter().filter(|&s| s != own) {
                let knowledge = &mut self.knowledge[side.index()];
                if knowledge.identifies(id, self.tick - 1) {
                    let handle = knowledge.track(id).expect("an identified track").id;
                    knowledge.saw_destroyed(id);
                    knowing.push((side, handle.0));
                }
            }
            // A downed aircraft falls first; its wreck lies where it lands.
            if unit.airborne() {
                let air = unit.air.expect("an airborne unit has flight state");
                self.crashes.push(crate::crash::Crash::new(
                    id,
                    unit.position,
                    air.velocity.xy(),
                    unit.yaw,
                    death.source,
                    knowing,
                ));
                continue;
            }
            let wreck = unit.unit_type(&self.rules).hull().map(|h| h.wreck.clone());
            let (hulls, soldiers) = crate::arrangement::ground_taken(&self.units, id);
            let rest = crate::movement::drive::death_roll(
                &self.world,
                &self.rules,
                unit,
                &hulls,
                &soldiers,
            );
            let unit_hull = unit.hull;
            let ground_carrier = unit_hull.is_some() && unit.air.is_none();
            let unit_yaw = unit.yaw;
            let wreck_of = self.rules.catalog.id(unit.kind).to_string();
            let unit_z = unit.position.z;
            let wreck = match (unit_hull, wreck) {
                (Some(half), Some(kind)) => Some(self.world.add_prop(&PropDefinition {
                    kind,
                    center: [rest.x, rest.y],
                    yaw: unit_yaw,
                    half_extents: [half.x, half.y, half.z],
                    base_z: Some(unit_z),
                    wreck_of: Some(wreck_of),
                })),
                _ => None,
            };
            if ground_carrier {
                self.bail_out_carrier(
                    id,
                    rest,
                    wreck.expect("a ground hull leaves a wreck"),
                    death.source,
                    &mut destroyed,
                    contract::scenario::BailOutHost::Ground,
                );
            }
            if let Some(prop) = wreck.and_then(|w| self.world.prop(w)) {
                for (side, _) in knowing {
                    self.sides[side.index()].learn(
                        prop,
                        self.authored_props,
                        self.rules.pushing.relearn_m,
                        true,
                    );
                }
            }
        }
        for death in &destroyed {
            // A kill bark is for an enemy; a friendly loss is not a kill.
            if let Some(source) = death.source.filter(|s| {
                self.units[s.unit.0 as usize].side != self.units[death.victim.0 as usize].side
            }) {
                self.shout_kills.insert(source.unit);
            }
        }
        if let Some(skirmish) = &mut self.skirmish {
            for death in &destroyed {
                let victim = &self.units[death.victim.0 as usize];
                let killer = death.source.and_then(|source| {
                    let unit = self.units.get(source.unit.0 as usize)?;
                    Some(contract::skirmish::MatchUnit {
                        side: source.side,
                        id: source.unit,
                        kind: unit.kind,
                        owner: {
                            let owner = unit.owner.as_deref();
                            debug_assert!(owner.is_some(), "skirmish killer unit has no owner");
                            owner
                                .and_then(|id| skirmish.player_index(id))
                                .map(|i| contract::players::PlayerIndex(i as u16))
                        },
                    })
                });
                let victim_cost = skirmish.entered.get(&death.victim).map_or_else(
                    || self.rules.catalog.get(victim.kind).cost,
                    |unit| unit.price,
                );
                let killer_cost = death.source.and_then(|source| {
                    skirmish
                        .entered
                        .get(&source.unit)
                        .map(|unit| unit.price)
                        .or_else(|| {
                            self.units
                                .get(source.unit.0 as usize)
                                .map(|unit| self.rules.catalog.get(unit.kind).cost)
                        })
                });
                skirmish.record_kill(
                    contract::skirmish::MatchUnit {
                        side: victim.side,
                        id: victim.id,
                        kind: victim.kind,
                        owner: victim
                            .owner
                            .as_deref()
                            .and_then(|id| skirmish.player_index(id))
                            .map(|i| contract::players::PlayerIndex(i as u16)),
                    },
                    victim_cost,
                    killer,
                    killer_cost,
                    self.tick,
                    death
                        .source
                        .and_then(|source| self.units.get(source.unit.0 as usize))
                        .and_then(|unit| {
                            let owner = unit.owner.as_deref();
                            debug_assert!(owner.is_some(), "skirmish killer unit has no owner");
                            owner.and_then(|id| skirmish.player_index(id))
                        }),
                );
            }
            let deaths: Vec<_> = destroyed
                .iter()
                .filter_map(|death| {
                    let entered = skirmish.entered.get(&death.victim)?;
                    let unit = &self.units[death.victim.0 as usize];
                    Some(crate::settlement::EconomicDeath {
                        victim: death.victim,
                        side: unit.side,
                        price: entered.price,
                        source: death.source,
                        supply: unit.unit_type(&self.rules).capabilities.supply.is_some(),
                    })
                })
                .collect();
            let starting_budget = u64::from(skirmish.setup.rules.credits_per_minute)
                * u64::from(skirmish.setup.rules.starting_minutes);
            let killer_players = self
                .units
                .iter()
                .filter_map(|unit| {
                    let owner = unit.owner.as_deref();
                    debug_assert!(owner.is_some(), "skirmish unit has no owner");
                    owner
                        .and_then(|id| skirmish.player_index(id))
                        .map(|player| (unit.id, player))
                })
                .collect();
            skirmish.settlement.settle_players(
                &deaths,
                starting_budget,
                &killer_players,
                &mut skirmish.players,
            );
        }
    }

    /// A carrier's wreck is the escape origin, under the bail-out row for its
    /// kind (`host_kind`: ground or air). Passengers are kept out
    /// of combat by `on_field` until this runs, so the killing burst cannot
    /// spend damage rolls on them. The carrier hull is omitted from the
    /// occupancy (its wreck is ignored) while every other body remains solid.
    pub(super) fn bail_out_carrier(
        &mut self,
        carrier: UnitId,
        rest: V2,
        wreck: PropId,
        source: Option<damage::LethalSource>,
        destroyed: &mut Vec<damage::UnitDeath>,
        host_kind: contract::scenario::BailOutHost,
    ) {
        let passengers = crate::carriage::passengers(&self.units, carrier);
        if passengers.is_empty() {
            return;
        }
        let (hulls, mut soldiers) = crate::arrangement::ground_taken(&self.units, carrier);
        let ignored_props = [wreck];
        let rule = crate::bail_out::row(&self.rules, host_kind);
        for index in passengers {
            for member in self.units[index]
                .members
                .iter_mut()
                .filter(|member| member.alive())
            {
                member.position = v3(rest.x, rest.y, member.position.z);
            }
            self.units[index].position = v3(rest.x, rest.y, self.units[index].position.z);
            let host = self.units[index]
                .hold
                .as_ref()
                .expect("carrier passenger hold")
                .host;
            let out = crate::bail_out::apply(
                &self.world,
                &mut self.units[index],
                host,
                rule,
                self.seed,
                self.tick,
                crate::bail_out::Occupancy {
                    radius: self.rules.physics.soldier_radius_m,
                    hulls: &hulls,
                    soldiers: &soldiers,
                    ignored_props: &ignored_props,
                },
            );
            let unit = &mut self.units[index];
            unit.hold = None;
            if matches!(unit.orders.front(), Some(UnitOrder::Board { .. })) {
                unit.orders.pop_front();
            }
            if out == 0 {
                unit.orders.clear();
                unit.route = None;
                destroyed.push(damage::UnitDeath {
                    victim: unit.id,
                    source,
                });
            } else {
                unit.settle();
                unit.suppression = unit.suppression.max(self.rules.suppression.pinned.level);
                unit.suppressed_at = self.tick;
                soldiers.extend(unit.member_positions().map(|position| position.xy()));
            }
        }
    }

    /// Return-fire permission lasts only while the side still knows the attacker.
    pub(super) fn prune_attackers(&mut self) {
        for unit in &mut self.units {
            let knowledge = &self.knowledge[unit.side.index()];
            unit.attackers.retain(|&a| {
                knowledge.track(a).is_some() || knowledge.all_contacts().any(|c| c.emitter == a)
            });
        }
    }
}
