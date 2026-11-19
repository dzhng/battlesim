//! A scripted purchase plan: an authored list of cards the doctrine buys in
//! place of its own shopping, retried in order when one is refused.

use super::*;

#[test]
fn a_scripted_purchase_plan_replaces_the_doctrines_own_shopping() {
    let (rules, mut frame, entry) = empty_match();
    let plan = vec![
        rules.catalog.index("test_tank").unwrap(),
        rules.catalog.index("test_rifle").unwrap(),
    ];
    let expected = plan
        .iter()
        .map(|&kind| rules.catalog.id(kind).to_owned())
        .collect::<Vec<_>>();
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(plan);
    let first = ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        )
        .into_iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        first, expected,
        "an affordable scripted opening is one batch"
    );
    for _ in 0..expected.len() {
        ai.purchase_result(true, 1);
    }
    frame.tick += 5 * u64::from(rules.tick_hz);
    let second = ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        )
        .into_iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(second.is_empty(), "the batch exhausted before readiness");
    assert_eq!(
        first.into_iter().chain(second).collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn a_refused_scripted_purchase_is_retried_in_order() {
    let (rules, mut frame, entry) = empty_match();
    me(&mut frame).credits = 1_000.0;
    let plan = vec![
        rules.catalog.index("test_tank").unwrap(),
        rules.catalog.index("test_rifle").unwrap(),
    ];
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(plan.clone());
    let purchase = |ai: &mut BalancedDoctrine, frame: &mut ObservationFrame| {
        frame.tick += 5 * u64::from(rules.tick_hz);
        ai.decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            frame,
            &rules,
            |_, _| Some(1.0),
        )
        .into_iter()
        .find_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .unwrap()
    };
    let first = purchase(&mut ai, &mut frame);
    ai.purchase_result(false, 1);
    assert_eq!(purchase(&mut ai, &mut frame), first);
    ai.purchase_result(true, 1);
    assert_eq!(
        purchase(&mut ai, &mut frame),
        rules.catalog.id(plan[1]).to_owned()
    );
}
