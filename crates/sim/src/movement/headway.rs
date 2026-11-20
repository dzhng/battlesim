//! Whether a unit with somewhere to go is getting there: the one owner of
//! its stuck clocks.
//!
//! Two watches run while it has a route. The near one notes each time it
//! comes nearer its next waypoint: `STALL_REPLAN_S` without that is a stall,
//! and has it plan again (a vehicle, round the knot it waits in). The far
//! one is the stuck clock: where it stood when the clock started, and when.
//! Still within `STUCK_REACH_M` of that spot `GIVE_UP_S` later, it stops if
//! it is near enough its goal, and otherwise finds no way; blocked, the same
//! clock times its next try, and the third time it finds no way it gives the
//! order up. So no unit is stuck or blocked for good.
use contract::ids::Tick;

use super::PROGRESS_EPSILON_M;
use crate::digest::Digest;
use crate::math::V2;

/// Seconds without closing on the next waypoint before a route is replanned.
pub(super) const STALL_REPLAN_S: f64 = 2.0;
/// How long a unit may get nowhere before it stops near enough to its goal
/// or finds no way, and how long it holds blocked before it tries again:
/// longer than a truck's shunt out of a tight spot takes. Waiting behind a
/// rolling vehicle is traffic, not a stall.
const GIVE_UP_S: f64 = 30.0;
/// A unit that has got no farther than this from where its stuck clock
/// started is not getting anywhere, however its routes change; a reversing
/// leg can take a vehicle most of this way and back.
const STUCK_REACH_M: f64 = 15.0;
/// A unit that has found no way to its goal this many times gives the
/// order up and stands, ready for another.
const MAX_TRIES: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Headway {
    /// The least it has had left to its next waypoint, and the tick it last
    /// came nearer it (or was not expected to).
    left_m: f64,
    nearer: Tick,
    /// The stuck clock: where it stood when the clock started, and when.
    from: V2,
    since: Tick,
    /// For a squad, the least way its men have had left to their spots
    /// since the clock started.
    spots_m: f64,
    /// Times it has found no way to its current goal.
    tries: u8,
}

fn ticks(seconds: f64, tick_hz: u32) -> Tick {
    (seconds * f64::from(tick_hz)) as Tick
}

impl Headway {
    /// A unit placed at `at` with nowhere to go yet.
    pub fn new(at: V2) -> Self {
        Self {
            left_m: f64::INFINITY,
            nearer: 0,
            from: at,
            since: 0,
            spots_m: f64::INFINITY,
            tries: 0,
        }
    }

    /// A fresh waypoint to close on: the last one reached, or a route taken.
    pub(super) fn waypoint(&mut self, tick: Tick) {
        (self.left_m, self.nearer) = (f64::INFINITY, tick);
    }

    /// Its next waypoint is another without its having got anywhere (one
    /// dropped as passed): the near watch runs on.
    pub(super) fn waypoint_dropped(&mut self) {
        self.left_m = f64::INFINITY;
    }

    /// It has `left_m` to its waypoint: nearer than ever is headway.
    pub(super) fn closes(&mut self, left_m: f64, tick: Tick) {
        if left_m < self.left_m - PROGRESS_EPSILON_M {
            (self.left_m, self.nearer) = (left_m, tick);
        }
    }

    /// It is not expected to close on its waypoint this tick: holding to
    /// engage or to pack, or backing to give a turn room.
    pub(super) fn excused(&mut self, tick: Tick) {
        self.nearer = tick;
    }

    /// Whether it has gone a stall's length without closing on its waypoint.
    pub(super) fn stalled(&self, tick: Tick, tick_hz: u32) -> bool {
        tick.saturating_sub(self.nearer) > ticks(STALL_REPLAN_S, tick_hz)
    }

    /// Start the stuck clock again from `at`.
    pub(super) fn restart(&mut self, at: V2, tick: Tick) {
        (self.from, self.since, self.spots_m) = (at, tick, f64::INFINITY);
    }

    /// A goal somewhere new: the clock starts again and its tries are forgotten.
    pub(super) fn new_goal(&mut self, at: V2, tick: Tick) {
        self.restart(at, tick);
        self.tries = 0;
    }

    /// A squad at `at` whose men have `left_m` to their spots: nearer than
    /// ever since the clock started winds it back, however little its middle
    /// moves. A man pacing away and back does not.
    pub(super) fn spots_close(&mut self, at: V2, left_m: f64, tick: Tick) {
        if left_m < self.spots_m - PROGRESS_EPSILON_M {
            (self.from, self.since, self.spots_m) = (at, tick, left_m);
        }
    }

    /// Whether `at` is well away from where the stuck clock started.
    pub(super) fn got_away(&self, at: V2) -> bool {
        (at - self.from).outside_radius(STUCK_REACH_M)
    }

    /// Whether a give-up spell has run since the stuck clock started.
    pub(super) fn spent(&self, tick: Tick, tick_hz: u32) -> bool {
        tick.saturating_sub(self.since) > ticks(GIVE_UP_S, tick_hz)
    }

    /// When the stuck clock started.
    pub(super) fn since(&self) -> Tick {
        self.since
    }

    /// It has found no way to its goal: whether that is once too often.
    pub(super) fn no_way(&mut self) -> bool {
        self.tries += 1;
        self.tries >= MAX_TRIES
    }

    pub fn digest(&self, d: &mut Digest) {
        d.f64(self.left_m)
            .u64(self.nearer)
            .f64(self.from.x)
            .f64(self.from.y)
            .u64(self.since)
            .f64(self.spots_m)
            .u64(u64::from(self.tries));
    }
}
