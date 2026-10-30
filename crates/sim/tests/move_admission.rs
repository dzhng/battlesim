//! Accepted movement markers must describe executable moves on known ground.
use contract::command::{CommandEnvelope, MoveDirection, Order, OrderError, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

use crate::common;

fn boxed() -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": {"size": [100, 80], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "props": [
                {"kind": "wall", "center": [50, 37], "yaw": 0, "half_extents": [30, 0.5, 2]},
                {"kind": "wall", "center": [50, 45], "yaw": 0, "half_extents": [30, 0.5, 2]},
                {"kind": "wall", "center": [34, 41], "yaw": 0, "half_extents": [0.5, 3.5, 2]}
            ]},
        "rules": common::scenario_rules(),
        "units": [
            {"side": "blue", "kind": "test_jeep", "position": [40, 41], "yaw": 0},
            {"side": "blue", "kind": "test_supply", "position": [47, 41], "yaw": 0}
        ], "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn move_to(units: &[u32], goal: [f64; 2]) -> Order {
    Order::Move {
        units: units.iter().copied().map(UnitId).collect(),
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    }
}

#[test]
fn hidden_enemy_hulls_do_not_change_preview_or_future_battle_state() {
    let setup = common::scenario(
        r#"{"size":[800,120],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        json!([{"side":"blue","kind":"test_jeep","position":[30,61],"yaw":0}]),
        json!([]),
    );
    let mut hidden_setup = setup.clone();
    let mut enemy = setup.units[0].clone();
    enemy.side = Side::Red;
    enemy.position = [600.0, 61.0];
    hidden_setup.units.push(enemy);
    let mut clear = Battle::new(&setup, 1);
    let mut hidden = Battle::new(&hidden_setup, 1);
    assert!(
        hidden.observe(Side::Blue).identified.is_empty(),
        "the enemy is beyond sensor range"
    );
    let request = contract::command::MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [600.0, 61.0],
        ..Default::default()
    };
    let clear_digest = clear.digest();
    let hidden_digest = hidden.digest();
    let expected = clear.preview_move(Side::Blue, &request).unwrap();
    assert!(expected[0].placed, "the public terrain permits the move");
    assert_eq!(hidden.preview_move(Side::Blue, &request).unwrap(), expected);
    assert_eq!(
        clear.digest(),
        clear_digest,
        "preview is not battle progress"
    );
    assert_eq!(
        hidden.digest(),
        hidden_digest,
        "unknown hulls remain private"
    );
    let mut unqueried = Battle::new(&hidden_setup, 1);
    for battle in [&mut hidden, &mut unqueried] {
        assert_eq!(
            battle
                .accept(CommandEnvelope {
                    side: Side::Blue,
                    player: None,
                    seq: 1,
                    queued: false,
                    order: move_to(&[0], [90.0, 61.0]),
                })
                .error,
            None
        );
    }
    for _ in 0..40 {
        hidden.step();
        unqueried.step();
        assert_eq!(
            hidden.digest(),
            unqueried.digest(),
            "preview cannot alter future movement planning"
        );
    }
}

#[test]
fn an_accepted_queued_destination_runs_after_its_pending_waypoint() {
    let setup = common::scenario(
        r#"{"size":[160,120],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        json!([{"side":"blue","kind":"test_jeep","position":[25,61],"yaw":0}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let first = [65.0, 61.0];
    let last = [25.0, 61.0];
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 1,
                queued: false,
                order: move_to(&[0], first),
            })
            .error,
        None
    );
    let queued = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 2,
        queued: true,
        order: move_to(&[0], last),
    });
    assert_eq!(queued.error, None);
    let destination = &queued.placement.unwrap().destinations[0];
    assert!(destination.placed);
    assert_eq!(destination.goal, last);
    let mut visited_first = false;
    for _ in 0..3000 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        visited_first |= (unit.position.xy() - sim::math::v2(first[0], first[1])).length() < 1.0;
        if unit.state == contract::observation::MoveState::Idle {
            break;
        }
    }
    assert!(visited_first, "queued move cannot skip its predecessor");
    let unit = battle.unit(UnitId(0)).unwrap();
    assert_eq!(unit.state, contract::observation::MoveState::Idle);
    assert!(
        (unit.position.xy() - sim::math::v2(last[0], last[1])).length() < 1.0,
        "the vehicle must reach the accepted queued marker: {:?}",
        unit.position
    );
    assert_eq!(battle.observe(Side::Blue).own[0].goal, None);
}

#[test]
fn the_same_boxed_vehicle_can_depart_with_its_blocker_in_a_group() {
    let mut battle = boxed();
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: move_to(&[0, 1], [88.0, 41.0]),
    });
    assert_eq!(
        ack.error, None,
        "moving members may clear one another: {ack:?}"
    );
    let destinations = ack.placement.unwrap().destinations;
    assert!(destinations.iter().all(|d| d.placed));
    for _ in 0..3000 {
        battle.step();
        let jeep = battle.unit(UnitId(0)).unwrap();
        let blocker = battle.unit(UnitId(1)).unwrap();
        assert!(
            !jeep
                .ground_footprint()
                .unwrap()
                .overlaps(&blocker.ground_footprint().unwrap()),
            "group departure preserves physical hull separation at tick {}",
            battle.tick()
        );
        if [jeep, blocker]
            .iter()
            .all(|u| u.state == contract::observation::MoveState::Idle)
        {
            break;
        }
    }
    for destination in destinations {
        let unit = battle.unit(destination.unit).unwrap();
        assert_eq!(unit.state, contract::observation::MoveState::Idle);
        assert!(
            (unit.position.xy() - sim::math::v2(destination.goal[0], destination.goal[1])).length()
                < 1.0,
            "unit {:?} must reach its accepted group marker",
            destination.unit
        );
    }
}

#[test]
fn a_move_through_more_trees_is_valid_after_clearing_a_forest_lane() {
    let setup = common::scenario(
        &json!({"size":[400,80],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "forests":[
            {"shape":{"kind":"polygon","ring":[[70,0],[130,0],[130,80],[70,80]]}},
            {"shape":{"kind":"polygon","ring":[[220,0],[280,0],[280,80],[220,80]]}}
        ]})
        .to_string(),
        json!([{"side":"blue","kind":"test_tank","position":[15,40],"yaw":0,
            "engagement":"return_fire_only"}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 1,
                queued: false,
                order: move_to(&[0], [185.0, 40.0]),
            })
            .error,
        None
    );
    for _ in 0..3000 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().state == contract::observation::MoveState::Idle {
            break;
        }
    }
    assert!(
        battle.world().cleared_cells() > 0,
        "the first live move must clear forest ground"
    );
    assert!(
        battle.unit(UnitId(0)).unwrap().position.x > 150.0,
        "the tank must leave the first forest before receiving its next order"
    );
    let before = battle.digest();
    let preview = battle
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0)],
                goal: [330.0, 40.0],
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        preview[0].placed,
        "the heavy vehicle can clear the next forest too"
    );
    assert_eq!(
        battle.digest(),
        before,
        "scratch forest clearance cannot alter the battle"
    );
    let accepted = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 2,
        queued: false,
        order: move_to(&[0], [330.0, 40.0]),
    });
    assert_eq!(accepted.error, None);
    assert!(accepted.placement.unwrap().destinations[0].placed);
}

#[test]
fn opposing_groups_keep_their_markers_when_traffic_requires_a_detour() {
    use contract::command::MovePreviewRequest;
    use contract::observation::MoveState;
    let mut scene = crate::movement_scenarios::scenario("sa6-columns-through-bridge");
    scene.scripts = json!([]);
    let mut battle = Battle::new(&crate::movement_scenarios::definition(&scene), scene.seed);
    let mut goals = [[0.0; 2]; 8];
    for (side, ids, goal) in [
        (Side::Blue, vec![0, 1, 2, 3], [60.0, 460.0]),
        (Side::Red, vec![4, 5, 6, 7], [60.0, 20.0]),
    ] {
        let request = MovePreviewRequest {
            units: ids.iter().copied().map(UnitId).collect(),
            goal,
            route: RoutePolicy::Fastest,
            ..Default::default()
        };
        let preview = battle.preview_move(side, &request).unwrap();
        assert!(
            preview.iter().all(|p| p.placed),
            "{side:?} placements: {preview:?}"
        );
        let ack = battle.accept(CommandEnvelope {
            side,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Move {
                units: request.units,
                gesture: 1,
                goal,
                route: RoutePolicy::Fastest,
                direction: MoveDirection::Forward,
                facing: None,
            },
        });
        assert_eq!(ack.error, None);
        assert_eq!(ack.applied_tick, 1, "both groups start together");
        assert_eq!(ack.placement.unwrap().destinations, preview);
        for p in preview {
            goals[p.unit.0 as usize] = p.goal;
        }
    }
    let mut arrived = [false; 8];
    let mut rear_continues = [false; 2];
    for _ in 0..100 * battle.rules().tick_hz {
        battle.step();
        for i in 0..8 {
            let unit = battle.unit(UnitId(i)).unwrap();
            let goal = goals[i as usize];
            let gap = (unit.position.x - goal[0]).hypot(unit.position.y - goal[1]);
            arrived[i as usize] |= unit.state == MoveState::Idle && gap < 1.5;
            assert!(
                battle
                    .world()
                    .surface_at(unit.position.x, unit.position.y)
                    .unwrap()
                    .traversable
            );
            for j in i + 1..8 {
                assert!(!unit
                    .ground_footprint()
                    .unwrap()
                    .overlaps(&battle.unit(UnitId(j)).unwrap().ground_footprint().unwrap()));
            }
        }
        for (side, lead, rear) in [(0, 2, 0), (1, 6, 4)] {
            rear_continues[side] |= arrived[lead]
                && !arrived[rear]
                && battle
                    .unit(UnitId(rear as u32))
                    .unwrap()
                    .movement_goal()
                    .is_some();
        }
    }
    assert!(
        rear_continues.iter().all(|v| *v),
        "rear vehicles continue after their leaders park"
    );
    for i in 0..8 {
        let unit = battle.unit(UnitId(i)).unwrap();
        let goal = goals[i as usize];
        let gap = (unit.position.x - goal[0]).hypot(unit.position.y - goal[1]);
        assert_eq!(unit.state, MoveState::Idle);
        assert!(gap < 1.5, "unit {i} misses its accepted marker by {gap} m");
    }
}

/// Market Town (6 km, generated) with the attacker's column of its saved
/// `assault` at the south edge and no defender: nothing fights, so a move
/// is refused or fails only for the ground's reasons.
fn market_town_column() -> ScenarioDefinition {
    let mut assault = sim::maps::scenario("market-town-test", "assault", common::rules()).unwrap();
    assault.units.retain(|u| u.side == Side::Blue);
    ScenarioDefinition {
        scripts: Vec::new(),
        opponent: None,
        encounter: None,
        ..assault
    }
}

/// Step until every unit in `destinations` stands idle at its marker, or
/// `seconds` pass; then hold each to it.
fn arrive(battle: &mut Battle, destinations: &[contract::command::MoveDestination], seconds: u64) {
    use contract::observation::MoveState;
    let at_marker = |battle: &Battle, d: &contract::command::MoveDestination| {
        let unit = battle.unit(d.unit).unwrap();
        // A squad stands round its marker; a hull stands on it.
        let reach = if unit.is_vehicle() { 1.5 } else { 12.0 };
        let gap = (unit.position.xy() - sim::math::v2(d.goal[0], d.goal[1])).length();
        (unit.state == MoveState::Idle && gap < reach, gap)
    };
    for _ in 0..seconds * battle.rules().tick_hz as u64 {
        battle.step();
        if destinations.iter().all(|d| at_marker(battle, d).0) {
            break;
        }
    }
    for d in destinations {
        let (there, gap) = at_marker(battle, d);
        assert!(
            there,
            "unit {:?} is {gap:.0} m from its accepted marker, {:?} at {:?}, at tick {}",
            d.unit,
            battle.unit(d.unit).unwrap().state,
            battle.unit(d.unit).unwrap().position.xy(),
            battle.tick()
        );
    }
}

#[test]
fn cross_map_moves_on_a_generated_map_are_placed_and_then_arrive() {
    let mut battle = Battle::new(&market_town_column(), 1);
    // A jeep alone, a rifle squad alone, and a tank, a squad, an AT team and
    // a supply truck together: from the south edge to the north one, 5.5 km.
    let mut accepted = Vec::new();
    for (seq, units, goal) in [
        (1, vec![0], [2511.0, 5706.0]),
        (2, vec![4], [2811.0, 5706.0]),
        (3, vec![2, 5, 7, 8], [2661.0, 5706.0]),
    ] {
        let mut order = move_to(&units, goal);
        if let Order::Move { gesture, .. } = &mut order {
            *gesture = seq;
        }
        let ack = battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq,
            order,
            queued: false,
        });
        let destinations = ack.placement.clone().unwrap().destinations;
        assert_eq!(ack.error, None, "units {units:?}: {destinations:?}");
        assert!(
            destinations.iter().all(|d| d.placed),
            "units {units:?}: {destinations:?}"
        );
        accepted.extend(destinations);
    }
    // On foot at under 3 m/s the squads need over half an hour.
    arrive(&mut battle, &accepted, 45 * 60);
}

/// A strip of open country 3 km long with a jeep and a rifle squad at its
/// west end, and `extra` map sections (`,"key":...`) laid on it.
fn strip(extra: &str, more_units: serde_json::Value) -> Battle {
    let mut units = vec![
        json!({"side": "blue", "kind": "test_jeep", "position": [100, 120], "yaw": 0}),
        json!({"side": "blue", "kind": "test_rifle", "position": [100, 90], "yaw": 0}),
    ];
    units.extend(more_units.as_array().unwrap().iter().cloned());
    let map = format!(
        r#"{{"size":[3000,240],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
    );
    Battle::new(&common::scenario(&map, json!(units), json!([])), 1)
}

fn placed(battle: &mut Battle, units: &[u32], goal: [f64; 2]) -> Vec<bool> {
    let request = contract::command::MovePreviewRequest {
        units: units.iter().copied().map(UnitId).collect(),
        goal,
        ..Default::default()
    };
    let preview = battle.preview_move(Side::Blue, &request).unwrap();
    preview.iter().map(|d| d.placed).collect()
}

#[test]
fn a_far_destination_across_an_unbridged_river_is_refused() {
    // The river cuts the strip from edge to edge half way along it.
    let river = r#""rivers":[{"points":[{"xy":[1500,0],"width_m":24,"depth_m":2},
        {"xy":[1500,240],"width_m":24,"depth_m":2}],"surface_z":-1.5}]"#;
    let bridge = r#""bridges":[{"deck":"bridge_deck","center":[1500,120],
        "half_extents":[18,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#;
    let mut bridged = strip(&format!(",{river},{bridge}"), json!([]));
    assert_eq!(
        placed(&mut bridged, &[0, 1], [2800.0, 120.0]),
        [true, true],
        "over the bridge both can go there"
    );
    let mut unbridged = strip(&format!(",{river}"), json!([]));
    assert_eq!(
        placed(&mut unbridged, &[0, 1], [2800.0, 120.0]),
        [false, false]
    );
    let ack = unbridged.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0, 1], [2800.0, 120.0]),
        queued: false,
    });
    assert_eq!(ack.error, Some(OrderError::NoValidDestination));
}

/// A yard walled on every side, far down the strip: the squad is sent (its
/// way is found on the move, and it stops where none is left), and the jeep
/// parks outside the yard instead.
#[test]
fn a_far_destination_walled_in_on_every_side_sends_a_squad_and_parks_a_vehicle_outside() {
    let wall = |center: [f64; 2], half: [f64; 2]| {
        format!(
            r#"{{"kind":"wall","center":[{},{}],"yaw":0,"half_extents":[{},{},2]}}"#,
            center[0], center[1], half[0], half[1]
        )
    };
    let three = [
        wall([2816.0, 120.0], [1.0, 17.0]),
        wall([2800.0, 103.0], [17.0, 1.0]),
        wall([2800.0, 137.0], [17.0, 1.0]),
    ]
    .join(",");
    let west = wall([2784.0, 120.0], [1.0, 17.0]);
    let mut open = strip(&format!(r#","props":[{three}]"#), json!([]));
    let mut closed = strip(&format!(r#","props":[{three},{west}]"#), json!([]));
    for unit in [0, 1] {
        assert_eq!(
            placed(&mut open, &[unit], [2800.0, 120.0]),
            [true],
            "unit {unit} can go into the yard through its open side"
        );
    }
    assert_eq!(
        placed(&mut closed, &[1], [2800.0, 120.0]),
        [true],
        "walls are found on the way, not refused at the click"
    );
    let request = contract::command::MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [2800.0, 120.0],
        ..Default::default()
    };
    let jeep = &closed.preview_move(Side::Blue, &request).unwrap()[0];
    assert!(jeep.placed, "{jeep:?}");
    let [x, y] = jeep.goal;
    assert!(
        !((2784.0..=2816.0).contains(&x) && (103.0..=137.0).contains(&y)),
        "the jeep parks outside the closed yard, not at {:?}",
        jeep.goal
    );
}

#[test]
fn four_infantry_squads_and_a_supply_truck_can_move_across_open_ground() {
    let setup = common::scenario(
        &json!({"size":[1600,400], "fog_cell_m":8, "height_grid_m":4,
            "slope_cutoff_deg":35, "props":[]})
        .to_string(),
        json!([
            {"side":"blue", "kind":"test_rifle", "position":[100,120]},
            {"side":"blue", "kind":"test_rifle", "position":[100,160]},
            {"side":"blue", "kind":"test_rifle", "position":[100,200]},
            {"side":"blue", "kind":"test_rifle", "position":[100,240]},
            {"side":"blue", "kind":"test_supply", "position":[100,280], "yaw":0}
        ]),
        json!([]),
    );
    for distance in [500.0, 700.0, 900.0, 1400.0] {
        let mut battle = Battle::new(&setup, 1);
        let goal = [100.0 + distance, 200.0];
        let preview = battle
            .preview_move(
                Side::Blue,
                &contract::command::MovePreviewRequest {
                    units: (0..5).map(UnitId).collect(),
                    goal,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            preview.iter().all(|d| d.placed),
            "cursor preview at {distance} m: {preview:?}"
        );
        let ack = battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order: move_to(&[0, 1, 2, 3, 4], goal),
            queued: false,
        });
        assert_eq!(
            ack.error, None,
            "open ground is reachable at {distance} m: {ack:?}"
        );
        let destinations = ack.placement.unwrap().destinations;
        assert!(destinations.iter().all(|d| d.placed), "{destinations:?}");
        arrive(&mut battle, &destinations, 20 * 60);
    }
}

#[test]
fn a_mixed_infantry_group_can_take_a_fast_road_move_and_upgrade_its_normal_move() {
    let setup = common::scenario(
        &json!({"size":[1600,400], "fog_cell_m":8, "height_grid_m":4,
        "slope_cutoff_deg":35, "surfaces":[
            {"kind":"road", "shape":{"kind":"stroke", "points":[[20,350],[1580,350]], "width_m":8}}
        ]})
        .to_string(),
        json!([
            {"side":"blue", "kind":"test_rifle", "position":[100,120]},
            {"side":"blue", "kind":"test_rifle", "position":[100,160]},
            {"side":"blue", "kind":"test_rifle", "position":[100,200]},
            {"side":"blue", "kind":"test_rifle", "position":[100,240]},
            {"side":"blue", "kind":"test_supply", "position":[100,280], "yaw":0}
        ]),
        json!([]),
    );
    for (route, apply_before_upgrade) in [
        (RoutePolicy::Fastest, false),
        (RoutePolicy::Shortest, false),
        (RoutePolicy::Shortest, true),
    ] {
        let mut battle = Battle::new(&setup, 1);
        let goal = [1000.0, 200.0];
        let mut order = move_to(&[0, 1, 2, 3, 4], goal);
        if let Order::Move { route: policy, .. } = &mut order {
            *policy = route;
        }
        let preview = battle
            .preview_move(
                Side::Blue,
                &contract::command::MovePreviewRequest {
                    units: (0..5).map(UnitId).collect(),
                    goal,
                    route,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(preview.iter().all(|d| d.placed), "{route:?}: {preview:?}");
        let ack = battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order,
            queued: false,
        });
        assert_eq!(ack.error, None, "{route:?}: {ack:?}");
        let destinations = ack.placement.unwrap().destinations;
        assert!(destinations.iter().all(|d| d.placed), "{destinations:?}");
        if route == RoutePolicy::Shortest {
            if apply_before_upgrade {
                battle.step();
            }
            let upgrade = battle.accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 2,
                queued: false,
                order: Order::UpgradeMove {
                    gesture: 1,
                    route: RoutePolicy::Fastest,
                },
            });
            assert_eq!(
                upgrade.error, None,
                "applied={apply_before_upgrade}: {upgrade:?}"
            );
        }
        let mut truck_used_road = false;
        for _ in 0..20 * 60 * battle.rules().tick_hz {
            battle.step();
            let truck = battle.unit(UnitId(4)).unwrap();
            truck_used_road |= (truck.position.y - 350.0).abs() < 4.0;
            if battle
                .observe(Side::Blue)
                .own
                .iter()
                .all(|u| u.goal.is_none())
            {
                break;
            }
        }
        assert!(truck_used_road, "the fast move must actually use the road");
        arrive(&mut battle, &destinations, 1);
    }
}

/// Step until unit `id` is at rest or has given up, for at most `seconds`,
/// holding it every tick to no overlap with a body it cannot shove. The
/// answer is how it ended.
fn settle(battle: &mut Battle, id: u32, seconds: u64) -> contract::observation::MoveState {
    use contract::map::MoverClass;
    use contract::observation::MoveState;
    for _ in 0..seconds * battle.rules().tick_hz as u64 {
        battle.step();
        let unit = battle.unit(UnitId(id)).unwrap();
        if let Some(hull) = unit.ground_footprint() {
            for prop in battle.world().props_near(hull.center, hull.half.length()) {
                assert!(
                    !prop.blocks(MoverClass::Vehicle)
                        || unit.ground().push.pushes(prop.body.weight_class)
                        || !hull.overlaps(&prop.footprint()),
                    "unit {id} runs into a {:?} at tick {}",
                    prop.kind,
                    battle.tick()
                );
            }
        }
        if battle.tick() > 2 && matches!(unit.state, MoveState::Idle | MoveState::RouteBlocked) {
            return unit.state;
        }
    }
    battle.unit(UnitId(id)).unwrap().state
}

/// Pathfinding on the way. A lane jammed by two crates side by side: the
/// truck can shove one, but not one into the other. It drives up, finds
/// it cannot pass, and gives up; down the open lane it arrives.
#[test]
fn a_truck_gives_up_on_a_lane_its_shove_cannot_clear() {
    use contract::observation::MoveState;
    let lane = |blocked: bool| {
        let mut props = vec![
            json!({"kind":"wall", "center":[600,55], "yaw":0, "half_extents":[100,55,2]}),
            json!({"kind":"wall", "center":[600,185], "yaw":0, "half_extents":[100,55,2]}),
        ];
        if blocked {
            props.push(
                json!({"kind":"crate", "center":[600,120], "yaw":0, "half_extents":[1.5,9.9,2]}),
            );
            props.push(
                json!({"kind":"crate", "center":[603,120], "yaw":0, "half_extents":[1.5,9.9,2]}),
            );
        }
        let map = json!({"size":[1200,240], "fog_cell_m":8, "height_grid_m":4,
            "slope_cutoff_deg":35, "props":props});
        Battle::new(
            &common::scenario(
                &map.to_string(),
                json!([{"side":"blue", "kind":"test_supply", "position":[100,120], "yaw":0}]),
                json!([]),
            ),
            1,
        )
    };
    for blocked in [true, false] {
        let mut battle = lane(blocked);
        let ack = battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order: move_to(&[0], [1100.0, 120.0]),
            queued: false,
        });
        assert_eq!(ack.error, None, "the lane is open ground: {ack:?}");
        let ended = settle(&mut battle, 0, 600);
        let x = battle.unit(UnitId(0)).unwrap().position.x;
        if blocked {
            assert_eq!(
                ended,
                MoveState::RouteBlocked,
                "the jammed lane, at x = {x:.0}"
            );
            assert!(x < 600.0, "it never got past the crates: x = {x:.0}");
        } else {
            assert_eq!(ended, MoveState::Idle, "the open lane, at x = {x:.0}");
            assert!(x > 1090.0, "it arrives: x = {x:.0}");
        }
    }
}

/// A truck boxed in a lane by a vehicle that stays where it is: it waits
/// rather than driving through it.
#[test]
fn a_vehicle_boxed_by_a_parked_one_never_drives_through_it() {
    let mut battle = boxed();
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0], [70.0, 41.0]),
        queued: false,
    });
    assert_eq!(ack.error, None);
    for _ in 0..60 * battle.rules().tick_hz as u64 {
        battle.step();
        let [jeep, truck] =
            [0, 1].map(|id| battle.unit(UnitId(id)).unwrap().ground_footprint().unwrap());
        assert!(
            !jeep.overlaps(&truck),
            "the jeep runs into the truck at tick {}",
            battle.tick()
        );
        assert!(
            jeep.center.x < 47.0,
            "it is never past the truck: {:?}",
            jeep.center
        );
    }
}

/// A squad ringed in by soldiers who stand where they are: it never walks
/// through one of them.
#[test]
fn a_squad_ringed_in_by_standing_soldiers_never_walks_through_them() {
    let mut rules = common::scenario_rules();
    rules["physics"]["soldier_radius_m"] = json!(0.3);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({"body":{"squad":{"slots":["test_rifleman"]}}}),
    );
    let mut units = vec![json!({"side":"blue","kind":"test_rifle","position":[60,60],
        "yaw":0,"engagement":"return_fire_only"})];
    for i in 0..8 {
        let angle = i as f64 * std::f64::consts::TAU / 8.0;
        units.push(json!({"side":"red","kind":"test_rifle",
            "position":[60.0 + 0.8 * angle.cos(),60.0 + 0.8 * angle.sin()],
            "yaw":0,"engagement":"return_fire_only"}));
    }
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[120,100],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]},
        "rules":rules,"units":units,"events":[],"scripts":[]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: move_to(&[0], [100.0, 60.0]),
    });
    assert_eq!(ack.error, None);
    let apart = 2.0 * battle.rules().physics.soldier_radius_m - 1e-6;
    for _ in 0..60 * battle.rules().tick_hz as u64 {
        battle.step();
        let me = battle.unit(UnitId(0)).unwrap().members[0].position.xy();
        for i in 1..9 {
            let them = battle.unit(UnitId(i)).unwrap().members[0].position.xy();
            assert!(
                (me - them).length() >= apart,
                "the squad walks through soldier {i} at tick {}",
                battle.tick()
            );
        }
    }
}

#[path = "move_admission/oversized_hulls.rs"]
mod oversized_hulls;

#[path = "move_admission/destination_placement.rs"]
mod destination_placement;
