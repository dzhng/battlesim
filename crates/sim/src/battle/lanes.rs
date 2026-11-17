//! Lanes through forest: a hull that knocks trees down leaves open ground
//! behind it.
use super::*;

impl Battle {
    /// Every vehicle that knocks trees down and moved this tick clears the
    /// forest ground its hull (and `ground.lane_margin_m` to either side) has left
    /// behind: the lane stops being forest (Q16), with crushed-ground marks.
    /// The ground under the hull itself stays forest until it has passed, so
    /// carving a lane goes at forest speed and only the lane is open ground.
    pub(super) fn clear_lanes(&mut self, before: &Poses) {
        // A forest is its trees: what knocks them down is its tree's weight.
        let tree = self.world.types().by_id(&self.rules.forests.tree).body;
        for (u, was) in self.units.iter().zip(&before.units) {
            let (Some(hull), Some(h)) = (u.ground_footprint(), u.hull) else {
                continue;
            };
            let knocks = tree.topples && u.ground().push.pushes(tree.weight_class);
            if !u.alive() || !knocks || (was.base - u.position).length() <= 1e-9 {
                continue;
            }
            if !self.world.forest_near(
                hull.center,
                hull.half.length() + self.rules.ground.lane_margin_m,
            ) {
                continue;
            }
            let left = Obb2 {
                center: was.base.xy(),
                yaw: was.yaw,
                // Wider, not longer: a margin ahead would clear the ground
                // the hull is about to cover.
                half: h.xy() + v2(0.0, self.rules.ground.lane_margin_m),
            };
            for cell in self.world.clear(&left, &hull) {
                self.ground.clear(cell);
            }
        }
    }
}
