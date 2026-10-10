//! Group destinations reserve settled footprints and rotate about their front.
use contract::ids::UnitId;
use contract::scenario::FormationRules;
use sim::formation::{self, Member};
use sim::math::v2;

#[test]
fn queued_destinations_do_not_assume_an_unfinished_attack_will_end() {
    use contract::command::{CommandEnvelope, Order, TargetRef};
    use contract::ids::Side;
    use sim::battle::Battle;
    let setup = crate::common::scenario(
        r#"{"size":[300,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        serde_json::json!([{"side":"blue","kind":"test_jeep","position":[30,100]}]),
        serde_json::json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    assert_eq!(
        battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                seq: 1,
                queued: false,
                order: Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Ground {
                        point: [150.0, 100.0, 0.0]
                    }
                },
            })
            .error,
        None
    );
    let preview = battle
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0)],
                goal: [200.0, 100.0],
                queued: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        !preview[0].placed,
        "an indefinite attack has no terminal pose for a queued move"
    );
}

#[test]
fn a_free_destination_across_an_unbridged_river_is_not_validated() {
    use contract::command::MoveDirection;
    use contract::ids::Side;
    use sim::battle::Battle;
    let setup = crate::common::scenario(
        &serde_json::json!({
            "size": [300, 200], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35,
            "rivers": [{ "points": [
                { "xy": [150, 0], "width_m": 20, "depth_m": 1.5 },
                { "xy": [150, 200], "width_m": 20, "depth_m": 1.5 }
            ], "surface_z": -0.5 }]
        })
        .to_string(),
        serde_json::json!([{ "side": "blue", "kind": "test_jeep", "position": [30, 100], "yaw": 0 }]),
        serde_json::json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let destinations = b
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0)],
                goal: [250.0, 100.0],
                facing: None,
                direction: MoveDirection::Forward,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        !destinations[0].placed,
        "standing room across water is not an executable move"
    );
}

#[test]
fn crowded_mixed_groups_place_every_unit_without_overlapping() {
    let members: Vec<_> = (0..100)
        .map(|id| Member {
            id: UnitId(id),
            position: v2(20.0, 20.0),
            radius: if id % 2 == 0 { 14.0 } else { 4.0 },
            yaw: 0.0,
        })
        .collect();
    let rules = FormationRules::default();
    let plan = formation::place(
        &members,
        v2(500.0, 500.0),
        Some(0.0),
        &rules,
        2000.0,
        |_, p| (p.x >= 0.0 && p.y >= 0.0 && p.x <= 1000.0 && p.y <= 1000.0).then_some(p),
    );
    assert!(
        plan.slots.iter().all(|s| s.point.is_some()),
        "an open field fits the full group: failed {:?}, checks {}, extent {}",
        plan.slots
            .iter()
            .filter(|s| s.point.is_none())
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        plan.checks,
        plan.radius_m
    );
    assert_eq!(plan.slots.len(), 100);
    for (i, a) in plan.slots.iter().enumerate() {
        for b in &plan.slots[i + 1..] {
            let radius =
                members[a.id.0 as usize].radius + members[b.id.0 as usize].radius + rules.spacing_m;
            assert!(
                (a.point.unwrap() - b.point.unwrap()).length() + 1e-9 >= radius,
                "{} overlaps {}",
                a.id.0,
                b.id.0
            );
        }
    }
    assert!(plan.radius_m > 40.0);
    assert!(plan.checks <= members.len() + rules.candidate_checks as usize);
}

#[test]
fn an_obstructed_destination_expands_past_the_old_snap_reach() {
    let members = [Member {
        id: UnitId(0),
        position: v2(20.0, 20.0),
        radius: 4.0,
        yaw: 0.0,
    }];
    let plan = formation::place(
        &members,
        v2(500.0, 500.0),
        None,
        &FormationRules::default(),
        1500.0,
        |_, p| (p.x < 450.0 && p.x >= 0.0 && p.y >= 0.0 && p.y <= 1000.0).then_some(p),
    );
    let point = plan.slots[0]
        .point
        .expect("the bank behind the obstruction has room");
    assert!(point.x < 450.0);
    assert!(
        (point - v2(500.0, 500.0)).length() < 100.0,
        "nearby usable bank, not a distant scan artifact: {point:?}"
    );
    assert!(plan.radius_m > 40.0);
}

#[test]
fn a_failed_member_does_not_discard_the_members_that_fit() {
    let members = [
        Member {
            id: UnitId(7),
            position: v2(20.0, 20.0),
            radius: 4.0,
            yaw: 0.0,
        },
        Member {
            id: UnitId(9),
            position: v2(20.0, 40.0),
            radius: 4.0,
            yaw: 0.0,
        },
    ];
    let plan = formation::place(
        &members,
        v2(500.0, 500.0),
        None,
        &FormationRules::default(),
        1500.0,
        |id, p| (id == UnitId(7)).then_some(p),
    );
    assert_eq!(plan.slots[0].point, Some(v2(496.0, 490.0)));
    assert_eq!(plan.slots[1].id, UnitId(9));
    assert_eq!(plan.slots[1].point, None);
    assert!(plan.checks <= members.len() + FormationRules::default().candidate_checks as usize);
}

#[test]
fn a_partial_order_moves_the_units_that_fit_and_holds_the_others() {
    use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
    use contract::ids::Side;
    use sim::battle::Battle;
    let map = serde_json::json!({"size": [300, 300], "fog_cell_m": 8,
        "height_grid_m": 4, "slope_cutoff_deg": 35, "props": []});
    let mut setup = crate::common::scenario(
        &map.to_string(),
        serde_json::json!([
            {"side": "blue", "kind": "test_tank", "position": [30, 100]},
            {"side": "blue", "kind": "test_rifle", "position": [260, 260]}
        ]),
        serde_json::json!([]),
    );
    let mut rules = crate::common::scenario_rules();
    // Its hull is past what the game admits.
    sim::fixtures::lift_hull_limits(&mut rules);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        serde_json::json!({
            "body": {"hull": {"half_extents_m": [2, 200, 2]}}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let mut battle = Battle::new(&setup, 1);
    let order = Order::Move {
        units: vec![UnitId(0), UnitId(1)],
        gesture: 3,
        goal: [290.0, 280.0],
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    };
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None);
    let accepted = ack.placement.unwrap().destinations;
    assert!(
        !accepted[0].placed,
        "the oversized tank cannot stand anywhere on this map"
    );
    assert!(accepted[1].placed, "the squad still gets a destination");
    assert_eq!(
        accepted[0].goal,
        [30.0, 100.0],
        "an unplaced unit previews holding its position"
    );
    assert_eq!(accepted[0].facing, 0.0);
    battle.step();
    let observed = battle.observe(Side::Blue);
    assert_eq!(observed.own[0].goal, None);
    assert_eq!(observed.own[1].goal, Some(accepted[1].goal));
    for _ in 0..600 * setup.rules.tick_hz {
        battle.step();
        if battle.observe(Side::Blue).own[1].goal.is_none() {
            break;
        }
    }
    assert_eq!(battle.observe(Side::Blue).own[1].goal, None);
}

#[test]
fn preview_queries_cannot_change_later_navigation_or_replay() {
    use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
    use contract::ids::Side;
    use sim::battle::Battle;
    let setup = crate::common::scenario(
        crate::common::saved_map("geometry"),
        serde_json::json!([
            {"side": "blue", "kind": "test_tank", "position": [30, 150]}
        ]),
        serde_json::json!([]),
    );
    let mut queried = Battle::new(&setup, 1);
    let mut quiet = Battle::new(&setup, 1);
    let command = CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [270.0, 150.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    };
    for _ in 0..3 {
        queried
            .preview_move(
                Side::Blue,
                &contract::command::MovePreviewRequest {
                    units: vec![UnitId(0)],
                    goal: [170.0, 160.0],
                    facing: None,
                    direction: MoveDirection::Forward,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    assert_eq!(queried.accept(command.clone()), quiet.accept(command));
    for _ in 0..500 {
        queried
            .preview_move(
                Side::Blue,
                &contract::command::MovePreviewRequest {
                    units: vec![UnitId(0)],
                    goal: [170.0, 160.0],
                    facing: None,
                    direction: MoveDirection::Forward,
                    ..Default::default()
                },
            )
            .unwrap();
        queried.step();
        quiet.step();
        assert_eq!(queried.digest(), quiet.digest());
    }
    let mut replay = Battle::from_replay(&setup, &queried.replay()).unwrap();
    for _ in 0..500 {
        replay.step();
    }
    assert_eq!(replay.digest(), queried.digest());
}

#[test]
fn partial_placement_keeps_moving_destinations_clear_of_units_that_hold() {
    let members = [
        Member {
            id: UnitId(7),
            position: v2(500.0, 500.0),
            radius: 4.0,
            yaw: 0.0,
        },
        Member {
            id: UnitId(9),
            position: v2(500.0, 510.0),
            radius: 4.0,
            yaw: 0.0,
        },
    ];
    let rules = FormationRules::default();
    let plan = formation::place(&members, v2(504.0, 515.0), None, &rules, 1500.0, |id, p| {
        (id == UnitId(7)).then_some(p)
    });
    let goal = plan.slots[0]
        .point
        .expect("there is room for the unit that can move");
    assert_eq!(plan.slots[1].point, None);
    assert!(
        (goal - members[1].position).length() >= 9.0,
        "the other unit holds, so its footprint is still occupied: {goal:?}"
    );
    assert!(plan.checks <= members.len() + rules.candidate_checks as usize);
}

#[test]
fn a_previewed_squad_spreads_each_living_soldier_round_its_goal_and_a_hull_has_none() {
    use contract::ids::Side;
    use sim::battle::Battle;
    let setup = crate::common::scenario(
        r#"{"size":[300,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        serde_json::json!([
            {"side":"blue","kind":"test_rifle","position":[30,60]},
            {"side":"blue","kind":"test_jeep","position":[30,140]}
        ]),
        serde_json::json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let living = battle.observe(Side::Blue).own[0].members.len();
    let preview = battle
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0), UnitId(1)],
                goal: [200.0, 100.0],
                ..Default::default()
            },
        )
        .unwrap();
    let squad = preview.iter().find(|d| d.unit == UnitId(0)).unwrap();
    assert!(living > 1);
    assert_eq!(squad.spots.len(), living, "one spot per living soldier");
    for spot in &squad.spots {
        let off = (spot[0] - squad.goal[0]).hypot(spot[1] - squad.goal[1]);
        assert!(
            off < 15.0,
            "a soldier stands round the squad's goal, {off} m off"
        );
    }
    let distinct: std::collections::BTreeSet<_> = squad
        .spots
        .iter()
        .map(|s| ((s[0] * 10.0) as i64, (s[1] * 10.0) as i64))
        .collect();
    assert_eq!(distinct.len(), living, "soldiers spread out, not stacked");
    let hull = preview.iter().find(|d| d.unit == UnitId(1)).unwrap();
    assert!(hull.spots.is_empty(), "a vehicle stands as one body");
}

#[test]
fn a_held_squad_preview_keeps_its_spots_as_time_passes_and_the_order_sends_soldiers_there() {
    use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
    use contract::ids::Side;
    use sim::battle::Battle;
    let setup = crate::common::scenario(
        r#"{"size":[300,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}"#,
        serde_json::json!([{"side":"blue","kind":"test_rifle","position":[30,60]}]),
        serde_json::json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let request = contract::command::MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [200.0, 100.0],
        ..Default::default()
    };
    let first = battle.preview_move(Side::Blue, &request).unwrap();
    for _ in 0..5 {
        battle.step();
    }
    let later = battle.preview_move(Side::Blue, &request).unwrap();
    assert_eq!(
        later[0].spots, first[0].spots,
        "the same move shows the same soldier spots on every tick it is held"
    );
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: request.goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    while battle.tick() < ack.applied_tick + 1 {
        battle.step();
    }
    // Open ground has no cover to draw a soldier off his drawn spot.
    let sent: Vec<_> = battle.observe(Side::Blue).own[0]
        .member_orders
        .iter()
        .map(|m| m.spot)
        .collect();
    assert_eq!(
        sent, first[0].spots,
        "the ghost shows where each soldier goes"
    );
}
