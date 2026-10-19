//! Doctrine transport and unloading tests.

use super::*;

/// The test deck with both test carriers on its cards: eight seats and three.
fn transport_deck() -> serde_json::Value {
    let mut rules = deck();
    for id in ["test_apc", "test_scout_car"] {
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({ "roster": { "factions": ["us"], "category": "veh",
                "family_name": "Test", "variant": "Test" } }),
        );
    }
    rules
}

/// An active match on the transport deck with `units` of red's own, no
/// credits to spend, and the empty match's one objective.
pub(super) fn transport_match(units: Value) -> (Rules, ObservationFrame, EntrySite) {
    transport_match_on(transport_deck(), units)
}

/// [`transport_match`] on `deck`.
fn transport_match_on(deck: Value, units: Value) -> (Rules, ObservationFrame, EntrySite) {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck,"units":units,"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, entry) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.credits = 0.0;
    (setup.rules, frame, entry)
}

/// Seat the squad at `squad` aboard the carrier at `carrier`, as the
/// observation shows a passenger.
fn aboard(frame: &mut ObservationFrame, squad: usize, carrier: usize) {
    let host = frame.own[carrier].id.0;
    frame.own[squad].position = frame.own[carrier].position;
    frame.own[squad].goal = None;
    frame.own[squad].hold = Some(contract::observation::HoldState {
        host: contract::observation::HostRef::Carrier { id: host },
        phase: contract::observation::HoldPhase::Inside,
        progress: 1.0,
    });
}

/// Where the carrier's unload order lets its squad out.
fn unload_goal(orders: &[Order], carrier: UnitId) -> Option<[f64; 2]> {
    orders.iter().find_map(|order| match order {
        Order::Unload { units, goal, .. } if units == &vec![carrier] => *goal,
        _ => None,
    })
}

#[test]
fn the_ai_drives_its_squads_and_unloads_short_of_seen_weapons() {
    // A rifle squad and an eight-seat carrier stand at base, far from the
    // objective.
    let (rules, mut frame, entry) = transport_match(json!([
        {"side":"red","kind":"test_rifle","position":[500,60]},
        {"side":"red","kind":"test_apc","position":[530,60]}
    ]));
    let (squad, carrier) = (frame.own[0].id, frame.own[1].id);
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
    assert!(
        orders
            .iter()
            .any(|o| matches!(o, Order::Collect { units, squads, .. }
            if units == &vec![carrier] && squads == &vec![squad])),
        "the carrier collects its squad: {orders:?}"
    );
    assert!(
        !orders
            .iter()
            .any(|o| matches!(o, Order::AttackMove { units, .. } if units.contains(&squad))),
        "a squad waiting for its ride does not set off on foot"
    );

    // Aboard, with nothing seen, the carrier drives toward the objective and
    // lets the squad out short of it; the passenger takes no order.
    aboard(&mut frame, 0, 1);
    let start = [frame.own[1].position[0], frame.own[1].position[1]];
    let objective = [500.0, 500.0];
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
        |_, _| Some(1.0),
    );
    let open = unload_goal(&orders, carrier).expect("the loaded carrier drives out");
    assert!(distance(open, objective) < distance(start, objective));
    assert!(
        distance(open, objective) > 0.0,
        "it stops short of the objective"
    );
    assert!(!orders.iter().any(|o| match o {
        Order::Attack { units, .. } | Order::AttackMove { units, .. } | Order::Stop { units } =>
            units.contains(&squad),
        _ => false,
    }));

    // An enemy squad it sees holding the objective pushes the drop point out
    // of that squad's reach, still nearer than the base.
    let enemy = rules.catalog.index("test_rifle").unwrap();
    let reach = transport::ground_reach(&rules, enemy);
    frame.identified = vec![identified(7, enemy, [500.0, 500.0, 0.0])];
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
        |_, _| Some(1.0),
    );
    let covered = unload_goal(&orders, carrier).expect("it re-aims for the seen enemy");
    assert!(
        distance(covered, objective) >= reach - 1e-6,
        "outside the enemy's reach"
    );
    assert!(distance(covered, objective) > distance(open, objective));
    assert!(distance(covered, objective) < distance(start, objective));
}

#[test]
fn the_ai_ignores_unseen_enemies_when_choosing_where_to_unload() {
    let (rules, mut frame, entry) = transport_match(json!([
        {"side":"red","kind":"test_rifle","position":[500,60]},
        {"side":"red","kind":"test_apc","position":[530,60]}
    ]));
    aboard(&mut frame, 0, 1);
    let carrier = frame.own[1].id;
    let unload = |frame: &ObservationFrame| {
        let orders = BalancedDoctrine::default().decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            frame,
            &rules,
            |_, _| Some(1.0),
        );
        unload_goal(&orders, carrier).unwrap()
    };
    let clear = unload(&frame);
    // Evidence without an identity (a shot heard at the objective) names no
    // weapon to stay out of: the drop point stands where it was.
    frame.contacts = vec![firing_contact(
        [500.0, 500.0, 0.0],
        frame.tick,
        frame.tick + 100,
    )];
    assert_eq!(unload(&frame), clear);
    // The same enemy, identified, moves it.
    let enemy = rules.catalog.index("test_rifle").unwrap();
    frame.identified = vec![identified(7, enemy, [500.0, 500.0, 0.0])];
    assert_ne!(unload(&frame), clear);
}

/// A carrier no squad needs is a combat unit if its mounts can fight: it
/// advances like any other. One with no weapon waits for a squad.
#[test]
fn an_empty_armed_carrier_fights_and_an_unarmed_one_waits() {
    let mut deck = transport_deck();
    crate::fixtures::patch_catalog(
        &mut deck,
        "units",
        "test_scout_car",
        json!({ "mounts": [] }),
    );
    let (rules, frame, entry) = transport_match_on(
        deck,
        json!([
            {"side":"red","kind":"test_apc","position":[530,60]},
            {"side":"red","kind":"test_scout_car","position":[600,60]}
        ]),
    );
    let (armed, unarmed) = (frame.own[0].id, frame.own[1].id);
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
    let ordered = |unit: UnitId| {
        orders.iter().any(|o| match o {
            Order::Attack { units, .. }
            | Order::AttackMove { units, .. }
            | Order::Move { units, .. }
            | Order::Unload { units, .. }
            | Order::Collect { units, .. } => units.contains(&unit),
            _ => false,
        })
    };
    assert!(
        orders
            .iter()
            .any(|o| matches!(o, Order::AttackMove { units, .. } if units == &vec![armed])),
        "the armed carrier advances on the objective: {orders:?}"
    );
    assert!(!ordered(unarmed), "the unarmed carrier waits: {orders:?}");
}

/// Pairs are read afresh at each decision, so a carrier already driving to a
/// squad must stay its ride: handing the squad to a nearer carrier at every
/// decision sends both back and forth and nobody ever boards.
#[test]
fn a_carrier_already_coming_for_a_squad_stays_its_ride() {
    let (rules, mut frame, entry) = transport_match(json!([
        {"side":"red","kind":"test_rifle","position":[500,60]},
        {"side":"red","kind":"test_apc","position":[540,60]},
        {"side":"red","kind":"test_apc","position":[700,60]}
    ]));
    let (squad, near, coming) = (frame.own[0].id, frame.own[1].id, frame.own[2].id);
    frame.own[2].goal = Some([500.0, 60.0]);
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
    let names = |o: &Order, unit: UnitId| match o {
        Order::Collect { units, .. }
        | Order::AttackMove { units, .. }
        | Order::Move { units, .. } => units.contains(&unit),
        _ => false,
    };
    assert!(
        !orders.iter().any(|o| names(o, squad)
            || matches!(o, Order::Collect { units, .. } if units.contains(&near))),
        "the squad waits for the carrier already coming: {orders:?}"
    );
    assert!(
        !orders.iter().any(|o| names(o, coming)),
        "its standing collection is left to run: {orders:?}"
    );
}

#[test]
fn a_lost_transport_is_rebought_and_a_lost_squad_replaced() {
    // What the AI buys first, holding a truck and `unit`, with credits.
    let bought = |unit: &str| {
        let (rules, mut frame, entry) = transport_match(json!([
            {"side":"red","kind":"test_supply","position":[400,40]},
            {"side":"red","kind":unit,"position":[500,300]}
        ]));
        frame.skirmish.as_mut().unwrap().credits = 1000.0;
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
        let Some(Order::ConfirmPurchase {
            variant,
            destination,
        }) = orders
            .iter()
            .find(|o| matches!(o, Order::ConfirmPurchase { .. }))
        else {
            panic!("it buys: {orders:?}")
        };
        let held = rules.catalog.by_id(unit).clone();
        let bought = rules.catalog.by_id(variant).clone();
        (held, bought, *destination == entry.center)
    };
    let seats = |unit: &contract::catalog::UnitType| {
        unit.capabilities.carriage.map_or(0, |c| c.seats as usize)
    };
    // A squad whose transport was lost is bought the fewest seats that take
    // it, a carrier that waits for it at base.
    for squad in ["test_rifle", "test_at"] {
        let (squad, carrier, at_base) = bought(squad);
        let (size, seats) = (squad.squad_size(), seats(&carrier));
        assert!(seats >= size, "{size} soldiers, {seats} seats");
        assert!(size > 3 || seats <= 3, "{size} soldiers, {seats} seats");
        assert!(at_base);
    }
    // A carrier whose squad was lost is bought a squad it seats, and that
    // fewer seats would not.
    let (carrier, squad, _) = bought("test_apc");
    let size = squad.squad_size();
    assert!(size > 3 && size <= seats(&carrier), "{size} soldiers");
}
