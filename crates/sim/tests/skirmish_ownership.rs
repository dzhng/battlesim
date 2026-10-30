#![allow(clippy::approx_constant)]

use crate::common::Seat;
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;

fn setup() -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost": 200,
            "roster": {"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"}
        }),
    );
    serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [],
        "players": [
            {"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},
            {"id":"blue-2","side":"blue","faction":"us","controller":"human","human_capable":true},
            {"id":"red-1","side":"red","faction":"eastern","controller":"passive_ai"},
            {"id":"red-2","side":"red","faction":"eastern","controller":"passive_ai"}
        ],
        "skirmish": {
            "rules": {"credits_per_minute":200,"starting_minutes":5,"preparation_s":60,"max_units":30,"max_players_per_side":10,"dispatch_interval_s":1,"capture_s":20,"majority_s":1800,"victory_score":1000},
            "sites": {"entries":[
                {"side":"blue","center":[250,10],"yaw":1.5707963267948966},
                {"side":"blue","center":[550,10],"yaw":1.5707963267948966},
                {"side":"red","center":[250,590],"yaw":-1.5707963267948966},
                {"side":"red","center":[550,590],"yaw":-1.5707963267948966}
            ],"objectives":[
                {"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null}
            ,{"id":"north","center":[600,400],"radius_m":50,"kind":"field","counterpart":"south"},
            {"id":"south","center":[200,200],"radius_m":50,"kind":"field","counterpart":"north"}]}
        }
    })).unwrap()
}

fn command(side: Side, player: &str, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        player: Some(player.into()),
        seq,
        order,
        queued: false,
    }
}

#[test]
fn client_orders_for_ai_skirmish_players_wait_for_human_takeover() {
    let mut setup = setup();
    setup.players[1].controller = contract::players::ControllerKind::ActiveAi;
    let mut battle = Battle::new(&setup, 1);

    let refused = battle.accept(command(Side::Blue, "blue-2", 1, Order::Ready));
    assert_eq!(refused.error, Some(OrderError::AiControlled));
    assert!(battle.replay().accepted.is_empty());

    let human = battle.accept(command(Side::Blue, "blue-1", 1, Order::Ready));
    assert_eq!(human.error, None);

    // The AI holds the seat for a while and spends its sequence numbers; the
    // human who takes it over continues from where the battle says.
    for _ in 0..battle.rules().tick_hz {
        battle.step();
    }
    battle.set_controllers(vec![(
        "blue-2".into(),
        contract::players::ControllerKind::Human,
    )]);
    let seq = match battle
        .accept(command(Side::Blue, "blue-2", 0, Order::Ready))
        .error
    {
        Some(OrderError::OutOfSequence { expected }) => expected,
        other => panic!("a human seat is sequenced, not refused: {other:?}"),
    };
    let order = command(Side::Blue, "blue-2", seq, Order::Ready);
    assert_eq!(battle.accept(order.clone()).error, None);
    assert!(battle
        .replay()
        .accepted
        .iter()
        .any(|(_, recorded)| *recorded == order));
}

#[test]
fn players_sequence_orders_independently_in_the_same_tick() {
    let mut battle = Battle::new(&setup(), 1);
    for (player, goal) in [("blue-1", [350.0, 300.0]), ("blue-2", [450.0, 300.0])] {
        let ack = battle.accept(command(
            Side::Blue,
            player,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: goal,
            },
        ));
        assert_eq!(
            ack.error, None,
            "{player} should have an independent sequence"
        );
    }
}

#[test]
fn an_ally_cannot_ready_or_cancel_another_players_purchase() {
    let mut battle = Battle::new(&setup(), 1);
    let ack = battle.accept(command(
        Side::Blue,
        "blue-1",
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 300.0],
        },
    ));
    assert_eq!(ack.error, None);
    battle.step();
    let purchase = battle.observe(Side::Blue).seat().pending[0].id;
    let cancel = battle.accept(command(
        Side::Blue,
        "blue-2",
        1,
        Order::CancelPending { purchase },
    ));
    assert!(cancel.error.is_some());
}

#[test]
fn one_player_ready_does_not_start_the_match() {
    let mut battle = Battle::new(&setup(), 1);
    assert_eq!(
        battle
            .accept(command(Side::Blue, "blue-1", 1, Order::Ready))
            .error,
        None
    );
    battle.step();
    assert_eq!(
        battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase,
        contract::skirmish::Phase::Preparation
    );
}

#[test]
fn each_players_cap_is_independent() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.credits_per_minute = 10_000;
    setup.rules.commands.orders_per_s = 1_000;
    let mut battle = Battle::new(&setup, 1);
    for player in ["blue-1", "blue-2"] {
        for seq in 1..=30 {
            let ack = battle.accept(command(
                Side::Blue,
                player,
                seq,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination: [400.0, 300.0],
                },
            ));
            assert_eq!(ack.error, None, "{player} purchase {seq}");
        }
    }
    assert_eq!(
        battle
            .accept(command(
                Side::Blue,
                "blue-1",
                31,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination: [400.0, 300.0],
                }
            ))
            .error,
        Some(contract::command::OrderError::UnitLimitReached)
    );
    battle.step();
    assert_eq!(battle.observe(Side::Blue).seat().pending.len(), 30);
}

#[test]
fn a_two_player_replay_keeps_tick_zero_controllers_and_digest() {
    let mut setup = setup();
    for player in setup
        .players
        .iter_mut()
        .filter(|player| player.side == Side::Red)
    {
        player.controller = contract::players::ControllerKind::Human;
        player.human_capable = true;
    }
    let mut battle = Battle::new(&setup, 7);
    for (player, seq) in [("blue-1", 1), ("blue-2", 1), ("red-1", 1), ("red-2", 1)] {
        assert_eq!(
            battle
                .accept(command(
                    if player.starts_with("blue") {
                        Side::Blue
                    } else {
                        Side::Red
                    },
                    player,
                    seq,
                    Order::Ready
                ))
                .error,
            None
        );
    }
    battle.step();
    let digest = battle.digest();
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    replay.step();
    assert_eq!(replay.replay().controller_transitions.len(), 4);
    assert_eq!(replay.digest(), digest);
}
