//! Route planning contracts on small crafted maps.
use contract::command::RoutePolicy;
use contract::map::{MapDefinition, MoverClass, PropDefinition};
use contract::scenario::PushClass;
use sim::math::{v2, V2};
use sim::navigation::{BlockReason, Leg, Mobility, NavBase, NavGrid, Plan, RoadNet};
use sim::world::WorldGeometry;
use std::sync::Arc;

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
    world_with_rules(extra, &crate::common::rules())
}

fn world_with_rules(extra: &str, rules: &contract::scenario::Rules) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
    ))
    .unwrap();
    WorldGeometry::new(&map, rules)
}
/// What a side plans a leg with: its grid of the bodies it knows, and the
/// map's roads.
struct Known {
    grid: NavGrid,
    roads: RoadNet,
}

impl std::ops::Deref for Known {
    type Target = NavGrid;
    fn deref(&self) -> &NavGrid {
        &self.grid
    }
}

impl Known {
    fn of(grid: NavGrid, w: &WorldGeometry) -> Self {
        Known {
            grid,
            roads: RoadNet::build(w),
        }
    }

    /// The whole route, by the shipped planning rules.
    fn plan(&self, from: V2, goal: V2, m: &Mobility, policy: RoutePolicy) -> Plan {
        self.search(from, goal, m, policy).0
    }

    /// The whole route, and how many cells its searches reached.
    fn search(&self, from: V2, goal: V2, m: &Mobility, policy: RoutePolicy) -> (Plan, usize) {
        let leg = Leg {
            from,
            goal,
            m,
            policy,
            avoid: &[],
        };
        let rules = crate::common::rules().navigation;
        let (plan, work) = sim::navigation::plan(&self.grid, &self.roads, leg, &rules);
        (plan, work.cells)
    }
}

/// A side that knows every body on the map.
fn grid(w: &WorldGeometry) -> Known {
    Known::of(NavGrid::new(Arc::new(NavBase::build(w, w.props(), 0.3))), w)
}

/// Fixed road admission and costs make failure/recovery tracers independent
/// of later game-data tuning; the map has only 20,000 cells to exhaust.
fn road_fixture(extra: &str) -> (Known, contract::scenario::NavigationRules) {
    let mut rules = crate::common::rules();
    rules
        .surfaces
        .get_mut(&contract::map::SurfaceKind::Road)
        .unwrap()
        .speed_factor = 1.0;
    rules
        .surfaces
        .get_mut(&contract::map::SurfaceKind::DirtTrack)
        .unwrap()
        .speed_factor = 0.75;
    rules.navigation.search_cells_base = 20_000;
    rules.navigation.search_cells_per_m = 200;
    rules.navigation.road_access_m = 256.0;
    (grid(&world_with_rules(extra, &rules)), rules.navigation)
}

fn route(plan: Plan) -> Vec<V2> {
    match plan {
        Plan::Route(r) => r,
        Plan::Blocked(b) => panic!("blocked: {b:?}"),
    }
}

/// Every point along the route (sampled) satisfies `ok`.
/// Whether a footprint could stand in the middle of the cell `p` lies in:
/// the room every step of a route is judged by. (A point off its cell's
/// middle has less room, which `route_fits` accounts for along a segment.)
fn cell_fits(g: &NavGrid, p: V2, m: &Mobility) -> bool {
    let middle = |v: f64| (v / 2.0).floor() * 2.0 + 1.0;
    g.fits_at(v2(middle(p.x), middle(p.y)), m)
}

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
    let g = grid(&w);
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
fn road_exits_do_not_multiply_an_enclosed_destinations_search() {
    // The tank can stand in this courtyard, but cannot cross its walls.
    // Every road ends outside it; adding exits cannot make the goal reachable.
    let roads: Vec<_> = (0..12)
        .map(|k| format!(r#"{{"kind":"road","shape":{{"kind":"stroke","points":[[20,{}],[280,{}]],"width_m":6}}}}"#, 10 + k * 15, 10 + k * 15))
        .collect();
    let (g, rules) = road_fixture(&format!(
        r#", "surfaces":[{}], "props":[
            {{"kind":"wall","center":[290,100],"half_extents":[1,12,2],"yaw":0}},
            {{"kind":"wall","center":[310,100],"half_extents":[1,12,2],"yaw":0}},
            {{"kind":"wall","center":[300,89],"half_extents":[11,1,2],"yaw":0}},
            {{"kind":"wall","center":[300,111],"half_extents":[11,1,2],"yaw":0}}
        ]"#,
        roads.join(",")
    ));
    let (from, goal) = (v2(20.0, 100.0), v2(300.0, 100.0));
    assert!(g.fits_at(from, &TANK) && g.fits_at(goal, &TANK));
    let (plan, work) = sim::navigation::plan(
        &g.grid,
        &g.roads,
        Leg {
            from,
            goal,
            m: &TANK,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        },
        &rules,
    );
    assert_eq!(plan, Plan::Blocked(BlockReason::NoRoute));
    // At most one search of the 200×100 grid, plus local road access and
    // the courtyard's fewer than 100 cells, before proving the obstruction.
    assert!(
        work.expanded < 21_000,
        "one failed connector and courtyard proof, not a map search per exit: {work:?}"
    );
}

#[test]
fn thin_closed_goals_do_not_repeat_a_search_for_each_road_exit() {
    let roads: Vec<_> = (0..128).map(|k| format!(r#"{{"kind":"road","shape":{{"kind":"stroke","points":[[20,{}],[280,{}]],"width_m":6}}}}"#,10.0+k as f64*180.0/127.0,10.0+k as f64*180.0/127.0)).collect();
    let (g, rules) = road_fixture(&format!(
        r#", "surfaces":[{}], "props":[
    {{"kind":"wall","center":[289.25,100],"half_extents":[0.1,14,2],"yaw":0}},
    {{"kind":"wall","center":[311.25,100],"half_extents":[0.1,14,2],"yaw":0}},
    {{"kind":"wall","center":[300,87.25],"half_extents":[12,0.1,2],"yaw":0}},
    {{"kind":"wall","center":[300,113.25],"half_extents":[12,0.1,2],"yaw":0}}]"#,
        roads.join(",")
    ));
    let narrow = Mobility {
        half_width_m: 0.3,
        ..TANK
    };
    let (from, goal) = (v2(20.0, 100.0), v2(300.0, 100.0));
    assert!(g.fits_at(from, &narrow) && g.fits_at(goal, &narrow));
    let before = g.grid.work();
    let (plan, work) = sim::navigation::plan(
        &g.grid,
        &g.roads,
        Leg {
            from,
            goal,
            m: &narrow,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        },
        &rules,
    );
    assert_eq!(plan, Plan::Blocked(BlockReason::NoRoute));
    assert!(
        g.grid.work() - before < 300_000,
        "one map search and flat road admission, not a repeated graph per exit: {}",
        g.grid.work() - before
    );
    // One failed outside search can visit the whole 20,000-cell map.
    // The closed courtyard has fewer than 200 cells; later exits only
    // require membership in that same final-connector component.
    assert!(
        work.expanded < 21_000,
        "a closed goal must not repeat the outside search per exit: {work:?}"
    );
}

#[test]
fn filtered_near_accesses_do_not_hide_a_far_sampled_road_into_the_goal_component() {
    let walls: Vec<_> = (0..100)
        .map(|k| {
            format!(
                r#"{{"kind":"wall","center":[{},{}],"half_extents":[0.05,0.05,2],"yaw":0}}"#,
                201 + 2 * k,
                199 - 2 * k
            )
        })
        .collect();
    let (g, mut rules) = road_fixture(&format!(
        r#", "rivers":[{{"points":[{{"xy":[0,30],"width_m":12,"depth_m":1.5}},{{"xy":[400,30],"width_m":12,"depth_m":1.5}}],"surface_z":-0.5}}], "bridges":[{{"deck":"bridge_deck","center":[330,30],"half_extents":[16,5],"yaw":1.5707963267948966,"deck_z":0.1,"thickness_m":0.8}}], "props":[{}], "surfaces":[
      {{"kind":"road","shape":{{"kind":"stroke","points":[[380,3],[330,3],[330,50],[195,194]],"width_m":10}}}},
      {{"kind":"road","shape":{{"kind":"stroke","points":[[180,194],[195,194]],"width_m":10}}}},
      {{"kind":"dirt_track","shape":{{"kind":"stroke","points":[[380,3],[330,3],[330,50],[390,110],[250,175]],"width_m":1}}}}]"#,
        walls.join(",")
    ));
    rules.road_access_m = 20.0;
    let narrow = Mobility {
        half_width_m: 0.2,
        road_mps: 30.0,
        ..TANK
    };
    let (from, goal) = (v2(380.0, 3.0), v2(210.0, 194.0));
    assert!(g.fits_at(from, &narrow) && g.fits_at(goal, &narrow));
    assert!(
        g.route_fits(
            from,
            &[
                v2(330.0, 3.0),
                v2(330.0, 50.0),
                v2(390.0, 110.0),
                v2(250.0, 175.0),
                goal
            ],
            &narrow
        ),
        "the farther sampled road reaches the strict component"
    );
    let planned = route(
        sim::navigation::plan(
            &g.grid,
            &g.roads,
            Leg {
                from,
                goal,
                m: &narrow,
                policy: RoutePolicy::Fastest,
                avoid: &[],
            },
            &rules,
        )
        .0,
    );
    assert!(g.route_fits(from, &planned, &narrow));
    assert!(g.route_time(from, &planned, &narrow).is_finite());
    assert_eq!(planned.last(), Some(&goal));
}

#[test]
fn an_inconclusive_goal_probe_keeps_a_legal_alternate_road_exit() {
    let (g, mut rules) = road_fixture(
        r#", "surfaces":[
        {"kind":"road","shape":{"kind":"stroke","points":[[20,100],[280,100]],"width_m":10}},
        {"kind":"road","shape":{"kind":"stroke","points":[[20,100],[20,190],[310,190],[310,100]],"width_m":10}}
        ], "props":[{"kind":"wall","center":[290,100],"half_extents":[1,80,2],"yaw":0}]"#,
    );
    rules.search_cells_base = 30;
    rules.search_cells_per_m = 0;
    let (from, goal) = (v2(20.0, 100.0), v2(310.0, 100.0));
    // The destination is connected through the distant opening. A short
    // reverse search toward the cheap exit cannot prove its component closed.
    assert_eq!(
        sim::navigation::plan(
            &g.grid,
            &g.roads,
            Leg {
                from: goal,
                goal: v2(218.0, 100.0),
                m: &TANK,
                policy: RoutePolicy::Shortest,
                avoid: &[]
            },
            &rules
        )
        .0,
        Plan::Blocked(BlockReason::SearchLimit),
    );
    let planned = route(
        sim::navigation::plan(
            &g.grid,
            &g.roads,
            Leg {
                from,
                goal,
                m: &TANK,
                policy: RoutePolicy::Fastest,
                avoid: &[],
            },
            &rules,
        )
        .0,
    );
    assert!(
        planned.iter().any(|p| p.y > 180.0),
        "takes the distant legal road exit: {planned:?}"
    );
    assert!(g.route_fits(from, &planned, &TANK));
    assert!(g.route_time(from, &planned, &TANK).is_finite());
    assert_eq!(planned.last(), Some(&goal));
}

#[test]
fn goal_component_proofs_include_sampled_corner_crossings() {
    let walls: Vec<_> = (0..50)
        .map(|k| {
            format!(
                r#"{{"kind":"wall","center":[{},{}],"half_extents":[0.05,0.05,2],"yaw":0}}"#,
                1 + k * 2,
                99 - k * 2
            )
        })
        .collect();
    let (g, rules) = road_fixture(&format!(
        r#", "props":[{}], "surfaces":[
        {{"kind":"dirt_track","shape":{{"kind":"stroke","points":[[21,21],[81,81]],"width_m":1}}}},
        {{"kind":"road","shape":{{"kind":"stroke","points":[[21,21],[49,45]],"width_m":10}}}}
        ]"#,
        walls.join(",")
    ));
    let narrow = Mobility {
        half_width_m: 0.3,
        off_road_mps: 8.0,
        ..TANK
    };
    let (from, goal) = (v2(21.0, 21.0), v2(81.0, 81.0));
    assert!(g.fits_at(from, &narrow) && g.fits_at(goal, &narrow));
    assert!(
        g.route_fits(from, &[goal], &narrow),
        "the diagonal crossing is sampled-clear"
    );
    // The ordinary no-corner cell graph rejects this crossing. Roads use
    // the sampled reader, so exhausting that stricter graph is not a proof.
    assert_eq!(
        sim::navigation::plan(
            &g.grid,
            &RoadNet::default(),
            Leg {
                from,
                goal,
                m: &narrow,
                policy: RoutePolicy::Fastest,
                avoid: &[]
            },
            &rules
        )
        .0,
        Plan::Blocked(BlockReason::NoRoute)
    );
    let planned = route(
        sim::navigation::plan(
            &g.grid,
            &g.roads,
            Leg {
                from,
                goal,
                m: &narrow,
                policy: RoutePolicy::Fastest,
                avoid: &[],
            },
            &rules,
        )
        .0,
    );
    assert!(g.route_fits(from, &planned, &narrow));
    assert!(g.route_time(from, &planned, &narrow).is_finite());
    assert_eq!(planned.last(), Some(&goal));
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
    let g = grid(&w);
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
        r#","rivers":[{"points":[{"xy":[200,0],"width_m":20,"depth_m":1.5},{"xy":[200,200],"width_m":20,"depth_m":1.5}],"surface_z":-0.5}],
           "bridges":[{"deck":"bridge_deck","center":[200,40],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
    );
    let g = grid(&w);
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
    let g = grid(&w);
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
    let g = grid(&w);
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let foot = route(g.plan(from, to, &INFANTRY, RoutePolicy::Shortest));
    assert!(route_length(from, &foot) < 110.0, "infantry uses the gap");
    let tank = route(g.plan(from, to, &TANK, RoutePolicy::Shortest));
    assert!(
        tank.iter().any(|p| p.y > 170.0),
        "the tank detours to the wide opening"
    );
    assert!(g.route_fits(from, &tank, &TANK));
    assert!(all_along(from, &tank, |p| cell_fits(&g, p, &TANK)));
}

/// Q27: every solid body stops infantry too. A solid line of tank wrecks
/// (too heavy for a tank to shove) sends a squad and a tank alike round
/// the wide opening at the top.
#[test]
fn a_line_of_wrecks_stops_squads_and_tanks_alike() {
    let wrecks: Vec<String> = (0..17)
        .map(|k| {
            format!(
                r#"{{"kind":"heavy_wreck","center":[200,{}],"yaw":1.5708,"half_extents":[5,2,1.2]}}"#,
                5 + k * 10
            )
        })
        .collect();
    let w = world(&format!(r#","props":[{}]"#, wrecks.join(",")));
    let g = grid(&w);
    let (from, to) = (v2(150.0, 60.0), v2(250.0, 60.0));
    for m in [&INFANTRY, &TANK] {
        let r = route(g.plan(from, to, m, RoutePolicy::Shortest));
        assert!(r.iter().any(|p| p.y > 170.0), "{:?} goes round", m.class);
        assert!(g.route_fits(from, &r, m));
        assert!(all_along(from, &r, |p| cell_fits(&g, p, m)));
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
    for kind in ["tooth", "heavy_wreck", "crate", "fence"] {
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
    let g = grid(&w);
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
    let map = Arc::new(NavBase::build(&w, w.props(), 0.3));
    let wall = w.add_prop(&PropDefinition {
        kind: "wall".into(),
        center: [200.0, 100.0],
        yaw: 0.0,
        half_extents: [0.5, 80.0, 2.0],
        base_z: None,
    });
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    let unknown = Known::of(NavGrid::new(Arc::clone(&map)), &w);
    assert!(
        route_length(
            from,
            &route(unknown.plan(from, to, &TANK, RoutePolicy::Shortest))
        ) < 101.0
    );
    let mut learned = NavGrid::new(map);
    let seen = [(wall, w.prop(wall).cloned())];
    learned.update(&w, seen.into_iter(), std::iter::empty());
    let known = Known::of(learned, &w);
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
    let g = grid(&w);
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
    assert_eq!(g.storage().cell_pages, 0, "open ground stores no cells");
    assert!(g.fits_at(v2(390.0, 190.0), &TANK));
}

#[test]
fn an_unbroken_water_strip_leaves_no_route() {
    let w = world(
        r#","rivers":[{"points":[{"xy":[200,0],"width_m":20,"depth_m":1.5},{"xy":[200,200],"width_m":20,"depth_m":1.5}],"surface_z":-0.5}]"#,
    );
    let g = grid(&w);
    let (plan, cells) = g.search(
        v2(40.0, 100.0),
        v2(360.0, 100.0),
        &TANK,
        RoutePolicy::Shortest,
    );
    assert_eq!(plan, Plan::Blocked(BlockReason::NoRoute));
    assert_eq!(cells, 0, "public terrain proves the banks disconnected");
}

#[test]
fn a_bridge_search_bounds_work_without_changing_the_crossing() {
    let w = world(
        r#", "rivers":[{"points":[{"xy":[200,0],"width_m":20,"depth_m":1.5},{"xy":[200,200],"width_m":20,"depth_m":1.5}],"surface_z":-0.5}],
        "bridges":[{"deck":"bridge_deck","center":[200,40],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
    );
    for m in [&TANK, &INFANTRY] {
        for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
            let g = grid(&w);
            let (from, to) = (v2(40.0, 150.0), v2(360.0, 150.0));
            let (plan, cells) = g.search(from, to, m, policy);
            let r = route(plan);
            assert_eq!(r.last(), Some(&to));
            assert!(g.route_fits(from, &r, m));
            assert!(cells < 8_000, "{policy:?}: {cells}");
        }
    }
}

/// The river lab's meander lies between each start and its goal: a squad and
/// a tank both go round by the country road's bridge, up its ramp and over
/// its deck, and no step of either route is on water.
#[test]
fn a_meandering_river_is_crossed_by_its_bridge_by_squads_and_tanks() {
    let map: MapDefinition = sim::maps::load("river").unwrap().definition;
    let w = WorldGeometry::new(&map, &crate::common::rules());
    let river = &w.rivers()[0];
    for m in [&TANK, &INFANTRY] {
        for (from, to) in [
            (v2(125.0, 195.0), v2(125.0, 300.0)),
            (v2(450.0, 176.0), v2(420.0, 300.0)),
            (v2(60.0, 185.0), v2(60.0, 310.0)),
        ] {
            let g = grid(&w);
            let r = route(g.plan(from, to, m, RoutePolicy::Shortest));
            assert_eq!(r.last(), Some(&to));
            assert!(g.route_fits(from, &r, m));
            let crossed = std::cell::Cell::new(false);
            assert!(all_along(from, &r, |p| {
                let s = w.surface_at(p.x, p.y).unwrap();
                // Wherever the route is over the river's water it is on the deck.
                let over_water = river.inside([p.x, p.y]) >= 0.0;
                crossed.set(crossed.get() || over_water);
                s.traversable && (!over_water || s.kind == sim::world::SurfaceKind::Bridge)
            }));
            assert!(
                crossed.get(),
                "the route from {from:?} never crossed the river"
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
    let g = grid(&w);
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
    let shared = grid(&w);
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

#[test]
fn placement_does_not_spend_or_pre_pay_route_work() {
    let w =
        world(r#", "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[1,30,2]}]"#);
    let warm = grid(&w);
    let cold = grid(&w);
    let before = warm.work();
    warm.placement_point(v2(193.0, 95.0), &TANK);
    assert_eq!(warm.work(), before, "placement has no route-work charge");
    let from = v2(180.0, 100.0);
    let to = v2(220.0, 100.0);
    let a = route(warm.plan(from, to, &TANK, RoutePolicy::Shortest));
    let b = route(cold.plan(from, to, &TANK, RoutePolicy::Shortest));
    assert_eq!(a, b);
    assert_eq!(
        warm.work() - before,
        cold.work(),
        "warming values must not make the later route cheaper to schedule"
    );
}

/// A planned infantry town detour must carry a finite physical travel time.
#[test]
fn infantry_town_detours_have_finite_route_times() {
    let (from, to) = (v2(150.0, 100.0), v2(250.0, 100.0));
    for (yaw, offset) in [
        (0.0, 0.0),
        (0.0, 0.5),
        (0.0, 1.0),
        (0.13, 0.0),
        (0.27, 0.5),
        (0.6, 1.0),
    ] {
        let w = world(&format!(
            r#", "props":[{{"kind":"wall","center":[200,{}],"yaw":{},"half_extents":[10,10,4]}}]"#,
            100.0 + offset,
            yaw
        ));
        let g = grid(&w);
        let path = route(g.plan(from, to, &INFANTRY, RoutePolicy::Shortest));
        let time = g.route_time(from, &path, &INFANTRY);
        assert!(g.route_fits(from, &path, &INFANTRY));
        assert!(
            time.is_finite(),
            "yaw {yaw}, offset {offset}, route {path:?}, time {time}"
        );
    }
}

#[test]
fn infantry_timing_keeps_start_and_goal_boundary_failures_explicit() {
    let w = world("");
    let g = grid(&w);
    assert_eq!(
        g.plan(
            v2(-1.0, 100.0),
            v2(100.0, 100.0),
            &INFANTRY,
            RoutePolicy::Shortest
        ),
        Plan::Blocked(BlockReason::StartEnclosed)
    );
    assert_eq!(
        g.plan(
            v2(100.0, 100.0),
            v2(420.0, 100.0),
            &INFANTRY,
            RoutePolicy::Shortest
        ),
        Plan::Blocked(BlockReason::NoRoute)
    );
    let p = v2(0.0, 100.0);
    let same = route(g.plan(p, p, &INFANTRY, RoutePolicy::Shortest));
    assert_eq!(g.route_time(p, &same, &INFANTRY), 0.0);
    let moved = route(g.plan(p, v2(20.0, 100.0), &INFANTRY, RoutePolicy::Shortest));
    assert!(g.route_time(p, &moved, &INFANTRY) > 0.0);
    assert!(g.route_fits(p, &moved, &INFANTRY));
}

#[test]
fn a_one_man_town_passage_has_a_finite_time_without_opening_a_wall() {
    let w = world(
        r#", "props":[
        {"kind":"wall","center":[200,50.2],"yaw":0,"half_extents":[1,50.2,4]},
        {"kind":"wall","center":[200,150.8],"yaw":0,"half_extents":[1,49.2,4]}]"#,
    );
    let g = grid(&w);
    let from = v2(150.0, 101.0);
    let to = v2(250.0, 101.0);
    let foot = route(g.plan(from, to, &INFANTRY, RoutePolicy::Fastest));
    assert!(g.route_fits(from, &foot, &INFANTRY));
    let time = g.route_time(from, &foot, &INFANTRY);
    assert!(time.is_finite() && time > 0.0);
    assert!(matches!(
        g.plan(from, to, &TANK, RoutePolicy::Shortest),
        Plan::Blocked(_)
    ));
    let closed =
        world(r#", "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[1,100,4]}]"#);
    let closed = grid(&closed);
    assert!(closed.route_time(from, &[to], &INFANTRY).is_infinite());
    assert!(!closed.route_fits(from, &[to], &INFANTRY));
    assert_eq!(
        closed.plan(from, to, &INFANTRY, RoutePolicy::Shortest),
        Plan::Blocked(BlockReason::NoRoute)
    );
    let river = world(
        r#", "rivers":[{"points":[{"xy":[200,0],"width_m":20,"depth_m":1.5},{"xy":[200,200],"width_m":20,"depth_m":1.5}],"surface_z":-0.5}]"#,
    );
    assert_eq!(
        grid(&river).plan(from, to, &INFANTRY, RoutePolicy::Shortest),
        Plan::Blocked(BlockReason::NoRoute)
    );
}

#[test]
fn a_nonstanding_infantry_goal_projects_to_a_legal_timed_endpoint() {
    let w =
        world(r#", "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[0.5,4,4]}]"#);
    let g = grid(&w);
    let from = v2(150.0, 100.0);
    let goal = v2(200.0, 100.0);
    assert!(!g.fits_at(goal, &INFANTRY));
    let path = route(g.plan(from, goal, &INFANTRY, RoutePolicy::Shortest));
    assert_ne!(path.last(), Some(&goal));
    assert!(g.route_fits(from, &path, &INFANTRY));
    assert!(g.route_time(from, &path, &INFANTRY).is_finite());
}
