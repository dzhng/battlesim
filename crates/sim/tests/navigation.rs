//! Route planning contracts on small crafted maps.
use contract::command::RoutePolicy;
use contract::map::{MapDefinition, PropDefinition, PropKind};
use sim::math::{v2, V2};
use sim::navigation::{route_length, BlockReason, Mobility, NavGrid, Plan};
use sim::world::WorldGeometry;

const TANK: Mobility = Mobility {
    off_road_mps: 6.0,
    road_mps: 12.0,
    forest_multiplier: 0.4,
    half_width_m: 1.8,
};
const INFANTRY: Mobility = Mobility {
    off_road_mps: 3.0,
    road_mps: 3.9,
    forest_multiplier: 0.7,
    half_width_m: 0.5,
};

fn world(extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[400,200],"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
    ))
    .unwrap();
    WorldGeometry::new(&map)
}

fn grid(w: &WorldGeometry) -> NavGrid {
    NavGrid::build(w, w.props())
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
    let w = world(r#","roads":[{"points":[[20,20],[20,160],[380,160],[380,20]],"width_m":10}]"#);
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
        TANK.speed(true, true, 0.0),
        12.0,
        "roads take precedence over forest"
    );
    assert!((TANK.speed(false, true, 0.0) - 2.4).abs() < 1e-12);
    assert!((TANK.speed(false, false, 25.0) - 3.0).abs() < 1e-12);
    assert!(
        (TANK.speed(false, false, 34.0) - 6.0 * 0.35).abs() < 1e-12,
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
           "bridges":[{"center":[200,40],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
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
        kind: PropKind::Wall,
        center: [200.0, 100.0],
        yaw: 0.0,
        half_extents: [0.5, 80.0, 2.0],
        base_z: None,
    });
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let mut unknown = NavGrid::build(&w, std::iter::empty());
    assert!(
        route_length(
            from,
            &route(unknown.plan(from, to, &TANK, RoutePolicy::Shortest))
        ) < 101.0
    );
    let mut known = NavGrid::build(&w, w.prop(wall).into_iter());
    assert!(
        route_length(
            from,
            &route(known.plan(from, to, &TANK, RoutePolicy::Shortest))
        ) > 150.0
    );
}
