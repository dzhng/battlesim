//! A squad's corridor: the one way it plans, which each soldier walks
//! his own lane beside.
use super::{left, v2, PATH_REACHED_M, V2};

/// A squad's corridor: its remaining waypoints and where it runs from.
#[derive(Clone, Copy)]
pub struct Corridor<'a> {
    pub from: V2,
    pub route: &'a [V2],
}

impl Corridor<'_> {
    pub(super) fn start(&self, leg: usize) -> V2 {
        if leg == 0 {
            self.from
        } else {
            self.route[leg - 1]
        }
    }

    pub(super) fn last(&self) -> usize {
        self.route.len() - 1
    }

    pub(super) fn end(&self) -> V2 {
        self.route[self.last()]
    }

    /// Where `p` projects along leg `leg`, as a share of it (unclamped).
    pub(super) fn along(&self, leg: usize, p: V2) -> f64 {
        let (a, b) = (self.start(leg), self.route[leg]);
        let d = b - a;
        let l2 = d.dot(d);
        if l2 < 1e-12 {
            1.0
        } else {
            (p - a).dot(d) / l2
        }
    }

    pub(super) fn direction(&self, leg: usize) -> V2 {
        let d = self.route[leg] - self.start(leg);
        if d.length() < 1e-9 {
            if leg > 0 {
                return self.direction(leg - 1);
            }
            return v2(1.0, 0.0);
        }
        d.normalized()
    }

    /// The corridor point `ahead` metres on from share `t` of leg `leg`. It
    /// stops at the corridor's end.
    pub(super) fn ahead(&self, leg: usize, t: f64, ahead: f64) -> V2 {
        let lane = self.lane(leg, t, ahead, 0.0);
        lane[lane.len() - 1]
    }

    /// First standing point ahead within a soldier's local rejoin reach.
    pub(super) fn rejoin(
        &self,
        leg: usize,
        t: f64,
        step: f64,
        here: V2,
        reach: f64,
        mut stands: impl FnMut(V2) -> bool,
    ) -> Option<V2> {
        // Once the remaining arc is exhausted, ahead keeps returning the
        // endpoint. One extra sample covers rounding at an exact step boundary.
        let samples = (self.remaining(leg, t) / step).ceil() as usize + 1;
        (1..=samples)
            .map(|k| self.ahead(leg, t, step * k as f64))
            .take_while(|p| (*p - here).within_radius(reach))
            .find(|p| stands(*p))
    }

    /// His lane point `ahead` metres on from share `t` of leg `leg`, never
    /// past that leg's end: the leg shifted `offset` to its left. Held to his
    /// own leg, the point never swings round a waypoint onto the next leg's
    /// side, so he walks each leg's lane to its end before turning.
    pub(super) fn lane_point(&self, leg: usize, t: f64, ahead: f64, offset: f64) -> V2 {
        let (a, b) = (self.start(leg), self.route[leg]);
        let length = (b - a).length();
        let dir = self.direction(leg);
        a + dir * (length * t + ahead).min(length) + left(dir) * offset
    }

    /// His lane from share `t` of leg `leg` to `ahead` metres on, as he
    /// walks it: at each waypoint it passes, the end of that leg's lane and
    /// the start of the next's, then the lane point `ahead` metres on.
    pub(super) fn lane(&self, leg: usize, t: f64, ahead: f64, offset: f64) -> Vec<V2> {
        let (mut leg, mut left_m) = (leg, ahead);
        let mut at = self.start(leg) + (self.route[leg] - self.start(leg)) * t;
        let mut points = Vec::new();
        loop {
            let to = self.route[leg];
            let span = (to - at).length();
            let dir = self.direction(leg);
            if left_m <= span || leg == self.last() {
                points.push(at + dir * left_m.min(span) + left(dir) * offset);
                return points;
            }
            left_m -= span;
            at = to;
            leg += 1;
            points.push(to + left(dir) * offset);
            points.push(to + left(self.direction(leg)) * offset);
        }
    }

    /// Whether `p` has passed leg `leg`: its projection lies within
    /// [`PATH_REACHED_M`] of the leg's end, or beyond.
    pub(super) fn passed(&self, leg: usize, p: V2) -> bool {
        let length = (self.route[leg] - self.start(leg)).length();
        (1.0 - self.along(leg, p)) * length < PATH_REACHED_M
    }

    /// Metres of corridor left from share `t` of leg `leg`.
    pub(super) fn remaining(&self, leg: usize, t: f64) -> f64 {
        let at = self.start(leg) + (self.route[leg] - self.start(leg)) * t;
        let mut total = (self.route[leg] - at).length();
        for k in leg + 1..self.route.len() {
            total += (self.route[k] - self.route[k - 1]).length();
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocked_nearby_corridor_end_exhausts_the_rejoin_search() {
        let route = [v2(65.0, 50.0)];
        let corridor = Corridor {
            from: v2(50.0, 50.0),
            route: &route,
        };
        let mut attempts = 0;
        let point = corridor.rejoin(0, 0.0, 3.0, corridor.from, 18.0, |_| {
            attempts += 1;
            // Fail the old infinite endpoint loop without hanging the test runner.
            assert!(attempts <= 6, "rejoin keeps retrying the blocked endpoint");
            false
        });
        assert_eq!(point, None);
    }
    #[test]
    fn rejoin_still_accepts_the_first_open_point_including_the_end() {
        let end = v2(65.0, 50.0);
        let route = [end];
        let corridor = Corridor {
            from: v2(50.0, 50.0),
            route: &route,
        };
        for t in [0.0, 0.2, 0.8, 1.0] {
            assert_eq!(
                corridor.rejoin(0, t, 3.0, corridor.from, 18.0, |p| p == end),
                Some(end)
            );
        }
        assert_eq!(
            corridor.rejoin(0, 0.0, 3.0, corridor.from, 18.0, |p| p.x >= 56.0),
            Some(v2(56.0, 50.0))
        );
    }
}
