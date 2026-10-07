//! The encounter's defender: the opponent policy every battle whose scenario
//! carries one runs, authored or planned. It reads only its own side's
//! observation and acts only through ordinary commands.
use std::collections::BTreeSet;

use contract::command::{Order, RoutePolicy, TargetRef};
use contract::ids::UnitId;
use contract::observation::ObservationFrame;
use contract::scenario::{Opponent, Rules};

/// What the defender remembers between its decisions (its own choices only).
#[derive(Clone, Debug, Default)]
pub struct Defender {
    started: bool,
    /// AT teams that have made their one explicit attack.
    attacked: BTreeSet<u32>,
    /// Units that have already fallen back (once each).
    retreated: BTreeSet<u32>,
}

impl Defender {
    /// This tick's orders from the side's own observation.
    pub fn decide(&mut self, op: &Opponent, frame: &ObservationFrame, rules: &Rules) -> Vec<Order> {
        let mut orders = Vec::new();
        if !self.started {
            self.started = true;
            for &[unit, building] in &op.garrisons {
                orders.push(Order::Garrison {
                    units: vec![UnitId(unit)],
                    building,
                });
            }
        }
        for u in &frame.own {
            let id = u.id.0;
            // An AT team makes one explicit attack, on the costliest tank its
            // own optics identify in range; after it, the team fires at will.
            let t = rules.catalog.get(u.kind);
            if t.has_role("at") && !self.attacked.contains(&id) {
                let best = frame
                    .identified
                    .iter()
                    .filter(|e| rules.catalog.get(e.kind).has_role("mbt") && u.sees.contains(&e.id))
                    .filter(|e| {
                        let d = [e.position[0] - u.position[0], e.position[1] - u.position[1]];
                        libm::hypot(d[0], d[1]) <= op.at_attack_range_m
                    })
                    .max_by(|a, b| a.cost.cmp(&b.cost).then(b.id.cmp(&a.id)));
                if let Some(tank) = best {
                    self.attacked.insert(id);
                    orders.push(Order::Attack {
                        units: vec![u.id],
                        target: TargetRef::Identified { id: tank.id },
                    });
                }
            }
            if self.retreated.contains(&id) {
                continue;
            }
            // Fall back once when badly hurt, judged from own state only.
            let fallback = match t.hull() {
                Some(hull) => (t.has_role("mbt") && u.hp < op.tank_retreat_hp_fraction * hull.hp)
                    .then_some(op.tank_fallback),
                None => {
                    let original = t.squad_size() as f64;
                    ((u.members.len() as f64) < op.infantry_retreat_survivor_fraction * original)
                        .then_some(op.infantry_fallback)
                }
            };
            if let Some(goal) = fallback {
                self.retreated.insert(id);
                orders.push(Order::Move {
                    units: vec![u.id],
                    gesture: 1_000_000 + id as u64,
                    goal,
                    route: RoutePolicy::Shortest,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                });
            }
        }
        orders
    }
}
