//! A unit's host and stationary hold state.

use contract::scenario::Rules;
use std::ops::{Deref, DerefMut};

use crate::garrison::Garrison;
use crate::math::V2;
use crate::units::{Unit, UnitOrder};
use crate::world::PropId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Stationary beside a host for this many ticks.
    Entering(u32),
    Inside,
    /// Stationary inside a host, leaving, for this many ticks.
    Exiting(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Building(PropId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub enter_exit_s: f64,
    pub reach_m: f64,
}

impl Host {
    pub fn timing(self, rules: &Rules) -> Timing {
        match self {
            Self::Building(_) => Timing {
                enter_exit_s: rules.garrison.enter_exit_s,
                reach_m: rules.garrison.entry_distance_m,
            },
        }
    }

    pub fn building(self) -> PropId {
        match self {
            Self::Building(id) => id,
        }
    }
}

/// Host-specific placement data carried by a hold.
#[derive(Clone, Debug)]
pub enum Place {
    Building(Garrison),
}

impl Place {
    pub fn entry(&self) -> V2 {
        match self {
            Self::Building(garrison) => garrison.entry,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hold {
    pub host: Host,
    pub phase: Phase,
    pub place: Place,
}

impl Hold {
    pub fn building(&self) -> Option<&Garrison> {
        match (&self.host, &self.place) {
            (Host::Building(_), Place::Building(garrison)) => Some(garrison),
        }
    }

    pub fn building_mut(&mut self) -> Option<&mut Garrison> {
        match (&self.host, &mut self.place) {
            (Host::Building(_), Place::Building(garrison)) => Some(garrison),
        }
    }
}

impl Deref for Hold {
    type Target = Garrison;

    fn deref(&self) -> &Self::Target {
        self.building().expect("building hold")
    }
}

impl DerefMut for Hold {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.building_mut().expect("building hold")
    }
}

use crate::arrangement;
use crate::garrison::{self, Want};
use crate::world::WorldGeometry;
use contract::observation::MoveState;

fn ticks(seconds: f64, rules: &Rules) -> u32 {
    ((seconds * rules.tick_hz as f64).round() as u32).max(1)
}

/// Advance every squad's garrison one tick: start entering beside the
/// ordered building, seat the squad when the timer ends unless another holds it,
/// leave after the exit timer. Transitions are stationary: a squad with a
/// building has no movement goal. `seed`, the squad and where it heads
/// fix the arrangement a leaving squad spreads into.
pub(crate) fn advance(
    world: &WorldGeometry,
    sides: &[crate::movement::SideGeometry; 3],
    authored: PropId,
    units: &mut [Unit],
    rules: &Rules,
    seed: u64,
) {
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let want = garrison::want(&units[i]);
        let phase = units[i].hold.as_ref().map(|h| (h.host.building(), h.phase));
        let timer = phase.map_or(1, |(building, _)| {
            ticks(Host::Building(building).timing(rules).enter_exit_s, rules)
        });
        match (phase, want) {
            (None, Want::Enter(b)) => {
                let Some(_) = sides[units[i].side.index()]
                    .prop(world, authored, b)
                    .filter(|p| p.body.garrison)
                else {
                    units[i].orders.pop_front(); // its fall was discovered
                    continue;
                };
                if world
                    .structure_parts(b)
                    .into_iter()
                    .filter_map(|id| sides[units[i].side.index()].prop(world, authored, id))
                    .any(|p| {
                        p.footprint().contains(
                            units[i].position.xy(),
                            Host::Building(b).timing(rules).reach_m,
                        )
                    })
                {
                    let unit = &mut units[i];
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.state = MoveState::Idle;
                    unit.hold = Some(Hold {
                        host: Host::Building(b),
                        phase: Phase::Entering(1),
                        place: Place::Building(Garrison {
                            slots: Vec::new(),
                            seats: Vec::new(),
                            entry: unit.position.xy(),
                        }),
                    });
                }
            }
            (None, Want::Leave) if matches!(units[i].orders.front(), Some(UnitOrder::Exit)) => {
                units[i].orders.pop_front(); // nothing to leave
            }
            (None, _) => {}
            (Some((b, Phase::Entering(_))), w)
                if !matches!(w, Want::Enter(x) if x == b)
                    || garrison::building(world, b).is_none() =>
            {
                // A new order (or Stop) cancels entering where the squad stands.
                units[i].hold = None;
                if matches!(units[i].orders.front(), Some(UnitOrder::Exit)) {
                    units[i].orders.pop_front();
                }
            }
            (Some((_, Phase::Entering(n))), _) if n < timer => {
                units[i].hold.as_mut().unwrap().phase = Phase::Entering(n + 1);
            }
            (Some((b, Phase::Entering(_))), _) => {
                // Another squad (an enemy's) got in first: the order lapses
                // and the squad stands where it is.
                if garrison::held_by_another(units, b, i) {
                    units[i].hold = None;
                    units[i].orders.pop_front();
                    continue;
                }
                let entry = units[i].hold.as_ref().unwrap().place.entry();
                let prop = garrison::building(world, b).expect("standing building");
                let slots = garrison::seats_for_state(world, prop, rules);
                // Supply may restore soldiers during the entry timer. Admission
                // still belongs to the whole living squad when it takes seats.
                if units[i].members.iter().filter(|s| s.alive()).count() > slots.len() {
                    units[i].hold = None;
                    units[i].orders.pop_front();
                    continue;
                }
                let seats = garrison::seat_evenly(&slots, &units[i]);
                let unit = &mut units[i];
                for (s, seat) in unit.members.iter_mut().zip(&seats) {
                    if let Some(k) = seat {
                        s.position = slots[*k].position;
                    }
                }
                unit.settle();
                unit.hold = Some(Hold {
                    host: Host::Building(b),
                    phase: Phase::Inside,
                    place: Place::Building(Garrison {
                        slots,
                        seats,
                        entry,
                    }),
                });
                unit.orders.pop_front();
            }
            (Some((b, Phase::Inside)), Want::Enter(x)) if x == b => {
                units[i].orders.pop_front(); // already there
            }
            (Some((_, Phase::Inside)), Want::Enter(_) | Want::Leave) => {
                units[i].hold.as_mut().unwrap().phase = Phase::Exiting(1);
            }
            (Some((_, Phase::Inside)), Want::Stay) => {}
            (Some((_, Phase::Exiting(_))), Want::Stay) => {
                // Stop (or an order that stays) keeps the squad inside.
                units[i].hold.as_mut().unwrap().phase = Phase::Inside;
            }
            (Some((_, Phase::Exiting(n))), _) if n < timer => {
                units[i].hold.as_mut().unwrap().phase = Phase::Exiting(n + 1);
            }
            (Some((b, Phase::Exiting(_))), _) => {
                let unit = &units[i];
                let prop = garrison::building(world, b).expect("occupied building stands");
                let entry = unit.hold.as_ref().unwrap().place.entry();
                let preferred = match unit.orders.front() {
                    Some(UnitOrder::Garrison { approach, .. }) => *approach,
                    Some(o) => o.movement().map_or(entry, |m| m.destination),
                    None => entry,
                };
                // No room outside: the squad stays inside and tries again.
                let mut draws = arrangement::rng(seed, unit.id.0, arrangement::arrival(preferred));
                if let Some(spots) =
                    garrison::exit_spots(world, prop, unit, preferred, rules, &mut draws)
                {
                    let unit = &mut units[i];
                    let living = unit.members.iter_mut().filter(|s| s.alive());
                    for (s, p) in living.zip(spots) {
                        let z = world.surface_at(p.x, p.y).map_or(s.position.z, |g| g.z);
                        s.position = p.with_z(z);
                    }
                    unit.settle();
                    // Placed out of the building: the squad holds round there.
                    unit.anchor = Some(crate::cover::Anchor {
                        at: unit.position.xy(),
                        halt: false,
                    });
                    unit.hold = None;
                    if matches!(unit.orders.front(), Some(UnitOrder::Exit)) {
                        unit.orders.pop_front();
                    }
                }
            }
        }
    }
}
