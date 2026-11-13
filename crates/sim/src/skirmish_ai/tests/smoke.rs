//! A unit losing a fight pops its smoke and backs out (D4, D22).

use super::*;
use contract::command::MoveDirection;
use contract::observation::OwnUnit;
use transport_tests::transport_match_on;

/// A red carrier of `kind` facing east at mid-map, an enemy tank it can
/// shoot 200 m ahead, and an AI whose smoke tuning the tests set: a unit
/// losing a fifth of its strength within 10 s pops and reverses 40 m.
fn smoke_match(kind: &str) -> (Rules, ObservationFrame, EntrySite, BalancedDoctrine) {
    let (rules, mut frame, entry) = transport_match_on(
        deck(),
        json!([{ "side": "red", "kind": kind, "position": [500, 500], "yaw": 0.0 }]),
    );
    let tank = rules.catalog.index("test_tank").unwrap();
    frame.identified = vec![identified(1, tank, [700.0, 500.0, 0.0])];
    let mut ai = BalancedDoctrine::default();
    ai.tuning.smoke_loss_share = 0.2;
    ai.tuning.smoke_window_s = 10.0;
    ai.tuning.smoke_reverse_m = 40.0;
    (rules, frame, entry, ai)
}

/// The next decision, `s` seconds on.
fn decide_after(
    ai: &mut BalancedDoctrine,
    frame: &mut ObservationFrame,
    rules: &Rules,
    entry: &EntrySite,
    s: u64,
) -> Vec<Order> {
    frame.tick += s * u64::from(rules.tick_hz);
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

/// The unit keeps `share` of its strength: a hull's health, or each of a
/// squad's soldiers'.
fn wound(unit: &mut OwnUnit, share: f64) {
    unit.hp *= share;
    for hp in &mut unit.member_hp {
        *hp *= share;
    }
}

fn pops(orders: &[Order], unit: UnitId) -> Option<usize> {
    orders
        .iter()
        .position(|order| matches!(order, Order::PopSmoke { units } if units.contains(&unit)))
}

/// Where and in which order the unit is sent backwards, if it is.
fn reverse(orders: &[Order], unit: UnitId) -> Option<(usize, [f64; 2])> {
    orders
        .iter()
        .enumerate()
        .find_map(|(at, order)| match order {
            Order::Move {
                units,
                goal,
                direction: MoveDirection::Reverse,
                ..
            } if units == &vec![unit] => Some((at, *goal)),
            _ => None,
        })
}

/// A carrier, hull or squad, that loses a fight pops its smoke, and the
/// order after the pop sends it straight back from the way it faces, out
/// of the cloud it stands in.
#[test]
fn a_losing_carrier_pops_smoke_then_reverses_out() {
    for kind in ["test_smoke_tank", "test_smoke_squad"] {
        let (rules, mut frame, entry, mut ai) = smoke_match(kind);
        let unit = frame.own[0].id;
        let start = frame.own[0].position;
        let first = decide_after(&mut ai, &mut frame, &rules, &entry, 0);
        assert_eq!(pops(&first, unit), None, "{kind} popped before any loss");
        wound(&mut frame.own[0], 0.7);
        let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
        let popped = pops(&orders, unit).unwrap_or_else(|| panic!("{kind} never popped"));
        let (moved, goal) =
            reverse(&orders, unit).unwrap_or_else(|| panic!("{kind} never reversed"));
        assert!(popped < moved, "{kind}'s reverse came before its pop");
        let back = start[0] - goal[0];
        assert!(
            (35.0..=45.0).contains(&back) && (goal[1] - start[1]).abs() < 1.0,
            "{kind} was sent to {goal:?} from {start:?}, not straight back"
        );
        assert!(
            attack_targets(&orders, unit).is_empty(),
            "{kind} was sent back into the fight in the same decision"
        );
    }
}

/// While it backs out, a carrier is left to its reverse: the AI neither
/// pops a second charge nor sends it back at the enemy, though it is
/// still being hit.
#[test]
fn a_reversing_carrier_is_neither_popped_again_nor_sent_to_attack() {
    let (rules, mut frame, entry, mut ai) = smoke_match("test_smoke_tank");
    let unit = frame.own[0].id;
    decide_after(&mut ai, &mut frame, &rules, &entry, 0);
    wound(&mut frame.own[0], 0.7);
    let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
    let (_, goal) = reverse(&orders, unit).expect("it reversed");
    let tank = &mut frame.own[0];
    tank.smoke = tank.smoke.map(|charges| charges - 1);
    tank.goal = Some(goal);
    tank.direction = Some(MoveDirection::Reverse);
    tank.reversing = true;
    wound(tank, 0.6);
    let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
    assert_eq!(pops(&orders, unit), None, "popped again while reversing");
    assert!(
        attack_targets(&orders, unit).is_empty(),
        "sent to attack while reversing: {orders:?}"
    );
    assert!(reverse(&orders, unit).is_none(), "its reverse was reissued");
}

/// An untouched carrier in a fight fights: it never pops.
#[test]
fn an_untouched_carrier_never_pops() {
    let (rules, mut frame, entry, mut ai) = smoke_match("test_smoke_tank");
    let unit = frame.own[0].id;
    for _ in 0..6 {
        let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
        assert_eq!(pops(&orders, unit), None);
        assert!(reverse(&orders, unit).is_none());
        assert_eq!(attack_targets(&orders, unit), vec![ObservedTargetId(1)]);
    }
}

/// A unit that has spent its charges, or a loss spread thinner than the
/// window, keeps fighting.
#[test]
fn a_carrier_without_charges_or_losing_slowly_keeps_fighting() {
    let (rules, mut frame, entry, mut ai) = smoke_match("test_smoke_tank");
    let unit = frame.own[0].id;
    frame.own[0].smoke = Some(0);
    decide_after(&mut ai, &mut frame, &rules, &entry, 0);
    wound(&mut frame.own[0], 0.5);
    let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
    assert_eq!(pops(&orders, unit), None, "popped with no charge");
    assert!(reverse(&orders, unit).is_none());
    assert_eq!(attack_targets(&orders, unit), vec![ObservedTargetId(1)]);

    let (rules, mut frame, entry, mut ai) = smoke_match("test_smoke_tank");
    let unit = frame.own[0].id;
    let full = frame.own[0].hp;
    // A tenth of its health every 10 s: never a fifth inside one window.
    for step in 1..=6 {
        frame.own[0].hp = full * (1.0 - 0.05 * f64::from(step));
        let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
        assert_eq!(pops(&orders, unit), None, "popped on a slow loss at {step}");
    }
}

/// The AI reads its own unit's strength, not who took it: a loss with no
/// enemy in sight, as from its own side's shells, pops smoke all the same.
/// An accepted quirk (D22): a player would not pop against friendly fire.
#[test]
fn friendly_fire_counts_as_losing() {
    let (rules, mut frame, entry, mut ai) = smoke_match("test_smoke_tank");
    frame.identified.clear();
    let unit = frame.own[0].id;
    decide_after(&mut ai, &mut frame, &rules, &entry, 0);
    wound(&mut frame.own[0], 0.7);
    let orders = decide_after(&mut ai, &mut frame, &rules, &entry, 5);
    assert!(pops(&orders, unit).is_some());
    assert!(reverse(&orders, unit).is_some());
}
