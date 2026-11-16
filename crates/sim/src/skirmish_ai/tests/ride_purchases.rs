//! A squad and its ride bought as one purchase (M17): the opening's pairs,
//! a new squad no carrier waits for, the scripted plan, and no second ride
//! for a pair still on its way.

use super::purchases::us_cards;
use super::transport_tests::transport_match;
use super::*;
use crate::carriage::seats;

/// Each purchase in `orders`: the card, and the carrier bought with it.
fn bought(rules: &Rules, orders: &[Order]) -> Vec<(TypeIndex, Option<TypeIndex>)> {
    let index = |id: &str| rules.catalog.index(id).unwrap();
    orders
        .iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase {
                variant, carrier, ..
            } => Some((index(variant), carrier.as_deref().map(index))),
            _ => None,
        })
        .collect()
}

fn decide(
    ai: &mut BalancedDoctrine,
    rules: &Rules,
    frame: &ObservationFrame,
    entry: &EntrySite,
) -> Vec<Order> {
    ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry,
            assignment: None,
        },
        frame,
        rules,
        |_, _| Some(1.0),
    )
}

fn price(rules: &Rules, kind: TypeIndex) -> f64 {
    f64::from(rules.catalog.get(kind).cost)
}

fn size(rules: &Rules, kind: TypeIndex) -> usize {
    rules.catalog.get(kind).squad_size()
}

/// The card pairing gives `squad`: the fewest seats that take it.
fn ride(rules: &Rules, squad: TypeIndex) -> Option<TypeIndex> {
    transport::carrier_for(rules, &us_cards(rules), size(rules, squad))
}

/// A preparing match on the transport deck holding `units`, with `credits`.
fn preparing(units: Value, credits: f64) -> (Rules, ObservationFrame, EntrySite) {
    let (rules, mut frame, entry) = transport_match(units);
    frame.skirmish.as_mut().unwrap().phase = Phase::Preparation;
    me(&mut frame).credits = credits;
    (rules, frame, entry)
}

/// An active match holding a supply truck (so no truck is bought first),
/// with `credits`.
fn reinforcing(credits: f64) -> (Rules, ObservationFrame, EntrySite) {
    let (rules, mut frame, entry) = transport_match(json!([
        {"side":"red","kind":"test_supply","position":[400,40]}
    ]));
    me(&mut frame).credits = credits;
    me(&mut frame).unit_count = 1;
    (rules, frame, entry)
}

#[test]
fn the_opening_buys_each_squad_with_its_ride_as_one_purchase() {
    let (rules, mut frame, entry) = preparing(json!([]), 100_000.0);
    let orders = decide(&mut BalancedDoctrine::default(), &rules, &frame, &entry);
    let purchases = bought(&rules, &orders);
    let squads: Vec<_> = (purchases.iter())
        .filter(|&&(kind, _)| ride(&rules, kind).is_some())
        .collect();
    assert!(!squads.is_empty(), "the opening fields squads: {orders:?}");
    for &&(squad, carrier) in &squads {
        assert_eq!(
            carrier,
            ride(&rules, squad),
            "each squad rides in: {orders:?}"
        );
    }
    // Every squad has its ride on the way, so pairing asks for no carrier.
    confirm_purchases(&rules, &mut frame, &orders);
    let pending = me(&mut frame).pending.iter().flat_map(|p| p.kinds());
    assert!(
        !matches!(
            transport::want(&rules, &us_cards(&rules), pending),
            Some(transport::Want::Carrier(_))
        ),
        "{orders:?}"
    );

    // Credits for the first squad but not its ride: the squad goes alone.
    let &(first, _) = squads[0];
    let short = price(&rules, first) + price(&rules, ride(&rules, first).unwrap()) - 1.0;
    let (rules, frame, entry) = preparing(json!([]), short);
    let orders = decide(&mut BalancedDoctrine::default(), &rules, &frame, &entry);
    assert_eq!(bought(&rules, &orders)[0], (first, None), "{orders:?}");
}

#[test]
fn a_new_squad_no_carrier_waits_for_is_bought_with_its_ride() {
    let (rules, frame, entry) = reinforcing(10_000.0);
    // The rotation opens on a squad, which no carrier the player has seats.
    let orders = decide(&mut BalancedDoctrine::default(), &rules, &frame, &entry);
    let [(squad, carrier)] = bought(&rules, &orders)[..] else {
        panic!("one purchase: {orders:?}")
    };
    let carrier = carrier.expect("the squad rides in");
    assert_eq!(Some(carrier), ride(&rules, squad));
    assert!(seats(&rules, carrier) >= size(&rules, squad));

    // Credits for the squad alone: the rotation saves for both.
    let (rules, mut frame, entry) = reinforcing(price(&rules, squad));
    let orders = decide(&mut BalancedDoctrine::default(), &rules, &frame, &entry);
    assert_eq!(bought(&rules, &orders), [], "{orders:?}");
    // Room under the cap for one unit: the squad goes alone.
    me(&mut frame).credits = 10_000.0;
    me(&mut frame).unit_count = me(&mut frame).cap - 1;
    let orders = decide(&mut BalancedDoctrine::default(), &rules, &frame, &entry);
    assert_eq!(bought(&rules, &orders), [(squad, None)], "{orders:?}");
}

/// A pending pair's squad has its ride: the AI buys no second carrier for
/// it, as it does for a squad bought on foot.
#[test]
fn a_pending_pair_asks_for_no_second_carrier() {
    let (rules, mut frame, entry) = reinforcing(10_000.0);
    let squad = rules.catalog.index("test_rifle").unwrap();
    let carrier = ride(&rules, squad).unwrap();
    let lone_carriers = |frame: &ObservationFrame| {
        let orders = decide(&mut BalancedDoctrine::default(), &rules, frame, &entry);
        (bought(&rules, &orders).into_iter())
            .filter(|&(kind, _)| seats(&rules, kind) > 0)
            .count()
    };
    let mut row = pending(1, squad, entry.center, 0);
    me(&mut frame).pending = vec![row.clone()];
    assert_eq!(lone_carriers(&frame), 1, "a squad on foot gets its carrier");
    row.carrier = Some(carrier);
    me(&mut frame).pending = vec![row];
    assert_eq!(lone_carriers(&frame), 0, "a squad aboard needs none");
}

#[test]
fn a_scripted_squad_and_the_card_after_it_that_seats_it_are_one_purchase() {
    let (rules, frame, entry) = preparing(json!([]), 10_000.0);
    let card = |id: &str| rules.catalog.index(id).unwrap();
    let (squad, tank) = (card("test_rifle"), card("test_tank"));
    let carrier = ride(&rules, squad).unwrap();
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(vec![squad, carrier, tank]);
    let orders = decide(&mut ai, &rules, &frame, &entry);
    assert_eq!(
        bought(&rules, &orders),
        [(squad, Some(carrier)), (tank, None)]
    );
    // A refused pair is tried again whole; an admitted one moves the plan
    // past both its cards.
    let mut frame = frame;
    let mut again = |ai: &mut BalancedDoctrine| {
        frame.tick += 5 * u64::from(rules.tick_hz);
        bought(&rules, &decide(ai, &rules, &frame, &entry))
    };
    ai.purchase_result(false, 2);
    assert_eq!(again(&mut ai)[0], (squad, Some(carrier)));
    ai.purchase_result(true, 2);
    assert_eq!(again(&mut ai), [(tank, None)]);

    // A carrier too small for the squad is bought on its own.
    let small = (us_cards(&rules).into_iter())
        .find(|&k| (1..size(&rules, squad)).contains(&seats(&rules, k)))
        .expect("the deck has a carrier the squad does not fit");
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(vec![squad, small]);
    let orders = decide(&mut ai, &rules, &frame, &entry);
    assert_eq!(bought(&rules, &orders), [(squad, None), (small, None)]);
}
