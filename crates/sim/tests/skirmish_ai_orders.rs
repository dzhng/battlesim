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

/// The world may refuse an AI's well-formed order: its order bucket is out
/// of a view the AI does not have. The refusal is receipted, and the battle
/// goes on (I09).
#[test]
fn a_rate_limited_ai_order_is_receipted_and_the_battle_goes_on() {
    let mut setup = crate::skirmish::setup();
    setup.players[0].controller = ControllerKind::ActiveAi;
    // One order a second: the opening decision's purchase spends it, so the
    // readiness behind it is refused.
    setup.rules.commands.orders_per_s = 1;
    let mut battle = Battle::new(&setup, 8);
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    battle.script_purchases("blue-1", vec![tank], None);
    battle.step();
    let refused: Vec<_> = battle
        .ai_receipts()
        .iter()
        .filter(|receipt| receipt.player == "blue-1" && !receipt.accepted)
        .collect();
    assert!(
        refused
            .iter()
            .any(|receipt| receipt.error == Some(OrderError::RateLimited)),
        "{:?}",
        battle.ai_receipts()
    );
}

/// A refusal is malformed when the order broke the command contract, which
/// the sender could have seen before sending it; the world's state decides
/// every other refusal, and an AI may meet it (I09).
#[test]
fn only_a_contract_breaking_refusal_is_malformed() {
    use contract::ids::UnitId;
    let unit = UnitId(0);
    for malformed in [
        OrderError::PassiveSide,
        OrderError::OutOfSequence { expected: 1 },
        OrderError::NotOwnUnit { unit },
        OrderError::WrongPlayer { unit },
        OrderError::AiControlled,
        OrderError::OutOfBounds,
        OrderError::UnknownTarget,
        OrderError::Destroyed { unit },
    ] {
        assert!(malformed.is_malformed(), "{malformed:?}");
    }
    for world in [
        OrderError::RateLimited,
        OrderError::NoValidDestination,
        OrderError::InsufficientCredits,
        OrderError::UnitLimitReached,
        OrderError::BuildingOccupied,
        OrderError::CapacityFull,
        OrderError::NoRoomToUnload,
        OrderError::MatchFinished,
    ] {
        assert!(!world.is_malformed(), "{world:?}");
    }
}
