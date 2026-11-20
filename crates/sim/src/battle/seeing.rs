//! Each side's sight: its sensors identify what they see, and its fog sweep
//! lights the ground in view and learns the bodies standing there.

use super::*;

impl Battle {
    /// This side's own sensors, folded into its knowledge: the observers
    /// due this tick (`all`: every one) identify afresh, the rest keep their
    /// last sightings ([`sensing::SENSE_EVERY`]).
    pub(super) fn sense(&mut self, side: Side, all: bool) {
        let tick = self.tick;
        let due = |u: &Unit| all || sensing::due(u, tick);
        let units = &self.units;
        let mut sightings: Vec<Sighting> = self.sightings[side.index()]
            .iter()
            .filter(|s| !due(&units[s.observer.0 as usize]))
            .filter_map(|s| sensing::kept(s, units, &self.rules))
            .collect();
        sightings.extend(sensing::evaluate(
            &self.world,
            units,
            &self.windows,
            &self.rules,
            side,
            due,
        ));
        sightings.sort_by_key(|s| (s.observer, s.target));
        let identified = self.knowledge[side.index()].update(tick, &sightings, units, &self.rules);
        self.knowledge[side.index()].update_radar(tick, side, units, &self.rules);
        self.sightings[side.index()] = sightings;
        // An attack on an area whose cause is now identified carries over to
        // that enemy, queued ones too, as if the player had right-clicked it.
        for (area, enemy) in identified {
            for unit in self.units.iter_mut().filter(|u| u.side == side) {
                for order in &mut unit.orders {
                    match order {
                        UnitOrder::Attack { target, .. } if *target == Target::Contact(area) => {
                            *target = Target::Unit(enemy);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Recompute what ground this side sees, and learn any new obstacle in view.
    pub(super) fn sweep_fog(&mut self, side: Side) {
        self.sweep_fog_profiled(side, &mut |_| {});
    }

    pub(super) fn sweep_fog_profiled(&mut self, side: Side, completed: &mut impl FnMut(TickPhase)) {
        self.occlusion
            .retain_observers(|id| self.units[id.0 as usize].on_field());
        let mut field = self.occlusion.field();
        let mut candidates = Vec::new();
        let mut remembered = Vec::new();
        let mut views = Vec::new();
        for unit in self.units.iter().filter(|u| u.side == side && u.on_field()) {
            // What doesn't see the ground never lights its fog; the optical
            // sensing pass still handles air targets separately.
            if !sensing::sees_ground(unit) {
                continue;
            }
            let sight = sight::of(unit, &self.rules);
            let eyes = sensing::eyes(unit, &self.windows, &self.rules);
            for eye in &eyes {
                // A ray can step one cell past its reach, and the marked cell
                // can contain a footprint sample a diagonal farther away.
                views.push((eye.xy(), sight.max_range() + 3.0 * field.cell_m));
            }
            self.occlusion.sweep_observer(
                &self.world,
                &self.rules.sensors,
                unit.id,
                &eyes,
                &sight,
                &mut field,
            );
        }
        // Only bodies not as the map placed them can be news to a side.
        self.world.altered_ids_near_many(&views, &mut candidates);
        self.sides[side.index()].standing_ids_near_many(&views, &mut remembered);
        completed(TickPhase::Fog);
        // Enemy fallen in view are remembered, and the ground in view learned.
        let knowledge = &mut self.knowledge[side.index()];
        knowledge.learn_ground(&self.ground, &field);
        for f in self.fallen.values() {
            if field.visible(f.at[0], f.at[1]) {
                knowledge.learn_fallen(f.prop);
            }
        }
        for u in self.units.iter().filter(|u| u.side != side) {
            for s in &u.members {
                if let Some(fallen) = s.corpse.filter(|f| field.visible(f.at.x, f.at.y)) {
                    knowledge.note_corpse(s.id, fallen);
                }
            }
        }
        // Bodies in view are learned where they stand: new ones, and known
        // ones seen moved (L1, L2); trees seen fallen are gone (Q16).
        let known = &mut self.sides[side.index()];
        let revealed: std::collections::BTreeSet<_> = candidates
            .iter()
            .filter_map(|&id| self.world.prop(id))
            .chain(remembered.iter().filter_map(|id| known.standing().get(id)))
            .filter_map(|p| self.world.building_of(p.id).map(|owner| (owner, p)))
            .filter(|(_, p)| footprint_seen(&field, p))
            .map(|(owner, _)| owner)
            .collect();
        // Seeing one physical part reveals the entire aggregate, including
        // parts beyond the eye's reach. Keep learning in ascending prop order.
        candidates.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.current_building_parts(owner).iter().copied()),
        );
        candidates.sort_unstable();
        candidates.dedup();
        remembered.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.historical_building_parts(owner).iter().copied()),
        );
        remembered.sort_unstable();
        remembered.dedup();
        let fallen: Vec<PropId> = remembered
            .into_iter()
            .filter_map(|id| known.standing().get(&id))
            .filter(|p| {
                footprint_seen(&field, p)
                    || self
                        .world
                        .building_of(p.id)
                        .is_some_and(|id| revealed.contains(&id))
            })
            .map(|p| p.id)
            .collect();
        for id in fallen {
            known.saw_fallen(id);
        }
        let relearn = self.rules.pushing.relearn_m;
        for prop in candidates.into_iter().filter_map(|id| self.world.prop(id)) {
            if (prop.id >= self.authored_props || known.seen.contains_key(&prop.id))
                && (footprint_seen(&field, prop)
                    || self
                        .world
                        .building_of(prop.id)
                        .is_some_and(|id| revealed.contains(&id)))
            {
                let resting = self.world.resting(prop.id, self.tick);
                known.learn(prop, self.authored_props, relearn, resting);
            }
        }
        self.fog[side.index()] = field;
    }
}

/// Whether any fog cell under a prop's footprint is seen: large remains
/// (a ruin) are learned when any of them is in view, not only their middle.
fn footprint_seen(field: &VisibilityField, prop: &crate::world::Prop) -> bool {
    let step = field.cell_m / 2.0;
    let rotation = Rotation::new(prop.yaw);
    let (nx, ny) = (
        (2.0 * prop.half.x / step).ceil().max(1.0) as usize,
        (2.0 * prop.half.y / step).ceil().max(1.0) as usize,
    );
    (0..=nx).any(|i| {
        (0..=ny).any(|j| {
            let local = v2(
                -prop.half.x + 2.0 * prop.half.x * i as f64 / nx as f64,
                -prop.half.y + 2.0 * prop.half.y * j as f64 / ny as f64,
            );
            let p = prop.center + rotation.apply(local);
            field.visible(p.x, p.y)
        })
    })
}
