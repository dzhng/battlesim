//! Route planning as a battle does it: a share of each tick's allowance at a
//! time, the unit holding where it is until its route is ready.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

use crate::common;

/// Open ground 400 × 200 m with a wall across the middle, open at its ends:
/// a route from one side to the other has to be searched for.
const WALLED: &str = r#"{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
    "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[0.4,80,0.6]}]}"#;
const POST: &str = r#"{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
    "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[0.4,10,0.6]}]}"#;
/// The same ground with nothing on it, and with the wall closed end to end.
const OPEN: &str = r#"{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#;
const CLOSED: &str = r#"{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
    "props":[{"kind":"wall","center":[200,100],"yaw":0,"half_extents":[0.4,100,0.6]}]}"#;

/// The shipped rules with `work` planning work a tick, so a test decides how
/// many ticks a search takes whatever the shipped allowance is.
fn scenario(map: &str, units: serde_json::Value, work: u32) -> ScenarioDefinition {
    let mut rules = common::scenario_rules();
    rules["navigation"]["work_per_tick"] = json!(work);
    let map: serde_json::Value = serde_json::from_str(map).unwrap();
    serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": [], "scripts": [],
    }))
    .unwrap()
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

fn xy(u: &OwnUnit) -> [f64; 2] {
    [u.position[0], u.position[1]]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn go(units: &[u32], goal: [f64; 2]) -> Order {
    Order::Move {
        units: units.iter().map(|&u| UnitId(u)).collect(),
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    }
}

fn send(b: &mut Battle, seq: u64, order: Order) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None);
}

fn one_tank() -> serde_json::Value {
    json!([{ "side": "blue", "kind": "tank", "position": [100, 100] }])
}

#[test]
fn a_new_body_during_infantry_refinement_replans_without_panicking() {
    let mut setup = scenario(
        r#"{"size":[120,120],"fog_cell_m":4,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([{"side":"blue","kind":"recon","position":[20,20]}]),
        1,
    );
    setup.events = serde_json::from_value(json!([{
        "tick": 60,
        "add_prop": {
            "kind": "wall", "center": [62, 60],
            "half_extents": [0.5, 8, 2], "yaw": 0
        }
    }]))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    send(&mut battle, 1, go(&[0], [100.0, 100.0]));
    let mut digests = Vec::new();
    let mut planned = false;
    // One work item per tick deliberately stretches refinement across the event.
    // This watchdog checks completion, not the shipped planning latency.
    for _ in 0..9000 {
        battle.step();
        digests.push(battle.digest());
        let unit = battle.unit(UnitId(0)).unwrap();
        if battle.tick() > 60 && unit.route.is_some() {
            planned = true;
            break;
        }
    }
    assert!(
        planned,
        "the changed edge must finish a safe replan: state={:?} pending={} position={:?}",
        battle.unit(UnitId(0)).unwrap().state,
        battle.load().routes_pending,
        battle.unit(UnitId(0)).unwrap().position
    );
    assert_eq!(battle.load().routes_pending, 0);
    let grid = sim::navigation::NavGrid::new(std::sync::Arc::new(sim::navigation::NavBase::build(
        battle.world(),
        battle.world().props(),
        setup.rules.physics.soldier_radius_m,
    )));
    let unit = battle.unit(UnitId(0)).unwrap();
    assert!(grid.route_fits(
        unit.route_from,
        unit.route.as_ref().unwrap(),
        &unit.mobility
    ));
    let replay = serde_json::from_str(&serde_json::to_string(&battle.replay()).unwrap()).unwrap();
    let mut copy = Battle::from_replay(&setup, &replay).unwrap();
    for expected in digests {
        copy.step();
        assert_eq!(copy.digest(), expected);
    }
}

#[test]
fn a_unit_holds_where_it_is_while_its_route_is_planned_then_drives_it() {
    let mut b = Battle::new(&scenario(WALLED, one_tank(), 100), 1);
    send(&mut b, 1, go(&[0], [300.0, 100.0]));
    b.step();
    let start = xy(&own(&b, 0));
    assert_eq!(own(&b, 0).state, MoveState::Planning);
    let mut planning = 1;
    while own(&b, 0).state == MoveState::Planning {
        assert_eq!(xy(&own(&b, 0)), start, "it holds while it plans");
        assert!(own(&b, 0).route.is_empty(), "no route before it is whole");
        b.step();
        planning += 1;
        assert!(planning < 600, "planning never finished");
    }
    assert!(planning > 2, "a search this size outlasts one tick's work");
    assert_eq!(own(&b, 0).state, MoveState::Moving);
    for _ in 0..3000 {
        b.step();
        if own(&b, 0).state == MoveState::Idle {
            break;
        }
    }
    assert!(dist(xy(&own(&b, 0)), [300.0, 100.0]) < 1.0);
}

/// The planning work a lone tank's order costs until it sets off or gives up.
fn work_to_plan(
    map: &str,
    goal: [f64; 2],
    rules: impl FnOnce(&mut ScenarioDefinition),
) -> (u64, MoveState) {
    let mut setup = scenario(map, one_tank(), 100);
    rules(&mut setup);
    let mut b = Battle::new(&setup, 1);
    send(&mut b, 1, go(&[0], goal));
    let mut work = 0;
    for _ in 0..3000 {
        b.step();
        work += b.load().planning_work;
        if own(&b, 0).state != MoveState::Planning {
            return (work, own(&b, 0).state);
        }
    }
    panic!("planning never finished");
}

#[test]
fn a_search_stays_near_the_line_it_is_asked_to_cross() {
    // Across open ground the search walks the line: 190 cells of it here.
    let (open, state) = work_to_plan(OPEN, [380.0, 180.0], |_| {});
    assert_eq!(state, MoveState::Moving);
    assert!(open < 1000, "open ground cost {open}");
    // A 20 m wall on the line costs the pocket in front of it and the
    // clearance round it: nothing like the map's 20,000 cells.
    let (post, state) = work_to_plan(POST, [380.0, 100.0], |_| {});
    assert_eq!(state, MoveState::Moving);
    assert!(post < 4000, "the short wall cost {post}");
}

#[test]
fn a_search_gives_up_at_its_limit_and_reports_the_route_blocked() {
    use sim::math::v2;
    use sim::navigation::{Journey, Leg, NavBase, NavGrid, Plan, RoadNet};
    use sim::world::WorldGeometry;
    // An impossible live command is rejected before scheduling a route.
    // Exercise the route owner directly to retain its bounded-search proof.
    let search = |configure: &dyn Fn(&mut ScenarioDefinition)| {
        let mut setup = scenario(CLOSED, one_tank(), 100);
        configure(&mut setup);
        let world = WorldGeometry::new(&setup.map, &setup.rules);
        let grid = NavGrid::new(std::sync::Arc::new(NavBase::build(
            &world,
            world.props(),
            setup.rules.physics.soldier_radius_m,
        )));
        let roads = RoadNet::build(&world);
        let mobility = sim::units::mobility(setup.rules.catalog.by_id("tank"), &setup.rules);
        let before = grid.work();
        let mut journey = Journey::new(
            &grid,
            &roads,
            None,
            Leg {
                from: v2(100.0, 100.0),
                goal: v2(300.0, 100.0),
                m: &mobility,
                policy: RoutePolicy::Shortest,
                avoid: &[],
            },
            &setup.rules.navigation,
        );
        let allowance = setup.rules.navigation.work_per_tick as u64;
        for _ in 0..3000 {
            let spent = journey.advance(&grid, &roads, allowance);
            assert!(spent <= allowance + sim::navigation::LARGEST_STEP);
            if journey.plan().is_some() {
                assert!(matches!(journey.plan(), Some(Plan::Blocked(_))));
                return grid.work() - before;
            }
        }
        panic!("a limited search must finish");
    };
    // No way across: the whole near half (10,000 cells) could be searched.
    let exhaustive = search(&|_| {});
    assert!(exhaustive > 5000, "it looked everywhere: {exhaustive}");
    // The rules bound how far a search looks: 4 cells a metre of its line.
    let bounded = search(&|setup| {
        setup.rules.navigation.search_cells_base = 200;
        setup.rules.navigation.search_cells_per_m = 4;
    });
    assert!(
        (1000..3000).contains(&bounded),
        "200 + 4 × 200 m cells, and the clearance worked out on the way: {bounded}"
    );
}

#[test]
fn stop_while_planning_leaves_the_unit_idle_and_drops_its_request() {
    let mut b = Battle::new(&scenario(WALLED, one_tank(), 100), 1);
    send(&mut b, 1, go(&[0], [300.0, 100.0]));
    b.step();
    let start = xy(&own(&b, 0));
    assert_eq!(b.load().routes_pending, 1);
    send(
        &mut b,
        2,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    for _ in 0..600 {
        b.step();
        assert_eq!(own(&b, 0).state, MoveState::Idle);
        assert_eq!(b.load().routes_pending, 0);
        assert_eq!(b.load().planning_work, 0, "nothing is searched for it");
    }
    assert_eq!(xy(&own(&b, 0)), start, "the stopped unit never set off");
}

#[test]
fn a_new_order_replaces_the_route_being_planned() {
    let mut b = Battle::new(&scenario(WALLED, one_tank(), 100), 1);
    send(&mut b, 1, go(&[0], [300.0, 100.0]));
    b.step();
    b.step();
    assert_eq!(own(&b, 0).state, MoveState::Planning);
    // Back the way it faces away from, over open ground: no search needed.
    send(&mut b, 2, go(&[0], [40.0, 100.0]));
    for _ in 0..3000 {
        b.step();
        let unit = own(&b, 0);
        assert!(
            unit.position[0] < 101.0 && unit.route.iter().all(|p| p[0] < 101.0),
            "it never gets or drives the route it was told to forget: {:?}",
            unit.route
        );
        assert!(b.load().routes_pending <= 1);
        if own(&b, 0).state == MoveState::Idle {
            break;
        }
    }
    assert!(dist(xy(&own(&b, 0)), [40.0, 100.0]) < 1.0);
    assert_eq!(b.load().routes_pending, 0);
}

/// Six tanks either side of the wall, each sent across it at once.
fn crossing() -> (ScenarioDefinition, Vec<Order>) {
    let units: Vec<_> = (0..6)
        .map(|k| json!({ "side": "blue", "kind": "tank", "position": [60 + 12 * k, 40 + 24 * k] }))
        .collect();
    let orders = (0..6)
        .map(|k| go(&[k], [330.0 - 12.0 * k as f64, 40.0 + 24.0 * k as f64]))
        .collect();
    (scenario(WALLED, json!(units), 400), orders)
}

#[test]
fn many_long_orders_share_each_ticks_allowance_and_every_unit_gets_its_route() {
    let (setup, orders) = crossing();
    let allowance = setup.rules.navigation.work_per_tick as u64;
    let mut b = Battle::new(&setup, 1);
    for (k, order) in orders.into_iter().enumerate() {
        send(&mut b, k as u64 + 1, order);
    }
    let mut ticks = 0;
    let mut most = 0;
    let mut total = 0;
    while (0..6).any(|id| own(&b, id).state == MoveState::Planning) || ticks == 0 {
        b.step();
        ticks += 1;
        let work = b.load().planning_work;
        most = most.max(work);
        total += work;
        assert!(ticks < 2000, "planning never finished");
    }
    assert!(ticks > 6, "the searches outlast one tick: {ticks}");
    assert!(
        most <= allowance + sim::navigation::LARGEST_STEP,
        "a tick spent {most} of an allowance of {allowance}"
    );
    assert!(total <= ticks * allowance + sim::navigation::LARGEST_STEP);
    for id in 0..6 {
        assert_eq!(own(&b, id).state, MoveState::Moving, "unit {id}");
    }
}

#[test]
fn a_short_order_is_not_starved_by_long_ones() {
    let (setup, orders) = crossing();
    let mut b = Battle::new(&setup, 1);
    // Five long searches, then the sixth tank a few metres over open ground.
    for (k, order) in orders.into_iter().take(5).enumerate() {
        send(&mut b, k as u64 + 1, order);
    }
    b.step();
    b.step();
    assert!((0..5).all(|id| own(&b, id).state == MoveState::Planning));
    let near = xy(&own(&b, 5));
    send(&mut b, 6, go(&[5], [near[0] - 20.0, near[1]]));
    b.step();
    assert_eq!(
        own(&b, 5).state,
        MoveState::Moving,
        "its share of the tick is enough for a short route"
    );
    assert!((0..5).any(|id| own(&b, id).state == MoveState::Planning));
}

/// Every tick's digest of a battle given `orders` at tick 0 and a stop for
/// unit 2 at tick 5: mid-search ticks included.
fn digests(setup: &ScenarioDefinition, orders: &[Order], ticks: u64) -> (Vec<u64>, Battle) {
    let mut b = Battle::new(setup, 7);
    for (k, order) in orders.iter().enumerate() {
        send(&mut b, k as u64 + 1, order.clone());
    }
    let mut out = Vec::new();
    for t in 0..ticks {
        if t == 5 {
            let stop = Order::Stop {
                units: vec![UnitId(2)],
            };
            send(&mut b, orders.len() as u64 + 1, stop);
        }
        b.step();
        out.push(b.digest());
    }
    (out, b)
}

#[test]
fn two_battles_given_the_same_orders_plan_alike_tick_for_tick() {
    let (setup, orders) = crossing();
    let (first, _) = digests(&setup, &orders, 400);
    let (second, _) = digests(&setup, &orders, 400);
    assert_eq!(first, second);
    // The allowance is part of the battle: with less, routes come later.
    let mut slower = setup.clone();
    slower.rules.navigation.work_per_tick /= 2;
    let (halved, _) = digests(&slower, &orders, 400);
    assert_ne!(first[..40], halved[..40]);
}

#[test]
fn a_replay_plans_the_same_routes_on_the_same_ticks() {
    let (setup, orders) = crossing();
    let (live, battle) = digests(&setup, &orders, 400);
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    for (tick, digest) in live.iter().enumerate() {
        replay.step();
        assert_eq!(replay.digest(), *digest, "tick {}", tick + 1);
    }
}

#[test]
fn an_enclosed_road_goal_finishes_its_counted_proof_and_replays() {
    use sim::math::v2;
    use sim::navigation::{Journey, Leg, NavBase, NavGrid, Plan, RoadNet};
    use sim::world::WorldGeometry;
    let map = r#"{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[20,100],[280,100]],"width_m":10}}],
        "props":[
            {"kind":"wall","center":[290,100],"half_extents":[1,12,2],"yaw":0},
            {"kind":"wall","center":[310,100],"half_extents":[1,12,2],"yaw":0},
            {"kind":"wall","center":[300,89],"half_extents":[11,1,2],"yaw":0},
            {"kind":"wall","center":[300,111],"half_extents":[11,1,2],"yaw":0}
        ]}"#;
    let setup = scenario(
        map,
        json!([{"side":"blue","kind":"tank","position":[20,100]}]),
        200,
    );
    let mut battle = Battle::new(&setup, 7);
    let start = xy(&own(&battle, 0));
    let mut order = go(&[0], [300.0, 100.0]);
    if let Order::Move { route, .. } = &mut order {
        *route = RoutePolicy::Fastest;
    }
    // One search of the 20,000-cell map plus local proof and road checks
    // fits this allowance; recertifying the goal from another outside start does not.
    // Admission refuses this command before it can schedule a live search.
    let world = WorldGeometry::new(&setup.map, &setup.rules);
    let grid = NavGrid::new(std::sync::Arc::new(NavBase::build(
        &world,
        world.props(),
        setup.rules.physics.soldier_radius_m,
    )));
    let roads = RoadNet::build(&world);
    let mobility = sim::units::mobility(setup.rules.catalog.by_id("tank"), &setup.rules);
    let mut journey = Journey::new(
        &grid,
        &roads,
        None,
        Leg {
            from: v2(start[0], start[1]),
            goal: v2(300.0, 100.0),
            m: &mobility,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        },
        &setup.rules.navigation,
    );
    for _ in 0..130 {
        assert!(journey.advance(&grid, &roads, 200) <= 200 + sim::navigation::LARGEST_STEP);
        if journey.plan().is_some() {
            break;
        }
    }
    assert!(matches!(journey.plan(), Some(Plan::Blocked(_))));
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order,
        queued: false,
    });
    assert_eq!(
        ack.error,
        Some(contract::command::OrderError::NoValidDestination)
    );
    assert!(!ack.placement.unwrap().destinations[0].placed);
    let mut live = Vec::new();
    for _ in 0..130 {
        battle.step();
        live.push(battle.digest());
        assert!(battle.load().planning_work <= 200 + sim::navigation::LARGEST_STEP);
        assert_eq!(
            xy(&own(&battle, 0)),
            start,
            "a refused destination never starts movement"
        );
        assert_eq!(own(&battle, 0).state, MoveState::Idle);
        assert_eq!(own(&battle, 0).goal, None);
    }
    assert!(battle.load().routes_pending == 0);
    let record = serde_json::from_str(&serde_json::to_string(&battle.replay()).unwrap()).unwrap();
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for (tick, digest) in live.iter().enumerate() {
        replay.step();
        assert_eq!(replay.digest(), *digest, "tick {}", tick + 1);
    }
}

/// A side that learns of a body while a route is being searched gets a
/// route that fits what it now knows, not one searched on the old picture.
#[test]
fn a_route_searched_while_the_side_learns_of_a_body_fits_what_it_now_knows() {
    use sim::math::v2;
    use sim::navigation::{Mobility, NavBase, NavGrid, Plan};
    use sim::route_planner::{Request, RoutePlanner};
    use sim::world::WorldGeometry;
    // A wall with its nearer gap to the south; then a second wall closes it.
    let wall = r#"{"kind":"wall","center":[200,110],"yaw":0,"half_extents":[0.4,45,0.6]}"#;
    let closed = r#"{"kind":"wall","center":[200,32],"yaw":0,"half_extents":[0.4,34,0.6]}"#;
    let picture = |props: &str| {
        let map = serde_json::from_str(&format!(
            r#"{{"size":[400,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[{props}]}}"#
        ))
        .unwrap();
        let world = WorldGeometry::new(&map, &common::rules());
        let grid = NavGrid::new(std::sync::Arc::new(NavBase::build(
            &world,
            world.props(),
            0.3,
        )));
        (world, grid)
    };
    let both = format!("{wall},{closed}");
    let ((_, before), (_, after)) = (picture(wall), picture(&both));
    let rules = common::rules();
    let tank = sim::units::mobility(rules.catalog.by_id("tank"), &rules);
    let (from, goal) = (v2(100.0, 100.0), v2(300.0, 100.0));
    let request = || Request {
        side: Side::Blue,
        from,
        goal,
        mobility: tank,
        policy: RoutePolicy::Shortest,
        detour: Vec::new(),
        kept: None,
        new_goal: true,
    };
    let fits = |grid: &NavGrid, plan: &Plan, m: &Mobility| match plan {
        Plan::Route(route) => grid.route_fits(from, route, m),
        Plan::Blocked(_) => false,
    };
    let roads = sim::navigation::RoadNet::default();
    // How many 40-work ticks the search takes on the first picture.
    let slow = contract::scenario::NavigationRules {
        work_per_tick: 40,
        ..rules.navigation
    };
    let mut twin = RoutePlanner::default();
    twin.submit(UnitId(0), request());
    let mut ticks = 0;
    let old = loop {
        ticks += 1;
        if let Some((_, _, plan)) = twin
            .advance(&slow, &roads, [Some((&before, 1)), None])
            .pop()
        {
            break plan;
        }
    };
    assert!(ticks > 3 && fits(&before, &old, &tank));
    assert!(
        !fits(&after, &old, &tank),
        "the old route runs through the new wall"
    );
    // The same search on a fresh grid (a grid keeps what earlier searches
    // worked out), but the side learns of the second wall on its last tick:
    // the grid it is searching takes the wall in, as a battle's does.
    let (mut world, mut known) = picture(wall);
    let mut planner = RoutePlanner::default();
    planner.submit(UnitId(0), request());
    for _ in 1..ticks {
        assert!(planner
            .advance(&slow, &roads, [Some((&known, 1)), None])
            .is_empty());
    }
    let seen = world.add_prop(&serde_json::from_str(closed).unwrap());
    let learned = [(seen, world.prop(seen).cloned())];
    known.update(&world, learned.into_iter(), std::iter::empty());
    let plan = loop {
        if let Some((_, _, plan)) = planner
            .advance(&slow, &roads, [Some((&known, 2)), None])
            .pop()
        {
            break plan;
        }
    };
    assert!(fits(&known, &plan, &tank), "{plan:?}");
    assert!(fits(&after, &plan, &tank), "{plan:?}");
}

/// The moment a battle's planning grid has to keep up with: a jeep driving
/// a road when a wreck appears in its lane ahead. The jeep's side sees the
/// wreck and the jeep goes round it; the tick that takes the wreck into
/// the side's grid works out only the cells under it. The other side,
/// whose jeep is coming the other way two kilometres off, has not seen it,
/// and drives exactly as it would have with no wreck there.
#[test]
fn a_wreck_appearing_on_the_road_ahead_is_driven_round_by_the_side_that_sees_it() {
    use sim::math::{v2, Obb2};
    const ROAD: &str = r#"{"size":[3000,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[20,200],[2980,200]],"width_m":8}}]}"#;
    let wreck = Obb2 {
        center: v2(400.0, 200.0),
        yaw: 0.0,
        half: v2(3.5, 1.7),
    };
    let battle = |events: serde_json::Value| {
        let drive = |side: &str, unit: u32, goal: [f64; 2]| {
            json!({ "tick": 1, "side": side, "order": { "kind": "move", "units": [unit],
                "gesture": 1, "goal": goal, "route": "shortest" } })
        };
        Battle::new(
            &common::scenario_with(
                ROAD,
                json!([
                    { "side": "blue", "kind": "jeep", "position": [100, 200], "yaw": 0.0,
                      "engagement": "return_fire_only" },
                    { "side": "red", "kind": "jeep", "position": [2900, 200],
                      "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
                ]),
                events,
                json!([
                    drive("blue", 0, [2900.0, 200.0]),
                    drive("red", 1, [100.0, 200.0])
                ]),
            ),
            1,
        )
    };
    let mut calm = battle(json!([]));
    let mut hit = battle(json!([{ "tick": 90, "add_prop": { "kind": "heavy_wreck",
        "center": [wreck.center.x, wreck.center.y], "yaw": wreck.yaw,
        "half_extents": [wreck.half.x, wreck.half.y, 1.2] } }]));
    let allowance = hit.rules().navigation.work_per_tick as u64;
    // Whether the way from `from` along `route` runs through the wreck.
    let through = |from: sim::math::V2, route: &[sim::math::V2]| {
        let mut a = from;
        route.iter().any(|&b| {
            let meets = wreck.meets_segment(a, b, 0.0);
            a = b;
            meets
        })
    };
    let (blue, red) = (UnitId(0), UnitId(1));
    let mut relaid = 0;
    for tick in 1..=600 {
        calm.step();
        hit.step();
        let (unseen, never) = (hit.unit(red).unwrap(), calm.unit(red).unwrap());
        assert_eq!(
            (unseen.position, unseen.yaw, unseen.state, &unseen.route),
            (never.position, never.yaw, never.state, &never.route),
            "tick {tick}: the side that has not seen the wreck drives as if it were not there"
        );
        let load = hit.load();
        assert!(
            load.planning_work <= allowance + sim::navigation::LARGEST_STEP,
            "tick {tick}: {} planning work",
            load.planning_work
        );
        let cells = load.grid_cells_relaid - std::mem::replace(&mut relaid, load.grid_cells_relaid);
        assert!(cells <= 64, "tick {tick}: {cells} cells worked out again");
        let seer = hit.unit(blue).unwrap();
        assert!(
            wreck.distance(seer.position.xy()) > 1.0,
            "tick {tick}: the jeep drove into the wreck"
        );
    }
    assert!(relaid > 0, "the wreck was laid on blue's grid");
    assert!(hit.navigation_revision(Side::Blue) > 0);
    assert_eq!(hit.navigation_revision(Side::Red), 0);
    let (seer, unseen) = (hit.unit(blue).unwrap(), hit.unit(red).unwrap());
    assert!(
        seer.position.x > wreck.center.x + 100.0,
        "the jeep passed the wreck: {:?}",
        seer.position
    );
    assert!(
        !through(seer.position.xy(), seer.route.as_ref().unwrap()),
        "and its way on is clear of it"
    );
    assert!(
        through(unseen.position.xy(), unseen.route.as_ref().unwrap()),
        "the other side's old route still runs through where the wreck lies"
    );
}

#[test]
fn rules_without_planning_work_are_refused_at_load() {
    let mut rules = common::scenario_rules();
    rules["navigation"]["work_per_tick"] = json!(0);
    let error = serde_json::from_value::<contract::scenario::Rules>(rules).unwrap_err();
    assert!(error.to_string().contains("work_per_tick"), "{error}");
}

#[test]
fn checking_a_long_route_after_learning_stays_inside_the_tick_allowance() {
    use sim::math::v2;
    use sim::navigation::{Mobility, NavBase, NavGrid, RoadNet};
    use sim::route_planner::{Request, RoutePlanner};
    use sim::world::WorldGeometry;
    let rules = common::rules();
    let map = serde_json::from_value(json!({"size":[10000,10000],"fog_cell_m":8,
        "height_grid_m":4,"slope_cutoff_deg":35}))
    .unwrap();
    let world = WorldGeometry::new(&map, &rules);
    let known = NavGrid::new(std::sync::Arc::new(NavBase::build(
        &world,
        world.props(),
        0.3,
    )));
    let mut planner = RoutePlanner::default();
    let m: Mobility = sim::units::mobility(rules.catalog.by_id("tank"), &rules);
    let (from, goal) = (v2(100.0, 100.0), v2(9900.0, 100.0));
    let slow = contract::scenario::NavigationRules {
        work_per_tick: 40,
        ..rules.navigation
    };
    planner.submit(
        UnitId(0),
        Request {
            side: Side::Blue,
            from,
            goal,
            mobility: m,
            policy: RoutePolicy::Shortest,
            detour: vec![],
            kept: None,
            new_goal: true,
        },
    );
    assert!(planner
        .advance(&slow, &RoadNet::default(), [Some((&known, 1)), None])
        .is_empty());
    for _ in 0..10000 {
        let plans = planner.advance(&slow, &RoadNet::default(), [Some((&known, 2)), None]);
        assert!(
            planner.spent() <= 40 + sim::navigation::LARGEST_STEP,
            "revision validation spent {} work in one tick",
            planner.spent()
        );
        if let Some((_, _, plan)) = plans.first() {
            let sim::navigation::Plan::Route(route) = plan else {
                panic!("{plan:?}");
            };
            assert_eq!(route.last(), Some(&goal));
            assert!(known.route_fits(from, route, &m));
            return;
        }
    }
    panic!("revision validation never finished");
}

#[test]
fn initial_long_open_orders_do_not_sample_every_half_metre() {
    use sim::math::v2;
    use sim::navigation::{NavBase, NavGrid, RoadNet};
    use sim::route_planner::{Request, RoutePlanner};
    use sim::world::WorldGeometry;
    let rules = common::rules();
    let map = serde_json::from_value(
        json!({"size":[10000,10000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}),
    )
    .unwrap();
    let world = WorldGeometry::new(&map, &rules);
    let grid = NavGrid::new(std::sync::Arc::new(NavBase::build(
        &world,
        world.props(),
        0.3,
    )));
    let m = sim::units::mobility(rules.catalog.by_id("tank"), &rules);
    let budget = contract::scenario::NavigationRules {
        work_per_tick: 1,
        ..rules.navigation
    };
    let initial = |goal| {
        let mut p = RoutePlanner::default();
        p.submit(
            UnitId(0),
            Request {
                side: Side::Blue,
                from: v2(100.0, 100.0),
                goal,
                mobility: m,
                policy: RoutePolicy::Fastest,
                detour: vec![],
                kept: None,
                new_goal: true,
            },
        );
        assert!(p
            .advance(&budget, &RoadNet::default(), [Some((&grid, 0)), None])
            .is_empty());
        p.spent()
    };
    let short = initial(v2(200.0, 100.0));
    let long = initial(v2(9900.0, 100.0));
    let diagonal = initial(v2(9900.0, 9900.0));
    assert!(
        diagonal <= short + 4,
        "one open terrain band at any bearing: {diagonal} vs {short}"
    );
    assert!(
        long <= short + 4,
        "one open terrain run, not metre samples: {long} vs {short}"
    );
}

#[test]
fn an_irregular_diagonal_terrain_probe_yields_within_a_tiny_allowance() {
    use sim::math::v2;
    use sim::navigation::{NavBase, NavGrid, RoadNet};
    use sim::route_planner::{Request, RoutePlanner};
    use sim::world::WorldGeometry;
    let rules = common::rules();
    let points: Vec<_> = (0..=500)
        .map(|k| json!({"xy":[30,k*20],"width_m":if k%2==0 {16} else {32},"depth_m":1.5}))
        .collect();
    let map=serde_json::from_value(json!({"size":[10000,10000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"rivers":[{"points":points,"surface_z":-0.5}]})).unwrap();
    let world = WorldGeometry::new(&map, &rules);
    let grid = NavGrid::new(std::sync::Arc::new(NavBase::build(
        &world,
        world.props(),
        0.3,
    )));
    let m = sim::units::mobility(rules.catalog.by_id("tank"), &rules);
    let budget = contract::scenario::NavigationRules {
        work_per_tick: 1,
        ..rules.navigation
    };
    let mut p = RoutePlanner::default();
    p.submit(
        UnitId(0),
        Request {
            side: Side::Blue,
            from: v2(100.0, 100.0),
            goal: v2(9900.0, 9900.0),
            mobility: m,
            policy: RoutePolicy::Fastest,
            detour: vec![],
            kept: None,
            new_goal: true,
        },
    );
    for tick in 0..4 {
        assert!(p
            .advance(&budget, &RoadNet::default(), [Some((&grid, 0)), None])
            .is_empty());
        assert_eq!(
            p.spent(),
            1,
            "tick {tick}: terrain work resumes before constructing a physical route search"
        );
    }
    let roads = RoadNet::default();
    let mut finished = None;
    for _ in 0..10000 {
        let plans = p.advance(
            &contract::scenario::NavigationRules {
                work_per_tick: 4000,
                ..rules.navigation
            },
            &roads,
            [Some((&grid, 0)), None],
        );
        if let Some((_, _, plan)) = plans.into_iter().next() {
            finished = Some(plan);
            break;
        }
    }
    assert!(
        matches!(finished, Some(sim::navigation::Plan::Route(_))),
        "the clear diagonal eventually commits"
    );
}
