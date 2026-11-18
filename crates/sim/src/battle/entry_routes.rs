//! The side's quickest road: which entry a unit comes in by or goes home
//! to, judged by its travel time on the side's planning geometry.

use super::*;

impl Battle {
    /// Travel time for a catalog unit from an entry site to a point, using the
    /// side's planning geometry. Entry selection uses the same route owner as
    /// ordinary movement admission.
    pub(super) fn entry_route_time(
        &self,
        side: Side,
        kind: TypeIndex,
        from: [f64; 2],
        destination: [f64; 2],
    ) -> Option<f64> {
        // Movement admission refuses a goal off the map; so does the route
        // estimate, or a planner would pick a goal its order cannot reach.
        self.world.height_at(destination[0], destination[1])?;
        let t = self.rules.catalog.get(kind);
        let from = v2(from[0], from[1]);
        let goal = v2(destination[0], destination[1]);
        let mobility = if let Some(mobility) = units::ground_mobility(t, &self.rules) {
            mobility
        } else {
            let units::Motion::Air(f) = units::motion(t, &self.rules) else {
                unreachable!("an admitted entry unit moves on the ground or in the air")
            };
            return Some((goal - from).length() / f.cruise_mps);
        };
        let mut known = self.sides[side.index()].clone();
        let grid = known.grid(&self.world, self.authored_props);
        let (plan, _) = crate::navigation::plan(
            grid,
            &self.roads,
            crate::navigation::Leg {
                from,
                goal,
                m: &mobility,
                policy: RoutePolicy::Fastest,
                avoid: &[],
            },
            &self.rules.navigation,
        );
        let crate::navigation::Plan::Route(route) = plan else {
            return None;
        };
        let time = grid.route_time(from, &route, &mobility);
        debug_assert!(
            time.is_nan() || time >= mobility.least_route_time(from, goal),
            "a route beat its least time"
        );
        time.is_finite().then_some(time)
    }

    /// A time [`Self::entry_route_time`] cannot beat, without a search.
    fn least_entry_time(&self, kind: TypeIndex, from: [f64; 2], destination: [f64; 2]) -> f64 {
        let (from, goal) = (v2(from[0], from[1]), v2(destination[0], destination[1]));
        if let Some(mobility) = units::ground_mobility(self.rules.catalog.get(kind), &self.rules) {
            mobility.least_route_time(from, goal)
        } else {
            let units::Motion::Air(f) = units::motion(self.rules.catalog.get(kind), &self.rules)
            else {
                unreachable!("an admitted entry unit moves on the ground or in the air")
            };
            (goal - from).length() / f.cruise_mps
        }
    }

    /// The side's entry with the quickest route for `kind` to `point`.
    pub(super) fn select_entry(
        &self,
        side: Side,
        kind: TypeIndex,
        point: [f64; 2],
    ) -> crate::entries::EntryIndex {
        let sites = &self
            .skirmish
            .as_ref()
            .expect("skirmish entry selection")
            .setup
            .sites;
        crate::entries::select(
            sites,
            side,
            kind,
            point,
            |entry, kind, destination| self.entry_route_time(side, kind, entry.center, destination),
            |entry| self.least_entry_time(kind, entry.center, point),
        )
    }
}
