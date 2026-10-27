//! A planned route checked against what the side now knows: whether it
//! still fits, whether it shoves a body aside, and how long it takes.
use super::{Mobility, Mover, NavGrid, NAV_CELL_M};
use crate::math::V2;
use crate::world::PropId;
use contract::command::RoutePolicy;
use contract::map::MoverClass;
use contract::scenario::PushClass;

impl NavGrid {
    /// Whether any part of the remaining route crosses a known body this
    /// footprint would shove aside: a pusher weighs every such body against
    /// a detour when it learns of it (Q13).
    /// Only bodies more than `reach` metres along it count: one nearer than a
    /// vehicle can steer round is pushed through, not weighed again. Every
    /// other body counts, for diagnostics and tests; a replan asks
    /// [`route_pushes_where`](Self::route_pushes_where).
    pub fn route_pushes(&self, from: V2, route: &[V2], m: &Mobility, reach: f64) -> bool {
        self.route_pushes_where(from, route, m, reach, &|_| true)
    }

    /// [`route_pushes`](Self::route_pushes), of the bodies a replan `weighs`.
    pub fn route_pushes_where(
        &self,
        from: V2,
        route: &[V2],
        m: &Mobility,
        reach: f64,
        weighs: &dyn Fn(PropId) -> bool,
    ) -> bool {
        if m.class == MoverClass::Infantry {
            return false;
        }
        let (mut a, mut along) = (from, 0.0);
        route.iter().any(|&b| {
            let length = (b - a).length();
            let samples = ((length / (NAV_CELL_M / 4.0)).ceil() as usize).max(1);
            let pushes = (0..samples).any(|k| {
                let t = (k as f64 + 0.5) / samples as f64;
                along + length * t > reach
                    && !self.bodies_clear_counting(
                        a + (b - a) * t,
                        m.half_width_m,
                        PushClass::None,
                        weighs,
                    )
            });
            (a, along) = (b, along + length);
            pushes
        })
    }

    /// Whether any known blocker now overlaps the remaining route for this footprint.
    pub fn route_fits(&self, from: V2, route: &[V2], m: &Mobility) -> bool {
        let mut a = from;
        route.iter().all(|&b| {
            let ok = self
                .segment_cost(a, b, Mover::free(m), RoutePolicy::Shortest)
                .is_some();
            a = b;
            ok
        })
    }

    /// Travel time of a route for this footprint, for diagnostics and tests.
    pub fn route_time(&self, from: V2, route: &[V2], m: &Mobility) -> f64 {
        let mut a = from;
        route
            .iter()
            .map(|&b| {
                let t = self
                    .segment_cost(a, b, Mover::free(m), RoutePolicy::Fastest)
                    .unwrap_or(f64::INFINITY);
                a = b;
                t
            })
            .sum()
    }
}
