//! Recorded administrative events through the battle's public boundary.
use contract::command::{Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::players::{ControllerKind, PlayerIndex};
use contract::skirmish::PlayerView;
use sim::battle::{Battle, SeatError};

pub(super) fn setup() -> contract::scenario::ScenarioDefinition {
    let mut setup = crate::skirmish::setup();
    let mut receiver = setup.players[0].clone();
    receiver.id = "blue-2".into();
    receiver.controller = ControllerKind::PassiveAi;
    setup.players.push(receiver);
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 0;
    setup.skirmish.as_mut().unwrap().sites = sim::skirmish::synthetic_sites(
        setup.map.size,
        contract::players::entries_per_side(&setup.players),
    );
    setup
}

pub(super) fn order(
    battle: &mut Battle,
    player: &str,
    order: Order,
) -> contract::command::CommandAck {
    battle.accept_order(Side::Blue, Some(player.into()), order, false)
}

pub(super) fn seat<'a>(battle: &'a Battle, player: &str) -> &'a PlayerView {
    battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .players
        .iter()
        .find(|p| p.id == player)
        .unwrap()
}

pub(super) fn buy(battle: &mut Battle, player: &str, variant: &str) {
    assert_eq!(
        order(
            battle,
            player,
            Order::ConfirmPurchase {
                variant: variant.into(),
                destination: [400.0, 100.0]
            }
        )
        .error,
        None
    );
}

pub(super) fn entered(battle: &mut Battle, player: &str) -> UnitId {
    buy(battle, player, "test_tank");
    for _ in 0..600 {
        battle.step();
        if let Some(unit) = battle
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.owner == Some(PlayerIndex(0)))
        {
            return unit.id;
        }
    }
    panic!("purchase did not arrive");
}

#[test]
fn hand_over_changes_order_authority_and_preserves_the_side_wallet() {
    let mut battle = Battle::new(&setup(), 1);
    let unit = entered(&mut battle, "blue-1");
    let before = seat(&battle, "blue-1").credits + seat(&battle, "blue-2").credits;
    battle.hand_over("blue-1").unwrap();
    battle.step();
    assert_eq!(seat(&battle, "blue-1").credits, 0.0);
    let income = f64::from(setup().skirmish.unwrap().rules.credits_per_minute)
        / (60.0 * f64::from(battle.rules().tick_hz));
    let after = seat(&battle, "blue-1").credits + seat(&battle, "blue-2").credits;
    assert!((after - before - 2.0 * income).abs() < 0.000002);
    assert_eq!(seat(&battle, "blue-1").unit_count, 0);
    assert_eq!(seat(&battle, "blue-2").unit_count, 1);
    assert!(seat(&battle, "blue-2").credits >= before);
    assert!(
        order(&mut battle, "blue-1", Order::Stop { units: vec![unit] })
            .error
            .is_some()
    );
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    assert_eq!(
        order(&mut battle, "blue-2", Order::Stop { units: vec![unit] }).error,
        None
    );
    battle.step();
    assert_eq!(
        battle.observe(Side::Blue).own[0].owner,
        Some(PlayerIndex(2))
    );
}

#[test]
fn handed_over_refund_pays_receiver_and_keeps_the_purchase_history() {
    let mut battle = Battle::new(&setup(), 2);
    let unit = entered(&mut battle, "blue-1");
    let spent = battle.match_report().unwrap().totals[0].spent;
    assert!(spent > 0);
    battle.hand_over("blue-1").unwrap();
    battle.step();
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    assert_eq!(
        order(&mut battle, "blue-2", Order::Refund { units: vec![unit] }).error,
        None
    );
    let source = seat(&battle, "blue-1").credits;
    let receiver = seat(&battle, "blue-2").credits;
    for _ in 0..3000 {
        battle.step();
        if battle.observe(Side::Blue).own.iter().all(|u| u.id != unit) {
            break;
        }
    }
    assert!(!battle.unit(unit).unwrap().alive());
    let report = battle.match_report().unwrap();
    assert_eq!(report.totals[0].spent, spent);
    assert_eq!(report.totals[2].spent, 0);
    assert_eq!(report.totals[0].refunded, 0.0);
    let refund = report.totals[2].refunded;
    assert!(refund > 0.0);
    assert!(
        (seat(&battle, "blue-2").credits
            - receiver
            - (seat(&battle, "blue-1").credits - source)
            - refund)
            .abs()
            < 0.000001
    );
    assert!(report.events.iter().any(|e| matches!(e, contract::skirmish::MatchEvent::Spawn { unit: u, .. } if u.id == unit && u.owner == Some(PlayerIndex(0)))));
    assert!(report.events.iter().any(|e| matches!(e, contract::skirmish::MatchEvent::Refund { unit: u, .. } if u.id == unit && u.owner == Some(PlayerIndex(2)))));
}

#[test]
fn pending_purchases_are_renumbered_without_cancelling_another_seats_card() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 60;
    let mut third = setup.players[0].clone();
    third.id = "blue-3".into();
    setup.players.push(third);
    setup.skirmish.as_mut().unwrap().sites = sim::skirmish::synthetic_sites(
        setup.map.size,
        contract::players::entries_per_side(&setup.players),
    );
    let mut battle = Battle::new(&setup, 3);
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    buy(&mut battle, "blue-1", "test_tank");
    buy(&mut battle, "blue-2", "test_supply");
    buy(&mut battle, "blue-3", "test_supply");
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::PassiveAi)]);
    battle.hand_over("blue-1").unwrap();
    battle.step();
    let pending = seat(&battle, "blue-2").pending.clone();
    assert_eq!(pending.len(), 2);
    assert_ne!(pending[0].id, pending[1].id);
    assert_eq!(
        pending[1].kind,
        battle.rules().catalog.index("test_tank").unwrap()
    );
    assert!(seat(&battle, "blue-1").pending.is_empty());
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    let wallet = seat(&battle, "blue-2").credits;
    assert_eq!(
        order(
            &mut battle,
            "blue-2",
            Order::CancelPending {
                purchase: pending[1].id
            }
        )
        .error,
        None
    );
    battle.step();
    assert_eq!(seat(&battle, "blue-2").pending, vec![pending[0].clone()]);
    let price = battle.rules().catalog.get(pending[1].kind).cost;
    assert_eq!(seat(&battle, "blue-2").credits, wallet + f64::from(price));
    // Both remaining seats have their own purchase zero. The receiver's
    // cancellation must reach its own card and leave the other one alone.
    assert_eq!(
        order(
            &mut battle,
            "blue-2",
            Order::CancelPending {
                purchase: pending[0].id
            }
        )
        .error,
        None
    );
    battle.step();
    assert!(seat(&battle, "blue-2").pending.is_empty());
    assert_eq!(seat(&battle, "blue-3").pending.len(), 1);
    assert_eq!(seat(&battle, "blue-3").pending[0].kind, pending[0].kind);
}

pub(super) fn assemble(battle: &mut Battle, purchases: &[(&str, usize)]) {
    let mut expected = 0;
    for (player, count) in purchases {
        battle.set_controllers(vec![((*player).into(), ControllerKind::Human)]);
        for _ in 0..*count {
            buy(battle, player, "test_tank");
            battle.step();
            expected += 1;
        }
    }
    for _ in 0..3000 {
        if battle.observe(Side::Blue).own.len() == expected {
            return;
        }
        battle.step();
    }
    panic!(
        "not all purchases entered: {:?}",
        battle.observe(Side::Blue).skirmish
    );
}

#[test]
fn the_receiver_is_the_least_populated_ai_with_seat_order_breaking_a_tie() {
    let mut setup = setup();
    for (id, controller) in [
        ("blue-3", ControllerKind::PassiveAi),
        ("blue-4", ControllerKind::ActiveAi),
        ("blue-5", ControllerKind::Human),
    ] {
        let mut player = setup.players[0].clone();
        player.id = id.into();
        player.controller = controller;
        setup.players.push(player);
    }
    setup.skirmish.as_mut().unwrap().sites = sim::skirmish::synthetic_sites(
        setup.map.size,
        contract::players::entries_per_side(&setup.players),
    );
    for p in &mut setup.players {
        p.controller = ControllerKind::Human;
    }
    let mut battle = Battle::new(&setup, 4);
    assemble(
        &mut battle,
        &[("blue-1", 1), ("blue-2", 2), ("blue-3", 1), ("blue-4", 1)],
    );
    battle.set_controllers(vec![
        ("blue-2".into(), ControllerKind::PassiveAi),
        ("blue-3".into(), ControllerKind::PassiveAi),
        ("blue-4".into(), ControllerKind::ActiveAi),
    ]);
    // Keep the active AI's shopping empty: population, not its wallet or
    // purchase plan, chooses the receiver. The human with zero must be skipped.
    battle.script_purchases("blue-4", vec![], None);
    battle.hand_over("blue-1").unwrap();
    battle.step();
    assert_eq!(
        battle.unit(UnitId(0)).unwrap().owner.as_deref(),
        Some("blue-3")
    );
    assert_eq!(seat(&battle, "blue-1").unit_count, 0);
    assert_eq!(seat(&battle, "blue-5").unit_count, 0);
    assert_eq!(seat(&battle, "blue-3").unit_count, 2);
}

#[test]
fn a_pending_purchase_arrives_under_its_receiver_but_its_buyer_keeps_the_spending() {
    let mut battle = Battle::new(&setup(), 5);
    buy(&mut battle, "blue-1", "test_tank");
    buy(&mut battle, "blue-1", "test_tank");
    battle.hand_over("blue-1").unwrap();
    for _ in 0..600 {
        battle.step();
        if battle.observe(Side::Blue).own.len() == 2 {
            break;
        }
    }
    assert_eq!(battle.observe(Side::Blue).own.len(), 2);
    assert!(battle
        .observe(Side::Blue)
        .own
        .iter()
        .all(|u| u.owner == Some(PlayerIndex(2))));
    assert!(seat(&battle, "blue-1").pending.is_empty());
    assert!(seat(&battle, "blue-2").pending.is_empty());
    let report = battle.match_report().unwrap();
    assert_eq!(report.totals[0].spawned, 2);
    assert_eq!(report.totals[2].spawned, 0);
    assert_eq!(report.totals[2].spent, 0);
    assert!(report.totals[0].spent > 0);
    assert!(report.events.iter().all(|e| matches!(e,contract::skirmish::MatchEvent::Spawn { unit, .. } if unit.owner == Some(PlayerIndex(0)))));
}

#[test]
fn a_receiver_keeps_an_over_cap_army_and_can_buy_only_after_combat_losses() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.max_units = 2;
    let mut deck = crate::skirmish::deck();
    sim::fixtures::patch_catalog(
        &mut deck,
        "units",
        "test_tank",
        serde_json::json!({"cost":1,"roster":{"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"},"body":{"hull":{"hp":0.01,"armor":{"front":0,"side":0,"rear":0,"roof":0,"ricochet":{"front":0,"side":0,"rear":0,"roof":0}}}}}),
    );
    deck["catalog"].as_array_mut().unwrap().push(serde_json::json!({"units":{"test_seat_attacker":{"extends":"test_tank","body":{"hull":{"hp":1000000}}}}}));
    setup.rules = serde_json::from_value(deck).unwrap();
    let mut battle = Battle::new(&setup, 6);
    assemble(&mut battle, &[("blue-1", 2), ("blue-2", 1)]);
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::PassiveAi)]);
    battle.hand_over("blue-1").unwrap();
    battle.step();
    assert_eq!(seat(&battle, "blue-2").unit_count, 3);
    assert_eq!(battle.observe(Side::Blue).own.len(), 3);
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    let purchase = || Order::ConfirmPurchase {
        variant: "test_tank".into(),
        destination: [400.0, 100.0],
    };
    assert_eq!(
        order(&mut battle, "blue-2", purchase()).error,
        Some(OrderError::UnitLimitReached)
    );
    assert_eq!(
        battle
            .accept_order(
                Side::Red,
                Some("red-1".into()),
                Order::ConfirmPurchase {
                    variant: "test_seat_attacker".into(),
                    destination: [400.0, 100.0]
                },
                false
            )
            .error,
        None
    );
    for _ in 0..3000 {
        battle.step();
        if seat(&battle, "blue-2").unit_count < seat(&battle, "blue-2").cap {
            break;
        }
    }
    assert!(seat(&battle, "blue-2").unit_count < seat(&battle, "blue-2").cap);
    assert_eq!(order(&mut battle, "blue-2", purchase()).error, None);
    assert!(battle.match_report().unwrap().events.iter().any(|e| matches!(e, contract::skirmish::MatchEvent::Kill {victim,..} if victim.owner == Some(PlayerIndex(2)))));
}

#[test]
fn refused_hand_overs_change_neither_the_state_nor_the_replay() {
    let mut battle = Battle::new(&setup(), 7);
    let before = (battle.digest(), battle.replay());
    assert_eq!(battle.hand_over("stranger"), Err(SeatError::MissingPlayer));
    assert_eq!((battle.digest(), battle.replay()), before);
    battle.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    let before = (battle.digest(), battle.replay());
    assert_eq!(battle.hand_over("blue-1"), Err(SeatError::NoReceiver));
    assert_eq!((battle.digest(), battle.replay()), before);
    let mut replay = Battle::from_replay(&setup(), &battle.replay()).unwrap();
    let before = (replay.digest(), replay.replay());
    assert_eq!(replay.hand_over("blue-1"), Err(SeatError::ReplayInProgress));
    assert_eq!((replay.digest(), replay.replay()), before);
}
