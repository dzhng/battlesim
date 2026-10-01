//! Route planning contracts on small crafted maps.
use contract::command::RoutePolicy;
use contract::map::{MapDefinition, MoverClass, PropDefinition};
use contract::scenario::PushClass;
use sim::math::{v2, V2};
use sim::navigation::{BlockReason, Mobility, NavGrid, Plan};
use sim::world::WorldGeometry;

/// Length of the polyline from `from` through `route`.
fn route_length(from: V2, route: &[V2]) -> f64 {
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

const TANK: Mobility = Mobility {
    off_road_mps: 6.0,
    road_mps: 12.0,
    forest_multiplier: 0.4,
    half_width_m: 1.8,
    class: MoverClass::Vehicle,
    push: PushClass::Heavy,
    drive: None,
};
const INFANTRY: Mobility = Mobility {
    off_road_mps: 3.0,
    road_mps: 3.9,
    forest_multiplier: 0.7,
    half_width_m: 0.5,
    class: MoverClass::Infantry,
    push: PushClass::None,
    drive: None,
};

fn world(extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
    ))
    .unwrap();
    WorldGeometry::new(&map, &crate::common::rules())
}
fn grid(w: &WorldGeometry) -> NavGrid {
    NavGrid::build(w, w.props().cloned(), 0.3)
}

fn route(plan: Plan) -> Vec<V2> {
    match plan {
        Plan::Route(r) => r,
        Plan::Blocked(b) => panic!("blocked: {b:?}"),
    }
}

/// Every point along the route (sampled) satisfies `ok`.
fn all_along(from: V2, r: &[V2], ok: impl Fn(V2) -> bool) -> bool {
    let mut a = from;
    r.iter().all(|&b| {
        let n = ((b - a).length() / 0.5).ceil().max(1.0) as usize;
        let good = (0..=n).all(|k| ok(a + (b - a) * (k as f64 / n as f64)));
        a = b;
        good
    })
}

#[test]
fn the_fastest_route_takes_the_road_and_the_shortest_does_not() {
    let w = world(
        r#","surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[20,20],[20,160],[380,160],[380,20]],"width_m":10}}]"#,
    );
    let mut g = grid(&w);
    let (from, to) = (v2(20.0, 20.0), v2(380.0, 20.0));
    let shortest = route(g.plan(from, to, &TANK, RoutePolicy::Shortest));
    let fastest = route(g.plan(from, to, &TANK, RoutePolicy::Fastest));
    assert!(
        route_length(from, &shortest) < 365.0,
        "{}",
        route_length(from, &shortest)
    );
    assert!(
        route_length(from, &fastest) > 550.0,
        "the fast route follows the road"
    );
    assert!(g.route_time(from, &fastest, &TANK) < g.route_time(from, &shortest, &TANK));
    // Infantry gains only ×1.3 on roads, so the detour is not worth it.
    let foot = route(g.plan(from, to, &INFANTRY, RoutePolicy::Fastest));
    assert!(route_length(from, &foot) < 365.0);
}

#[test]
fn speed_follows_surface_and_slope() {
    assert_eq!(
        TANK.speed(1.0, true, 0.0),
        12.0,
        "roads take precedence over forest"
    );
    assert!((TANK.speed(0.0, true, 0.0) - 2.4).abs() < 1e-12);
    assert!((TANK.speed(0.0, false, 25.0) - 3.0).abs() < 1e-12);
    assert!(
        (TANK.speed(0.0, false, 34.0) - 6.0 * 0.35).abs() < 1e-12,
        "floor near the cutoff"
    );
}

#[test]
fn one_slope_cutoff_blocks_everyone_and_routes_go_around() {
    let w = world(
        r#","relief":[{"kind":"mesa","rect":[180,0,40,160],"height_m":30,"side_degrees":45}]"#,
    );
    let mut g = grid(&w);
    let (from, to) = (v2(40.0, 60.0), v2(360.0, 60.0));
    for m in [&TANK, &INFANTRY] {
        let r = route(g.plan(from, to, m, RoutePolicy::Shortest));
        assert!(all_along(from, &r, |p| w
            .surface_at(p.x, p.y)
            .unwrap()
            .traversable));
        assert!(r.iter().any(|p| p.y > 160.0), "goes round the north end");
    }
}

#[test]
fn water_is_crossed_only_by_the_bridge() {
    let w = world(
        r#","water":[{"rect":[190,0,20,200],"bed_z":-2,"surface_z":-0.5}],
           "bridges":[{"deck":"bridge_deck","center":[200,40],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
    );
    let mut g = grid(&w);
    let (from, to) = (v2(40.0, 150.0), v2(360.0, 150.0));
    let r = route(g.plan(from, to, &TANK, RoutePolicy::Shortest));
    assert!(all_along(from, &r, |p| w
        .surface_at(p.x, p.y)
        .unwrap()
        .traversable));
    assert!(all_along(from, &r, |p| !(190.0..=210.0).contains(&p.x)
        || (p.y - 40.0).abs() < 5.0));
}

#[test]
fn routes_stay_on_the_map_and_never_cut_a_blocked_corner() {
    // Two crates touching only at a corner: the diagonal between them is closed.
    let w = world(
        r#","props":[{"kind":"crate","center":[199,99],"yaw":0,"half_extents":[1,1,1]},
                     {"kind":"crate","center":[201,101],"yaw":0,"half_extents":[1,1,1]}]"#,
    );
    let mut g = grid(&w);
    let (from, to) = (v2(190.0, 110.0), v2(210.0, 90.0));
    let r = route(g.plan(from, to, &INFANTRY, RoutePolicy::Shortest));
    assert!(all_along(from, &r, |p| w
        .props()
        .all(|c| !c.footprint().contains(p, 0.0))));
    let edge = route(g.plan(v2(5.0, 5.0), v2(395.0, 5.0), &TANK, RoutePolicy::Shortest));
    assert!(all_along(v2(5.0, 5.0), &edge, |p| w
        .height_at(p.x, p.y)
        .is_some()));
}

#[test]
fn a_gap_admits_infantry_but_not_a_tank() {
    // A wall across the map with a 5 m gap at y = 100 (y 97.5..102.5) and a wide
    // opening at the top. At the 2 m planning resolution a gap needs about 4 m
    // to be sure of a free cell; a tank's footprint needs about 6 m.
    let w = world(
        r#","props":[{"kind":"wall","center":[200,48.75],"yaw":0,"half_extents":[0.5,48.75,2]},
                     {"kind":"wall","center":[200,136.25],"yaw":0,"half_extents":[0.5,33.75,2]}]"#,
    );
    let mut g = grid(&w);
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let foot = route(g.plan(from, to, &INFANTRY, RoutePolicy::Shortest));
    assert!(route_length(from, &foot) < 110.0, "infantry uses the gap");
    let tank = route(g.plan(from, to, &TANK, RoutePolicy::Shortest));
    assert!(
        tank.iter().any(|p| p.y > 170.0),
        "the tank detours to the wide opening"
    );
    assert!(all_along(from, &tank, |p| g.fits_at(p, &TANK)));
}

/// Q27: every solid body stops infantry too. A solid line of tank wrecks
/// (too heavy for a tank to shove) sends a squad and a tank alike round
/// the wide opening at the top.
#[test]
fn a_line_of_wrecks_stops_squads_and_tanks_alike() {
    let wrecks: Vec<String> = (0..17)
        .map(|k| {
            format!(
                r#"{{"kind":"tank_wreck","center":[200,{}],"yaw":1.5708,"half_extents":[5,2,1.2]}}"#,
                5 + k * 10
            )
        })
        .collect();
    let w = world(&format!(r#","props":[{}]"#, wrecks.join(",")));
    let mut g = grid(&w);
    let (from, to) = (v2(150.0, 60.0), v2(250.0, 60.0));
    for m in [&INFANTRY, &TANK] {
        let r = route(g.plan(from, to, m, RoutePolicy::Shortest));
        assert!(r.iter().any(|p| p.y > 170.0), "{:?} goes round", m.class);
        assert!(all_along(from, &r, |p| g.fits_at(p, m)));
    }
}

/// The prop types are data (Q19): the catalog's body rows, read through each
/// placed prop. Buildings stop everyone; the bridge deck and rubble are
/// ground nobody walks round; only big static bodies hide what is behind
/// them (Q25).
#[test]
fn the_body_table_decides_who_is_stopped_and_what_hides() {
    let t = |id: &str| crate::common::props().by_id(id).body;
    for class in MoverClass::ALL {
        assert!(t("building").blocks.class(class));
        assert!(!t("bridge_deck").blocks.class(class));
        assert!(!t("rubble").blocks.class(class));
    }
    for kind in ["tooth", "tank_wreck", "crate", "fence"] {
        assert!(t(kind).blocks.infantry && t(kind).blocks.vehicle, "{kind}");
        assert!(!t(kind).occludes, "{kind} hides nothing (Q25)");
    }
    for kind in ["building", "ruin", "wall"] {
        assert!(t(kind).occludes, "{kind}");
    }
    let w = world(
        r#","props":[{"kind":"rubble","center":[100,100],"yaw":0,"half_extents":[10,1,0.5]}]"#,
    );
    let rubble = w.props().next().unwrap();
    assert!(!rubble.blocks(MoverClass::Infantry) && !rubble.body.stops_rounds);
}

#[test]
fn an_enclosed_goal_is_blocked_and_says_why() {
    let w = world(
        r#","props":[{"kind":"wall","center":[300,80],"yaw":0,"half_extents":[20,0.5,2]},
                     {"kind":"wall","center":[300,120],"yaw":0,"half_extents":[20,0.5,2]},
                     {"kind":"wall","center":[280,100],"yaw":0,"half_extents":[0.5,20,2]},
                     {"kind":"wall","center":[320,100],"yaw":0,"half_extents":[0.5,20,2]}]"#,
    );
    let mut g = grid(&w);
    assert_eq!(
        g.plan(
            v2(40.0, 100.0),
            v2(300.0, 100.0),
            &TANK,
            RoutePolicy::Shortest
        ),
        Plan::Blocked(BlockReason::NoRoute)
    );
}

#[test]
fn only_known_props_shape_the_plan() {
    let mut w = world("");
    let wall = w.add_prop(&PropDefinition {
        kind: "wall".into(),
        center: [200.0, 100.0],
        yaw: 0.0,
        half_extents: [0.5, 80.0, 2.0],
        base_z: None,
    });
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let mut unknown = NavGrid::build(&w, std::iter::empty(), 0.3);
    assert!(
        route_length(
            from,
            &route(unknown.plan(from, to, &TANK, RoutePolicy::Shortest))
        ) < 101.0
    );
    let mut known = NavGrid::build(&w, w.prop(wall).cloned().into_iter(), 0.3);
    assert!(
        route_length(
            from,
            &route(known.plan(from, to, &TANK, RoutePolicy::Shortest))
        ) > 150.0
    );
}

/// Q27's coarse resolution: a 2 m cell is open to infantry when its 0.5 m
/// sub-cells hold a gap a soldier fits through. A line of small blocks 1.2 m
/// apart stays open to a squad and closed to a tank; a solid wall of the
/// same line stays closed to both.
#[test]
fn a_line_of_teeth_admits_infantry_where_a_wall_does_not() {
    let teeth: Vec<String> = (0..100)
        .map(|k| {
            format!(
                r#"{{"kind":"wall","center":[200,{}],"yaw":0,"half_extents":[0.4,0.4,0.6]}}"#,
                1.0 + 2.0 * k as f64
            )
        })
        .collect();
    let w = world(&format!(r#","props":[{}]"#, teeth.join(",")));
    let mut g = grid(&w);
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let foot = route(g.plan(from, to, &INFANTRY, RoutePolicy::Shortest));
    assert!(
        route_length(from, &foot) < 101.0,
        "the squad crosses the line: {foot:?}"
    );
    let radius = 0.3;
    assert!(all_along(from, &foot, |p| w
        .props()
        .all(|t| !t.footprint().contains(p, radius - 0.05))));
    assert!(matches!(
        g.plan(from, to, &TANK, RoutePolicy::Shortest),
        Plan::Blocked(_)
    ));
    let wall = world(
        r#","props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[0.4,100,0.6]}]"#,
    );
    assert!(matches!(
        grid(&wall).plan(from, to, &INFANTRY, RoutePolicy::Shortest),
        Plan::Blocked(_)
    ));
}

#[test]
fn empty_ground_does_not_allocate_navigation_per_square_metre() {
    let w = world("");
    let g = grid(&w);
    assert_eq!(g.storage().cells, 0, "open ground has no exceptional cells");
    assert!(g.fits_at(v2(390.0, 190.0), &TANK));
}

#[test]
fn an_unbroken_water_strip_proves_no_route_without_exploring_a_map_half() {
    let w = world(r#","water":[{"rect":[190,0,20,200],"bed_z":-2,"surface_z":-0.5}]"#);
    let mut g = grid(&w);
    let plan = g.plan(
        v2(40.0, 100.0),
        v2(360.0, 100.0),
        &TANK,
        RoutePolicy::Shortest,
    );
    assert_eq!(plan, Plan::Blocked(BlockReason::NoRoute));
    assert_eq!(
        g.storage().search_cells,
        0,
        "a separating strip needs no search of either open half"
    );
}

#[test]
fn a_bridge_search_bounds_work_without_changing_the_crossing() {
    let w = world(
        r#", "water":[{"rect":[190,0,20,200],"bed_z":-2,"surface_z":-0.5}],
        "bridges":[{"deck":"bridge_deck","center":[200,40],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
    );
    for m in [&TANK, &INFANTRY] {
        for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
            let mut g = grid(&w);
            let (from, to) = (v2(40.0, 150.0), v2(360.0, 150.0));
            let r = route(g.plan(from, to, m, policy));
            assert_eq!(r.last(), Some(&to));
            assert!(g.route_fits(from, &r, m));
            assert!(
                g.storage().search_cells < 8_000,
                "{:?}: {:?}",
                policy,
                g.storage()
            );
        }
    }
}

#[test]
fn a_nonfinite_cost_keeps_the_original_grid_winner() {
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size":[64,48],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35
    }))
    .unwrap();
    let w = WorldGeometry::new(&map, &crate::common::rules());
    let m = Mobility {
        off_road_mps: f64::NAN,
        road_mps: f64::NAN,
        ..TANK
    };
    let mut g = grid(&w);
    assert_eq!(
        g.plan(v2(5.0, 5.0), v2(59.0, 5.0), &m, RoutePolicy::Fastest),
        Plan::Route(vec![v2(7.0, 3.0), v2(57.0, 3.0), v2(59.0, 5.0)])
    );
}

#[test]
fn polygon_road_costs_are_shared_by_navigation_while_sidewalks_cost_ground() {
    let make = |kind: &str| {
        world(&format!(
            r#", "surfaces":[{{"kind":"{kind}","shape":{{"kind":"polygon","ring":[[8,8],[392,8],[392,48],[8,48]]}}}}]"#
        ))
    };
    let from = v2(20.0, 20.0);
    let to = v2(380.0, 20.0);
    let road = grid(&make("road"));
    let sidewalk = grid(&make("sidewalk"));
    let ground = grid(&world(""));
    let stroke = grid(&world(
        r#", "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[8,28],[392,28]],"width_m":40}}]"#,
    ));
    assert_eq!(
        road.route_time(from, &[to], &TANK),
        stroke.route_time(from, &[to], &TANK)
    );
    assert_eq!(
        road.route_time(from, &[to], &TANK) * 2.0,
        ground.route_time(from, &[to], &TANK)
    );
    assert_eq!(
        sidewalk.route_time(from, &[to], &TANK),
        ground.route_time(from, &[to], &TANK)
    );
}

/// One grid serves every mover and policy of its side: what an earlier plan
/// worked out about the whole map must not answer for a different mover.
#[test]
fn a_grid_plans_each_mover_and_policy_as_a_fresh_grid_would() {
    // A road that pays for vehicles, and a line of teeth only infantry pass.
    let teeth: Vec<String> = (0..100)
        .map(|k| {
            format!(
                r#"{{"kind":"wall","center":[200,{}],"yaw":0,"half_extents":[0.4,0.4,0.6]}}"#,
                1.0 + 2.0 * k as f64
            )
        })
        .collect();
    let w = world(&format!(
        r#","surfaces":[{{"kind":"road","shape":{{"kind":"stroke","points":[[20,20],[20,160],[180,160],[180,20]],"width_m":10}}}}],"props":[{}]"#,
        teeth.join(",")
    ));
    let jeep = Mobility {
        off_road_mps: 9.0,
        road_mps: 30.0,
        ..TANK
    };
    let legs = [
        (v2(20.0, 20.0), v2(180.0, 20.0)),
        (v2(150.0, 100.0), v2(250.0, 100.0)),
        (v2(230.0, 30.0), v2(380.0, 170.0)),
    ];
    let mut shared = grid(&w);
    for m in [&TANK, &INFANTRY, &jeep, &TANK] {
        for policy in [RoutePolicy::Fastest, RoutePolicy::Shortest] {
            for (from, to) in legs {
                assert_eq!(
                    shared.plan(from, to, m, policy),
                    grid(&w).plan(from, to, m, policy),
                    "{:?} {policy:?} {from:?} to {to:?}",
                    m.class
                );
            }
        }
    }
}
