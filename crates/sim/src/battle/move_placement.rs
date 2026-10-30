//! Move destinations on the ordering side's known ground.

use super::*;

impl Battle {
    /// Where `count` soldiers stand round `centre` as a squad arriving there
    /// spreads out on the ground `side` knows, before cover moves them, by
    /// the arrangement `draw` ([`arrangement::rng`]). It changes no state.
    pub(super) fn arrival_spots(
        &self,
        side: Side,
        centre: [f64; 2],
        count: usize,
        unit: u32,
        draw: u64,
    ) -> Vec<[f64; 2]> {
        if count == 0 {
            return vec![];
        }
        let known = &self.sides[side.index()];
        let solid = |p: &crate::world::Prop| {
            p.blocks(MoverClass::Infantry) && known.knows(p, self.authored_props)
        };
        let centre = v2(centre[0], centre[1]);
        let mut draws = arrangement::rng(self.seed, unit, draw);
        arrangement::squad_spots(
            &self.world,
            centre,
            count,
            &self.rules.infantry_movement,
            self.rules.physics.soldier_radius_m,
            &solid,
            &mut draws,
        )
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect()
    }

    pub fn preview_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<Vec<contract::command::MoveDestination>, OrderError> {
        self.place_move(side, request)
            .map(|(destinations, _)| destinations)
    }

    /// Each unit's destination, and where a hull sent onto ground it has no
    /// room on stops short of it instead, if the room nearest is a long way
    /// round ([`crate::units::StopShort`]).
    pub(super) fn place_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<
        (
            Vec<contract::command::MoveDestination>,
            BTreeMap<UnitId, crate::units::StopShort>,
        ),
        OrderError,
    > {
        let ids = &request.units;
        let goal = request.goal;
        let facing = request.facing;
        self.validate_units(side, ids)?;
        if self.world.height_at(goal[0], goal[1]).is_none()
            || facing.is_some_and(|f| !f.is_finite())
        {
            return Err(OrderError::OutOfBounds);
        }
        let members: Vec<_> = ids
            .iter()
            .map(|&id| {
                let u = &self.units[id.0 as usize];
                crate::formation::Member {
                    id,
                    position: u.position.xy(),
                    yaw: u.yaw,
                    radius: crate::formation::member_radius(&self.rules, u.unit_type(&self.rules)),
                }
            })
            .collect();
        let mut known = self.sides[side.index()].clone();
        let grid = known.grid(&self.world, self.authored_props);
        let pockets = crate::navigation::Pockets::default();
        // Per unit, the last point placed for it and where it stops short.
        let shorts = std::cell::RefCell::new(BTreeMap::new());
        // A queued leg sets off from where the queue ends.
        let from = |u: &Unit| {
            u.orders
                .iter()
                .rev()
                .filter(|_| request.queued)
                .find_map(|o| o.movement())
                .map_or(u.position.xy(), |m| m.destination)
        };
        // A hull that turns on the spot parks facing as ordered.
        let parking = |u: &Unit| {
            let pivots = u.ground().drive.is_some_and(|d| d.tracked);
            u.hull.map(|h| crate::navigation::Parking {
                half: h.xy(),
                facing: request.facing.filter(|_| pivots),
            })
        };
        // Formation alternatives may avoid a pocket beside a reachable goal,
        // but must not rescue an order to disconnected ground. Resolve the
        // goal as before: hulls may park nearby; a point with no standing room
        // still needs the formation search to find room in the first place.
        let unreachable: BTreeSet<_> = ids
            .iter()
            .copied()
            .filter(|id| {
                let u = &self.units[id.0 as usize];
                !u.airborne()
                    && grid
                        .destination_point(
                            v2(goal[0], goal[1]),
                            u.ground(),
                            parking(u),
                            from(u),
                            &pockets,
                        )
                        .is_some_and(|p| !grid.reaches(from(u), p, u.ground(), &pockets))
            })
            .collect();
        let plan = crate::formation::place(
            &members,
            v2(goal[0], goal[1]),
            facing,
            &self.rules.formation,
            libm::hypot(self.world.width(), self.world.depth()),
            |id, p| {
                if unreachable.contains(&id) {
                    return None;
                }
                let u = &self.units[id.0 as usize];
                // An aircraft hovers over its point, whatever stands there.
                if u.airborne() {
                    shorts.borrow_mut().insert(id, (p, None));
                    return Some(p);
                }
                let hull = parking(u);
                let placed = grid.destination_point(p, u.ground(), hull, from(u), &pockets)?;
                if !grid.reaches(from(u), placed, u.ground(), &pockets) {
                    return None;
                }
                let short = hull
                    .filter(|_| placed != p)
                    .and_then(|hull| grid.room_on_the_way(p, u.ground(), hull, from(u), &pockets))
                    .filter(|&at| at != placed)
                    .map(|at| crate::units::StopShort {
                        at,
                        detour_m: self.rules.navigation.stop_short_detour_ratio
                            * ((at - p).length() - (placed - p).length()).max(0.0),
                        nearest: None,
                    });
                shorts.borrow_mut().insert(id, (placed, short));
                Some(placed)
            },
        );
        // A placed marker is a place to stand that its unit can reach on the
        // ground its side knows; the way there is found and kept on the move.
        let source = self.move_source(side);
        let shorts = shorts.into_inner();
        // Where each placed unit stops short, if it was placed where that
        // was worked out for.
        let stops = plan
            .slots
            .iter()
            .filter_map(|slot| {
                let (placed, short) = shorts.get(&slot.id)?;
                (slot.point == Some(*placed)).then(|| Some((slot.id, short.clone()?)))?
            })
            .collect();
        let destinations = plan
            .slots
            .into_iter()
            .map(|slot| {
                let u = &self.units[slot.id.0 as usize];
                // An attack has no end to set a queued leg off from.
                let after_attack = request.queued
                    && source[slot.id.0 as usize]
                        .orders
                        .iter()
                        .any(|o| matches!(o, UnitOrder::Attack { .. }));
                let slot_point = slot.point.filter(|_| !after_attack);
                let goal = slot_point.unwrap_or(u.position.xy());
                let facing =
                    movement::final_yaw(u, request.facing, from(u), goal, request.direction);
                let living = if u.hull.is_some() {
                    0
                } else {
                    u.members.iter().filter(|s| s.alive()).count()
                };
                contract::command::MoveDestination {
                    unit: slot.id,
                    goal: [goal.x, goal.y],
                    placed: slot_point.is_some(),
                    facing: facing.unwrap_or(u.yaw),
                    spots: self.arrival_spots(
                        side,
                        [goal.x, goal.y],
                        living,
                        slot.id.0,
                        arrangement::arrival(goal),
                    ),
                }
            })
            .collect();
        Ok((destinations, stops))
    }
}
