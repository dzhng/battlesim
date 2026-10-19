//! Party (help, hold, claim) doctrine tests.

use super::*;
use transport_tests::transport_match;

#[test]
fn a_helping_detachment_moves_without_retasking_the_helper_lane() {
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
    me(&mut frame).unit_count = 2;
    frame.own = battle.observe(Side::Red).own.clone();
    let detachment = frame.own[0].id;
    let assignment = Assignment {
        main_objective: [500.0, 500.0],
        staging_point: [500.0, 100.0],
        force_advance: false,
        parties: vec![PartyTask {
            objective: [900.0, 500.0],
            units: vec![detachment],
            kind: team::PartyKind::Help,
        }],
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
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::AttackMove { units, goal, .. }
            if units == &vec![detachment] && *goal == [900.0, 500.0]
    )));
    let lane_unit = frame.own.iter().find(|unit| unit.id != detachment).unwrap();
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::AttackMove { units, goal, .. }
            if units == &vec![lane_unit.id]
                && (*goal == assignment.main_objective || *goal == assignment.staging_point)
    )));
}

#[test]
fn a_squad_sent_far_to_help_or_push_rides_its_own_carrier() {
    // A squad sent a long way, in a party of any kind (help for an ally's
    // lane, a claim) in either stance, or in a team push, is collected by
    // its player's free carrier rather than walking there.
    let parties = [
        (false, Some(team::PartyKind::Help)),
        (false, Some(team::PartyKind::Claim)),
        (true, Some(team::PartyKind::Claim)),
        (true, None),
    ];
    for (force_advance, party) in parties {
        let help = party.is_some();
        let (rules, frame, entry) = transport_match(json!([
            {"side":"red","kind":"test_rifle","position":[500,60]},
            {"side":"red","kind":"test_apc","position":[540,60]}
        ]));
        let (squad, carrier) = (frame.own[0].id, frame.own[1].id);
        let far = [500.0, 900.0];
        let assignment = Assignment {
            main_objective: if help { [520.0, 80.0] } else { far },
            staging_point: [520.0, 80.0],
            force_advance,
            parties: if help {
                vec![PartyTask {
                    objective: far,
                    units: vec![squad],
                    kind: party.unwrap(),
                }]
            } else {
                vec![]
            },
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
        assert!(
            orders
                .iter()
                .any(|o| matches!(o, Order::Collect { units, .. } if units.contains(&carrier))),
            "{party:?} push={force_advance}: its carrier collects it: {orders:?}"
        );
        assert!(
            !orders
                .iter()
                .any(|o| matches!(o, Order::AttackMove { units, .. } if units.contains(&squad))),
            "{party:?} push={force_advance}: it does not walk: {orders:?}"
        );
    }
}
