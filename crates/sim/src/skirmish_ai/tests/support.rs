//! The standing supplier: the truck that waits at the support post and is
//! replaced first when lost.

use super::*;

#[test]
fn support_truck_moves_behind_observed_troops_and_deploys_after_arrival() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[
            {"side":"red","kind":"test_rifle","position":[500,600]},
            {"side":"red","kind":"test_supply","position":[500,10]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    let mut ai = BalancedDoctrine::default();
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let support = orders
        .iter()
        .find_map(|o| match o {
            Order::Move { units, goal, .. } if units == &vec![UnitId(1)] => Some(*goal),
            _ => None,
        })
        .unwrap();
    assert!(support[1] > entry.center[1] && support[1] < frame.own[0].position[1]);
    frame.own[0].goal = Some([500.0, 500.0]);
    frame.own[1].position = [support[0], support[1], 0.0];
    frame.own[1].goal = None;
    // Explicitly packed own posture needs a deploy command; ordinary stopped
    // trucks already head to deployed through the deployment owner.
    frame.own[1].deployment.as_mut().unwrap().target = contract::observation::Posture::Packed;
    frame.tick += 5 * u64::from(rules.tick_hz);
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| panic!("standing deployment needs no new route"),
    );
    assert!(orders
        .iter()
        .any(|o| matches!(o,Order::SetDeployment{units,deployed:true} if units==&vec![UnitId(1)])));
}

#[test]
fn reinforcement_replaces_a_lost_truck_before_more_combat_units() {
    let (rules, mut frame, entry) = empty_match();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    let mut ai = BalancedDoctrine::default();
    let input = DecisionInput {
        side: Side::Red,
        faction: Faction::Us,
        entry: &entry,
        assignment: None,
    };
    let orders = ai.decide(input, &frame, &rules, |_, _| Some(1.0));
    let Order::ConfirmPurchase {
        variant,
        destination,
        ..
    } = &orders[0]
    else {
        panic!("replace truck")
    };
    let kind = rules.catalog.index(variant).unwrap();
    assert!(rules.catalog.get(kind).has_role("logistics"));
    assert!(rules.catalog.get(kind).mobility.locomotion().is_some());
    assert!(
        distance(*destination, entry.center) > 10.0,
        "replacement truck would remain on the entry: {destination:?}"
    );
    me(&mut frame)
        .pending
        .push(pending(1, kind, *destination, 0));
    assert!(ai
        .decide(input, &frame, &rules, |_, _| Some(1.0))
        .is_empty());
    frame.tick = 5 * u64::from(rules.tick_hz);
    let orders = ai.decide(input, &frame, &rules, |_, _| Some(1.0));
    let Order::ConfirmPurchase { variant, .. } = &orders[0] else {
        panic!("reinforce combat")
    };
    assert!(!rules.catalog.by_id(variant).has_role("logistics"));
}
