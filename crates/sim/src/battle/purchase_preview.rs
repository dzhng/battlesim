//! A purchase's placement on the buying side's known ground.

use super::*;

impl Battle {
    /// Preview placement on this side's known map without advancing time,
    /// recording a command, changing orders, or warming live navigation caches.
    /// Admission uses side knowledge, so an unseen enemy body cannot reject a ghost.
    pub fn preview_purchase(
        &mut self,
        side: Side,
        variant: &str,
        destination: [f64; 2],
    ) -> Result<contract::skirmish::PurchasePreview, OrderError> {
        if side == Side::Neutral {
            return Err(OrderError::NotSkirmish);
        }
        let sites = self
            .skirmish
            .as_ref()
            .ok_or(OrderError::NotSkirmish)?
            .setup
            .sites
            .clone();
        let kind = self
            .rules
            .catalog
            .index(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        let card = self
            .rules
            .catalog
            .card(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        if !card.roster.factions.contains(
            &self
                .skirmish
                .as_ref()
                .and_then(|_| self.player_faction(side))
                .ok_or(OrderError::WrongFaction)?,
        ) {
            return Err(OrderError::WrongFaction);
        }
        if !destination.iter().all(|n| n.is_finite())
            || self
                .world
                .height_at(destination[0], destination[1])
                .is_none()
        {
            return Err(OrderError::OutOfBounds);
        }
        let fixed_wing = matches!(
            units::motion(self.rules.catalog.get(kind), &self.rules),
            units::Motion::Air(f) if f.model == contract::catalog::FlightModel::FixedWing
        );
        let entry_index = if fixed_wing {
            crate::entries::centre(&sites, side)
        } else {
            self.select_entry(side, kind, destination)
        };
        let entry = crate::entries::site(&sites, entry_index);
        let (from, yaw) = (v2(entry.center[0], entry.center[1]), entry.yaw);
        let t = self.rules.catalog.get(kind);
        // A ground unit is sent where a move there from its entry would end:
        // the nearest ground to the point that it stands on, as its side knows
        // the map. An aircraft's destination is any point it can fly over.
        let destination = match units::ground_mobility(t, &self.rules) {
            None => destination,
            Some(mobility) => {
                let mut known = self.sides[side.index()].clone();
                let grid = known.grid(&self.world, self.authored_props);
                let pockets = crate::navigation::Pockets::default();
                let hull = t.hull().map(|h| crate::navigation::Parking {
                    half: v2(h.half_extents_m[0], h.half_extents_m[1]),
                    facing: None,
                });
                let member = crate::formation::Member {
                    id: UnitId(self.units.len() as u32),
                    position: from,
                    radius: crate::formation::member_radius(&self.rules, t),
                    yaw,
                };
                let plan = crate::formation::place(
                    &[member],
                    v2(destination[0], destination[1]),
                    None,
                    &self.rules.formation,
                    libm::hypot(self.world.width(), self.world.depth()),
                    |_, p| grid.destination_point(p, &mobility, hull, from, &pockets),
                );
                let placed = plan.slots[0].point.ok_or(OrderError::NoValidDestination)?;
                [placed.x, placed.y]
            }
        };
        let soldiers = if t.hull().is_some() {
            0
        } else {
            t.slots().map_or(0, |s| s.len())
        };
        let next = self.units.len() as u32;
        Ok(contract::skirmish::PurchasePreview {
            kind,
            entry: entry_index.0,
            destination,
            // A look while it is placed, not where its soldiers will stand: one
            // draw wherever it is put, so the ghost slides with the cursor.
            spots: self.arrival_spots(side, destination, soldiers, next, 0),
            area_radius_m: (soldiers > 0).then(|| crate::cover::area_radius(&self.rules, t)),
        })
    }
}
