//! Route quality against the frozen dense planner, which searched the whole
//! grid for the exact best route. The planner now walks its line and goes
//! by road where a road journey is worth it, so its routes may differ; this
//! measures by how much.
//!
//!     cargo run -p sim --release --example navigation_quality [1024|4096]
//!
//! What must still hold is asserted: a footprint fits only where the dense
//! grid says it does, and every new route fits the dense grid. Each route's
//! cost is then set beside the dense planner's on the dense grid (length
//! for the shortest policy, time for the fastest) and the ratios are
//! printed, with the legs only one planner found a route for and the legs
//! whose goal the new planner moved to standing room.
use contract::command::RoutePolicy;
use contract::map::{MapDefinition, MoverClass};
use contract::scenario::{NavigationRules, PushClass};
use math::{v2, Obb2, V2};
use sim::navigation::{Leg, Mobility, NavGrid, Plan, RoadNet};
pub use sim::{math, world};
use world::WorldGeometry;
#[allow(dead_code)]
mod dense;

fn mobility(class: MoverClass, push: PushClass) -> Mobility {
    Mobility {
        off_road_mps: 6.0,
        road_mps: 12.0,
        forest_multiplier: 0.4,
        half_width_m: if class == MoverClass::Infantry {
            0.5
        } else {
            1.8
        },
        class,
        push,
        drive: None,
    }
}
fn old_mobility(m: &Mobility) -> dense::Mobility {
    dense::Mobility {
        off_road_mps: m.off_road_mps,
        road_mps: m.road_mps,
        forest_multiplier: m.forest_multiplier,
        half_width_m: m.half_width_m,
        class: m.class,
        push: m.push,
        drive: None,
    }
}

/// One map: the planner's grid and roads beside the dense grid.
struct Pair {
    new: NavGrid,
    roads: RoadNet,
    old: dense::NavGrid,
}

impl Pair {
    fn of(map: serde_json::Value, rules: &contract::scenario::Rules) -> Self {
        let map: MapDefinition = serde_json::from_value(map).unwrap();
        let w = WorldGeometry::new(&map, rules);
        Pair {
            new: NavGrid::build(&w, w.props().cloned(), 0.3),
            roads: RoadNet::build(&w),
            old: dense::NavGrid::build(&w, w.props().cloned(), 0.3),
        }
    }
}

/// How the new routes' costs compare with the dense planner's, per policy.
#[derive(Default)]
struct Quality {
    routes: usize,
    worst: f64,
    sum: f64,
    /// Routes more than 1% and more than 10% dearer than the dense one.
    over_1pc: usize,
    over_10pc: usize,
    /// Routes cheaper than the dense one (it string-pulls greedily, so its
    /// route is not always the best the grid allows).
    cheaper: usize,
}

impl Quality {
    fn add(&mut self, ratio: f64) {
        self.routes += 1;
        self.worst = self.worst.max(ratio);
        self.sum += ratio;
        self.over_1pc += usize::from(ratio > 1.01);
        self.over_10pc += usize::from(ratio > 1.10);
        self.cheaper += usize::from(ratio < 1.0 - 1e-9);
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "routes": self.routes,
            "worst_ratio": self.worst,
            "mean_ratio": self.sum / self.routes.max(1) as f64,
            "over_1pc": self.over_1pc,
            "over_10pc": self.over_10pc,
            "cheaper": self.cheaper,
        })
    }
}

fn length(from: V2, route: &[V2]) -> f64 {
    let mut a = from;
    route
        .iter()
        .map(|&b| {
            let l = (b - a).length();
            a = b;
            l
        })
        .sum()
}

struct Report {
    rules: NavigationRules,
    cases: usize,
    blocked: usize,
    /// Legs only the dense planner, or only the new one, found a route for.
    only_dense: usize,
    only_new: usize,
    /// Legs whose route ends somewhere other than the dense planner's.
    moved_goals: usize,
    quality: [Quality; 2],
}

impl Report {
    /// Plan one leg both ways and set the routes side by side.
    fn leg(
        &mut self,
        pair: &mut Pair,
        from: V2,
        to: V2,
        m: &Mobility,
        policy: RoutePolicy,
        avoid: &[Obb2],
    ) {
        let om = old_mobility(m);
        let leg = Leg {
            from,
            goal: to,
            m,
            policy,
            avoid,
        };
        let (new, _) = sim::navigation::plan(&pair.new, &pair.roads, leg, &self.rules);
        let old = pair.old.plan_avoiding(from, to, &om, policy, avoid);
        self.cases += 1;
        let what = format!("{from:?} to {to:?} {m:?} {policy:?}");
        match (new, old) {
            (Plan::Route(new), dense::Plan::Route(old)) => {
                // A goal the new planner moved to standing room ends a
                // different leg: counted, not compared.
                if new.last() != old.last() {
                    self.moved_goals += 1;
                    return;
                }
                // (A leg that starts where its mover does not fit has no
                // route that fits from there, by either planner.)
                assert!(
                    pair.old.route_fits(from, &new, &om) || !pair.old.route_fits(from, &old, &om),
                    "the route fits the dense grid: {what}"
                );
                let cost = |route: &[V2]| match policy {
                    RoutePolicy::Shortest => length(from, route),
                    RoutePolicy::Fastest => pair.old.route_time(from, route, &om),
                };
                let (ours, theirs) = (cost(&new), cost(&old));
                if theirs > 0.0 && ours.is_finite() {
                    self.quality[policy as usize].add(ours / theirs);
                }
            }
            (Plan::Blocked(_), dense::Plan::Blocked(_)) => self.blocked += 1,
            (Plan::Blocked(_), dense::Plan::Route(_)) => self.only_dense += 1,
            (Plan::Route(_), dense::Plan::Blocked(_)) => self.only_new += 1,
        }
    }
}

fn main() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::village()).unwrap();
    let mut report = Report {
        rules: rules.navigation,
        cases: 0,
        blocked: 0,
        only_dense: 0,
        only_new: 0,
        moved_goals: 0,
        quality: Default::default(),
    };
    let variants = [
        serde_json::json!({}),
        serde_json::json!({"surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[4,8],[60,8]],"width_m":4}}]}),
        serde_json::json!({"props":[{"kind":"crate","center":[31,23],"yaw":0.4,"half_extents":[2,2,1]}]}),
        serde_json::json!({"rivers":[{"points":[{"xy":[32,0],"width_m":12,"depth_m":1.5},{"xy":[32,36],"width_m":12,"depth_m":1.5}],"surface_z":-0.5}]}),
        serde_json::json!({"rivers":[{"points":[{"xy":[32,0],"width_m":12,"depth_m":1.5},{"xy":[32,48],"width_m":12,"depth_m":1.5}],"surface_z":-0.5}], "bridges":[{"deck":"bridge_deck","center":[32,10],"half_extents":[12,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]}),
        serde_json::json!({"rivers":[{"points":[{"xy":[32,0],"width_m":12,"depth_m":1.5},{"xy":[32,48],"width_m":12,"depth_m":1.5}],"surface_z":-0.5}], "bridges":[{"deck":"bridge_deck","center":[32,10],"half_extents":[12,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}], "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[8,34],[24,34],[24,8]],"width_m":4}},{"kind":"road","shape":{"kind":"stroke","points":[[40,8],[40,40],[60,40]],"width_m":4}}]}),
        serde_json::json!({"relief":[{"kind":"mesa","rect":[28,12,8,20],"height_m":10,"side_degrees":45}]}),
        serde_json::json!({"forests":[{"shape":{"kind":"polygon","ring":[[24.0,12.0],[40.0,12.0],[40.0,32.0],[24.0,32.0]]}}]}),
        serde_json::json!({"props":[{"kind":"crate","center":[20,24],"yaw":0.2,"half_extents":[2,2,1]},{"kind":"sandbags","center":[31,24],"yaw":-0.2,"half_extents":[2,2,1]},{"kind":"tooth","center":[42,24],"yaw":0.4,"half_extents":[2,2,1]},{"kind":"wall","center":[51,24],"yaw":-0.4,"half_extents":[2,2,2]}]}),
    ];
    let small = |extra: &serde_json::Value, size: [f64; 2]| {
        let mut map = serde_json::json!({
            "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35
        });
        for (key, value) in extra.as_object().unwrap() {
            map[key] = value.clone();
        }
        map
    };
    for extra in &variants {
        let mut pair = Pair::of(small(extra, [64.0, 48.0]), &rules);
        for class in [MoverClass::Infantry, MoverClass::Vehicle] {
            for push in PushClass::ALL {
                let m = mobility(class, push);
                let om = old_mobility(&m);
                for x in [2.0, 2.25, 15.0, 31.5, 61.0] {
                    for y in [2.0, 16.25, 23.0, 45.0] {
                        let from = v2(x, y);
                        // The planner asks more room of a vehicle standing
                        // off the middle of its cell; it never finds room
                        // the dense grid does not.
                        assert!(
                            !pair.new.fits_at(from, &m) || pair.old.fits_at(from, &om),
                            "fit {extra} {m:?} {from:?}"
                        );
                        for to in [v2(3.0, 3.0), v2(59.25, 13.0), v2(60.0, 44.0)] {
                            for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                                report.leg(&mut pair, from, to, &m, policy, &[]);
                            }
                        }
                    }
                }
            }
        }
    }
    // A 32-cell storage boundary intersects both blockers and temporary traffic.
    for kind in ["crate", "sandbags", "tooth", "wall"] {
        for x in [63.0, 64.0, 65.0] {
            let props = serde_json::json!({
                "props":[{"kind":kind,"center":[x,64],"yaw":0.35,"half_extents":[2,2,2]}]
            });
            let mut pair = Pair::of(small(&props, [128.0, 96.0]), &rules);
            for class in [MoverClass::Infantry, MoverClass::Vehicle] {
                for push in PushClass::ALL {
                    let m = mobility(class, push);
                    for y in [61.5, 63.5, 64.0, 65.0, 66.5] {
                        let from = v2(8.25, y);
                        let to = v2(120.0, 128.0 - y);
                        let avoid = [Obb2 {
                            center: v2(64.0, y + 4.0),
                            half: v2(1.5, 2.0),
                            yaw: 0.2,
                        }];
                        for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                            report.leg(&mut pair, from, to, &m, policy, &avoid);
                        }
                    }
                }
            }
        }
    }
    let long_extent = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>().unwrap())
        .unwrap_or(1024);
    assert!(
        matches!(long_extent, 1024 | 4096),
        "only admitted dense oracle extents"
    );
    // Long, nearly diagonal routes across open ground.
    let side = f64::from(long_extent);
    let mut pair = Pair::of(small(&serde_json::json!({}), [side, side]), &rules);
    let long_goal = side - 7.0;
    for class in [MoverClass::Infantry, MoverClass::Vehicle] {
        let m = mobility(class, PushClass::Heavy);
        for epsilon in [0.0, 1e-8, 1e-6, 0.001, 0.01, 0.1, 0.5, 1.0] {
            for (from, to) in [
                (v2(5.0, 5.0), v2(long_goal, long_goal + epsilon)),
                (v2(long_goal, long_goal + epsilon), v2(5.0, 5.0)),
            ] {
                let policies: &[RoutePolicy] = if long_extent == 4096 {
                    &[RoutePolicy::Shortest]
                } else {
                    &[RoutePolicy::Shortest, RoutePolicy::Fastest]
                };
                for &policy in policies {
                    report.leg(&mut pair, from, to, &m, policy, &[]);
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "cases": report.cases,
            "blocked_in_both": report.blocked,
            "route_only_in_dense": report.only_dense,
            "route_only_in_new": report.only_new,
            "goals_moved": report.moved_goals,
            "shortest": report.quality[RoutePolicy::Shortest as usize].json(),
            "fastest": report.quality[RoutePolicy::Fastest as usize].json(),
        })
    );
}
