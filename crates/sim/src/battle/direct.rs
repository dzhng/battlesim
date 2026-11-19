//! An authored scenario's director, run once a tick before orders apply:
//! each waiting beat of the open chapter whose trigger holds fires, and its
//! actions act through the battle's own owners (an order through the
//! authored-order path, credits into the wallet, purchases into the shop).

use super::*;
use contract::director::{Action, DirectedOrder, Selector, Trigger, UnitFact};
use contract::scenario_document::{credits_covering, CreditGrant};
use contract::skirmish::{OfferedPurchase, PurchaseSpec};

impl Battle {
    pub(super) fn direct(&mut self) {
        let Some(mut run) = self.director.take() else {
            return;
        };
        run.open_chapter(self.tick);
        let mut actions = Vec::new();
        for (place, beat) in run.waiting() {
            if self.holds(&run, &beat.when) {
                run.fire(place, self.tick);
                actions.extend(
                    beat.then
                        .into_iter()
                        .map(|action| (beat.id.clone(), action)),
                );
            }
        }
        for (beat, action) in actions {
            self.act(&mut run, &beat, action);
        }
        self.director = Some(run);
    }

    /// Whether `when` holds now.
    fn holds(&self, run: &DirectorRun, when: &Trigger) -> bool {
        match when {
            Trigger::ChapterOpens => true,
            Trigger::Fired { beat, after_s } => run.fired_at(beat).is_some_and(|at| {
                at + (after_s * f64::from(self.rules.tick_hz)).ceil() as Tick <= self.tick
            }),
            Trigger::Phase(phase) => self.skirmish.as_ref().is_some_and(|s| s.phase == *phase),
            Trigger::Bought { player, purchase } => self.has_bought(player, purchase),
            Trigger::Units { who, are } => self.selected(run, who).any(|unit| match are {
                UnitFact::In(zone) => {
                    unit.on_field() && inside(&run.setup.zones[zone], unit.position.xy())
                }
                UnitFact::Engagement(e) => unit.held_engagement.unwrap_or(unit.engagement) == *e,
            }),
            Trigger::Identifies { side, who } => {
                let known = &self.knowledge[side.index()];
                self.selected(run, who)
                    .any(|unit| known.track(unit.id).is_some())
            }
            Trigger::ContactIn { side, zone } => {
                let zone = &run.setup.zones[zone];
                self.knowledge[side.index()].all_contacts().any(|c| {
                    libm::hypot(c.center.x - zone.center[0], c.center.y - zone.center[1])
                        <= zone.radius_m + c.radius
                })
            }
            Trigger::All(all) => all.iter().all(|t| self.holds(run, t)),
            Trigger::Any(any) => any.iter().any(|t| self.holds(run, t)),
        }
    }

    /// The living units `who` names.
    fn selected<'a>(
        &'a self,
        run: &'a DirectorRun,
        who: &'a Selector,
    ) -> impl Iterator<Item = &'a Unit> {
        let kind = match who {
            Selector::Bought { variant, .. } => self.rules.catalog.index(variant),
            Selector::Force(_) => None,
        };
        self.units.iter().filter(move |unit| {
            unit.alive()
                && match who {
                    Selector::Force(name) => run.setup.forces.get(name) == Some(&unit.id),
                    Selector::Bought { player, .. } => {
                        unit.owner.as_deref() == Some(player.as_str()) && Some(unit.kind) == kind
                    }
                }
        })
    }

    /// Whether `player` has `purchase` on its way or on the field.
    fn has_bought(&self, player: &str, purchase: &PurchaseSpec) -> bool {
        let Some(offer) = self.offered(purchase) else {
            return false;
        };
        let Some(skirmish) = &self.skirmish else {
            return false;
        };
        let pending = skirmish.player_index(player).is_some_and(|index| {
            skirmish.players[index]
                .reservations
                .iter()
                .any(|r| r.data.kind == offer.kind && r.data.carrier == offer.carrier)
        });
        let owns = |kind| {
            self.units
                .iter()
                .any(|u| u.alive() && u.kind == kind && u.owner.as_deref() == Some(player))
        };
        pending || (owns(offer.kind) && offer.carrier.is_none_or(owns))
    }

    /// A purchase by unit type.
    fn offered(&self, purchase: &PurchaseSpec) -> Option<OfferedPurchase> {
        let kind = |id: &str| self.rules.catalog.index(id);
        Some(OfferedPurchase {
            kind: kind(&purchase.variant)?,
            carrier: match &purchase.carrier {
                Some(carrier) => Some(kind(carrier)?),
                None => None,
            },
        })
    }

    fn act(&mut self, run: &mut DirectorRun, beat: &str, action: Action) {
        match action {
            Action::Order { units, order } => {
                let mut owners: BTreeMap<String, (Side, Vec<UnitId>)> = BTreeMap::new();
                for unit in self.selected(run, &units) {
                    let Some(owner) = &unit.owner else { continue };
                    owners
                        .entry(owner.clone())
                        .or_insert((unit.side, Vec::new()))
                        .1
                        .push(unit.id);
                }
                for (owner, (side, units)) in owners {
                    // Only a directed seat takes the director's orders.
                    if self.controller(&owner) != Some(&ControllerKind::DirectedAi) {
                        continue;
                    }
                    let point = |zone: &str| run.setup.zones[zone].center;
                    let order = match &order {
                        DirectedOrder::MoveTo(zone) => Order::Move {
                            units,
                            gesture: 0,
                            goal: point(zone),
                            route: contract::command::RoutePolicy::Shortest,
                            direction: contract::command::MoveDirection::Forward,
                            facing: None,
                        },
                        DirectedOrder::AttackGround(zone) => {
                            let [x, y] = point(zone);
                            let z = self.world.height_at(x, y).unwrap_or(0.0);
                            Order::Attack {
                                units,
                                target: TargetRef::Ground { point: [x, y, z] },
                            }
                        }
                        DirectedOrder::Engage(engagement) => Order::SetEngagement {
                            units,
                            policy: *engagement,
                        },
                    };
                    if let Err(refusal) = self.apply_authored_order(side, Some(owner), order, false)
                    {
                        run.refusals.push((beat.to_owned(), refusal));
                    }
                }
            }
            Action::Grant { player, credits } => {
                let credits = match credits {
                    CreditGrant::Exactly(credits) => credits,
                    CreditGrant::Covering(purchases) => {
                        credits_covering(&self.rules.catalog, &purchases)
                            .expect("admitted purchases")
                    }
                };
                if let Some(skirmish) = &mut self.skirmish {
                    if let Some(index) = skirmish.player_index(&player) {
                        skirmish.grant(index, credits);
                    }
                }
            }
            Action::Buy {
                player,
                purchase,
                at,
            } => {
                let destination = run.setup.zones[&at].center;
                let order = Order::ConfirmPurchase {
                    variant: purchase.variant,
                    carrier: purchase.carrier,
                    destination,
                };
                self.order_for(run, beat, &player, order);
            }
            Action::Ready { player } => self.order_for(run, beat, &player, Order::Ready),
            Action::Offer { player, purchases } => {
                let offers: Vec<_> = purchases.iter().filter_map(|p| self.offered(p)).collect();
                if let Some(skirmish) = &mut self.skirmish {
                    if let Some(index) = skirmish.player_index(&player) {
                        skirmish.offer(index, &offers);
                    }
                }
            }
        }
    }
}

impl Battle {
    /// Give a directed seat's player an order of its own (a purchase, its
    /// ready); a seat that isn't directed gives none.
    fn order_for(&mut self, run: &mut DirectorRun, beat: &str, player: &str, order: Order) {
        if self.controller(player) != Some(&ControllerKind::DirectedAi) {
            return;
        }
        let Some(side) = self
            .skirmish
            .as_ref()
            .and_then(|s| s.player_index(player).map(|i| s.players[i].side))
        else {
            return;
        };
        if let Err(refusal) = self.apply_authored_order(side, Some(player.to_owned()), order, false)
        {
            run.refusals.push((beat.to_owned(), refusal));
        }
    }
}

fn inside(zone: &contract::scenario_document::Zone, at: V2) -> bool {
    libm::hypot(at.x - zone.center[0], at.y - zone.center[1]) <= zone.radius_m
}
