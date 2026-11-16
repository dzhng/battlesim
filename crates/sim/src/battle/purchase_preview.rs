//! A purchase's placement on the buying side's known ground, and its
//! admission against the buyer's credits and unit cap.

use super::*;

impl Battle {
    /// Preview placement on this side's known map without advancing time,
    /// recording a command, changing orders, or warming live navigation caches.
    /// Admission uses side knowledge, so an unseen enemy body cannot reject a ghost.
    ///
    /// With a `carrier`, the squad `variant` is bought aboard it: the pair is
    /// refused unless the carrier seats every soldier, and the placement is
    /// the carrier's, since it is the unit that enters and drives there.
    pub fn preview_purchase(
        &mut self,
        side: Side,
        variant: &str,
        carrier: Option<&str>,
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
        let kind = self.sold(side, variant)?;
        let carrier = carrier.map(|c| self.sold(side, c)).transpose()?;
        if let Some(carrier) = carrier {
            self.seats_squad(kind, carrier)?;
        }
        if !destination.iter().all(|n| n.is_finite())
            || self
                .world
                .height_at(destination[0], destination[1])
                .is_none()
        {
            return Err(OrderError::OutOfBounds);
        }
        let arriving = carrier.unwrap_or(kind);
        let fixed_wing = matches!(
            units::motion(self.rules.catalog.get(arriving), &self.rules),
            units::Motion::Air(f) if f.model == contract::catalog::FlightModel::FixedWing
        );
        let entry_index = if fixed_wing {
            crate::entries::centre(&sites, side)
        } else {
            self.select_entry(side, arriving, destination)
        };
        let entry = crate::entries::site(&sites, entry_index);
        let (from, yaw) = (v2(entry.center[0], entry.center[1]), entry.yaw);
        let t = self.rules.catalog.get(arriving);
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
        // A squad aboard stands nowhere until it is unloaded.
        let soldiers = if t.hull().is_some() {
            0
        } else {
            t.slots().map_or(0, |s| s.len())
        };
        let next = self.units.len() as u32;
        Ok(contract::skirmish::PurchasePreview {
            kind,
            carrier,
            entry: entry_index.0,
            destination,
            // A look while it is placed, not where its soldiers will stand: one
            // draw wherever it is put, so the ghost slides with the cursor.
            spots: self.arrival_spots(side, destination, soldiers, next, 0),
            area_radius_m: (soldiers > 0).then(|| crate::cover::area_radius(&self.rules, t)),
        })
    }

    /// The card `variant`, if the side's faction sells it.
    fn sold(&self, side: Side, variant: &str) -> Result<TypeIndex, OrderError> {
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
        let faction = self
            .skirmish
            .as_ref()
            .and_then(|_| self.player_faction(side))
            .ok_or(OrderError::WrongFaction)?;
        if !card.roster.factions.contains(&faction) {
            return Err(OrderError::WrongFaction);
        }
        Ok(kind)
    }

    /// Whether `carrier` seats every soldier of the squad `kind`. Nothing
    /// checks seats for a squad that is already aboard, so a pair is admitted
    /// only when they fit, as boarding would insist.
    fn seats_squad(&self, kind: TypeIndex, carrier: TypeIndex) -> Result<(), OrderError> {
        let size = self.rules.catalog.get(kind).squad_size();
        if size == 0 || crate::carriage::seats(&self.rules, carrier) == 0 {
            return Err(OrderError::NotACarrier);
        }
        if !crate::carriage::seats_squad(&self.rules, carrier, size) {
            return Err(OrderError::CapacityFull);
        }
        Ok(())
    }

    /// Reserve a confirmed purchase: its place under the player's cap (two
    /// for a squad bought in a carrier) and every unit's price.
    pub(super) fn reserve_purchase(
        &mut self,
        command: &CommandEnvelope,
        player: usize,
        planning_tick: Tick,
    ) -> Result<(), OrderError> {
        let Order::ConfirmPurchase {
            variant,
            carrier,
            destination,
        } = &command.order
        else {
            unreachable!("only a confirmed purchase is reserved")
        };
        let preview =
            self.preview_purchase(command.side, variant, carrier.as_deref(), *destination)?;
        let living = self
            .units
            .iter()
            .filter(|u| {
                u.side == command.side
                    && u.alive()
                    && u.owner.as_deref() == command.player.as_deref()
            })
            .count() as u32;
        let cost = |kind: TypeIndex| self.rules.catalog.get(kind).cost;
        let purchase = crate::skirmish::Purchase {
            kind: preview.kind,
            price: cost(preview.kind),
            carrier: preview.carrier.map(|kind| (kind, cost(kind))),
        };
        self.skirmish.as_mut().unwrap().reserve_at(
            player,
            purchase,
            preview.destination,
            (crate::entries::EntryIndex(preview.entry), planning_tick + 1),
            living,
        )?;
        Ok(())
    }
}
