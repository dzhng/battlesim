//! Safe matched navigation oracle. The frozen planner is evidence only.
use contract::command::RoutePolicy;
use contract::map::{MapDefinition, MoverClass};
use contract::scenario::PushClass;
use math::{v2, V2};
use sim::navigation::{Mobility, NavGrid, Plan};
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
fn route(p: Plan) -> Option<Vec<V2>> {
    match p {
        Plan::Route(r) => Some(r),
        Plan::Blocked(_) => None,
    }
}
fn main() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::village()).unwrap();
    let variants = [
        "",
        r#", "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[4,8],[60,8]],"width_m":4}}]"#,
        r#", "props":[{"kind":"crate","center":[31,23],"yaw":0.4,"half_extents":[2,2,1]}]"#,
        r#", "water":[{"rect":[28,0,8,36],"bed_z":-2,"surface_z":-0.5}]"#,
        r#", "water":[{"rect":[28,0,8,48],"bed_z":-2,"surface_z":-0.5}], "bridges":[{"deck":"bridge_deck","center":[32,10],"half_extents":[12,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
        r#", "water":[{"rect":[28,0,8,48],"bed_z":-2,"surface_z":-0.5}], "bridges":[{"deck":"bridge_deck","center":[32,10],"half_extents":[12,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}], "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[8,34],[24,34],[24,8]],"width_m":4}},{"kind":"road","shape":{"kind":"stroke","points":[[40,8],[40,40],[60,40]],"width_m":4}}]"#,
        r#", "relief":[{"kind":"mesa","rect":[28,12,8,20],"height_m":10,"side_degrees":45}]"#,
        r#", "forests":[{"shape":{"kind":"polygon","ring":[[24.0,12.0],[40.0,12.0],[40.0,32.0],[24.0,32.0]]}}]"#,
        r#", "props":[{"kind":"crate","center":[20,24],"yaw":0.2,"half_extents":[2,2,1]},{"kind":"sandbags","center":[31,24],"yaw":-0.2,"half_extents":[2,2,1]},{"kind":"tooth","center":[42,24],"yaw":0.4,"half_extents":[2,2,1]},{"kind":"wall","center":[51,24],"yaw":-0.4,"half_extents":[2,2,2]}]"#,
    ];
    let mut cases = 0;
    for extra in variants {
        let map: MapDefinition = serde_json::from_str(&format!(
            r#"{{"size":[64,48],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
        ))
        .unwrap();
        let w = WorldGeometry::new(&map, &rules);
        let mut new = NavGrid::build(&w, w.props().cloned(), 0.3);
        let mut old = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
        for class in [MoverClass::Infantry, MoverClass::Vehicle] {
            for push in PushClass::ALL {
                let m = mobility(class, push);
                let om = old_mobility(&m);
                for x in [2.0, 2.25, 15.0, 31.5, 61.0] {
                    for y in [2.0, 16.25, 23.0, 45.0] {
                        let from = v2(x, y);
                        assert_eq!(
                            new.fits_at(from, &m),
                            old.fits_at(from, &om),
                            "fit {extra} {m:?} {from:?}"
                        );
                        assert_eq!(new.snap(from, &m, 6.0), old.snap(from, &om, 6.0));
                        for to in [v2(3.0, 3.0), v2(59.25, 13.0), v2(60.0, 44.0)] {
                            for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                                let a = new.plan(from, to, &m, policy);
                                let b = old.plan(from, to, &om, policy);
                                match (&a, &b) {
                                    (Plan::Route(a), dense::Plan::Route(b)) => assert_eq!(
                                        a, b,
                                        "route {extra} {m:?} {from:?} {to:?} {policy:?}"
                                    ),
                                    (Plan::Blocked(a), dense::Plan::Blocked(b)) => {
                                        assert_eq!(format!("{a:?}"), format!("{b:?}"))
                                    }
                                    _ => panic!("mismatch {a:?} {b:?}"),
                                }
                                if let Some(r) = route(a) {
                                    assert_eq!(
                                        new.route_time(from, &r, &m),
                                        old.route_time(from, &r, &om)
                                    );
                                }
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    // A 32-cell storage boundary intersects both blockers and temporary traffic.
    // Compare the real public calls with the original planner at every offset
    // across that boundary; retained scratch is reused between all calls.
    for kind in ["crate", "sandbags", "tooth", "wall"] {
        for x in [63.0, 64.0, 65.0] {
            let map: MapDefinition = serde_json::from_value(serde_json::json!({
                "size":[128,96], "fog_cell_m":8,"height_grid_m":4, "slope_cutoff_deg":35,
                "props":[{"kind":kind,"center":[x,64],"yaw":0.35,"half_extents":[2,2,2]}]
            }))
            .unwrap();
            let w = WorldGeometry::new(&map, &rules);
            let mut new = NavGrid::build(&w, w.props().cloned(), 0.3);
            let mut old = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
            for class in [MoverClass::Infantry, MoverClass::Vehicle] {
                for push in PushClass::ALL {
                    let m = mobility(class, push);
                    let om = old_mobility(&m);
                    for y in [61.5, 63.5, 64.0, 65.0, 66.5] {
                        let from = v2(8.25, y);
                        let to = v2(120.0, 128.0 - y);
                        let avoid = [math::Obb2 {
                            center: v2(64.0, y + 4.0),
                            half: v2(1.5, 2.0),
                            yaw: 0.2,
                        }];
                        for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                            assert_eq!(new.fits_at(v2(x, y), &m), old.fits_at(v2(x, y), &om));
                            assert_eq!(new.snap(v2(x, y), &m, 6.0), old.snap(v2(x, y), &om, 6.0));
                            let a = new.plan_avoiding(from, to, &m, policy, &avoid);
                            let b = old.plan_avoiding(from, to, &om, policy, &avoid);
                            assert_eq!(
                                format!("{a:?}"),
                                format!("{b:?}"),
                                "boundary {kind} {x} {y} {m:?} {policy:?}"
                            );
                            if let Some(r) = route(a) {
                                assert_eq!(
                                    new.route_time(from, &r, &m),
                                    old.route_time(from, &r, &om)
                                );
                                assert_eq!(
                                    new.route_fits(from, &r, &m),
                                    old.route_fits(from, &r, &om)
                                );
                            }
                            cases += 1;
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
    // Nearly collinear long routes expose accumulated-cost rounding at the
    // string-pulling threshold; coarse angle samples cannot cover that seam.
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size":[long_extent,long_extent],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35
    }))
    .unwrap();
    let w = WorldGeometry::new(&map, &rules);
    let mut new = NavGrid::build(&w, w.props().cloned(), 0.3);
    let mut old = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
    let long_goal = f64::from(long_extent) - 7.0;
    for class in [MoverClass::Infantry, MoverClass::Vehicle] {
        let m = mobility(class, PushClass::Heavy);
        let om = old_mobility(&m);
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
                    let a = new.plan(from, to, &m, policy);
                    let b = old.plan(from, to, &om, policy);
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{b:?}"),
                        "long {epsilon} {from:?} {to:?} {class:?} {policy:?}"
                    );
                    cases += 1;
                }
            }
        }
    }
    let thin: MapDefinition = serde_json::from_value(
        serde_json::json!({"size":[2,32],"fog_cell_m":8,"height_grid_m":2,"slope_cutoff_deg":35}),
    )
    .unwrap();
    let w = WorldGeometry::new(&thin, &rules);
    let mut new = NavGrid::build(&w, w.props().cloned(), 0.3);
    let mut old = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
    let m = mobility(MoverClass::Infantry, PushClass::None);
    let om = old_mobility(&m);
    let a = new.plan(v2(1.0, 3.0), v2(1.0, 29.0), &m, RoutePolicy::Shortest);
    let b = old.plan(v2(1.0, 3.0), v2(1.0, 29.0), &om, RoutePolicy::Shortest);
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "narrow-grid oracle");
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size":[64,48],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35
    }))
    .unwrap();
    let w = WorldGeometry::new(&map, &rules);
    let mut new = NavGrid::build(&w, w.props().cloned(), 0.3);
    let mut old = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
    for speed in [0.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE, 1e-12, 1e12] {
        let mut m = mobility(MoverClass::Vehicle, PushClass::Heavy);
        m.off_road_mps = speed;
        m.road_mps = speed;
        let om = old_mobility(&m);
        for to in [v2(59.0, 5.0), v2(43.0, 43.00000001)] {
            let from = v2(5.0, 5.0);
            let a = new.plan(from, to, &m, RoutePolicy::Fastest);
            let b = old.plan(from, to, &om, RoutePolicy::Fastest);
            assert_eq!(
                format!("{a:?}"),
                format!("{b:?}"),
                "raw speed {speed:?} {to:?}"
            );
            cases += 1;
        }
    }
    println!("{{\"matched_cases\":{cases}}}");
}
