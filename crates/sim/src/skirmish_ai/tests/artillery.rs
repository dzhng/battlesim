//! Artillery fires at ground its side has evidence for: a contact, a sighting, an objective in reach.

use super::*;

#[test]
fn artillery_fires_a_bounded_ground_guess_from_a_contact() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({
            "roles": ["artillery"],
            "roster": { "factions": ["us"], "category": "sup", "family_name": "Test", "variant": "Test artillery" }
        }),
    );
    authored["weapons"]["hmg"]["trajectory"] = json!("indirect");
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jeep","position":[100,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, _) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    frame.own[0].kind = rules.catalog.index("test_jeep").unwrap();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    frame
        .contacts
        .push(contract::observation::ApproximateContact {
            id: contract::observation::ContactId(4),
            source: contract::observation::ContactSource::Firing,
            center: [220.0, 220.0, 0.0],
            layer: contract::catalog::AltitudeLayer::Ground,
            aloft: false,
            radius: 30.0,
            evidence_tick: 0,
            expires_tick: 120,
            kind: None,
            heard: 0,
            bearing_deg: None,
            range_m: None,
        });
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry_from([100.0, 100.0]),
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::Attack { units, target: contract::command::TargetRef::Ground { point } }
            if units == &vec![UnitId(0)] && point[0] != 220.0 && point[1] != 220.0
    )));
}

fn artillery_frame() -> (Rules, ObservationFrame, EntrySite) {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({"roles":["artillery"]}),
    );
    authored["weapons"]["hmg"]["trajectory"] = json!("indirect");
    authored["weapons"]["hmg"]["min_range_m"] = json!(50);
    authored["weapons"]["hmg"]["targets"]["ground"]["range_m"] = json!(500);
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[{"side":"red","kind":"test_jeep","position":[100,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, entry) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    (setup.rules, frame, entry)
}

fn ground_orders(orders: &[Order]) -> Vec<[f64; 3]> {
    orders
        .iter()
        .filter_map(|order| match order {
            Order::Attack {
                target: TargetRef::Ground { point },
                ..
            } => Some(*point),
            _ => None,
        })
        .collect()
}

#[test]
fn artillery_refuses_stale_expired_and_unreachable_reports() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.tick = 20 * u64::from(rules.tick_hz);
    for contact in [
        firing_contact([220.0, 100.0, 0.0], 0, frame.tick + 1000),
        firing_contact([220.0, 100.0, 0.0], frame.tick, frame.tick - 1),
        firing_contact([900.0, 100.0, 0.0], frame.tick, frame.tick + 1000),
    ] {
        frame.contacts = vec![contact];
        let orders = BalancedDoctrine::default().decide(
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
        assert!(
            ground_orders(&orders).is_empty(),
            "stale or unreachable evidence must not fire: {orders:?}"
        );
    }
}

#[test]
fn artillery_prediction_uses_only_a_matching_sighting_and_stays_bounded() {
    let (rules, mut frame, entry) = artillery_frame();
    let target_kind = rules.catalog.index("test_tank").unwrap();
    let target = contract::observation::IdentifiedUnit {
        id: contract::observation::ObservedTargetId(4),
        kind: target_kind,
        cost: 100,
        position: [220.0, 220.0, 0.0],
        yaw: 0.0,
        velocity: [10.0, 0.0],
        members: vec![],
        member_ids: vec![],
        member_slots: vec![],
        member_active_mounts: vec![],
        weapon_poses: vec![],
        reversing: false,
        smoking: false,
        member_leans: vec![],
    };
    frame.identified.push(target);
    // A second same-kind sighting is far from the firing report. The policy
    // must retain each observation by ID rather than borrowing its velocity.
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(5),
            kind: target_kind,
            cost: 100,
            position: [700.0, 700.0, 0.0],
            yaw: 0.0,
            velocity: [1000.0, 0.0],
            members: vec![],
            member_ids: vec![],
            member_slots: vec![],
            member_active_mounts: vec![],
            weapon_poses: vec![],
            reversing: false,
            smoking: false,
            member_leans: vec![],
        });
    // Hold fire while recording the first sighting, then release it for the
    // next decision so the prediction is the only possible salvo.
    frame.own[0].engagement = contract::command::Engagement::ReturnFireOnly;
    frame.skirmish.as_mut().unwrap().phase = Phase::Finished;
    let mut ai = BalancedDoctrine::default();
    assert!(ground_orders(&ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
    frame.tick = 2 * u64::from(rules.tick_hz);
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.own[0].engagement = contract::command::Engagement::FireAtWill;
    frame.identified.clear();
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
    let points = ground_orders(&orders);
    assert_eq!(points, vec![[240.0, 220.0, 0.0]], "orders: {orders:?}");

    frame.tick = 7 * u64::from(rules.tick_hz);
    assert!(ground_orders(&ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
}

#[test]
fn artillery_uses_a_range_checked_public_objective_when_contacts_are_stale() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.tick = 0;
    frame.contacts = vec![firing_contact([900.0, 900.0, 0.0], 0, 0)];
    frame.skirmish.as_mut().unwrap().objectives[0].owner = Some(Side::Blue);
    frame.skirmish.as_mut().unwrap().objectives[0].center = [900.0, 900.0];
    assert!(ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
    frame.skirmish.as_mut().unwrap().objectives[0].center = [300.0, 300.0];
    let points = ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    ));
    assert_eq!(points.len(), 1);
    let point = points[0];
    assert_ne!(
        point,
        [300.0, 300.0, 0.0],
        "objective aim keeps a fallible offset"
    );
    let orders = BalancedDoctrine::default().decide(
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
    assert_eq!(ground_orders(&orders), vec![point]);
}

#[test]
fn artillery_final_aim_is_in_one_weapon_envelope_and_return_fire_holds() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.contacts = vec![firing_contact([599.0, 100.0, 0.0], 0, 100)];
    let points = ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    ));
    assert_eq!(points.len(), 1);
    assert!(
        distance([100.0, 100.0], [points[0][0], points[0][1]]) <= 500.0,
        "final offset left weapon range: {points:?}"
    );
    frame.own[0].engagement = contract::command::Engagement::ReturnFireOnly;
    let orders = BalancedDoctrine::default().decide(
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
    assert!(
        ground_orders(&orders).is_empty(),
        "policy must not override hold-fire: {orders:?}"
    );
}
