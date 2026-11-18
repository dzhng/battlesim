use super::*;

#[test]
fn active_policy_uses_an_ordinary_attack_for_a_known_target() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jeep","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    frame.identified.push(identified(
        9,
        rules.catalog.index("test_tank").unwrap(),
        [500.0, 500.0, 0.0],
    ));
    let decide = |frame: &ObservationFrame| {
        BalancedDoctrine::default().decide(
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
    };
    let orders = decide(&frame);
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::Attack { units, target: contract::command::TargetRef::Identified { id } }
            if units == &vec![UnitId(0)] && *id == contract::observation::ObservedTargetId(9)
    )));
    frame.own[0]
        .steps
        .push(contract::observation::OrderStep::Attack {
            target: Some(TargetRef::Identified {
                id: frame.identified[0].id,
            }),
            point: None,
        });
    assert!(
        attack_targets(&decide(&frame), UnitId(0)).is_empty(),
        "an ongoing attack must keep its progress at the next decision"
    );
}
