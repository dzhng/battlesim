//! Long legs go by road: an ordered leg over `navigation.road_leg_m` takes
//! the fastest policy, and the fastest policy takes a road journey where one
//! beats driving straight there. The road only says where to go: every
//! metre is still checked for the mover on what its side knows.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

/// Three kilometres of open ground with one road along its north side.
fn one_road() -> Value {
    json!({ "size": [3000, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "surfaces": [road([[20, 400], [2980, 400]])] })
}

/// A river down the middle, a road with a bridge south and north of each
/// other, and a road on either bank joining the two.
fn two_bridges() -> Value {
    let bridge = |y: f64| {
        json!({ "deck": "bridge_deck", "center": [1500, y],
        "half_extents": [16, 6], "yaw": 0, "deck_z": 0.1, "thickness_m": 0.8 })
    };
    json!({ "size": [3000, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "rivers": [{ "points": [{ "xy": [1500, 0], "width_m": 20, "depth_m": 1.5 }, { "xy": [1500, 800], "width_m": 20, "depth_m": 1.5 }], "surface_z": -0.5 }],
        "bridges": [bridge(200.0), bridge(600.0)],
        "surfaces": [
            road([[20, 200], [2980, 200]]),
            road([[20, 600], [2980, 600]]),
            road([[300, 200], [300, 600]]),
            road([[2700, 200], [2700, 600]]),
        ] })
}

fn road(points: [[i32; 2]; 2]) -> Value {
    json!({ "kind": "road", "shape": { "kind": "stroke", "points": points, "width_m": 8 } })
}

fn scenario(map: Value, units: Value, events: Value) -> ScenarioDefinition {
    serde_json::from_value(json!({
        "map": map, "rules": common::scenario_rules(), "units": units, "events": events, "scripts": [],
    }))
    .unwrap()
}

fn jeep(at: [f64; 2]) -> Value {
    json!([{ "side": "blue", "kind": "jeep", "position": at }])
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// An ordinary (shortest) move order for unit 0.
fn order(b: &mut Battle, seq: u64, goal: [f64; 2]) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: seq,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
}

/// Step until unit 0 has its route (or has none), never spending more than
/// a tick's planning allowance, and return it as its side sees it.
fn planned(b: &mut Battle) -> OwnUnit {
    let allowance = b.rules().navigation.work_per_tick as u64;
    for _ in 0..600 {
        b.step();
        assert!(b.load().planning_work <= allowance + sim::navigation::LARGEST_STEP);
        if own(b, 0).state != MoveState::Planning {
            return own(b, 0);
        }
    }
    panic!("planning never finished");
}

/// Step until unit 0 is idle; the seconds it took.
fn arrive(b: &mut Battle, limit_s: f64, mut each: impl FnMut(&OwnUnit)) -> f64 {
    let hz = b.rules().tick_hz as f64;
    let start = b.tick();
    while ((b.tick() - start) as f64) < limit_s * hz {
        b.step();
        let unit = own(b, 0);
        each(&unit);
        if unit.state == MoveState::Idle {
            return (b.tick() - start) as f64 / hz;
        }
    }
    panic!(
        "unit 0 never arrived: {:?} at {:?}",
        own(b, 0).state,
        own(b, 0).position
    );
}

/// How many of a route's waypoints lie on the 8 m road along the line `y`.
fn on_line(unit: &OwnUnit, y: f64) -> usize {
    unit.route
        .iter()
        .filter(|p| (p[1] - y).abs() <= 4.0)
        .count()
}

#[test]
fn a_leg_over_two_kilometres_goes_by_road_and_a_shorter_one_goes_straight() {
    let setup = scenario(one_road(), jeep([100.0, 100.0]), json!([]));
    // 2,600 m: over to the road 300 m away, along it, and back off it.
    let mut b = Battle::new(&setup, 1);
    order(&mut b, 1, [2700.0, 100.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.policy, Some(RoutePolicy::Fastest));
    assert!(
        on_line(&unit, 400.0) >= 2,
        "it joins the road and leaves it: {:?}",
        unit.route
    );
    let rules = common::rules();
    let (off_road_mps, _) = rules.catalog.by_id("jeep").mobility.speeds_mps();
    let took = arrive(&mut b, 400.0, |_| {});
    assert!(
        dist(
            [own(&b, 0).position[0], own(&b, 0).position[1]],
            [2700.0, 100.0]
        ) < 2.0
    );
    assert!(
        took < 0.75 * 2600.0 / off_road_mps,
        "the road pays: {took:.0} s against {:.0} s straight across country",
        2600.0 / off_road_mps
    );
    // 1,800 m: an ordinary move, straight there.
    let mut b = Battle::new(&setup, 1);
    order(&mut b, 1, [1900.0, 100.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.policy, Some(RoutePolicy::Shortest));
    assert_eq!(unit.route, vec![[1900.0, 100.0]]);
}

#[test]
fn the_road_rule_starts_strictly_over_the_distance_in_the_rules() {
    let policy = |goal_x: f64, road_leg_m: f64| {
        let mut setup = scenario(one_road(), jeep([100.0, 100.0]), json!([]));
        setup.rules.navigation.road_leg_m = road_leg_m;
        let mut b = Battle::new(&setup, 1);
        order(&mut b, 1, [goal_x, 100.0]);
        planned(&mut b).policy
    };
    let shipped = common::rules().navigation.road_leg_m;
    assert_eq!(shipped, 2000.0, "the distance the user chose");
    // Exactly the distance does not qualify; half a metre over does.
    assert_eq!(policy(2100.0, shipped), Some(RoutePolicy::Shortest));
    assert_eq!(policy(2100.5, shipped), Some(RoutePolicy::Fastest));
    // The distance is the rules': with 500 m there, an 800 m leg qualifies.
    assert_eq!(policy(900.0, 500.0), Some(RoutePolicy::Fastest));
    assert_eq!(policy(900.0, shipped), Some(RoutePolicy::Shortest));
}

#[test]
fn a_leg_keeps_the_road_when_it_plans_again_closer_in_round_a_wall_it_meets() {
    // A wall across the road 700 m short of the goal, added after the order:
    // the jeep's side knows nothing of it until the jeep drives up to it.
    let wall = json!([{ "tick": 2, "add_prop":
        { "kind": "wall", "center": [2000, 400], "yaw": 0, "half_extents": [0.5, 12, 1] } }]);
    let mut b = Battle::new(&scenario(one_road(), jeep([100.0, 400.0]), wall), 1);
    order(&mut b, 1, [2700.0, 400.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.route.last(), Some(&[2700.0, 400.0]));
    assert_eq!(
        on_line(&unit, 400.0),
        unit.route.len(),
        "down the road: {:?}",
        unit.route
    );
    let searches = b.route_searches(Side::Blue);
    let mut worst = 0.0f64;
    arrive(&mut b, 300.0, |unit| {
        if unit.goal.is_some() {
            assert_eq!(
                unit.policy,
                Some(RoutePolicy::Fastest),
                "the leg stays a road leg however short it has become"
            );
        }
        if (unit.position[0] - 2000.0).abs() < 3.0 {
            worst = worst.max((unit.position[1] - 400.0).abs());
        }
    });
    assert!(
        dist(
            [own(&b, 0).position[0], own(&b, 0).position[1]],
            [2700.0, 400.0]
        ) < 2.0
    );
    assert!(
        b.route_searches(Side::Blue) > searches,
        "it planned again when it met the wall"
    );
    assert!(
        worst > 12.0,
        "it drove round the wall's end: {worst:.1} m off the road"
    );
}

#[test]
fn a_road_journey_crosses_by_the_nearer_bridge() {
    let mut b = Battle::new(&scenario(two_bridges(), jeep([100.0, 200.0]), json!([])), 1);
    order(&mut b, 1, [2900.0, 200.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.route.last(), Some(&[2900.0, 200.0]));
    assert_eq!(
        on_line(&unit, 200.0),
        unit.route.len(),
        "one road, one bridge: {:?}",
        unit.route
    );
    arrive(&mut b, 200.0, |_| {});
}

#[test]
fn a_bridge_known_to_be_blocked_sends_the_journey_over_the_other_one() {
    // A wall right across the southern deck, on the map from the start (so
    // both sides know it): water either side, nothing gets round it.
    let mut map = two_bridges();
    map["props"] =
        json!([{ "kind": "wall", "center": [1500, 200], "yaw": 0, "half_extents": [0.5, 7, 1] }]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 200.0]), json!([])), 1);
    order(&mut b, 1, [2900.0, 200.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.state, MoveState::Moving);
    assert!(
        on_line(&unit, 600.0) >= 2,
        "over the northern bridge: {:?}",
        unit.route
    );
    let mut crossed_at = None;
    arrive(&mut b, 400.0, |unit| {
        if (unit.position[0] - 1500.0).abs() < 2.0 {
            crossed_at = Some(unit.position[1]);
        }
    });
    assert!((crossed_at.expect("it crossed the river") - 600.0).abs() < 6.0);
}

#[test]
fn a_road_with_no_bridge_is_no_road_across() {
    // The same two roads, neither with a deck over the river.
    let mut map = two_bridges();
    map["bridges"] = json!([]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 200.0]), json!([])), 1);
    order(&mut b, 1, [2900.0, 200.0]);
    let unit = planned(&mut b);
    assert_eq!(unit.state, MoveState::RouteBlocked);
    assert_eq!(
        unit.position[0], 100.0,
        "it never set off for a crossing that is not there"
    );
}

/// A jeep and a tank at the two ends of `map`'s one road, each sent to
/// where the other starts: the ticks either spent waiting for the other,
/// once both have arrived.
fn head_on(map: Value) -> u64 {
    let units = json!([
        { "side": "blue", "kind": "jeep", "position": [100, 400] },
        { "side": "blue", "kind": "tank", "position": [2900, 400], "yaw": std::f64::consts::PI },
    ]);
    let mut b = Battle::new(&scenario(map, units, json!([])), 1);
    for (seq, (unit, goal)) in [(0, 2900.0), (1, 100.0)].into_iter().enumerate() {
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: seq as u64 + 1,
            order: Order::Move {
                units: vec![UnitId(unit)],
                gesture: seq as u64 + 1,
                goal: [goal, 400.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None);
    }
    let hz = b.rules().tick_hz as u64;
    let mut waited = 0;
    for _ in 0..400 * hz {
        b.step();
        waited += u64::from(own(&b, 0).state == MoveState::Waiting);
        waited += u64::from(own(&b, 1).state == MoveState::Waiting);
        if own(&b, 0).state == MoveState::Idle && own(&b, 1).state == MoveState::Idle {
            break;
        }
    }
    assert!(
        (own(&b, 0).position[0] - 2900.0).abs() < 2.0,
        "the jeep arrived: {:?} at {:?}",
        own(&b, 0).state,
        own(&b, 0).position
    );
    assert!(
        (own(&b, 1).position[0] - 100.0).abs() < 2.0,
        "the tank arrived: {:?} at {:?}",
        own(&b, 1).state,
        own(&b, 1).position
    );
    waited
}

#[test]
fn two_vehicles_meeting_head_on_on_one_road_pass_each_other() {
    assert_eq!(
        head_on(one_road()),
        0,
        "each kept to its right: neither waited for the other"
    );
}

#[test]
fn on_a_narrow_track_a_tank_and_a_jeep_still_get_past_each_other() {
    // A 5 m track is narrower than the two side by side: each keeps as far
    // right as its own middle stays on the track, its right side off it.
    let mut map = one_road();
    map["surfaces"][0]["kind"] = json!("dirt_track");
    map["surfaces"][0]["shape"]["width_m"] = json!(5);
    let hz = common::rules().tick_hz as u64;
    assert!(head_on(map) < 20 * hz, "no long standoff");
}

#[test]
fn two_columns_meeting_on_a_narrow_track_all_get_past() {
    // A jeep and a tank each way on a 5 m track, in the open.
    let mut map = one_road();
    map["surfaces"][0]["kind"] = json!("dirt_track");
    map["surfaces"][0]["shape"]["width_m"] = json!(5);
    let units = json!([
        { "side": "blue", "kind": "jeep", "position": [100, 400] },
        { "side": "blue", "kind": "tank", "position": [60, 400] },
        { "side": "blue", "kind": "jeep", "position": [2900, 400], "yaw": std::f64::consts::PI },
        { "side": "blue", "kind": "tank", "position": [2940, 400], "yaw": std::f64::consts::PI },
    ]);
    let goals = [2900.0, 2940.0, 100.0, 60.0];
    let mut b = Battle::new(&scenario(map, units, json!([])), 1);
    for (unit, goal) in goals.into_iter().enumerate() {
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: unit as u64 + 1,
            order: Order::Move {
                units: vec![UnitId(unit as u32)],
                gesture: unit as u64 + 1,
                goal: [goal, 400.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None);
    }
    let hz = b.rules().tick_hz as u64;
    for _ in 0..400 * hz {
        b.step();
        if (0..4).all(|id| own(&b, id).state == MoveState::Idle) {
            break;
        }
    }
    for (unit, goal) in goals.into_iter().enumerate() {
        let unit = own(&b, unit as u32);
        assert!(
            (unit.position[0] - goal).abs() < 2.0,
            "unit {:?} arrived: {:?} at {:?}",
            unit.id,
            unit.state,
            unit.position
        );
    }
}

#[test]
fn a_queued_leg_is_measured_from_where_it_starts_not_from_where_it_was_ordered() {
    // A 500 m leg, then a queued one. From the order's place the second
    // goal is 2,400 m or 2,600 m off; from the first leg's end, 1,900 m or
    // 2,100 m.
    let policy_of_second_leg = |goal_x: f64| {
        let mut b = Battle::new(&scenario(one_road(), jeep([100.0, 100.0]), json!([])), 1);
        order(&mut b, 1, [600.0, 100.0]);
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: 2,
            order: Order::Move {
                units: vec![UnitId(0)],
                gesture: 2,
                goal: [goal_x, 100.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: true,
        });
        assert_eq!(ack.error, None);
        for _ in 0..4000 {
            b.step();
            let unit = own(&b, 0);
            match unit.goal {
                Some(goal) if goal[0] < 700.0 => {
                    assert_eq!(unit.policy, Some(RoutePolicy::Shortest), "the first leg")
                }
                // The second leg, once it has its route.
                Some(_) if !unit.route.is_empty() => return unit.policy,
                _ => {}
            }
        }
        panic!("the second leg never started");
    };
    assert_eq!(policy_of_second_leg(2500.0), Some(RoutePolicy::Shortest));
    assert_eq!(policy_of_second_leg(2700.0), Some(RoutePolicy::Fastest));
}

#[test]
fn a_fast_move_takes_the_road_round_a_wood_and_beats_the_straight_move_through_it() {
    // The movement lab: two tanks west of a wood, roads round its north.
    // As the crow flies over open ground the road is the longer way; over
    // the wood that lies on the straight line it is the quicker one.
    let units = json!([
        { "side": "blue", "kind": "tank", "position": [40, 190] },
        { "side": "blue", "kind": "tank", "position": [40, 215] },
    ]);
    let map: Value = serde_json::from_str(common::MOVEMENT_LAB).unwrap();
    let mut b = Battle::new(&scenario(map, units, json!([])), 1);
    for (unit, goal, route) in [
        (0u32, [275.0, 185.0], RoutePolicy::Shortest),
        (1, [300.0, 215.0], RoutePolicy::Fastest),
    ] {
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: unit as u64 + 1,
            order: Order::Move {
                units: vec![UnitId(unit)],
                gesture: unit as u64 + 1,
                goal,
                route,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None);
    }
    b.step();
    let length = |unit: &OwnUnit| {
        let mut at = [unit.position[0], unit.position[1]];
        unit.route
            .iter()
            .map(|p| {
                let leg = dist(at, *p);
                at = *p;
                leg
            })
            .sum::<f64>()
    };
    let (short, fast) = (own(&b, 0), own(&b, 1));
    assert!(
        length(&fast) > 1.5 * length(&short),
        "the fast move goes the long way round: {:?}",
        fast.route
    );
    let hz = b.rules().tick_hz as u64;
    let mut arrived = [0, 0];
    for _ in 0..200 * hz {
        b.step();
        for (id, tick) in arrived.iter_mut().enumerate() {
            if *tick == 0 && own(&b, id as u32).state == MoveState::Idle {
                *tick = b.tick();
            }
        }
    }
    assert!(
        arrived[1] > 0 && arrived[1] < arrived[0],
        "the fast move arrives first: {arrived:?}"
    );
}

#[test]
fn a_bridge_detour_is_chosen_over_an_impossible_straight_crossing() {
    let mut map = two_bridges();
    map["size"] = json!([3000, 6000]);
    map["rivers"][0]["points"][1]["xy"] = json!([1500, 6000]);
    map["bridges"] = json!([map["bridges"][0].clone()]);
    map["surfaces"] = json!([
        road([[20, 200], [2980, 200]]),
        road([[100, 200], [100, 5800]]),
        road([[2900, 200], [2900, 5800]])
    ]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 5600.0]), json!([])), 1);
    order(&mut b, 1, [2900.0, 5600.0]);
    let unit = planned(&mut b);
    assert_eq!(
        unit.state,
        MoveState::Moving,
        "road graph connects the banks"
    );
    assert_eq!(unit.route.last(), Some(&[2900.0, 5600.0]));
    assert!(
        on_line(&unit, 200.0) >= 2,
        "the route takes the distant bridge"
    );
}

#[test]
fn a_bridge_journey_reaches_a_goal_beyond_the_access_radius() {
    let mut map = two_bridges();
    map["size"] = json!([3000, 6000]);
    map["rivers"][0]["points"][1]["xy"] = json!([1500, 6000]);
    map["bridges"] = json!([map["bridges"][0].clone()]);
    map["surfaces"] = json!([
        road([[20, 200], [2980, 200]]),
        road([[100, 200], [100, 5800]]),
        road([[1800, 200], [1800, 5800]])
    ]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 5600.0]), json!([])), 1);
    order(&mut b, 1, [2900.0, 5600.0]);
    let unit = planned(&mut b);
    assert_eq!(
        unit.state,
        MoveState::Moving,
        "road graph connects the banks"
    );
    assert_eq!(unit.route.last(), Some(&[2900.0, 5600.0]));
    assert!(
        on_line(&unit, 200.0) >= 2,
        "the route takes the distant bridge"
    );
}

#[test]
fn a_wrong_bank_access_does_not_hide_the_farther_legal_approach() {
    let mut map = two_bridges();
    map["size"] = json!([3000, 6000]);
    map["rivers"][0]["points"][1]["xy"] = json!([1500, 6000]);
    map["bridges"] = json!([map["bridges"][0].clone()]);
    map["surfaces"] = json!([
        road([[20, 200], [2980, 200]]),
        road([[100, 200], [100, 5800]]),
        road([[2900, 200], [2900, 5800]]),
        road([[1400, 200], [1400, 5800]])
    ]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 5600.0]), json!([])), 1);
    order(&mut b, 1, [1700.0, 5600.0]);
    let unit = planned(&mut b);
    assert_eq!(
        unit.state,
        MoveState::Moving,
        "road graph connects the banks"
    );
    assert_eq!(unit.route.last(), Some(&[1700.0, 5600.0]));
    assert!(
        on_line(&unit, 200.0) >= 2,
        "the route takes the distant bridge"
    );
}

#[test]
fn discovering_one_endpoint_does_not_hide_the_others_legal_approach() {
    let mut map = two_bridges();
    map["size"] = json!([3000, 6000]);
    map["rivers"][0]["points"][1]["xy"] = json!([1500, 6000]);
    map["bridges"] = json!([map["bridges"][0].clone()]);
    map["surfaces"] = json!([
        road([[20, 200], [2980, 200]]),
        road([[1300, 200], [1300, 5800]]),
        road([[2900, 200], [2900, 5800]]),
        road([[1400, 200], [1400, 5800]])
    ]);
    let mut b = Battle::new(&scenario(map, jeep([100.0, 5600.0]), json!([])), 1);
    order(&mut b, 1, [1700.0, 5600.0]);
    let unit = planned(&mut b);
    assert_eq!(
        unit.state,
        MoveState::Moving,
        "road graph connects the banks"
    );
    assert_eq!(unit.route.last(), Some(&[1700.0, 5600.0]));
    assert!(
        on_line(&unit, 200.0) >= 2,
        "the route takes the distant bridge"
    );
}
