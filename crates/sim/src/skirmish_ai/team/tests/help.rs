use super::*;

#[test]
fn help_uses_the_nearest_surplus_ally_and_holds_it_until_the_lane_is_relieved() {
    let rules: Rules = serde_json::from_value(crate::fixtures::test_game()).unwrap();
    let mut frame = ObservationFrame {
        skirmish: Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![
                contract::skirmish::PlayerView {
                    index: PlayerIndex(0),
                    id: "blue-1".into(),
                    controller: ControllerKind::ActiveAi,
                    ready: true,
                    credits: 0.0,
                    unit_count: 2,
                    cap: 30,
                    pending: vec![],
                },
                contract::skirmish::PlayerView {
                    index: PlayerIndex(1),
                    id: "blue-2".into(),
                    controller: ControllerKind::ActiveAi,
                    ready: true,
                    credits: 0.0,
                    unit_count: 10,
                    cap: 30,
                    pending: vec![],
                },
                contract::skirmish::PlayerView {
                    index: PlayerIndex(2),
                    id: "blue-3".into(),
                    controller: ControllerKind::ActiveAi,
                    ready: true,
                    credits: 0.0,
                    unit_count: 8,
                    cap: 30,
                    pending: vec![],
                },
            ],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 2,
            max_units: 30,
            pending: vec![],
            objectives: vec![],
            scores: [0.0; 2],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(&rules).unwrap(),
        "units":[
            {"side":"blue","kind":"test_tank","position":[250,400]},
            {"side":"blue","kind":"test_tank","position":[260,405]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Blue).own.clone();
    for unit in &mut frame.own {
        unit.owner = Some(PlayerIndex(1));
    }
    let objective = sites().objectives[0].clone();
    let lanes = vec![
        (
            PlayerIndex(0),
            EntryIndex(0),
            objective.clone(),
            [100.0, 400.0],
        ),
        (
            PlayerIndex(1),
            EntryIndex(1),
            sites().objectives[1].clone(),
            [300.0, 400.0],
        ),
        (
            PlayerIndex(2),
            EntryIndex(2),
            sites().objectives[2].clone(),
            [500.0, 400.0],
        ),
    ];
    let players = players();
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 1,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: vec![-2.0],
        }),
        previous_influence: vec![(objective.id.clone(), -1.0)].into_iter().collect(),
        ..TeamMemory::default()
    };
    let requests = update_help(
        Side::Blue,
        &frame,
        &players,
        &lanes,
        &mut memory,
        TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 1,
            distance_weight: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 1.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
            help_trend_threshold: 0.5,
            help_force_margin: 1.25,
            help_dwell_s: 10.0,
            stance_fallback_influence: -1.0,
            stance_push_influence: 0.1,
            stance_dwell_s: 20.0,
            push_timeout_s: 30.0,
        },
        &rules,
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].requester, PlayerIndex(0));
    assert_eq!(requests[0].answered_by, Some(PlayerIndex(1)));
    let repeat = update_help(
        Side::Blue,
        &frame,
        &players,
        &lanes,
        &mut memory,
        TeamTuning::shipped(),
        &rules,
    );
    assert_eq!(repeat[0].answered_by, Some(PlayerIndex(1)));

    // The pressure stops rising but the lane is still outgunned: the
    // detachment stays on its way, however long the trend is flat.
    let dwell = (TeamTuning::shipped().help_dwell_s * f64::from(rules.tick_hz)) as Tick;
    frame.tick += 3 * dwell;
    let flat = update_help(
        Side::Blue,
        &frame,
        &players,
        &lanes,
        &mut memory,
        TeamTuning::shipped(),
        &rules,
    );
    assert_eq!(
        flat.first().and_then(|request| request.answered_by),
        Some(PlayerIndex(1))
    );

    // The detachment is destroyed on the way while the lane is still
    // outgunned: nothing is reported as coming, and the call stands again.
    let fielded = std::mem::take(&mut frame.own);
    frame.tick += dwell;
    let lost = update_help(
        Side::Blue,
        &frame,
        &players,
        &lanes,
        &mut memory,
        TeamTuning::shipped(),
        &rules,
    );
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0].answered_by, None, "no one left to come: {lost:?}");
    assert!(memory.help_locks.is_empty());
    frame.own = fielded;

    // Relieved: no longer outgunned there, the request clears after its
    // dwell and the detachment goes back.
    memory.influence.as_mut().unwrap().values = vec![1.0];
    for _ in 0..3 {
        frame.tick += dwell;
        update_help(
            Side::Blue,
            &frame,
            &players,
            &lanes,
            &mut memory,
            TeamTuning::shipped(),
            &rules,
        );
    }
    assert!(memory.help_locks.is_empty());

    // A helper that has a force but no margin over its own lane is not
    // eligible: the request stays visible as unanswered.
    frame.own.truncate(1);
    frame.skirmish.as_mut().unwrap().players[1].unit_count = 1;
    memory.previous_influence.insert(objective.id.clone(), -1.0);
    frame.tick += dwell;
    let no_surplus = update_help(
        Side::Blue,
        &frame,
        &players,
        &lanes,
        &mut memory,
        TeamTuning::shipped(),
        &rules,
    );
    assert_eq!(
        no_surplus
            .iter()
            .find(|request| request.requester == PlayerIndex(0))
            .and_then(|request| request.answered_by),
        None
    );
}

#[test]
fn help_detachment_uses_weapon_target_classes_for_a_ground_threat() {
    let rules: Rules = serde_json::from_value(crate::fixtures::game()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(&rules).unwrap(),
        "units":[
            {"side":"blue","kind":"us_patriot_long","position":[250,400]},
            {"side":"blue","kind":"us_m1_abrams_sep_v2","position":[260,405]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    for unit in &mut frame.own {
        unit.owner = Some(PlayerIndex(1));
    }
    let target = rules.catalog.index("europe_leopard_2_2a6").unwrap();
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(1),
            kind: target,
            cost: 500,
            position: [250.0, 400.0, 0.0],
            yaw: 0.0,
            velocity: [0.0, 0.0],
            members: vec![],
            member_ids: vec![],
            member_slots: vec![],
            member_active_mounts: vec![],
            weapon_poses: vec![],
            reversing: false,
            smoking: false,
            member_leans: vec![],
        });
    let objective = sites().objectives[0].clone();
    let candidates =
        capable_detachment(&rules, &frame, PlayerIndex(1), &objective, &BTreeSet::new());
    assert_eq!(
        candidates.len(),
        1,
        "air defence must not answer a ground threat"
    );
    assert_eq!(candidates[0].1, frame.own[1].id);
}

#[test]
fn a_detachment_holds_ground_not_just_flies_over_it() {
    // The nearest unit able to fight is a helicopter, which cannot hold
    // an objective; a one-unit detachment is the tank behind it.
    let rules: Rules = serde_json::from_value(crate::fixtures::game()).unwrap();
    let objective = sites().objectives[0].clone();
    let at = objective.center;
    let setup: contract::scenario::ScenarioDefinition =
            serde_json::from_value(serde_json::json!({
                "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
                "rules":serde_json::to_value(&rules).unwrap(),
                "units":[
                    {"side":"blue","kind":"us_ah_6_mh_6_little_bird_mh_6m","position":[at[0] + 100.0, at[1]]},
                    {"side":"blue","kind":"us_m1_abrams_sep_v2","position":[at[0] + 300.0, at[1]]}
                ],"events":[],"scripts":[]
            }))
            .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    for unit in &mut frame.own {
        unit.owner = Some(PlayerIndex(1));
    }
    let tank = frame
        .own
        .iter()
        .find(|unit| crate::objectives::can_hold(rules.catalog.get(unit.kind)))
        .unwrap()
        .id;
    let capable = capable_detachment(&rules, &frame, PlayerIndex(1), &objective, &BTreeSet::new());
    assert_eq!(capable.len(), 2, "both can fight on the ground");
    assert_eq!(detachment(&rules, &frame, capable, 1), vec![tank]);
}

#[test]
fn help_detachments_are_disjoint_from_locked_units() {
    let rules: Rules = serde_json::from_value(crate::fixtures::test_game()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(&rules).unwrap(),
        "units":[
            {"side":"blue","kind":"test_tank","position":[250,400]},
            {"side":"blue","kind":"test_tank","position":[260,405]},
            {"side":"blue","kind":"test_tank","position":[270,410]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    for unit in &mut frame.own {
        unit.owner = Some(PlayerIndex(1));
    }
    let objective = sites().objectives[0].clone();
    let first = detachment(
        &rules,
        &frame,
        capable_detachment(&rules, &frame, PlayerIndex(1), &objective, &BTreeSet::new()),
        1,
    );
    let excluded = first.iter().copied().collect::<BTreeSet<_>>();
    let second = detachment(
        &rules,
        &frame,
        capable_detachment(&rules, &frame, PlayerIndex(1), &objective, &excluded),
        1,
    );
    assert!(!first.iter().any(|id| second.contains(id)));
}

#[test]
fn an_engaged_locked_detachment_is_not_lent_to_a_second_request() {
    let rules: Rules = serde_json::from_value(crate::fixtures::test_game()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(&rules).unwrap(),
        "units":[
            {"side":"blue","kind":"test_tank","position":[250,400]},
            {"side":"blue","kind":"test_tank","position":[260,405]},
            {"side":"blue","kind":"test_tank","position":[270,410]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    for unit in &mut frame.own {
        unit.owner = Some(PlayerIndex(1));
    }
    frame.own[0].engagement = contract::command::Engagement::FireAtWill;
    frame.own[0]
        .sees
        .push(contract::observation::ObservedTargetId(1));
    frame.skirmish = Some(contract::skirmish::SkirmishView {
        phase: contract::skirmish::Phase::Active,
        players: vec![
            contract::skirmish::PlayerView {
                index: PlayerIndex(0),
                id: "blue-1".into(),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: 0,
                cap: 30,
                pending: vec![],
            },
            contract::skirmish::PlayerView {
                index: PlayerIndex(1),
                id: "blue-2".into(),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: 3,
                cap: 30,
                pending: vec![],
            },
            contract::skirmish::PlayerView {
                index: PlayerIndex(2),
                id: "blue-3".into(),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: 0,
                cap: 30,
                pending: vec![],
            },
        ],
        ready: [true, false],
        preparation_remaining_s: 0.0,
        credits: 0.0,
        unit_count: 3,
        max_units: 30,
        pending: vec![],
        objectives: vec![],
        scores: [0.0; 2],
        result: None,
        report: None,
    });
    let sites = sites();
    let locked = frame.own[0].id;
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 1,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: vec![-2.0],
        }),
        previous_influence: sites
            .objectives
            .iter()
            .map(|objective| (objective.id.clone(), -1.0))
            .collect(),
        help_locks: [(
            PlayerIndex(0),
            HelpLock {
                answered_by: PlayerIndex(1),
                detachment: vec![locked],
                clear_since: None,
                raised_tick: 1,
            },
        )]
        .into_iter()
        .collect(),
        ..TeamMemory::default()
    };
    let lanes = vec![
        (
            PlayerIndex(0),
            EntryIndex(0),
            sites.objectives[0].clone(),
            [100.0, 400.0],
        ),
        (
            PlayerIndex(2),
            EntryIndex(2),
            sites.objectives[1].clone(),
            [500.0, 400.0],
        ),
    ];
    let requests = update_help(
        Side::Blue,
        &frame,
        &players(),
        &lanes,
        &mut memory,
        TeamTuning::shipped(),
        &rules,
    );
    let first = requests
        .iter()
        .find(|request| request.requester == PlayerIndex(0))
        .expect("the engaged lock remains");
    let second = requests
        .iter()
        .find(|request| request.requester == PlayerIndex(2))
        .expect("the second lane still asks");
    assert_eq!(first.answered_by, Some(PlayerIndex(1)));
    assert_eq!(second.answered_by, Some(PlayerIndex(1)));
    assert!(!second.detachment.contains(&locked));
}

#[test]
fn a_battle_observation_routes_help_to_a_human_lane_and_keeps_the_ally_lane() {
    let rules: Rules = serde_json::from_value(crate::fixtures::test_game()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(&rules).unwrap(),
        "units":[
            {"side":"blue","kind":"test_tank","position":[100,400]},
            {"side":"blue","kind":"test_tank","position":[300,400]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let sites = sites();
    let objective = sites.objectives[0].clone();
    let mut frame = ObservationFrame {
        skirmish: Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![
                contract::skirmish::PlayerView {
                    index: PlayerIndex(0),
                    id: "blue-1".into(),
                    controller: ControllerKind::Human,
                    ready: true,
                    credits: 0.0,
                    unit_count: 1,
                    cap: 30,
                    pending: vec![],
                },
                contract::skirmish::PlayerView {
                    index: PlayerIndex(1),
                    id: "blue-2".into(),
                    controller: ControllerKind::ActiveAi,
                    ready: true,
                    credits: 0.0,
                    unit_count: 3,
                    cap: 30,
                    pending: vec![],
                },
            ],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 2,
            max_units: 30,
            pending: vec![],
            objectives: sites
                .objectives
                .iter()
                .map(|o| contract::skirmish::ObjectiveView {
                    id: o.id.clone(),
                    center: o.center,
                    radius_m: o.radius_m,
                    owner: None,
                    capturing: None,
                    capture_progress: 0.0,
                    contested: false,
                })
                .collect(),
            scores: [0.0; 2],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    frame.own = battle.observe(Side::Blue).own.clone();
    frame.own[0].owner = Some(PlayerIndex(0));
    frame.own[0].goal = Some(objective.center);
    frame.own[1].owner = Some(PlayerIndex(1));
    let players = vec![
        Player {
            id: "blue-1".into(),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: ControllerKind::Human,
            human_capable: true,
        },
        Player {
            id: "blue-2".into(),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: ControllerKind::ActiveAi,
            human_capable: false,
        },
    ];
    let mut memory = TeamMemory::default();
    let mut tuning = TeamTuning::shipped();
    tuning.help_trend_threshold = 0.1;
    plan(
        Side::Blue,
        &rules,
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    frame.tick = 300;
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(1),
            kind: rules.catalog.index("test_tank").unwrap(),
            cost: 100,
            position: [objective.center[0], objective.center[1], 0.0],
            yaw: 0.0,
            velocity: [0.0, 0.0],
            members: vec![],
            member_ids: vec![],
            member_slots: vec![],
            member_active_mounts: vec![],
            weapon_poses: vec![],
            reversing: false,
            smoking: false,
            member_leans: vec![],
        });
    let plan = plan(
        Side::Blue,
        &rules,
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    let request = plan
        .help_requests
        .iter()
        .find(|request| request.requester == PlayerIndex(0))
        .expect("human lane raised help");
    assert_eq!(request.answered_by, Some(PlayerIndex(1)));
    assert!(!request.detachment.is_empty());
    let assignment = plan.assignment_for(PlayerIndex(1)).unwrap();
    assert_eq!(assignment.help.len(), 1);
    assert!(assignment.help[0]
        .detachment
        .iter()
        .all(|id| request.detachment.contains(id)));
    assert!(
        !assignment.force_advance,
        "a help task must not attack-move the helper's whole lane"
    );
    assert_ne!(assignment.main_objective, objective.center);
}

#[test]
fn a_human_without_an_order_or_objective_position_gets_no_guessed_lane() {
    let rules = team_rules();
    let sites = sites();
    let mut players = players();
    players[0].controller = ControllerKind::Human;
    let mut frame = ObservationFrame {
        skirmish: Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 0,
            max_units: 30,
            pending: vec![],
            objectives: sites
                .objectives
                .iter()
                .map(|objective| contract::skirmish::ObjectiveView {
                    id: objective.id.clone(),
                    center: objective.center,
                    radius_m: objective.radius_m,
                    owner: None,
                    capturing: None,
                    capture_progress: 0.0,
                    contested: false,
                })
                .collect(),
            scores: [0.0; 2],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let mut memory = TeamMemory::default();
    let first_plan = plan(
        Side::Blue,
        &rules,
        &frame,
        &players,
        &sites,
        &mut memory,
        TeamTuning::shipped(),
    );
    assert!(first_plan
        .help_requests
        .iter()
        .all(|request| request.requester != PlayerIndex(0)));
    assert!(first_plan.for_player(PlayerIndex(0)).is_none());

    // A later planner tick with no units still must not synthesize a lane
    // from the human's seat index.
    frame.tick = 1_000;
    let second_plan = plan(
        Side::Blue,
        &rules,
        &frame,
        &players,
        &sites,
        &mut memory,
        TeamTuning::shipped(),
    );
    assert!(second_plan
        .help_requests
        .iter()
        .all(|request| request.requester != PlayerIndex(0)));
}
