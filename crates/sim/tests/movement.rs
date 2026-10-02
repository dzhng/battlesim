//! Movement contracts at the authority: routes, groups, traffic, knowledge.
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use sim::battle::Battle;

use crate::common;

fn scenario(units: serde_json::Value, events: serde_json::Value) -> ScenarioDefinition {
    common::scenario(common::saved_map("geometry"), units, events)
}

struct Orders {
    seq: u64,
}
impl Orders {
    fn send(&mut self, b: &mut Battle, order: Order, queued: bool) {
        self.seq += 1;
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: self.seq,
            order,
            queued,
        });
        assert_eq!(ack.error, None);
    }
    fn go(
        &mut self,
        b: &mut Battle,
        units: &[u32],
        goal: [f64; 2],
        gesture: u64,
        route: RoutePolicy,
        queued: bool,
    ) {
        let units = units.iter().map(|&u| UnitId(u)).collect();
        self.send(
            b,
            Order::Move {
                units,
                gesture,
                goal,
                route,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
            },
            queued,
        );
    }
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

/// Step until every listed unit is idle or `limit` ticks pass; checks `each` every tick.
fn run(b: &mut Battle, ids: &[u32], limit: u32, mut each: impl FnMut(&Battle)) -> u32 {
    for t in 0..limit {
        b.step();
        each(b);
        if ids.iter().all(|&id| own(b, id).state == MoveState::Idle) {
            return t;
        }
    }
    limit
}

#[test]
fn a_truck_can_park_in_clear_space_behind_a_stationary_tank() {
    let setup = common::scenario(
        &serde_json::json!({
            "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35
        })
        .to_string(),
        serde_json::json!([
            { "side": "blue", "kind": "supply", "position": [30, 60], "yaw": 0 },
            { "side": "blue", "kind": "tank", "position": [70, 60], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let mut orders = Orders { seq: 0 };
    orders.go(&mut b, &[0], [60.0, 60.0], 1, RoutePolicy::Shortest, false);
    let end = run(&mut b, &[0], 1800, |b| {
        let truck = b.unit(UnitId(0)).unwrap().hull_box().unwrap();
        let tank = b.unit(UnitId(1)).unwrap().hull_box().unwrap();
        assert!(!truck.overlaps(&tank), "truck stays clear of the tank");
    });
    assert!(
        end < 1800,
        "truck arrives instead of waiting on an oversized following gap"
    );
    assert!(dist(xy(&own(&b, 0)), [60.0, 60.0]) < 1.0);
    assert_eq!(xy(&own(&b, 1)), [70.0, 60.0], "tank stays put");
}

#[test]
fn a_group_moves_a_truck_surrounded_by_its_other_members() {
    let positions = [
        [60.0, 60.0],
        [68.5, 60.0],
        [51.5, 60.0],
        [60.0, 67.7],
        [60.0, 52.3],
    ];
    let setup = common::scenario(
        &serde_json::json!({
            "size": [200, 120], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35
        })
        .to_string(),
        serde_json::Value::Array(
            positions
                .iter()
                .map(|at| {
                    serde_json::json!({
                        "side": "blue", "kind": "supply", "position": at, "yaw": 0
                    })
                })
                .collect(),
        ),
        serde_json::json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let ids: Vec<_> = (0..5).map(UnitId).collect();
    let destinations = b
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: ids.to_vec(),
                goal: [140.0, 60.0],
                facing: None,
                direction: contract::command::MoveDirection::Forward,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(destinations.iter().all(|d| d.placed));
    let mut orders = Orders { seq: 0 };
    orders.go(
        &mut b,
        &[0, 1, 2, 3, 4],
        [140.0, 60.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    let end = run(&mut b, &[0, 1, 2, 3, 4], 1800, |b| {
        for (i, id) in ids.iter().enumerate() {
            let hull = b.unit(*id).unwrap().hull_box().unwrap();
            for other in &ids[i + 1..] {
                assert!(
                    !hull.overlaps(&b.unit(*other).unwrap().hull_box().unwrap()),
                    "group hulls stay separate at tick {}",
                    b.tick()
                );
            }
        }
    });
    assert!(
        end < 1800,
        "all group members arrive, including the surrounded truck"
    );
    for destination in destinations {
        assert!(
            dist(xy(&own(&b, destination.unit.0)), destination.goal) < 1.0,
            "unit {} reaches its accepted marker",
            destination.unit.0
        );
    }
    assert!(
        b.route_searches(Side::Blue) < 40,
        "departure makes progress without repeated replanning"
    );
}

#[test]
fn a_tank_routes_around_walls_and_arrives() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [60, 40] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[0], [60.0, 110.0], 1, RoutePolicy::Shortest, false);
    let ticks = run(&mut b, &[0], 3000, |b| {
        let p = xy(&own(b, 0));
        for prop in b.world().props().filter(|p| p.blocks(MoverClass::Vehicle)) {
            assert!(
                !prop.footprint().contains(sim::math::v2(p[0], p[1]), 0.0),
                "inside a prop at {p:?}"
            );
        }
    });
    assert!(ticks < 3000);
    assert!(dist(xy(&own(&b, 0)), [60.0, 110.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 3,
        "searches {}",
        b.route_searches(Side::Blue)
    );
}

#[test]
fn a_right_drag_rotates_the_group_layout_around_its_destination() {
    let setup = common::scenario(
        &serde_json::json!({ "size": [300, 300], "fog_cell_m": 8,
            "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
        .to_string(),
        serde_json::json!([
            { "side": "blue", "kind": "tank", "position": [30, 140], "yaw": 0 },
            { "side": "blue", "kind": "tank", "position": [30, 160], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let half = common::hull("tank").half_extents_m;
    let front_y = 150.0 - half[0].hypot(half[1]);
    let digest = b.digest();
    let preview = b
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0), UnitId(1)],
                goal: [150.0, 150.0],
                facing: Some(std::f64::consts::FRAC_PI_2),
                direction: contract::command::MoveDirection::Forward,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(b.digest(), digest, "previewing must not change the battle");
    assert_eq!(preview.len(), 2);
    for (mark, (id, x)) in preview.iter().zip([(0, 160.0), (1, 140.0)]) {
        assert_eq!(mark.unit, UnitId(id));
        assert!((mark.goal[0] - x).abs() < 1.0 && (mark.goal[1] - front_y).abs() < 1.0);
        assert_eq!(mark.facing, std::f64::consts::FRAC_PI_2);
    }
    let mut orders = Orders { seq: 0 };
    orders.send(
        &mut b,
        Order::Move {
            units: vec![UnitId(0), UnitId(1)],
            gesture: 1,
            goal: [150.0, 150.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: Some(std::f64::consts::FRAC_PI_2),
        },
        false,
    );
    b.step();
    for (id, x) in [(0, 160.0), (1, 140.0)] {
        let unit = own(&b, id);
        let goal = unit.goal.unwrap();
        assert_eq!(
            goal, preview[id as usize].goal,
            "preview and committed destination agree"
        );
        assert!(
            (goal[0] - x).abs() < 1.0 && (goal[1] - front_y).abs() < 1.0,
            "unit {id}: {goal:?}"
        );
        assert!((unit.final_facing - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    }
    let mut digests = vec![b.digest()];
    let ticks = run(&mut b, &[0, 1], 3000, |b| digests.push(b.digest()));
    assert!(ticks < 3000, "the rotated formation must arrive");
    // Tracked units finish their pivot after the movement state becomes idle.
    for _ in 0..600 {
        if [0, 1]
            .iter()
            .all(|&id| (own(&b, id).yaw - std::f64::consts::FRAC_PI_2).abs() < 0.02)
        {
            break;
        }
        b.step();
        digests.push(b.digest());
    }
    for (id, x) in [(0, 160.0), (1, 140.0)] {
        let unit = own(&b, id);
        assert!(dist(xy(&unit), [x, front_y]) < 2.0);
        assert!(
            (unit.yaw - std::f64::consts::FRAC_PI_2).abs() < 0.02,
            "unit {id}: yaw {}",
            unit.yaw
        );
    }
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest);
    }
}

#[test]
fn a_group_keeps_its_arrangement_and_each_unit_its_own_speed() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [30, 150] },
        { "side": "blue", "kind": "rifle", "position": [30, 170] },
        { "side": "blue", "kind": "supply", "position": [30, 130] },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0, 1, 2],
        [150.0, 150.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    let goals: Vec<[f64; 2]> = (0..3).map(|i| own(&b, i).goal.unwrap()).collect();
    // Relative arrangement preserved: rifle north of the tank, supply south.
    assert!(
        (goals[1][1] - goals[0][1] - 20.0).abs() < 2.5
            && (goals[0][1] - goals[2][1] - 20.0).abs() < 2.5,
        "{goals:?}"
    );
    let mut arrived = [None; 3];
    run(&mut b, &[0, 1, 2], 3000, |b| {
        for (i, a) in arrived.iter_mut().enumerate() {
            if a.is_none() && own(b, i as u32).state == MoveState::Idle {
                *a = Some(b.tick());
            }
        }
    });
    let [tank, rifle, supply] = arrived.map(Option::unwrap);
    assert!(
        tank < rifle && supply < rifle,
        "infantry is slowest: {arrived:?}"
    );
    for i in 0..3 {
        assert!(
            dist(xy(&own(&b, i)), goals[i as usize]) < 1.5,
            "unit {i} at its own slot"
        );
    }
}

#[test]
fn a_new_obstacle_is_learned_on_contact_and_routed_around() {
    // A wall appears across the tank's straight path at tick 5; red is far away.
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [230, 120] },
        { "side": "red", "kind": "tank", "position": [370, 280] },
    ]);
    let events = serde_json::json!([{ "tick": 5, "add_prop":
        { "kind": "wall", "center": [260, 120], "yaw": 0, "half_extents": [0.5, 15, 2] } }]);
    let mut b = Battle::new(&scenario(units, events), 1);
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [300.0, 120.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    let first = own(&b, 0).route.clone();
    assert_eq!(first.len(), 1, "unknown wall: straight route");
    let wall = sim::math::v2(260.0, 120.0);
    let ticks = run(&mut b, &[0], 2000, |b| {
        let p = xy(&own(b, 0));
        assert!(
            (p[0] - wall.x).abs() > 0.5 || (p[1] - wall.y).abs() > 15.0,
            "drove through the wall at {p:?}"
        );
    });
    assert!(ticks < 2000 && dist(xy(&own(&b, 0)), [300.0, 120.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 4,
        "searches {}",
        b.route_searches(Side::Blue)
    );
    assert_eq!(b.route_searches(Side::Red), 0, "red never planned");
}

#[test]
fn an_unreachable_destination_is_rejected_without_live_route_searches() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [300, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    // The 45° plateau top is flat but walled by slopes past the cutoff.
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [330.0, 50.0],
            route: RoutePolicy::Shortest,
            direction: Default::default(),
            facing: None,
        },
    });
    assert_eq!(
        ack.error,
        Some(contract::command::OrderError::NoValidDestination)
    );
    for _ in 0..300 {
        b.step();
    }
    let u = own(&b, 0);
    assert_eq!(u.state, MoveState::Idle);
    assert_eq!(u.goal, None, "no invalid destination marker");
    assert_eq!(b.route_searches(Side::Blue), 0);
    // A new order replaces it and plans again.
    let mut o = Orders { seq: 1 };
    o.go(
        &mut b,
        &[0],
        [320.0, 150.0],
        2,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    assert_eq!(own(&b, 0).state, MoveState::Moving);
}

#[test]
fn double_click_upgrades_only_its_own_gesture_even_after_it_applied() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [30, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [150.0, 150.0],
        7,
        RoutePolicy::Shortest,
        false,
    );
    o.go(&mut b, &[0], [150.0, 100.0], 8, RoutePolicy::Shortest, true);
    o.go(&mut b, &[0], [150.0, 60.0], 7, RoutePolicy::Shortest, true);
    for _ in 0..5 {
        b.step();
    }
    o.send(
        &mut b,
        Order::UpgradeMove {
            gesture: 7,
            route: RoutePolicy::Fastest,
        },
        false,
    );
    b.step();
    let u = own(&b, 0);
    assert_eq!(u.policy, Some(RoutePolicy::Fastest));
    assert_eq!(
        u.queue,
        vec![[150.0, 100.0], [150.0, 60.0]],
        "no waypoint added or removed"
    );
    // Upgrading a finished gesture is an acknowledged no-op.
    run(&mut b, &[0], 6000, |_| {});
    let searches = b.route_searches(Side::Blue);
    o.send(
        &mut b,
        Order::UpgradeMove {
            gesture: 7,
            route: RoutePolicy::Fastest,
        },
        false,
    );
    b.step();
    assert_eq!(own(&b, 0).state, MoveState::Idle);
    assert_eq!(b.route_searches(Side::Blue), searches);
}

#[test]
fn head_on_vehicles_pass_without_overlapping() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [40, 25], "yaw": 0 },
        { "side": "blue", "kind": "tank", "position": [100, 25], "yaw": std::f64::consts::PI },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    // Each issued destination is clear even before the other tank gets its order.
    o.go(&mut b, &[0], [120.0, 25.0], 1, RoutePolicy::Shortest, false);
    o.go(&mut b, &[1], [20.0, 25.0], 2, RoutePolicy::Shortest, false);
    let end = run(&mut b, &[0, 1], 3000, |b| {
        let (a, c) = (own(b, 0), own(b, 1));
        assert!(
            dist(xy(&a), xy(&c)) > 3.5,
            "hulls overlapping at tick {}",
            b.tick()
        );
        for u in [&a, &c] {
            if u.state == MoveState::Waiting {
                assert!(u.blocker.is_some(), "a wait names its blocker");
            }
        }
    });
    assert!(
        end < 3000,
        "the lower-priority tank detours and both arrive"
    );
    assert!(
        dist(xy(&own(&b, 0)), [120.0, 25.0]) < 1.0 && dist(xy(&own(&b, 1)), [20.0, 25.0]) < 1.0
    );
    assert!(
        b.route_searches(Side::Blue) < 12,
        "bounded replanning: {}",
        b.route_searches(Side::Blue)
    );
}

#[test]
fn a_squad_walks_around_a_parked_tank() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [100, 25], "yaw": std::f64::consts::FRAC_PI_2 },
        { "side": "blue", "kind": "rifle", "position": [60, 25] },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[1], [140.0, 25.0], 1, RoutePolicy::Shortest, false);
    let hull = b.unit(UnitId(0)).unwrap().hull_box().unwrap();
    let radius = common::physics("soldier_radius_m");
    let end = run(&mut b, &[1], 3000, |b| {
        // No soldier's body ever enters the tank's hull.
        for p in b.unit(UnitId(1)).unwrap().member_positions() {
            assert!(!hull.contains(p.xy(), radius), "through the tank at {p:?}");
        }
    });
    assert!(end < 3000, "the squad got past");
    assert_eq!(own(&b, 1).members.len(), 8);
}

#[test]
fn idle_units_never_search() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "rifle", "position": [60, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    for _ in 0..600 {
        b.step();
    }
    assert_eq!(b.route_searches(Side::Blue), 0);
}

/// A 120 × 80 m flat map with these props, one blue rifle squad at `from`
/// sent to `goal` on tick 1.
fn one_squad_setup(props: serde_json::Value, from: [f64; 2], goal: [f64; 2]) -> ScenarioDefinition {
    serde_json::from_value(serde_json::json!({
        "map": { "size": [120, 80], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": common::game(),
        "units": [{ "side": "blue", "kind": "rifle", "position": from, "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": {
            "kind": "move", "units": [0], "gesture": 1, "goal": goal, "route": "shortest" } }],
    }))
    .unwrap()
}

fn one_squad(props: serde_json::Value, from: [f64; 2], goal: [f64; 2]) -> Battle {
    Battle::new(&one_squad_setup(props, from, goal), 1)
}

/// Each order costs the squad's corridor and at most one route of each
/// soldier's own (the final stretch, or back to the corridor): route
/// searches per order stay within 1 + soldiers, never one per tick.
#[test]
fn an_order_searches_at_most_once_for_the_squad_and_once_per_soldier() {
    let wall = |y: f64, half: f64| serde_json::json!({ "kind": "wall", "center": [60, y], "yaw": 0, "half_extents": [0.5, half, 1.5] });
    for (props, what) in [
        (serde_json::json!([]), "open ground"),
        (
            serde_json::json!([wall(18.75, 18.75), wall(61.25, 18.75)]),
            "a 5 m gap",
        ),
        (
            serde_json::json!([wall(19.625, 19.625), wall(60.375, 19.625)]),
            "a 1.5 m gap",
        ),
    ] {
        let mut b = one_squad(props, [20.0, 40.0], [100.0, 40.0]);
        let soldiers = b.unit(UnitId(0)).unwrap().members.len() as u64;
        let mut ticks = 0;
        while ticks < 3000 && (ticks < 2 || b.unit(UnitId(0)).unwrap().state != MoveState::Idle) {
            b.step();
            ticks += 1;
        }
        assert!(ticks < 3000, "{what}: the squad arrives");
        let searches = b.route_searches(Side::Blue);
        assert!(
            searches <= 1 + soldiers,
            "{what}: {searches} searches for one order of {soldiers} soldiers"
        );
    }
}

/// No soldier's body ever overlaps another's (Q10), even where a whole
/// squad files through a gap one man wide.
#[test]
fn soldiers_never_overlap_filing_through_a_one_man_gap() {
    let wall = |y: f64| serde_json::json!({ "kind": "wall", "center": [60, y], "yaw": 0, "half_extents": [0.5, 19.625, 1.5] });
    let mut b = one_squad(
        serde_json::json!([wall(19.625), wall(60.375)]),
        [20.0, 40.0],
        [100.0, 40.0],
    );
    let apart = 2.0 * common::physics("soldier_radius_m");
    for _ in 0..1500 {
        b.step();
        let at: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
        for (i, p) in at.iter().enumerate() {
            for q in &at[i + 1..] {
                assert!(
                    (p.xy() - q.xy()).length() >= apart - 1e-6,
                    "overlap at tick {}",
                    b.tick()
                );
            }
        }
    }
    assert_eq!(
        b.unit(UnitId(0)).unwrap().state,
        MoveState::Idle,
        "through and settled"
    );
}

#[test]
fn a_supply_truck_builds_road_speed_instead_of_jumping_to_it() {
    let map = serde_json::json!({
        "size": [1000, 200], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "surfaces": [{ "kind": "road", "shape": { "kind": "stroke",
            "points": [[200, 100], [950, 100]], "width_m": 20 } }]
    })
    .to_string();
    let mut setup = common::scenario(
        &map,
        serde_json::json!([
            { "side": "blue", "kind": "supply", "position": [40, 100], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut rules = common::scenario_rules();
    rules["movement"]["drive"]["acceleration_s"] = serde_json::json!(4.5);
    rules["movement"]["drive"]["braking_s"] = serde_json::json!(1.5);
    rules["surfaces"]["road"]["speed_factor"] = serde_json::json!(1.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "supply",
        serde_json::json!({
            "mobility": { "wheeled": { "offroad_kmh": 25, "road_kmh": 76,
                "turn_deg_s": 40, "turning_radius_m": 9, "reverse_fraction": 0.35 } },
            "capabilities": { "deploy": { "seconds": 3, "pack_seconds": 1 } }
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let hz = setup.rules.tick_hz;
    let mut b = Battle::new(&setup, 1);
    let mut orders = Orders { seq: 0 };
    orders.go(
        &mut b,
        &[0],
        [850.0, 100.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    let mut was_road = false;
    let mut entered = None;
    let mut road_speeds = Vec::new();
    for _ in 0..45 * hz {
        let before = b.unit(UnitId(0)).unwrap().position.xy();
        let road = b
            .world()
            .surface_at(before.x, before.y)
            .unwrap()
            .road_factor
            > 0.0;
        b.step();
        let after = b.unit(UnitId(0)).unwrap().position.xy();
        let speed = (after - before).length() * hz as f64;
        if road && !was_road {
            entered = Some(b.tick());
        }
        if entered.is_some() {
            road_speeds.push(speed);
            if road_speeds.len() >= (3.2 * hz as f64) as usize {
                break;
            }
        }
        was_road = road;
    }
    let full = 76.0 / 3.6;
    assert!(entered.is_some(), "truck reaches the road");
    assert!(
        road_speeds[0] < full * 0.8,
        "road entry jumped to {} m/s",
        road_speeds[0]
    );
    assert!(road_speeds[hz as usize] > 25.0 / 3.6 && road_speeds[hz as usize] < full * 0.9);
    assert!(
        road_speeds.last().unwrap() >= &(full * 0.99),
        "truck reaches road speed after about three seconds"
    );
}

#[test]
fn a_jeep_accelerates_from_rest_and_brakes_before_a_road_bend() {
    let map = serde_json::json!({
        "size": [500, 500], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "props": [{ "kind": "wall", "center": [350, 422.8], "yaw": 0,
            "half_extents": [60, 0.5, 1.5] }],
        "surfaces": [{ "kind": "road", "shape": { "kind": "stroke",
            "points": [[0, 50], [350, 50], [350, 450]], "width_m": 20 } }]
    })
    .to_string();
    let mut setup = common::scenario(
        &map,
        serde_json::json!([
            { "side": "blue", "kind": "jeep", "position": [20, 50], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut rules = common::scenario_rules();
    rules["movement"]["drive"]["acceleration_s"] = serde_json::json!(4.5);
    rules["movement"]["drive"]["braking_s"] = serde_json::json!(1.5);
    rules["surfaces"]["road"]["speed_factor"] = serde_json::json!(1.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "jeep",
        serde_json::json!({
            "body": { "hull": { "half_extents_m": [2.2, 1.0, 0.95] } },
            "mobility": { "wheeled": { "offroad_kmh": 32, "road_kmh": 110,
                "turn_deg_s": 60, "turning_radius_m": 6, "reverse_fraction": 0.5 } }
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let hz = setup.rules.tick_hz;
    let mut b = Battle::new(&setup, 1);
    let mut orders = Orders { seq: 0 };
    orders.go(&mut b, &[0], [350.0, 420.0], 1, RoutePolicy::Fastest, false);
    let full = 110.0 / 3.6;
    let mut peak: f64 = 0.0;
    let mut previous: f64 = 0.0;
    let mut braked_before_turn = false;
    let mut first = None;
    for _ in 0..35 * hz {
        let before = b.unit(UnitId(0)).unwrap().position.xy();
        b.step();
        let u = own(&b, 0);
        let speed = dist([before.x, before.y], xy(&u)) * hz as f64;
        first.get_or_insert(speed);
        if u.position[0] < 330.0 && u.yaw.abs() < 5.0f64.to_radians() {
            assert!(
                (speed - previous).abs() <= full / 1.5 / hz as f64 + 1e-6,
                "speed changes gradually on the incoming leg: {previous} -> {speed}"
            );
            peak = peak.max(speed);
            braked_before_turn |=
                u.position[0] > 250.0 && peak > full * 0.95 && speed < full * 0.85;
        }
        previous = speed;
        if u.state == MoveState::Idle {
            break;
        }
    }
    assert!(
        first.unwrap() < full * 0.1,
        "start from rest rather than road speed"
    );
    assert!(
        peak > full * 0.95,
        "straight road still reaches full speed: {peak}"
    );
    assert!(
        braked_before_turn,
        "brake while still facing along the incoming road"
    );
    let u = own(&b, 0);
    assert!(
        u.state == MoveState::Idle && dist(xy(&u), [350.0, 420.0]) < 2.0,
        "complete the turn and arrive: {:?} {:?}",
        u.state,
        u.position
    );
}

/// The corrected town corridor is a real move, not just a finite diagnostic.
#[test]
fn a_squad_follows_a_town_corner_corridor_and_replays() {
    let mut setup = one_squad_setup(
        serde_json::json!([
            {"kind":"wall","center":[60,41],"yaw":0,"half_extents":[10,10,4]}
        ]),
        [20.0, 40.0],
        [100.0, 40.0],
    );
    setup.rules.navigation.work_per_tick = 1;
    let mut live = Battle::new(&setup, 1);
    let mut digests = Vec::new();
    for _ in 0..3000 {
        live.step();
        let load = live.load();
        assert!(load.planning_work <= 1 + sim::navigation::LARGEST_STEP);
        let wall = live.world().prop(0).unwrap().footprint();
        for soldier in &live.unit(UnitId(0)).unwrap().members {
            assert!(
                !wall.contains(
                    soldier.position.xy(),
                    setup.rules.physics.soldier_radius_m - 1e-6
                ),
                "soldier inside wall at tick {}",
                live.tick()
            );
        }
        digests.push(live.digest());
        if live.tick() > 1 && live.unit(UnitId(0)).unwrap().state == MoveState::Idle {
            break;
        }
    }
    assert!(digests.len() < 3000, "town corner route never arrived");
    assert!(dist(xy(&own(&live, 0)), [100.0, 40.0]) < 2.0);
    let record = live.replay();
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest);
    }
    assert_eq!(
        serde_json::to_string(&replay.replay()).unwrap(),
        serde_json::to_string(&record).unwrap()
    );
}

#[test]
fn a_squad_leaves_a_body_containing_only_its_centroid() {
    let props = serde_json::json!([
        {"kind":"wall","center":[20,40],"yaw":0,"half_extents":[0.5,4,4]}
    ]);
    let setup = one_squad_setup(props, [20.0, 40.0], [100.0, 40.0]);
    let mut b = Battle::new(&setup, 1);
    let wall = b.world().prop(0).unwrap().footprint();
    assert!(wall.contains(b.unit(UnitId(0)).unwrap().position.xy(), 0.0));
    assert!(b
        .unit(UnitId(0))
        .unwrap()
        .members
        .iter()
        .all(|s| !wall.contains(s.position.xy(), setup.rules.physics.soldier_radius_m)));
    let u = b.unit(UnitId(0)).unwrap();
    let middle = u
        .members
        .iter()
        .map(|s| s.position.xy())
        .fold(sim::math::v2(0.0, 0.0), |sum, p| sum + p)
        * (1.0 / u.members.len() as f64);
    assert!(wall.contains(middle, 0.0));
    let mut digests = Vec::new();
    for _ in 0..1500 {
        b.step();
        assert!(b
            .unit(UnitId(0))
            .unwrap()
            .members
            .iter()
            .all(|s| !wall.contains(s.position.xy(), setup.rules.physics.soldier_radius_m - 1e-6)));
        digests.push(b.digest());
    }
    let unit = own(&b, 0);
    assert_eq!(unit.state, MoveState::Idle);
    assert!(
        dist(xy(&unit), [100.0, 40.0]) < 2.0,
        "legal soldiers failed to leave centroid body"
    );
    let record = b.replay();
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest);
    }
}
