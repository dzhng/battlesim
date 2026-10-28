//! Air defence holds the backline with the artillery (D57).

use super::*;
use transport_tests::transport_match_on;

/// The test deck with the cards this rule reads, built from weapon data:
/// `test_gun_jeep` a ground air-defence vehicle (missiles that reach only
/// aircraft), `test_jeep` an artillery piece, `test_tank` a recon vehicle
/// armed only against aircraft, and `test_heli` an aircraft armed only
/// against aircraft.
fn backline_deck() -> Value {
    let mut rules = deck();
    let air_only = |id: &str| json!({ "id": id, "weapons": ["stinger"] });
    for (id, category, patch) in [
        (
            "test_gun_jeep",
            "veh",
            json!({ "roles": ["air_defense"], "mounts": [air_only("HMG")] }),
        ),
        ("test_jeep", "veh", json!({ "roles": ["artillery"] })),
        (
            "test_tank",
            "rec",
            json!({ "roles": ["recon"], "mounts": [air_only("cannon"), air_only("HMG")] }),
        ),
        ("test_heli", "hel", json!({ "mounts": [air_only("HMG")] })),
    ] {
        crate::fixtures::patch_catalog(&mut rules, "units", id, patch);
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({ "roster": { "factions": ["us"], "category": category,
                "family_name": "Test", "variant": "Test" } }),
        );
    }
    rules
}

const AIR_DEFENCE: &str = "test_gun_jeep";
const ARTILLERY: &str = "test_jeep";
const AIR_ONLY_RECON: &str = "test_tank";
const AIR_ONLY_AIRCRAFT: &str = "test_heli";

/// What the doctrine orders `units` toward a far objective, pushing or not:
/// every order, the rules, the entry and the staging point.
fn decide(units: Value, force_advance: bool) -> (Rules, ObservationFrame, Vec<Order>, [f64; 2]) {
    let (rules, frame, entry) = transport_match_on(backline_deck(), units);
    let assignment = Assignment {
        main_objective: [500.0, 900.0],
        staging_point: [500.0, 400.0],
        force_advance,
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
    (rules, frame, orders, entry.center)
}

/// The movement order naming the unit at `index`.
fn order_for(frame: &ObservationFrame, orders: &[Order], index: usize) -> Option<Order> {
    let unit = frame.own[index].id;
    orders
        .iter()
        .find(|order| match order {
            Order::Move { units, .. } | Order::AttackMove { units, .. } => units.contains(&unit),
            _ => false,
        })
        .cloned()
}

fn goal(order: &Option<Order>) -> [f64; 2] {
    match order {
        Some(Order::Move { goal, .. } | Order::AttackMove { goal, .. }) => *goal,
        other => panic!("no movement order: {other:?}"),
    }
}

#[test]
fn air_defence_holds_behind_its_group_and_beside_its_artillery() {
    let staging = [500.0, 400.0];
    for force_advance in [true, false] {
        // No artillery: the post is on the lane behind the group, never
        // forward of the staging point toward the objective.
        let (rules, frame, orders, entry) = decide(
            json!([
                {"side":"red","kind":AIR_DEFENCE,"position":[500,60]},
                {"side":"red","kind":"test_rifle","position":[540,60]}
            ]),
            force_advance,
        );
        assert!(
            !unit_can_engage_ground(&rules, frame.own[0].kind),
            "the air-defence card cannot engage ground targets"
        );
        let order = order_for(&frame, &orders, 0);
        let post = goal(&order);
        let along = |p: [f64; 2]| (p[1] - entry[1]) / (staging[1] - entry[1]);
        assert!(
            along(post) > 0.0 && along(post) < 1.0,
            "push={force_advance}: behind the group, between entry {entry:?} and staging \
             {staging:?}: {post:?}"
        );
        assert!(
            matches!(order, Some(Order::Move { .. })),
            "push={force_advance}: it moves, it does not attack-move: {order:?}"
        );

        // With artillery: beside the artillery.
        let (_, frame, orders, _) = decide(
            json!([
                {"side":"red","kind":AIR_DEFENCE,"position":[500,60]},
                {"side":"red","kind":ARTILLERY,"position":[620,120]}
            ]),
            force_advance,
        );
        let post = goal(&order_for(&frame, &orders, 0));
        assert!(
            libm::hypot(post[0] - 620.0, post[1] - 120.0) < 1.0,
            "push={force_advance}: beside the artillery at [620, 120]: {post:?}"
        );
    }
}

#[test]
fn only_ground_units_that_cannot_engage_ground_hold_the_backline() {
    // A recon unit armed only against aircraft keeps its recon post, short of
    // the objective and facing it; an aircraft armed only against aircraft
    // keeps flying with its group. Neither is sent to the backline.
    let (rules, frame, orders, entry) = decide(
        json!([
            {"side":"red","kind":AIR_ONLY_RECON,"position":[500,60]},
            {"side":"red","kind":AIR_ONLY_AIRCRAFT,"position":[540,60]}
        ]),
        false,
    );
    for unit in &frame.own {
        assert!(!unit_can_engage_ground(&rules, unit.kind));
    }
    let behind = |p: [f64; 2]| p[1] < 400.0 && p[1] > entry[1];
    let recon = order_for(&frame, &orders, 0);
    assert!(
        matches!(
            recon,
            Some(Order::Move {
                facing: Some(_),
                ..
            })
        ),
        "recon moves to its post, facing the objective: {recon:?}"
    );
    assert!(
        !behind(goal(&recon)),
        "the recon post is forward of the staging point: {recon:?}"
    );
    let aircraft = order_for(&frame, &orders, 1);
    assert!(
        matches!(aircraft, Some(Order::AttackMove { .. })),
        "the aircraft flies with its group: {aircraft:?}"
    );
    assert!(
        !behind(goal(&aircraft)),
        "the aircraft is not posted behind the group: {aircraft:?}"
    );
}
