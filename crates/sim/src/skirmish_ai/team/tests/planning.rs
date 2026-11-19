use super::*;

/// Blue 1's home is the high-x road (entry 2, x 500); its one objective
/// sits beside the x 100 road (entry 0), 140 m closer than from home.
fn blue_one_lane(tick_s: f64, players: &[PlayerState]) -> (PlayerIndex, EntryIndex) {
    let mut sites = sites();
    sites.objectives.retain(|objective| objective.id == "west");
    let rules = team_rules();
    let frame = ObservationFrame {
        tick: (tick_s * f64::from(rules.tick_hz)) as Tick,
        ..ObservationFrame::default()
    };
    let tuning = TeamTuning {
        home_lane_bonus: 400.0,
        home_lane_fade_s: 300.0,
        ..TeamTuning::shipped()
    };
    let mut memory = TeamMemory::default();
    let plan = plan(
        Side::Blue,
        &rules,
        &frame,
        players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_eq!(plan.players.len(), 1);
    (plan.players[0].player, plan.players[0].lane)
}

#[test]
fn an_ai_holds_its_own_lane_at_the_start_and_goes_where_needed_as_the_pull_fades() {
    let mut blue = players();
    blue.truncate(3);
    // The pull starts well past the 140 m it costs, and is still past it
    // halfway through its fade.
    assert_eq!(blue_one_lane(0.0, &blue), (PlayerIndex(0), EntryIndex(2)));
    assert_eq!(blue_one_lane(150.0, &blue), (PlayerIndex(0), EntryIndex(2)));
    // Faded below it, the needier road wins.
    assert_eq!(blue_one_lane(280.0, &blue), (PlayerIndex(0), EntryIndex(0)));
    assert_eq!(blue_one_lane(600.0, &blue), (PlayerIndex(0), EntryIndex(0)));
}

#[test]
fn a_slot_past_the_sides_lanes_feels_no_pull() {
    // Three humans hold Blue's three lanes; the AI in slot 4 has no home
    // and takes the nearest road at once.
    let mut blue = players();
    for player in blue.iter_mut().take(3) {
        player.controller = ControllerKind::Human;
    }
    blue[3].side = Side::Blue;
    assert_eq!(blue_one_lane(0.0, &blue), (PlayerIndex(3), EntryIndex(0)));
}

#[test]
fn three_ai_players_get_distinct_lanes_and_objectives() {
    let players = players();
    let sites = sites();
    let mut memory = TeamMemory::default();
    let plan = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            home_lane_bonus: 0.0,
            home_lane_fade_s: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
            help_trend_threshold: 0.5,
            help_force_margin: 1.25,
            help_dwell_s: 10.0,
            stance_fallback_influence: -0.5,
            stance_push_influence: 0.5,
            stance_dwell_s: 20.0,
            push_timeout_s: 30.0,
            open_objective_bonus: 100.0,
            party_budget_share: 0.25,
            claim_party_size: 2,
            hold_party_size: 2,
        },
    );
    assert_eq!(plan.players.len(), 3);
    assert_eq!(
        plan.players
            .iter()
            .map(|player| player.lane)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        plan.players
            .iter()
            .map(|player| player.main_objective.id.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
}

#[test]
fn a_large_team_plan_uses_bounded_assignment_work() {
    let mut players = players();
    players.truncate(3);
    for index in 3..10 {
        let player = Player {
            id: format!("blue-{}", index + 1),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: ControllerKind::ActiveAi,
            human_capable: false,
        };
        players.push(PlayerState::new(&player, 0));
    }
    let mut sites = sites();
    for index in 3..7 {
        let mut objective = sites.objectives[index % 3].clone();
        objective.id = format!("extra-{index}");
        objective.center[0] += 25.0 * index as f64;
        sites.objectives.push(objective);
    }
    let mut memory = TeamMemory::default();
    let plan = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            home_lane_bonus: 0.0,
            home_lane_fade_s: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
            help_trend_threshold: 0.5,
            help_force_margin: 1.25,
            help_dwell_s: 10.0,
            stance_fallback_influence: -0.5,
            stance_push_influence: 0.5,
            stance_dwell_s: 20.0,
            push_timeout_s: 30.0,
            open_objective_bonus: 100.0,
            party_budget_share: 0.25,
            claim_party_size: 2,
            hold_party_size: 2,
        },
    );
    assert_eq!(plan.players.len(), 7);
    assert!(
        memory.planning_work < 100_000,
        "work={}",
        memory.planning_work
    );
}

#[test]
fn three_ai_players_reuse_two_lanes() {
    let players = players();
    let mut sites = sites();
    sites.entries.retain(|entry| {
        entry.side == Side::Red && entry.center[0] < 400.0
            || entry.side == Side::Blue && entry.center[0] < 400.0
    });
    let mut memory = TeamMemory::default();
    let plan = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            home_lane_bonus: 0.0,
            home_lane_fade_s: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
            help_trend_threshold: 0.5,
            help_force_margin: 1.25,
            help_dwell_s: 10.0,
            stance_fallback_influence: -0.5,
            stance_push_influence: 0.5,
            stance_dwell_s: 20.0,
            push_timeout_s: 30.0,
            open_objective_bonus: 100.0,
            party_budget_share: 0.25,
            claim_party_size: 2,
            hold_party_size: 2,
        },
    );
    assert_eq!(plan.players.len(), 3);
}

/// One AI beside a human on a three-lane side. Its only open objective
/// lies in front of the third lane, so that is the lane it plans for: the
/// first lane (the centre entry) is a candidate like any other, never the
/// only one.
#[test]
fn a_lone_ai_takes_the_lane_nearest_its_objective_not_the_first() {
    let mut players = players();
    players[0].controller = ControllerKind::Human;
    players[2].controller = ControllerKind::Human;
    let mut sites = sites();
    sites.objectives.retain(|objective| objective.id == "east");
    let mut memory = TeamMemory::default();
    let plan = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            home_lane_bonus: 0.0,
            home_lane_fade_s: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
            help_trend_threshold: 0.5,
            help_force_margin: 1.25,
            help_dwell_s: 10.0,
            stance_fallback_influence: -0.5,
            stance_push_influence: 0.5,
            stance_dwell_s: 20.0,
            push_timeout_s: 30.0,
            open_objective_bonus: 100.0,
            party_budget_share: 0.25,
            claim_party_size: 2,
            hold_party_size: 2,
        },
    );
    assert_eq!(plan.players.len(), 1);
    assert_eq!(plan.players[0].player, PlayerIndex(1));
    assert_eq!(plan.players[0].main_objective.id, "east");
    assert_eq!(
        plan.players[0].lane,
        EntryIndex(2),
        "the lane in front of its objective"
    );
}

#[test]
fn planning_is_repeatable_and_incumbency_survives_a_marginal_score_change() {
    let players = players();
    let sites = sites();
    let mut memory = TeamMemory::default();
    let tuning = TeamTuning {
        planner_decision_s: 5.0,
        grid_resolution: 4,
        distance_weight: 1.0,
        home_lane_bonus: 0.0,
        home_lane_fade_s: 1.0,
        influence_weight: 1.0,
        incumbency_bonus: 80.0,
        group_radius_m: 90.0,
        staging_distance_m: 100.0,
        influence_blend: 0.5,
        help_trend_threshold: 0.5,
        help_force_margin: 1.25,
        help_dwell_s: 10.0,
        stance_fallback_influence: -0.5,
        stance_push_influence: 0.5,
        stance_dwell_s: 20.0,
        push_timeout_s: 30.0,
        open_objective_bonus: 100.0,
        party_budget_share: 0.25,
        claim_party_size: 2,
        hold_party_size: 2,
    };
    let first = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    let first = first.clone();
    let second = plan(
        Side::Blue,
        &team_rules(),
        &ObservationFrame::default(),
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_eq!(first, second.clone());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn a_held_objective_replans_to_one_the_side_does_not_hold_immediately() {
    let players = players();
    let mut sites = sites();
    sites.objectives.truncate(3);
    let mut memory = TeamMemory::default();
    let tuning = TeamTuning {
        planner_decision_s: 5.0,
        grid_resolution: 4,
        distance_weight: 1.0,
        home_lane_bonus: 0.0,
        home_lane_fade_s: 1.0,
        influence_weight: 1.0,
        incumbency_bonus: 80.0,
        group_radius_m: 90.0,
        staging_distance_m: 100.0,
        influence_blend: 0.5,
        help_trend_threshold: 0.5,
        help_force_margin: 1.25,
        help_dwell_s: 10.0,
        stance_fallback_influence: -0.5,
        stance_push_influence: 0.5,
        stance_dwell_s: 20.0,
        push_timeout_s: 30.0,
        open_objective_bonus: 100.0,
        party_budget_share: 0.25,
        claim_party_size: 2,
        hold_party_size: 2,
    };
    let mut frame = ObservationFrame::default();
    frame.skirmish = Some(contract::skirmish::SkirmishView {
        phase: contract::skirmish::Phase::Active,
        players: vec![contract::skirmish::PlayerView {
            index: PlayerIndex(0),
            id: "blue-1".into(),
            controller: ControllerKind::ActiveAi,
            ready: true,
            credits: 0.0,
            income_per_minute: 0.0,
            unit_count: 0,
            cap: 30,
            pending: vec![],
            offered: None,
        }],
        preparation_remaining_s: 0.0,
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
    });
    let first = plan(
        Side::Blue,
        &team_rules(),
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    let first_id = first.players[0].main_objective.id.clone();
    let first_plan = first.clone();
    frame
        .skirmish
        .as_mut()
        .unwrap()
        .objectives
        .iter_mut()
        .find(|objective| objective.id == first_id)
        .unwrap()
        .contested = true;
    frame.tick = 1;
    let flicker = plan(
        Side::Blue,
        &team_rules(),
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_eq!(flicker, &first_plan);
    frame
        .skirmish
        .as_mut()
        .unwrap()
        .objectives
        .iter_mut()
        .find(|objective| objective.id == first_id)
        .unwrap()
        .contested = false;
    frame.tick = 2;
    let stable = plan(
        Side::Blue,
        &team_rules(),
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_eq!(stable, &first_plan);
    let view = frame.skirmish.as_mut().unwrap();
    view.objectives
        .iter_mut()
        .find(|objective| objective.id == first_id)
        .unwrap()
        .owner = Some(Side::Blue);
    frame.tick = 1;
    let second = plan(
        Side::Blue,
        &team_rules(),
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_ne!(second.players[0].main_objective.id, first_id);

    // With nothing neutral left, the enemy's objectives are still the ones
    // to take: the AI does not settle back on the one it holds.
    // Blue now holds its current assignment, the enemy holds the rest.
    let second_id = second.players[0].main_objective.id.clone();
    for objective in &mut frame.skirmish.as_mut().unwrap().objectives {
        objective.owner = Some(if objective.id == second_id {
            Side::Blue
        } else {
            Side::Red
        });
    }
    frame.tick = 2;
    let third = plan(
        Side::Blue,
        &team_rules(),
        &frame,
        &players,
        &sites,
        &mut memory,
        tuning,
    );
    assert_ne!(third.players[0].main_objective.id, second_id);
}
