//! What the battle does with the orders an AI player's decision returns.

use contract::command::OrderError;
use contract::ids::Side;
use contract::players::ControllerKind;
use sim::battle::Battle;

#[test]
fn a_refused_purchase_stops_the_turns_later_purchases_but_not_its_other_orders() {
    let mut setup = crate::skirmish::setup();
    setup.players[1].controller = ControllerKind::ActiveAi;
    let mut battle = Battle::new(&setup, 7);
    // The deck's tank is U.S. only, so the Eastern player's battle refuses
    // the card its script names, and its other purchases with it.
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    battle.script_purchases("red-1", vec![tank, tank], None);
    let hz = battle.rules().tick_hz;
    for _ in 0..20 * hz {
        battle.step();
    }
    let receipts: Vec<_> = battle
        .ai_receipts()
        .iter()
        .filter(|receipt| receipt.player == "red-1")
        .collect();
    let refused: Vec<_> = receipts
        .iter()
        .filter(|receipt| !receipt.accepted)
        .collect();
    assert!(
        refused
            .iter()
            .all(|receipt| receipt.error == Some(OrderError::WrongFaction)),
        "{refused:?}"
    );
    // One refusal per decision: the second card is never issued behind
    // the first, and the first is tried again at the next decision.
    let decisions: std::collections::BTreeSet<_> =
        receipts.iter().map(|receipt| receipt.tick).collect();
    assert!(decisions.len() >= 2, "{receipts:?}");
    assert_eq!(refused.len(), decisions.len(), "{receipts:?}");
    // Readiness is not a purchase: it is still issued.
    let status = battle.observe(Side::Red).skirmish.clone().unwrap();
    let red = status.players.iter().find(|player| player.id == "red-1");
    assert!(red.unwrap().ready, "{receipts:?}");
}
