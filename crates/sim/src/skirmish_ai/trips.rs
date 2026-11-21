//! What a seat remembers of the trips it has had worked out. It weighs a
//! move by asking the battle how long the way takes, and each answer is a
//! whole route: the dearest thing a decision does, and dearest of all when
//! there is no way, for that is searched for as far as a search may go.

use std::collections::BTreeMap;

use contract::command::Order;
use contract::ids::{Tick, UnitId};
use contract::observation::ObservationFrame;
use contract::scenario::Rules;

use super::tuning::Tuning;
use super::{BalancedDoctrine, DecisionInput};

/// A unit and where it would go.
type Trip = (UnitId, [u64; 2]);

/// The trips a seat found no way for, and when.
#[derive(Clone, Debug, Default)]
pub(super) struct Trips {
    no_way: BTreeMap<Trip, Tick>,
}

impl Trips {
    /// `route_time` as one decision at `tick` asks it: a trip the decision
    /// weighs more than once is worked out once, and one the seat found no
    /// way for within the last `memory` ticks is not worked out at all. A
    /// wall seldom opens by the next decision.
    fn asking<'a>(
        &'a mut self,
        tick: Tick,
        memory: Tick,
        route_time: &'a mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> impl FnMut(UnitId, [f64; 2]) -> Option<f64> + 'a {
        self.no_way
            .retain(|_, found| tick.saturating_sub(*found) < memory);
        let mut asked = BTreeMap::new();
        move |unit, goal| {
            let trip = (unit, goal.map(f64::to_bits));
            if self.no_way.contains_key(&trip) {
                return None;
            }
            let time = *asked.entry(trip).or_insert_with(|| route_time(unit, goal));
            if time.is_none() {
                self.no_way.insert(trip, tick);
            }
            time
        }
    }
}

impl BalancedDoctrine {
    /// Decide, asking the battle only for the trips the seat does not
    /// already know ([`Trips`]).
    pub(super) fn decide_remembering(
        &mut self,
        input: DecisionInput<'_>,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        let mut trips = std::mem::take(&mut self.trips);
        let memory = Tuning::ticks(self.tuning.no_way_memory_s, rules.tick_hz);
        let mut ask = trips.asking(frame.tick, memory, route_time);
        let orders = self.decide_impl(input, frame, rules, &mut ask);
        drop(ask);
        self.trips = trips;
        orders
    }
}
