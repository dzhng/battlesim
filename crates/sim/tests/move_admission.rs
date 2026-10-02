//! Accepted movement markers must describe executable moves on known ground.
use contract::command::{CommandEnvelope, MoveDirection, Order, OrderError, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

fn boxed() -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": {"size": [100, 80], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "props": [
                {"kind": "wall", "center": [50, 37], "yaw": 0, "half_extents": [30, 0.5, 2]},
                {"kind": "wall", "center": [50, 45], "yaw": 0, "half_extents": [30, 0.5, 2]},
                {"kind": "wall", "center": [34, 41], "yaw": 0, "half_extents": [0.5, 3.5, 2]}
            ]},
        "rules": crate::common::scenario_rules(),
        "units": [
            {"side": "blue", "kind": "jeep", "position": [40, 41], "yaw": 0},
            {"side": "blue", "kind": "supply", "position": [47, 41], "yaw": 0}
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
fn a_vehicle_boxed_by_a_stationary_nonmember_gets_no_destination_marker() {
    let mut battle = boxed();
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: move_to(&[0], [70.0, 41.0]),
        queued: false,
    });
    assert_eq!(ack.error, Some(OrderError::NoValidDestination));
    assert!(!ack.placement.unwrap().destinations[0].placed);
    battle.step();
    assert_eq!(battle.observe(Side::Blue).own[0].goal, None);
}

#[test]
fn an_accepted_stop_prevents_relying_on_a_nonmember_vacating() {
    let mut battle = boxed();
    let vacate = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: move_to(&[1], [88.0, 41.0]),
        queued: false,
    });
    assert_eq!(vacate.error, None, "vacate ack: {vacate:?}");
    battle.step();
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                seq: 2,
                order: Order::Stop {
                    units: vec![UnitId(1)]
                },
                queued: false,
            })
            .error,
        None
    );
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 3,
        order: move_to(&[0], [70.0, 41.0]),
        queued: false,
    });
    assert_eq!(ack.error, Some(OrderError::NoValidDestination));
    assert!(!ack.placement.unwrap().destinations[0].placed);
}

#[test]
fn a_route_upgrade_cannot_rely_on_a_vehicle_with_a_pending_stop() {
    let mut battle = boxed();
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                seq: 1,
                order: move_to(&[1], [88.0, 41.0]),
                queued: false,
            })
            .error,
        None
    );
    battle.step();
    let drive = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 2,
        order: move_to(&[0], [70.0, 41.0]),
        queued: false,
    });
    assert_eq!(
        drive.error, None,
        "the blocker can vacate before Stop: {drive:?}"
    );
    assert!(drive.placement.unwrap().destinations[0].placed);
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                seq: 3,
                order: Order::Stop {
                    units: vec![UnitId(1)]
                },
                queued: false,
            })
            .error,
        None
    );
    let upgrade = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 4,
        order: Order::UpgradeMove {
            gesture: 1,
            route: RoutePolicy::Fastest,
        },
        queued: false,
    });
    assert_eq!(upgrade.error, Some(OrderError::NoValidDestination));
    assert!(
        upgrade.placement.is_none(),
        "upgrades do not replace destination markers"
    );
}

#[test]
fn hidden_enemy_hulls_do_not_change_preview_or_future_battle_state() {
    let setup = crate::common::scenario(
        r#"{"size":[800,120],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        json!([{"side":"blue","kind":"jeep","position":[30,61],"yaw":0}]),
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
    let setup = crate::common::scenario(
        r#"{"size":[160,120],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        json!([{"side":"blue","kind":"jeep","position":[25,61],"yaw":0}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let first = [65.0, 61.0];
    let last = [25.0, 61.0];
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                seq: 1,
                queued: false,
                order: move_to(&[0], first),
            })
            .error,
        None
    );
    let queued = battle.accept(CommandEnvelope {
        side: Side::Blue,
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
                .hull_box()
                .unwrap()
                .overlaps(&blocker.hull_box().unwrap()),
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
    let setup = crate::common::scenario(
        &json!({"size":[400,80],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "forests":[
            {"shape":{"kind":"polygon","ring":[[70,0],[130,0],[130,80],[70,80]]}},
            {"shape":{"kind":"polygon","ring":[[220,0],[280,0],[280,80],[220,80]]}}
        ]})
        .to_string(),
        json!([{"side":"blue","kind":"tank","position":[15,40],"yaw":0,
            "engagement":"return_fire_only"}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
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
        seq: 2,
        queued: false,
        order: move_to(&[0], [330.0, 40.0]),
    });
    assert_eq!(accepted.error, None);
    assert!(accepted.placement.unwrap().destinations[0].placed);
}

#[test]
fn visible_stationary_soldiers_block_a_move_out_of_a_closed_ring() {
    let mut rules = crate::common::scenario_rules();
    rules["physics"]["soldier_radius_m"] = json!(0.3);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "rifle",
        json!({"body":{"squad":{"slots":["rifleman"]}}}),
    );
    let mut units = vec![json!({"side":"blue","kind":"rifle","position":[60,60],
        "yaw":0,"engagement":"return_fire_only"})];
    for i in 0..8 {
        let angle = i as f64 * std::f64::consts::TAU / 8.0;
        units.push(json!({"side":"red","kind":"rifle",
            "position":[60.0 + 0.8 * angle.cos(),60.0 + 0.8 * angle.sin()],
            "yaw":0,"engagement":"return_fire_only"}));
    }
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[120,100],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]},
        "rules":rules,"units":units,"events":[],"scripts":[]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    assert_eq!(battle.tick(), 0);
    assert_eq!(
        battle.observe(Side::Blue).identified.len(),
        8,
        "every stationary blocker is currently visible"
    );
    let radius = battle.rules().physics.soldier_radius_m;
    for i in 0..9 {
        let a = battle.unit(UnitId(i)).unwrap().members[0].position.xy();
        for j in i + 1..9 {
            let b = battle.unit(UnitId(j)).unwrap().members[0].position.xy();
            assert!(
                (a - b).length() >= 2.0 * radius,
                "soldiers {i} and {j} must not initially overlap"
            );
        }
    }
    let request = contract::command::MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [100.0, 60.0],
        ..Default::default()
    };
    let preview = battle.preview_move(Side::Blue, &request).unwrap();
    assert!(
        !preview[0].placed,
        "visible soldier bodies close every exit"
    );
    let accepted = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        queued: false,
        order: move_to(&[0], request.goal),
    });
    assert_eq!(accepted.error, Some(OrderError::NoValidDestination));
    assert!(!accepted.placement.unwrap().destinations[0].placed);
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
                    .hull_box()
                    .unwrap()
                    .overlaps(&battle.unit(UnitId(j)).unwrap().hull_box().unwrap()));
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

#[test]
fn hidden_deployment_progress_cannot_change_a_partial_group_destination() {
    use contract::command::MovePreviewRequest;
    let mut rules = sim::fixtures::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "supply",
        json!({
            "capabilities": { "deploy": { "seconds": 40, "pack_seconds": 40 } }
        }),
    );
    rules["navigation"]["move_validation_work"] = json!(2500);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [800, 200], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "props": [
                {"kind":"wall", "center":[50,37], "yaw":0, "half_extents":[30,0.5,2]},
                {"kind":"wall", "center":[50,45], "yaw":0, "half_extents":[30,0.5,2]},
                {"kind":"wall", "center":[34,41], "yaw":0, "half_extents":[0.5,3.5,2]}
            ] },
        "rules": rules,
        "units": [
            {"side":"blue", "kind":"tank", "position":[40,41], "yaw":0, "engagement":"return_fire_only"},
            {"side":"blue", "kind":"tank", "position":[49,41], "yaw":0, "engagement":"return_fire_only"},
            {"side":"blue", "kind":"jeep", "position":[40,101], "yaw":0, "engagement":"return_fire_only"},
            {"side":"red", "kind":"supply", "position":[700,101], "yaw":0, "engagement":"return_fire_only"}
        ], "events": [], "scripts": []
    })).unwrap();
    let mut packed = Battle::new(&setup, 1);
    let mut deploying = Battle::new(&setup, 1);
    for i in 0..4 {
        let hull = packed.unit(UnitId(i)).unwrap().hull_box().unwrap();
        for j in i + 1..4 {
            assert!(!hull.overlaps(&packed.unit(UnitId(j)).unwrap().hull_box().unwrap()));
        }
        for prop in packed
            .world()
            .props()
            .filter(|p| p.blocks(contract::map::MoverClass::Vehicle))
        {
            assert!(!hull.overlaps(&prop.footprint()));
        }
    }
    assert_eq!(
        packed
            .accept(CommandEnvelope {
                side: Side::Red,
                seq: 1,
                queued: false,
                order: Order::SetDeployment {
                    units: vec![UnitId(3)],
                    deployed: false
                }
            })
            .error,
        None
    );
    packed.step();
    deploying.step();
    assert_eq!(packed.observe(Side::Blue), deploying.observe(Side::Blue));
    assert!(packed.observe(Side::Blue).identified.is_empty());
    let request = MovePreviewRequest {
        units: vec![UnitId(0), UnitId(2)],
        goal: [90.0, 71.0],
        route: RoutePolicy::Shortest,
        ..Default::default()
    };
    let packed_digest = packed.digest();
    let deploying_digest = deploying.digest();
    let baseline = packed.preview_move(Side::Blue, &request).unwrap();
    assert_eq!(packed.digest(), packed_digest);
    assert!(!baseline[0].placed, "the boxed member must remain held");
    assert!(
        baseline[1].placed,
        "the free member must have a destination"
    );
    let hidden_preview = deploying.preview_move(Side::Blue, &request).unwrap();
    assert_eq!(deploying.digest(), deploying_digest);
    assert_eq!(hidden_preview, baseline);
}

#[test]
fn hidden_enemy_spotting_cannot_move_visible_blockers_during_admission() {
    use contract::command::MovePreviewRequest;
    let mut rules = crate::common::scenario_rules();
    rules["physics"]["soldier_radius_m"] = json!(0.3);
    rules["cover"]["reresolve_s"] = json!(1.0 / 30.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "rifle",
        json!({
            "body":{"squad":{"slots":["rifleman"]}}, "sensors":{"ground_m":0.1}
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "at",
        json!({
            "body":{"squad":{"slots":["rifleman"]}}, "sensors":{"ground_m":100}
        }),
    );
    let setup = |scout_x: f64| -> ScenarioDefinition {
        let mut units = vec![json!({"side":"blue", "kind":"at", "position":[60,60],
            "yaw":0, "engagement":"return_fire_only"})];
        for i in 0..8 {
            let angle = i as f64 * std::f64::consts::TAU / 8.0;
            units.push(json!({"side":"red", "kind":"rifle",
                "position":[60.0 + 0.8 * angle.cos(),60.0 + 0.8 * angle.sin()],
                "yaw":0, "engagement":"return_fire_only"}));
        }
        units.push(
            json!({"side":"red", "kind":"recon", "position":[scout_x,180],
            "yaw":std::f64::consts::PI, "engagement":"return_fire_only"}),
        );
        serde_json::from_value(json!({
            "map":{"size":[1600,200], "fog_cell_m":8, "height_grid_m":4,
                "slope_cutoff_deg":35, "props":[
                    {"kind":"wall", "center":[66,60], "half_extents":[0.3,4,2], "yaw":0}
                ]},
            "rules":rules, "units":units, "events":[], "scripts":[]
        }))
        .unwrap()
    };
    let mut near_scout = Battle::new(&setup(200.0), 1);
    let mut far_scout = Battle::new(&setup(1450.0), 1);
    assert_eq!(
        near_scout.observe(Side::Blue),
        far_scout.observe(Side::Blue)
    );
    assert_eq!(near_scout.observe(Side::Blue).identified.len(), 8);
    assert_eq!(near_scout.observe(Side::Red).identified.len(), 1);
    assert!(far_scout.observe(Side::Red).identified.is_empty());
    for i in 0..9 {
        let p = near_scout.unit(UnitId(i)).unwrap().members[0].position.xy();
        for j in i + 1..9 {
            let q = near_scout.unit(UnitId(j)).unwrap().members[0].position.xy();
            assert!((p - q).length() > 0.6, "initial bodies must not overlap");
        }
        for prop in near_scout.world().props() {
            assert!(!prop.footprint().contains(p, 0.3));
        }
    }
    let request = MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [90.0, 60.0],
        ..Default::default()
    };
    let near_digest = near_scout.digest();
    let far_digest = far_scout.digest();
    let near_preview = near_scout.preview_move(Side::Blue, &request).unwrap();
    let far_preview = far_scout.preview_move(Side::Blue, &request).unwrap();
    assert_eq!(near_scout.digest(), near_digest);
    assert_eq!(far_scout.digest(), far_digest);
    assert!(
        !far_preview[0].placed,
        "the stationary ring closes the exit"
    );
    assert_eq!(near_preview, far_preview);
}

#[test]
fn a_member_whose_journey_outlasts_the_allowance_does_not_unplace_the_group() {
    // A jeep and a rifle squad ordered to one place: the jeep stands beside
    // it, the squad is 280 m off on foot. The allowance covers the jeep's
    // move and runs out during the squad's walk.
    let mut rules = crate::common::scenario_rules();
    rules["navigation"]["move_validation_work"] = json!(10000);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": {"size": [2000, 200], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "props": []},
        "rules": rules,
        "units": [
            {"side": "blue", "kind": "jeep", "position": [1900, 100], "yaw": 0},
            {"side": "blue", "kind": "rifle", "position": [1650, 100], "yaw": 0}
        ], "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: move_to(&[0, 1], [1930.0, 100.0]),
        queued: false,
    });
    let placed: Vec<_> = ack
        .placement
        .as_ref()
        .unwrap()
        .destinations
        .iter()
        .map(|d| (d.unit.0, d.placed))
        .collect();
    assert_eq!(ack.error, None, "the jeep's marker stands: {placed:?}");
    assert_eq!(placed, [(0, true), (1, false)]);
}
