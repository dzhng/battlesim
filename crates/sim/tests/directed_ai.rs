//! A directed AI seat: the battle commands it, but it decides nothing itself.
//! Its units fight at their own engagement, and only an authored scenario's
//! director moves them.

use crate::common;
use crate::transport::{side_command, transport_skirmish_setup};
use contract::command::{Engagement, Order, OrderError};
use contract::ids::Side;
use contract::players::ControllerKind;
use contract::skirmish::Phase;
use serde_json::json;
use sim::battle::Battle;

/// Two riflemen in sight of each other, each owned by its side's player.
fn duel() -> contract::scenario::ScenarioDefinition {
    common::scenario(
        common::saved_map("geometry"),
        json!([
            { "side": "blue", "owner": "blue-1", "kind": "test_rifle", "position": [80.0, 100.0] },
            { "side": "red", "owner": "red-1", "kind": "test_rifle", "position": [200.0, 100.0] }
        ]),
        json!([]),
    )
}

#[test]
fn a_directed_seat_issues_nothing_and_fights_at_its_own_engagement() {
    let mut battle = Battle::new(&duel(), 51);
    battle.set_controllers(vec![
        ("blue-1".into(), ControllerKind::Human),
        ("red-1".into(), ControllerKind::DirectedAi),
    ]);
    for _ in 0..battle.rules().tick_hz * 3 {
        battle.step();
    }
    assert!(
        battle
            .replay()
            .accepted
            .iter()
            .all(|(_, command)| command.player.as_deref() != Some("red-1")),
        "a directed seat gives no orders of its own"
    );
    let red = &battle.observe(Side::Red).own;
    assert_eq!(
        red[0].engagement,
        Engagement::FireAtWill,
        "unlike a passive seat, a directed one is not held at return fire"
    );
}

#[test]
fn a_directed_seat_takes_no_client_orders() {
    let mut battle = Battle::new(&duel(), 52);
    battle.set_controllers(vec![("red-1".into(), ControllerKind::DirectedAi)]);
    let ack = battle.accept_order(
        Side::Red,
        Some("red-1".into()),
        Order::Stop {
            units: vec![contract::ids::UnitId(1)],
        },
        false,
    );
    assert_eq!(ack.error, Some(OrderError::AiControlled));
}

#[test]
fn a_directed_seat_no_person_may_take_never_holds_preparation_open() {
    // Red is directed; `human_capable` says whether a person could take it.
    let phase_after_blue_readies = |red_human_capable: bool| {
        let mut setup = transport_skirmish_setup();
        let skirmish = setup.skirmish.as_mut().unwrap();
        skirmish.rules.preparation_s = 600;
        setup.players[1].controller = ControllerKind::DirectedAi;
        setup.players[1].human_capable = red_human_capable;
        let mut battle = Battle::new(&setup, 53);
        assert!(battle
            .accept(side_command(Side::Blue, 1, Order::Ready))
            .error
            .is_none());
        battle.step();
        battle.step();
        battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .map(|view| view.phase)
    };
    assert_eq!(
        phase_after_blue_readies(false),
        Some(Phase::Active),
        "the human's ready starts the battle long before the countdown"
    );
    // A person's seat the director plays (a solution) says ready itself,
    // as the person would.
    assert_eq!(phase_after_blue_readies(true), Some(Phase::Preparation));
}
