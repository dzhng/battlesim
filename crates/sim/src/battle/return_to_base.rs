//! Return to base (a refund): the way home for a unit, ordered at once or
//! queued behind its other orders.

use super::*;

impl Battle {
    /// Where `unit` leaves the map going home from where it is.
    pub(super) fn way_home(&self, side: Side, unit: UnitId) -> ([f64; 2], f64) {
        let p = self.units[unit.0 as usize].position;
        self.way_home_from(side, unit, [p.x, p.y])
    }

    /// Where `unit` leaves the map going home from `origin`, and the
    /// heading it leaves on: a winged aircraft by its side's back edge,
    /// anything else by the side's quickest road from there.
    pub(super) fn way_home_from(
        &self,
        side: Side,
        unit: UnitId,
        origin: [f64; 2],
    ) -> ([f64; 2], f64) {
        let sites = &self.skirmish.as_ref().expect("skirmish refund").setup.sites;
        let current = &self.units[unit.0 as usize];
        if current.fixed_wing() {
            let edge = crate::air::edge::back_edge(side, sites, &self.world);
            let (point, heading) = crate::air::edge::entry(&edge, v2(origin[0], origin[1]));
            return ([point.x, point.y], heading);
        }
        let selected = self.select_entry(side, current.kind, origin);
        let entry = crate::entries::site(sites, selected);
        (entry.center, entry.yaw)
    }

    /// Head `unit` home along `goal`: the drive out, and the withdrawal that
    /// retires it at the edge.
    pub(super) fn head_home(unit: &mut Unit, goal: [f64; 2]) {
        unit.enqueue(
            UnitOrder::Move(MoveOrder {
                destination: v2(goal[0], goal[1]),
                policy: RoutePolicy::Fastest,
                gesture: 0,
                direction: Default::default(),
                facing: None,
                short: None,
            }),
            false,
        );
        unit.withdrawing = true;
    }

    /// Every queued return to base whose turn has come becomes the drive
    /// home, replacing the queue: nothing follows a unit that leaves. A
    /// squad riding a carrier drops it: it goes where its carrier goes.
    pub(super) fn start_withdrawals(&mut self) {
        for index in 0..self.units.len() {
            let unit = &self.units[index];
            if !unit.alive() || !matches!(unit.orders.front(), Some(UnitOrder::Withdraw)) {
                continue;
            }
            if unit.hold.as_ref().and_then(hold::Hold::riding).is_some() {
                self.units[index].orders.pop_front();
                continue;
            }
            let (goal, _) = self.way_home(unit.side, unit.id);
            Self::head_home(&mut self.units[index], goal);
        }
    }
}
