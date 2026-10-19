use super::*;

#[test]
fn stance_falls_back_immediately_but_push_waits_for_dwell_and_timeout() {
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
            objectives: vec![],
            scores: [0.0; 2],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let objective = sites().objectives[0].clone();
    let plans = vec![PlayerPlan {
        player: PlayerIndex(0),
        lane: EntryIndex(0),
        main_objective: objective,
        staging_point: [100.0, 400.0],
    }];
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 1,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: vec![-2.0],
        }),
        stance: TeamStance::Push,
        ..TeamMemory::default()
    };
    let mut tuning = TeamTuning::shipped();
    tuning.stance_dwell_s = 2.0;
    tuning.push_timeout_s = 2.0;
    tuning.stance_push_influence = 0.0;
    frame.tick = 1;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::FallBack
    );
    memory.influence.as_mut().unwrap().values[0] = 1.0;
    frame.tick = 2;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::FallBack
    );
    frame.tick = 1_000;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Hold
    );
    frame.tick = 1_060;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Push
    );
}

#[test]
fn push_hysteresis_survives_one_bad_sample_and_repush_waits() {
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
            objectives: vec![],
            scores: [0.0, 0.0],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let plans = (0..3)
        .map(|index| PlayerPlan {
            player: PlayerIndex(index),
            lane: EntryIndex(index as usize),
            main_objective: sites().objectives[index as usize].clone(),
            staging_point: [100.0 + index as f64 * 100.0, 400.0],
        })
        .collect::<Vec<_>>();
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 1,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: vec![-0.3],
        }),
        previous_score: Some(1.0),
        stance: TeamStance::Push,
        ..TeamMemory::default()
    };
    let mut tuning = TeamTuning::shipped();
    tuning.stance_dwell_s = 2.0;
    tuning.push_timeout_s = 2.0;
    tuning.stance_push_influence = 0.0;
    tuning.stance_fallback_influence = -1.0;
    frame.tick = 30;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Push
    );
    memory.influence.as_mut().unwrap().values[0] = -0.6;
    frame.tick = 31;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Push
    );
    frame.tick = 60;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Hold
    );
    frame.tick = 90;
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Hold
    );
    frame.tick = 120;
    memory.previous_score = Some(1.0);
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Push
    );
}

#[test]
fn one_outgunned_lane_does_not_pull_three_lane_team_back() {
    let frame = ObservationFrame {
        skirmish: Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 0,
            max_units: 30,
            pending: vec![],
            objectives: vec![],
            scores: [0.0; 2],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let plans = (0..3)
        .map(|index| PlayerPlan {
            player: PlayerIndex(index),
            lane: EntryIndex(index as usize),
            main_objective: sites().objectives[index as usize].clone(),
            staging_point: [100.0 + index as f64 * 100.0, 400.0],
        })
        .collect::<Vec<_>>();
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 5,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: {
                let mut values = vec![1.0; 25];
                values[10] = -2.0;
                values
            },
        }),
        stance: TeamStance::Push,
        ..TeamMemory::default()
    };
    let tuning = TeamTuning::shipped();
    assert_eq!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            tuning,
            team_rules().tick_hz
        ),
        TeamStance::Push
    );
}

#[test]
fn score_deficit_does_not_turn_even_force_into_a_retreat() {
    let frame = ObservationFrame {
        skirmish: Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 0,
            max_units: 30,
            pending: vec![],
            objectives: vec![],
            scores: [0.0, 10.0],
            result: None,
            report: None,
        }),
        ..Default::default()
    };
    let objective = sites().objectives[0].clone();
    let plans = vec![PlayerPlan {
        player: PlayerIndex(0),
        lane: EntryIndex(0),
        main_objective: objective,
        staging_point: [100.0, 400.0],
    }];
    let mut memory = TeamMemory {
        previous_score: Some(0.0),
        influence: Some(InfluenceGrid {
            resolution: 1,
            min: [0.0, 0.0],
            max: [1_000.0, 1_000.0],
            values: vec![0.0],
        }),
        ..TeamMemory::default()
    };
    assert_ne!(
        update_stance(
            Side::Blue,
            &frame,
            &plans,
            &mut memory,
            TeamTuning::shipped(),
            30
        ),
        TeamStance::FallBack
    );
}
