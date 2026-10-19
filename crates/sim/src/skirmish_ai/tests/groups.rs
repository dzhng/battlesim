//! A lane's group: who counts in it, how it gathers at staging, and how it
//! advances as one.

use super::*;

#[test]
fn group_positions_matches_the_units_that_advance_the_lane() {
    let authored = deck();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[
            {"side":"red","kind":"test_rifle","position":[500,100]},
            {"side":"red","kind":"test_supply","position":[500,110]},
            {"side":"red","kind":"test_heli","position":[500,120]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let positions = team::group_positions(&setup.rules, battle.observe(Side::Red).own.iter());
    assert_eq!(positions, vec![[500.0, 100.0], [500.0, 120.0]]);
}

#[test]
fn a_lane_of_mortar_carriers_gathers_and_advances_as_a_group() {
    let authored = deck();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[
            {"side":"red","kind":"test_apc","position":[500,100]},
            {"side":"red","kind":"test_apc","position":[510,105]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.skirmish.as_mut().unwrap().unit_count = 2;
    frame.own = battle.observe(Side::Red).own.clone();
    let assignment = Assignment {
        main_objective: [500.0, 500.0],
        staging_point: [500.0, 100.0],
        force_advance: false,
        parties: vec![],
    };
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: Some(&assignment),
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let advancing = orders
        .iter()
        .filter(|order| matches!(order, Order::AttackMove { goal, .. } if *goal == assignment.main_objective))
        .count();
    assert_eq!(
        advancing, 2,
        "mortar carriers must advance after gathering: {orders:?}"
    );
}

#[test]
fn a_ready_group_keeps_its_objective_after_leaving_staging() {
    let authored = deck();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[
            {"side":"red","kind":"test_rifle","position":[500,100]},
            {"side":"red","kind":"test_rifle","position":[510,105]}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.skirmish.as_mut().unwrap().unit_count = 2;
    frame.own = battle.observe(Side::Red).own.clone();
    let assignment = Assignment {
        main_objective: [500.0, 500.0],
        staging_point: [500.0, 100.0],
        force_advance: false,
        parties: vec![],
    };
    let mut ai = BalancedDoctrine::default();
    let first = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: Some(&assignment),
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(first.iter().any(|order| matches!(
        order,
        Order::AttackMove { goal, .. } if *goal == assignment.main_objective
    )));
    for unit in &mut frame.own {
        unit.position[0] = assignment.main_objective[0];
        unit.position[1] = assignment.main_objective[1];
        unit.goal = None;
    }
    frame.tick += 5 * u64::from(rules.tick_hz);
    let second = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: Some(&assignment),
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(second.iter().any(|order| matches!(
        order,
        Order::AttackMove { goal, .. } if *goal == assignment.main_objective
    )));
}
