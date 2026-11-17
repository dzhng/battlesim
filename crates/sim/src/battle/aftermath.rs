//! The ground and structures left by movement and physical fire.

use super::*;

impl Battle {
    /// Move the bodies vehicles shoved this tick (Q2). A side that has not
    /// seen an authored body move keeps it where it stood (L1); the soldiers
    /// a body now covers step out of it.
    pub(super) fn shove_props(&mut self, shoves: Vec<movement::Shove>) {
        self.shoved.clear();
        for s in shoves {
            let Some(prop) = self.world.prop(s.prop).cloned() else {
                continue;
            };
            if prop.body.topples {
                self.knock_down(prop, s.by, s.heading);
                continue;
            }
            for side in &mut self.sides {
                side.before_move(&prop, self.authored_props);
            }
            self.world.move_prop(s.prop, s.center, s.yaw, self.tick);
            if let Some(moved) = self.world.prop(s.prop).map(|p| p.footprint()) {
                let r = self.rules.physics.soldier_radius_m;
                movement::clear_of(&self.world, &mut self.units, &moved, r);
                self.shoved.push(s);
            }
        }
    }

    /// A vehicle of side `by`, driving along `heading`, knocked a tree down
    /// (Q16): its body goes and it falls the way the vehicle drove. Its side
    /// replans without it and knows the fall at once (contact). Every other
    /// side keeps it standing until it sees the ground where it stood (L1).
    pub(super) fn knock_down(&mut self, prop: crate::world::Prop, by: Side, heading: V2) {
        self.world.knock_down(prop.id);
        self.keep_standing(&prop, Some(by));
        self.fell(&prop, heading);
        self.knowledge[by.index()].learn_fallen(prop.id);
    }

    /// Record that toppling `prop` went down toward `toward` this tick. A
    /// degenerate direction (a burst at the foot) falls back to one from the
    /// body's id, so the record is always a unit vector and deterministic.
    pub(super) fn fell(&mut self, prop: &crate::world::Prop, toward: V2) {
        let len = toward.length();
        let toward = if len > 1e-9 {
            toward * (1.0 / len)
        } else {
            // No trigonometry: it must match bit for bit in Wasm.
            const D: f64 = std::f64::consts::FRAC_1_SQRT_2;
            const COMPASS: [(f64, f64); 8] = [
                (1.0, 0.0),
                (D, D),
                (0.0, 1.0),
                (-D, D),
                (-1.0, 0.0),
                (-D, -D),
                (0.0, -1.0),
                (D, -D),
            ];
            let (x, y) = COMPASS[prop.id as usize % 8];
            v2(x, y)
        };
        self.fallen.insert(
            prop.id,
            FallenBody {
                prop: prop.id,
                at: [prop.center.x, prop.center.y],
                toward: [toward.x, toward.y],
                tick: self.tick,
            },
        );
    }

    /// `prop` is gone: side `by` (the one that touched it) replans without
    /// it at once; every other side that planned with it keeps it standing
    /// until it sees where it stood (L1).
    pub(super) fn keep_standing(&mut self, prop: &crate::world::Prop, by: Option<Side>) {
        for side in Side::ALL {
            let known = &mut self.sides[side.index()];
            if Some(side) == by {
                known.forget(prop);
            } else if prop.id < self.authored_props || known.seen.contains_key(&prop.id) {
                known.keep_standing(prop.clone());
            }
        }
    }

    /// Destroy a prop whose integrity ran out (Q17) into its row's destroyed
    /// state: a tree falls and its spot becomes open ground, like a lane a
    /// tank knocks through; a crate is removed; sandbags, a wall, a building
    /// or a wreck leave their remains on the same plan (a building's
    /// occupants escape its collapse, L10). No side learns of it by contact:
    /// each sees it gone once it sees where it stood. A tree falls `toward`
    /// the way the felling blow pushed. Returns units destroyed.
    pub(super) fn destroy_prop(&mut self, id: PropId, toward: V2) -> Vec<UnitId> {
        let Some(owner) = self.world.structure_owner(id) else {
            return Vec::new();
        };
        let parts = self.world.structure_parts(owner);
        let replacements: Vec<_> = parts
            .iter()
            .copied()
            .filter_map(|part| self.destroy_part(part, toward).map(|new| (part, new)))
            .collect();
        self.world.replace_building_parts(owner, &replacements);
        // All shells/remains are final before any occupant attempts escape.
        garrison::collapse(
            &self.world,
            &mut self.units,
            owner,
            &self.rules,
            self.seed,
            self.tick,
            &parts,
        )
    }

    pub(super) fn destroy_part(&mut self, id: PropId, toward: V2) -> Option<PropId> {
        let prop = self.world.prop(id).cloned()?;
        let state = self.world.prop_type(prop.kind).destroyed.clone()?;
        self.keep_standing(&prop, None);
        // Whatever it leaves, a toppling body falls the way it was pushed.
        if prop.body.topples {
            self.fell(&prop, toward);
        }
        match state {
            Destroyed::Cleared => {
                self.world.knock_down(id);
                // The tree's own share of the forest, out to its spacing.
                let reach = if prop.forest_tree {
                    self.world.forest_rule().trunk_spacing_m
                } else {
                    self.world.cleared_cell_m()
                };
                for cell in self.world.clear_spot(prop.center, reach) {
                    self.ground.clear(cell);
                }
                None
            }
            Destroyed::Removed => {
                self.world.remove_prop(id);
                self.structures.note_removed(prop);
                None
            }
            Destroyed::Into {
                prop: remains,
                mut height_m,
                building,
            } => {
                let mut remains = remains;
                if let Some(rule) = building {
                    if let Some(definition) = self.world.building(id) {
                        let geometry = &definition.geometry;
                        if geometry.floor_z.len() > rule.collapse_max_floors {
                            remains = rule.gutted_prop;
                            height_m = 2.0 * prop.half.z;
                        } else {
                            height_m = (geometry.height_m * rule.height_fraction)
                                .clamp(height_m, rule.max_height_m);
                        }
                    }
                }
                self.world.remove_prop(id);
                let remains = self.world.add_replacement(
                    &PropDefinition {
                        kind: remains,
                        center: [prop.center.x, prop.center.y],
                        yaw: prop.yaw,
                        half_extents: [prop.half.x, prop.half.y, height_m / 2.0],
                        base_z: Some(prop.base_z),
                        // A lighter wreck is still the same unit's.
                        wreck_of: prop.wreck_of.map(|t| self.rules.catalog.id(t).to_string()),
                    },
                    id,
                );
                self.structures.note_replaced(remains, id);
                // Remains that still close an authored body's footprint to
                // every mover it stopped are planned with by every side, as
                // the body was: a fall a side never saw cannot open a route.
                let closes =
                    |c| !prop.blocks(c) || self.world.prop(remains).is_some_and(|r| r.blocks(c));
                if id < self.authored_props && MoverClass::ALL.into_iter().all(closes) {
                    self.world.set_known_to_all(remains);
                }
                Some(remains)
            }
        }
    }

    /// Every vehicle that knocks trees down and moved this tick clears the
    /// forest ground its hull (and `ground.lane_margin_m` to either side) has left
    /// behind: the lane stops being forest (Q16), with crushed-ground marks.
    /// The ground under the hull itself stays forest until it has passed, so
    /// carving a lane goes at forest speed and only the lane is open ground.
    pub(super) fn clear_lanes(&mut self, before: &Poses) {
        // A forest is its trees: what knocks them down is its tree's weight.
        let tree = self.world.types().by_id(&self.rules.forests.tree).body;
        for (u, was) in self.units.iter().zip(&before.units) {
            let (Some(hull), Some(h)) = (u.ground_footprint(), u.hull) else {
                continue;
            };
            let knocks = tree.topples && u.ground().push.pushes(tree.weight_class);
            if !u.alive() || !knocks || (was.base - u.position).length() <= 1e-9 {
                continue;
            }
            if !self.world.forest_near(
                hull.center,
                hull.half.length() + self.rules.ground.lane_margin_m,
            ) {
                continue;
            }
            let left = Obb2 {
                center: was.base.xy(),
                yaw: was.yaw,
                // Wider, not longer: a margin ahead would clear the ground
                // the hull is about to cover.
                half: h.xy() + v2(0.0, self.rules.ground.lane_margin_m),
            };
            for cell in self.world.clear(&left, &hull) {
                self.ground.clear(cell);
            }
        }
    }

    /// The lab emitter: a round of `weapon` bursts on the ground at `point`,
    /// marking it and wearing down the props its blast reaches.
    pub(super) fn burst_event(&mut self, point: [f64; 2], weapon: &str) {
        let w = self
            .arsenal
            .weapons
            .iter()
            .find(|w| w.id == weapon)
            .expect("burst weapons are checked at setup");
        let Some(z) = self.world.height_at(point[0], point[1]) else {
            return;
        };
        let at = v3(point[0], point[1], z);
        // Its blast wears the props it reaches down, as a round's would (Q17).
        let mut structural = Vec::new();
        damage::blast_props(&self.world, &w.def, at, None, None, &mut structural);
        self.ground
            .burst(&self.world, at, w.def.blast_radius_m, &self.rules.ground);
        for hit in structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                self.destroy_prop(hit.prop, hit.toward);
            }
        }
    }
}
