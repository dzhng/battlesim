//! A volley: what an attack fires before it hands off to the order queued
//! behind it — one firing cycle (a burst, else a magazine, else one shot)
//! from each weapon row working on its target, or for a jet attacking in
//! runs, one run in which it fired. The last attack in a queue fires no
//! volley; it keeps on until its target is dead or lost.

/// What an attack has fired since something was first queued behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Volley {
    /// The rows (a bit per mount index) that have ended a firing cycle at
    /// the target. A cycle already under way when counting began counts when
    /// it ends; a row not firing then must fire one whole cycle.
    pub done: u64,
    /// A jet's run under way: it was diving at the last fire step.
    pub diving: bool,
    /// The run under way has fired at the target.
    pub run_fired: bool,
}

/// One tick of a jet's runs at the attack's target, after the fire step.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunFire {
    /// It fired at the target this tick.
    pub fired: bool,
    /// It is flying a run (committed, nose on the aim).
    pub diving: bool,
    /// Rows still guiding a round at the target.
    pub guiding: u64,
    /// It has a weapon to fly a run with, able to hurt the target.
    pub can_act: bool,
}

/// One tick's fire against the attack's target, measured after the fire
/// step.
#[derive(Clone, Copy, Debug)]
pub struct VolleyFire {
    /// Rows working on the target: firing, aiming, reloading or traversing.
    pub working: u64,
    /// Rows that ended a firing cycle at the target this tick.
    pub ended: u64,
    /// Rows still guiding a round at the target: a launcher must stand by
    /// its missile until it lands.
    pub guiding: u64,
    /// Something can still act on the target: for a ground point, a row
    /// working on it or the unit closing on it; for a unit or contact, a row
    /// that can hurt it, while one works on it or the unit can go after it.
    pub can_act: bool,
}

impl Volley {
    /// Count this tick's fire; whether the attack can hand off: nothing can
    /// act on its target, or some row has ended a cycle and every row still
    /// working on it has too.
    pub fn observe(&mut self, fire: VolleyFire) -> bool {
        self.done |= fire.ended;
        let finished = self.done != 0 && fire.working & !self.done == 0;
        fire.guiding == 0 && (!fire.can_act || finished)
    }

    /// Count this tick of a jet's runs; whether the attack can hand off: it
    /// has nothing to fly a run with, or a run just ended having fired. A run
    /// broken off with nothing fired doesn't count; the next one lines up.
    pub fn observe_run(&mut self, run: RunFire) -> bool {
        let ended = self.diving && !run.diving;
        if run.diving || ended {
            self.run_fired |= run.fired;
        }
        self.diving = run.diving;
        // Nothing hands off while its missiles are still flying.
        if run.guiding != 0 {
            return false;
        }
        if !run.can_act {
            return true;
        }
        // A run that fired counts once it is over.
        let fired = !self.diving && self.run_fired;
        if fired {
            self.run_fired = false;
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire(working: u64, ended: u64, can_act: bool) -> VolleyFire {
        VolleyFire {
            working,
            ended,
            guiding: 0,
            can_act,
        }
    }

    fn run(fired: bool, diving: bool) -> RunFire {
        RunFire {
            fired,
            diving,
            guiding: 0,
            can_act: true,
        }
    }

    #[test]
    fn a_run_that_fired_hands_off_as_it_ends() {
        let mut v = Volley::default();
        assert!(!v.observe_run(run(false, true)));
        assert!(!v.observe_run(run(true, true)));
        assert!(v.observe_run(run(false, false)));
    }

    #[test]
    fn a_bomb_released_as_the_run_ends_counts() {
        let mut v = Volley::default();
        assert!(!v.observe_run(run(false, true)));
        assert!(v.observe_run(run(true, false)));
    }

    #[test]
    fn a_run_broken_off_with_nothing_fired_does_not_count() {
        let mut v = Volley::default();
        assert!(!v.observe_run(run(false, true)));
        assert!(!v.observe_run(run(false, false)));
        assert!(
            !v.observe_run(run(true, false)),
            "fire between runs is no run"
        );
        assert!(!v.observe_run(run(false, true)));
        assert!(v.observe_run(run(true, false)));
    }

    #[test]
    fn a_jet_out_of_missiles_still_guides_its_last_one_home() {
        let mut v = Volley::default();
        let last = RunFire {
            guiding: 0b1,
            ..RunFire::default()
        };
        assert!(!v.observe_run(last), "handed off with a missile in flight");
        assert!(v.observe_run(RunFire::default()));
    }

    #[test]
    fn a_jet_with_nothing_to_run_with_hands_off_at_once() {
        let mut v = Volley::default();
        assert!(v.observe_run(RunFire::default()));
    }

    #[test]
    fn a_launcher_guiding_its_missile_holds_the_volley() {
        let mut v = Volley::default();
        let guiding = VolleyFire {
            guiding: 0b1,
            ..fire(0b1, 0b1, true)
        };
        assert!(!v.observe(guiding));
        assert!(v.observe(fire(0b1, 0, true)));
    }

    #[test]
    fn a_run_waits_for_its_missiles_to_land() {
        let mut v = Volley::default();
        assert!(!v.observe_run(run(true, true)));
        let guiding = RunFire {
            guiding: 0b1,
            ..run(false, false)
        };
        assert!(!v.observe_run(guiding));
        assert!(v.observe_run(run(false, false)));
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
