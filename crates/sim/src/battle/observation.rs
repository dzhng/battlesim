//! Side observations and the knowledge each publication may expose.

use super::*;

impl Battle {
    /// A soldier's resolved place and cover (D2+): his spot while moving, his
    /// post while holding, else where he stands; the cover he has now against
    /// his squad's threat, in the true world as rounds meet it, and the tier
    /// his place was resolved to give. A garrison shelters instead (Q22).
    pub(super) fn member_order(
        world: &WorldGeometry,
        ground: &GroundLayer,
        rules: &Rules,
        u: &Unit,
        s: &Soldier,
        hulls: &[cover::Body],
    ) -> MemberOrder {
        let here = s.position.xy();
        let spot = s.spot.or(s.post).unwrap_or(here);
        let sheltered = u.hold.is_some();
        let cover_now = u
            .cover
            .threat
            .filter(|_| !sheltered)
            .and_then(|t| cover::at(world, ground, hulls, rules, here, t));
        MemberOrder {
            spot: [spot.x, spot.y],
            cover_now,
            cover_there: s.cover.filter(|_| !sheltered),
        }
    }

    pub(super) fn observe_all(&mut self) {
        let identified = Side::ALL
            .into_iter()
            .flat_map(|side| self.knowledge[side.index()].identified_now(self.tick))
            .map(|(id, _)| (id, self.units[id.0 as usize].layer()))
            .collect();
        let near = std::mem::take(&mut self.near_rounds);
        self.shout_evidence = if self.tick == 0 {
            ShoutEvidence::default()
        } else {
            let physical = shouts::Physical {
                under_fire: near.under_fire,
                incoming: shouts::incoming(
                    &self.world,
                    self.projectiles.active(),
                    &self.rounds,
                    &self.units,
                    &self.rules,
                ),
            };
            shouts::evidence(
                &self.shout_before,
                &self.units,
                &self.world,
                &physical,
                std::mem::take(&mut self.shout_fired),
                identified,
                &self.rules,
            )
        };
        self.shout_evidence.kills.append(&mut self.shout_kills);
        let own_shouts = Side::ALL.map(|side| {
            shouts::build(
                side,
                &self.shout_evidence,
                &self.units,
                &self.knowledge[side.index()],
                &self.rules,
                self.tick,
                &mut self.shouts[side.index()],
            )
        });
        // Every live hull a soldier's current cover may lie behind (D2+).
        let hulls = cover::hull_bodies(&crate::lean::hulls(&self.units, &self.rules));
        let low_hover = crate::movement::air::low_hover(&self.rules);
        for side in Side::ALL {
            let mut steps = self.own_steps(side).into_iter();
            let knowledge = &self.knowledge[side.index()];
            let spotted_by_visible_enemy: BTreeSet<UnitId> = Side::ALL
                .into_iter()
                .filter(|other| *other != side)
                .flat_map(|other| self.sightings[other.index()].iter())
                .filter(|s| {
                    knowledge
                        .track(s.observer)
                        .is_some_and(|t| t.last_seen == self.tick)
                })
                .map(|s| s.target)
                .collect();
            let fog = &self.fog[side.index()];
            let sees_round = |layer, point: V3| match layer {
                AltitudeLayer::Ground => fog.visible(point.x, point.y),
                _ => sensing::side_sees_round(
                    &self.world,
                    &self.rules,
                    &self.units,
                    &self.windows,
                    side,
                    point,
                    layer,
                ),
            };
            let player_ids = self.player_ids.clone();
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.identified.clear();
            frame.identified.extend(knowledge.identified(
                self.tick,
                &self.units,
                &self.windows,
                &self.rules,
            ));
            frame.ground_visibility.clone_from(fog);
            frame.smoke_clouds.clear();
            let seen = crate::smoke::seen(self.world.smoke(), side, fog);
            frame
                .smoke_clouds
                .extend(seen.map(|(p, _)| p.cloud.clone()));
            frame.contacts.clear();
            // An area hangs in the air when its cause flew higher over the
            // ground than the low hover (D33).
            let world = &self.world;
            frame.contacts.extend(knowledge.contacts(|c| {
                c.layer != AltitudeLayer::Ground
                    && c.center.z - world.height_at(c.center.x, c.center.y).unwrap_or(0.0)
                        > low_hover
            }));
            frame.audible.clone_from(&self.audible[side.index()]);
            // Refusals are transient evidence for this completed tick.
            frame.refusals = std::mem::take(&mut self.refusals[side.index()]);
            frame.shouts.clone_from(&own_shouts[side.index()]);
            for other in Side::ALL.into_iter().filter(|other| *other != side) {
                frame.shouts.extend(shouts::heard(
                    &own_shouts[other.index()],
                    knowledge,
                    self.tick,
                ));
                self.shouted_unseen[side.index()].extend(shouts::unheard(
                    &own_shouts[other.index()],
                    knowledge,
                    self.tick,
                ));
            }
            frame.known_props.clear();
            let known = &self.sides[side.index()];
            // Planning and publication share the complete remembered pose.
            let remembered = known.seen.keys().filter_map(|&id| {
                let p = known.prop(&self.world, self.authored_props, id)?;
                let replaces = if id < self.authored_props {
                    Some(id)
                } else {
                    self.structures.replaced_by(id)
                };
                Some((p, replaces, false))
            });
            // A learned removal without a replacement only hides its authored body.
            let removed = self
                .structures
                .removed()
                .filter(|p| p.id < self.authored_props && !known.standing().contains_key(&p.id))
                .map(|p| (p.clone(), Some(p.id), true));
            for (p, replaces, destroyed) in remembered.chain(removed) {
                let building = self.world.building_of(p.id);
                frame.known_props.push(KnownProp {
                    kind: p.kind,
                    center: [p.center.x, p.center.y],
                    yaw: p.yaw,
                    half_extents: [p.half.x, p.half.y, p.half.z],
                    base_z: p.base_z,
                    replaces,
                    destroyed,
                    id: p.id,
                    building,
                    structure_owner: building
                        .filter(|_| !destroyed)
                        .map(|_| self.world.remembered_structure_owner(p.id)),
                    authored_prop: self.world.authored_prop(p.id),
                    wreck_of: p.wreck_of,
                });
            }
            // A body shoved where this side sees it (its own pusher always
            // does) is drawn where it stands, ahead of the knowledge above;
            // a metal one throws sparks where the hull grinds it.
            frame.prop_display_poses.clear();
            frame.grinds.clear();
            for shove in &self.shoved {
                let Some(p) = self.world.prop(shove.prop) else {
                    continue;
                };
                let visible = shove.by == side || fog.visible(p.center.x, p.center.y);
                if !visible {
                    continue;
                }
                let drawn = frame.prop_display_poses.iter().any(|d| d.prop_id == p.id);
                if !drawn {
                    frame.prop_display_poses.push(PropDisplayPose {
                        prop_id: p.id,
                        center: [p.center.x, p.center.y],
                        yaw: p.yaw,
                        base_z: p.base_z,
                    });
                }
                if p.body.metal {
                    frame.grinds.push(Grind {
                        point: [shove.contact.x, shove.contact.y, shove.contact.z],
                        heading: [shove.heading.x, shove.heading.y],
                    });
                }
            }
            frame.projectiles.clear();
            for round in &self.segments {
                if round.side == side {
                    frame.projectiles.push(round.whole());
                } else {
                    frame
                        .projectiles
                        .extend(clip_to_seen(round, |point| sees_round(round.layer, point)));
                }
            }
            frame.blasts.clear();
            frame.blasts.extend(
                self.blasts
                    .iter()
                    .filter(|b| {
                        let at = if b.layer == AltitudeLayer::Ground {
                            v3(b.blast.point[0], b.blast.point[1], b.blast.point[2])
                        } else {
                            b.sample
                        };
                        b.side == side || sees_round(b.layer, at)
                    })
                    .map(|b| b.blast.clone()),
            );
            let target_ref = |t| order_steps::target_ref(knowledge, t);
            frame.own.clear();
            frame.own.extend(
                self.units
                    .iter()
                    .filter(|u| u.side == side && u.alive())
                    .map(|u| OwnUnit {
                        withdrawing: u.withdrawing,
                        fuel_s: u.air.as_ref().map(|air| air.fuel_s),
                        afterburner: u.air.as_ref().is_some_and(|air| air.afterburner),
                        in_dogfight: u
                            .air
                            .as_ref()
                            .is_some_and(|air| air.dogfight_target.is_some()),
                        flares: u.flares.and_then(|f| {
                            let capability = u.unit_type(&self.rules).capabilities.flares?;
                            Some(f.readiness(&capability, self.tick, self.rules.tick_hz))
                        }),
                        protection: u.protection.as_ref().map(|p| {
                            let capabilities = &u.unit_type(&self.rules).capabilities;
                            let capability = capabilities.active_protection.as_ref().unwrap();
                            p.readiness(capability, self.tick as f64, self.rules.tick_hz)
                        }),
                        id: u.id,
                        owner: state_digest::owner_index(&player_ids, u)
                            .map(|index| contract::players::PlayerIndex(index as u16)),
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u.movement_goal().map(|(g, _)| [g.x, g.y]),
                        policy: u.movement_goal().map(|(_, p)| p),
                        direction: u.movement_goal().map(|_| u.direction()),
                        reversing: u.reversing,
                        state: u.state,
                        // An enemy it waits for is named only if the side
                        // could name it anyway (Q14: contact reveals nothing).
                        blocker: u.blocker.filter(|b| self.units[b.0 as usize].side == side),
                        route: u.route.iter().flatten().map(|p| [p.x, p.y]).collect(),
                        steps: steps.next().expect("one per own unit"),
                        members: u
                            .members
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| s.alive())
                            .map(|(k, _)| {
                                let p = self.windows.body_position(u, k, &self.rules);
                                [p.x, p.y, p.z]
                            })
                            .collect(),
                        member_ids: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.id)
                            .collect(),
                        member_slots: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.slot as u8)
                            .collect(),
                        member_active_mounts: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.active_mount.map(|m| m as u8))
                            .collect(),
                        member_orders: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| {
                                Self::member_order(
                                    &self.world,
                                    &self.ground,
                                    &self.rules,
                                    u,
                                    s,
                                    &hulls,
                                )
                            })
                            .collect(),
                        member_leans: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.leaning(self.tick).map(|l| l.published()))
                            .collect(),
                        area: u.anchor.map(|a| SquadArea {
                            anchor: [a.at.x, a.at.y],
                            radius: crate::cover::area_radius(
                                &self.rules,
                                u.unit_type(&self.rules),
                            ),
                        }),
                        final_facing: movement::final_facing(u),
                        sees: knowledge.own_sensor(u.id),
                        engagement: u.engagement,
                        mounts: u
                            .mounts
                            .iter()
                            .map(|m| {
                                weapons::readiness(
                                    &self.arsenal,
                                    u,
                                    m,
                                    m.lock.as_ref().and_then(|l| target_ref(l.target)),
                                )
                            })
                            .collect(),
                        weapon_poses: u.mounts.iter().map(weapons::pose).collect(),
                        deployment: deployment::state(u),
                        hp: u.hp,
                        smoking: u.smoking(&self.rules),
                        shot_at: near.shot_at.contains(&u.id),
                        smoke: u.smoke.map(|s| s.charges),
                        member_hp: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.hp)
                            .collect(),
                        suppression: self.rules.suppression.tier(u.suppression),
                        concealed: sensing::concealed(&self.world, u, &self.rules)
                            && !spotted_by_visible_enemy.contains(&u.id)
                            && u.attackers.is_empty(),
                        stock: u.stock,
                        service: u.service,
                        hold: hold::state(&self.world, &self.units, u, &self.windows, &self.rules),
                        sight: {
                            let s = sight::of(u, &self.rules);
                            UnitSight {
                                // The side's drawn fog follows its rules: a
                                // unit that doesn't see the ground lights none.
                                eyes: if sensing::sees_ground(u) {
                                    sensing::eyes(u, &self.windows, &self.rules)
                                        .into_iter()
                                        .map(|e| [e.x, e.y, e.z])
                                        .collect()
                                } else {
                                    Vec::new()
                                },
                                forward: s.forward,
                                shape: s.shape,
                                range: s.range,
                            }
                        },
                    }),
            );
            frame.encounter = self.encounter;
            // Economy is a deliberately binary mode. Neutral still gets a
            // normal observation stream, but it has no wallet, purchase
            // queue, or objective score to publish.
            frame.skirmish = self
                .skirmish
                .as_ref()
                .filter(|_| side != Side::Neutral)
                .map(|s| s.view(side, self.tick, self.rules.tick_hz, &s.fielded(&self.units)));
            frame.guided.clear();
            frame
                .guided
                .extend(self.projectiles.active().iter().filter_map(|p| {
                    let own = self.rounds.get(&p.id).is_some_and(|r| r.side == side);
                    p.guidance.filter(|_| own).map(|g| GuidedMissile {
                        id: p.id.0,
                        position: [p.position.x, p.position.y, p.position.z],
                        point: [g.point.x, g.point.y, g.point.z],
                        supported: g.supported,
                    })
                }));
            frame.flares.clear();
            for (from, to, layer) in self.projectiles.flares() {
                for (from, to) in flown::visible_line(from, to, |point| sees_round(layer, point)) {
                    frame.flares.push(contract::observation::Flare {
                        from: xyz(from),
                        to: xyz(to),
                    });
                }
            }
            frame.air_transits.clear();
            frame
                .air_transits
                .extend(self.air_transits[side.index()].iter().copied());
            frame.corpses.clear();
            for u in &self.units {
                for s in &u.members {
                    let own = u.side == side;
                    let fallen = s.corpse.and_then(|actual| {
                        if own {
                            Some(actual)
                        } else {
                            knowledge.corpse(s.id, actual)
                        }
                    });
                    if let Some(f) = fallen {
                        frame.corpses.push(Corpse {
                            position: [f.at.x, f.at.y, f.at.z],
                            own,
                            soldier: s.id,
                            kind: u.kind,
                            slot: s.slot as u8,
                            yaw: f.yaw,
                        });
                    }
                }
            }
            // By place, so the list never groups enemy fallen by their hidden unit.
            frame.corpses.sort_by(|a, b| {
                (a.position[0], a.position[1])
                    .partial_cmp(&(b.position[0], b.position[1]))
                    .unwrap()
            });
            frame.fallen_bodies.clear();
            frame.fallen_bodies.extend(
                self.fallen
                    .values()
                    .filter(|f| knowledge.knows_fallen(f.prop))
                    .cloned(),
            );
            // A falling airframe, to the sides that saw it go down: the
            // same sides that learn its wreck.
            frame.crashes.clear();
            frame.crashes.extend(self.crashes.iter().filter_map(|c| {
                let unit = &self.units[c.unit.0 as usize];
                c.seen_by(side, unit.side, unit.kind)
            }));
        }
    }
}
