//! A volley: what an attack fires before it hands off to the order queued
//! behind it — one firing cycle (a burst, else a magazine, else one shot)
//! from each weapon row working on its target. The last attack in a queue
//! fires no volley; it keeps on until its target is dead or lost.

/// The rows (a bit per mount index) that have ended a firing cycle at the
/// target since something was first queued behind the attack. A cycle
/// already under way then counts when it ends; a row not firing then must
/// fire one whole cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Volley {
    pub done: u64,
}

/// One tick's fire against the attack's target, measured after the fire
/// step.
#[derive(Clone, Copy, Debug, Default)]
pub struct VolleyFire {
    /// Rows working on the target: firing, aiming, reloading or traversing.
    pub working: u64,
    /// Rows that ended a firing cycle at the target this tick.
    pub ended: u64,
    /// Something can still act on the target: for a ground point, a row
    /// working on it; for a unit or contact, a row that can hurt it, while
    /// one works on it or the unit can go after it.
    pub can_act: bool,
}

impl Volley {
    /// Count this tick's fire; whether the attack can hand off: nothing can
    /// act on its target, or some row has ended a cycle and every row still
    /// working on it has too.
    pub fn observe(&mut self, fire: VolleyFire) -> bool {
        self.done |= fire.ended;
        !fire.can_act || (self.done != 0 && fire.working & !self.done == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire(working: u64, ended: u64, can_act: bool) -> VolleyFire {
        VolleyFire {
            working,
            ended,
            can_act,
        }
    }

    #[test]
    fn nothing_able_to_act_hands_off_at_once() {
        assert!(Volley::default().observe(fire(0, 0, false)));
    }

    #[test]
    fn a_row_still_in_its_cycle_holds_the_volley() {
        let mut v = Volley::default();
        assert!(!v.observe(fire(0b1, 0, true)));
    }

    #[test]
    fn every_working_row_must_end_a_cycle() {
        let mut v = Volley::default();
        assert!(!v.observe(fire(0b11, 0b01, true)));
        assert!(v.observe(fire(0b11, 0b10, true)));
    }

    #[test]
    fn a_row_that_stops_working_no_longer_holds_it() {
        let mut v = Volley::default();
        assert!(!v.observe(fire(0b11, 0b01, true)));
        assert!(v.observe(fire(0b01, 0, true)));
    }

    #[test]
    fn a_row_reloading_after_its_cycle_counts_as_done() {
        let mut v = Volley::default();
        assert!(v.observe(fire(0b1, 0b1, true)));
        assert!(v.observe(fire(0b1, 0, true)));
    }
}
