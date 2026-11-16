//! An own unit's orders as its side publishes them: one step per order, the
//! current first. Read-only: every point comes from what the side knows and
//! from where the step before ends; nothing here plans a route for the unit
//! or writes battle state.

use contract::observation::{OrderStep, StepHost};

use super::*;

/// Each unit's queued return to base as last published: where the step
/// before it ended, and the way home from there. Finding the way home
/// searches a route per road in, so the step keeps its point while that
/// origin holds. Presentation only, never in the digest.
pub(super) type HomeWays = BTreeMap<UnitId, ([f64; 2], [f64; 2])>;

impl Battle {
    /// Each living unit of `side`'s steps, in unit order: what the side's
    /// own rows publish.
    pub(super) fn own_steps(&mut self, side: Side) -> Vec<Vec<OrderStep>> {
        let known = std::mem::take(&mut self.home_ways);
        let units = &self.units;
        let mut ways: HomeWays = (known.iter())
            .filter(|(u, _)| units[u.0 as usize].side != side)
            .map(|(&u, &way)| (u, way))
            .collect();
        let knowledge = &self.knowledge[side.index()];
        let steps = (self.units.iter())
            .filter(|u| u.side == side && u.alive())
            .map(|u| self.order_steps(u, knowledge, &known, &mut ways))
            .collect();
        self.home_ways = ways;
        steps
    }

    /// `unit`'s orders as steps. Each step starts where the one before ends
    /// (`from`): a move's destination, a building's approach, a carrier or
    /// squad it drives to, the way home; an attack, an unload or an exit
    /// moves it nowhere new.
    fn order_steps(
        &self,
        unit: &Unit,
        knowledge: &SideKnowledge,
        known: &HomeWays,
        ways: &mut HomeWays,
    ) -> Vec<OrderStep> {
        let xy = |p: V2| [p.x, p.y];
        let at = |id: UnitId| self.units[id.0 as usize].position.xy();
        let centre = |b: u32| self.world.structure_footprint(b).map(|f| xy(f.center));
        let mut from = xy(unit.position.xy());
        // The building it is in, or will be in once a garrison step is done.
        let mut inside = unit.hold.as_ref().and_then(|h| match h.host {
            hold::Host::Building(b) => Some(b),
            hold::Host::Carrier(_) => None,
        });
        let mut steps = Vec::with_capacity(unit.orders.len());
        for order in &unit.orders {
            let step = match order {
                UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                    from = xy(m.destination);
                    OrderStep::Move {
                        point: from,
                        attack_move: matches!(order, UnitOrder::AttackMove(_)),
                    }
                }
                UnitOrder::Attack {
                    target, last_known, ..
                } => attack_step(*target, *last_known, knowledge),
                UnitOrder::Garrison {
                    building, approach, ..
                } => {
                    from = xy(*approach);
                    inside = Some(*building);
                    OrderStep::Garrison {
                        point: from,
                        building: *building,
                    }
                }
                UnitOrder::Exit => OrderStep::Exit {
                    point: inside.take().and_then(centre).unwrap_or(from),
                },
                UnitOrder::Board { carrier, .. } => {
                    from = xy(at(*carrier));
                    OrderStep::Board {
                        point: from,
                        carrier: *carrier,
                    }
                }
                UnitOrder::Collect { squads, .. } => {
                    let first = (squads.iter())
                        .map(|s| &self.units[s.0 as usize])
                        .find(|s| s.alive());
                    let point = first.map(|s| xy(s.position.xy()));
                    from = point.unwrap_or(from);
                    OrderStep::Collect { point }
                }
                UnitOrder::Unload { into } => OrderStep::Unload {
                    point: from,
                    into: into.map(|host| match host {
                        hold::Host::Building(id) => StepHost::Building { id },
                        hold::Host::Carrier(id) => StepHost::Carrier { id },
                    }),
                },
                UnitOrder::Withdraw => {
                    let home = match known.get(&unit.id) {
                        Some(&(origin, home)) if origin == from => home,
                        _ => self.way_home_from(unit.side, unit.id, from).0,
                    };
                    ways.insert(unit.id, (from, home));
                    from = home;
                    OrderStep::ReturnToBase { point: home }
                }
            };
            steps.push(step);
        }
        steps
    }
}

/// `target` as the side names it: a unit by its track's id while the side
/// tracks it (none after), a contact, or a ground point. A round is never
/// published.
pub(crate) fn target_ref(knowledge: &SideKnowledge, target: Target) -> Option<TargetRef> {
    match target {
        Target::Unit(u) => knowledge
            .track(u)
            .map(|track| TargetRef::Identified { id: track.id }),
        Target::Contact(c) => Some(TargetRef::Contact { id: c }),
        Target::Ground(p) => Some(TargetRef::Ground {
            point: [p.x, p.y, p.z],
        }),
        Target::Round(_) => None,
    }
}

/// An attack's step: its target as the side names it (none once the side
/// no longer tracks the unit it names), and where the side knows it to be:
/// the target's track, else the attack's last known spot, else the contact,
/// else the ground point.
fn attack_step(target: Target, last_known: Option<V2>, knowledge: &SideKnowledge) -> OrderStep {
    let named = target_ref(knowledge, target);
    let known = match target {
        Target::Unit(u) => knowledge.track(u).map(|t| t.position.xy()),
        Target::Contact(c) => knowledge.contact(c).map(|c| c.center.xy()),
        Target::Ground(p) => Some(p.xy()),
        Target::Round(_) => None,
    };
    let point = match target {
        Target::Unit(_) => known.or(last_known),
        _ => last_known.or(known),
    };
    OrderStep::Attack {
        target: named,
        point: point.map(|p| [p.x, p.y]),
    }
}
