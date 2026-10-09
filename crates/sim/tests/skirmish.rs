#![allow(clippy::approx_constant)]

use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;

fn command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        seq,
        order,
        queued: false,
    }
}

/// A match on the test units, with a U.S. deck of a tank and a supply truck.
pub(crate) fn setup() -> contract::scenario::ScenarioDefinition {
    match_on(deck())
}

/// The test units, with a U.S. deck of a tank and a supply truck.
fn deck() -> serde_json::Value {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost": 200,
            "roster": { "factions": ["us"], "category": "veh", "family_name": "Test tank", "variant": "Test" }
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({
            "roster": { "factions": ["us"], "category": "sup", "family_name": "Test truck", "variant": "Test" }
        }),
    );
    rules
}

/// A U.S. against Eastern match on `rules`.
fn match_on(rules: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "skirmish": {
            "factions": ["us", "eastern"],
            "rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 60, "max_units": 30, "dispatch_interval_s": 1 },
            "sites": { "entries": [
                { "side": "blue", "center": [400, 10], "yaw": 1.5707963267948966 },
                { "side": "red", "center": [400, 590], "yaw": -1.5707963267948966 }
            ], "objectives": [
                { "id": "center", "center": [400, 300], "radius_m": 50, "kind": "junction", "counterpart": null },
                { "id": "south", "center": [200, 200], "radius_m": 50, "kind": "field", "counterpart": "north" },
                { "id": "north", "center": [600, 400], "radius_m": 50, "kind": "field", "counterpart": "south" }
            ] }
        }
    })).unwrap()
}

#[test]
fn preparation_starts_empty_and_freezes_the_credit_wallet() {
    let mut battle = Battle::new(&setup(), 1);
    let hz = battle.rules().tick_hz;
    for _ in 0..30 * hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert!(frame.own.is_empty());
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.phase, contract::skirmish::Phase::Preparation);
    assert_eq!(status.credits, 1000.0);
    assert_eq!(status.occupied_slots, 0);
}

#[test]
fn both_ready_start_early_and_income_adds_exactly_two_hundred_per_minute() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    for side in Side::ALL {
        assert!(battle
            .accept(command(side, 1, Order::Ready))
            .error
            .is_none());
    }
    battle.step();
    assert_eq!(
        battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase,
        contract::skirmish::Phase::Active
    );
    for _ in 0..60 * battle.rules().tick_hz {
        battle.step();
    }
    for side in Side::ALL {
        assert_eq!(
            battle.observe(side).skirmish.as_ref().unwrap().credits,
            1200.0
        );
    }
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}

#[test]
fn confirmed_purchases_reserve_credits_and_slots_atomically_during_preparation() {
    let mut battle = Battle::new(&setup(), 1);
    for seq in 1..=5 {
        let ack = battle.accept(command(
            Side::Blue,
            seq,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 300.0],
            },
        ));
        assert!(ack.error.is_none(), "{ack:?}");
    }
    let ack = battle.accept(command(
        Side::Blue,
        6,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 300.0],
        },
    ));
    assert_eq!(
        ack.error,
        Some(contract::command::OrderError::InsufficientCredits)
    );
    battle.step();
    let frame = battle.observe(Side::Blue);
    assert!(frame.own.is_empty());
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.credits, 0.0);
    assert_eq!(status.occupied_slots, 5);
    assert_eq!(status.pending.len(), 5);
    assert!(status
        .pending
        .iter()
        .all(|p| p.destination == [400.0, 300.0]));
}

#[test]
fn cancelling_a_pending_purchase_returns_the_price_and_slot_once() {
    let mut battle = Battle::new(&setup(), 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 300.0]
            }
        ))
        .error
        .is_none());
    battle.step();
    let id = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .pending[0]
        .id;
    for seq in 2..=3 {
        assert!(battle
            .accept(command(
                Side::Blue,
                seq,
                Order::CancelPending { purchase: id }
            ))
            .error
            .is_none());
    }
    battle.step();
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.credits, 1000.0);
    assert_eq!(status.occupied_slots, 0);
    assert!(status.pending.is_empty());
}

#[test]
fn paid_reinforcement_enters_at_base_once_and_moves_to_its_destination() {
    let mut battle = Battle::new(&setup(), 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 300.0]
            }
        ))
        .error
        .is_none());
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    battle.step();
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    assert!(
        frame.own[0].position[1] < 15.0,
        "entry is physical, not at the selected destination"
    );
    assert_eq!(frame.own[0].goal, Some([400.0, 300.0]));
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.credits, 800.0);
    assert_eq!(status.occupied_slots, 1);
    assert!(status.pending.is_empty());
    for _ in 0..5 * battle.rules().tick_hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    assert!(frame.own[0].position[1] > 15.0);
}

#[test]
fn occupied_entry_keeps_the_next_paid_unit_pending_without_overlap() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    for seq in 1..=2 {
        assert!(battle
            .accept(command(
                Side::Blue,
                seq,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination: [400.0, 10.0]
                }
            ))
            .error
            .is_none());
    }
    battle.accept(command(Side::Blue, 3, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..2 * battle.rules().tick_hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    assert_eq!(frame.own[0].position[0], 400.0);
    assert!((frame.own[0].position[1] - 10.0).abs() < 1.0);
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.occupied_slots, 2);
    assert_eq!(status.pending.len(), 1);
    assert!(status.pending[0].blocked);
    assert_eq!(status.pending[0].id.0, 1);
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}

#[test]
fn packed_match_header_exposes_only_the_observed_wallet_and_pending_count() {
    let mut battle = Battle::new(&setup(), 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 300.0],
        },
    ));
    battle.step();
    let layout: serde_json::Value =
        serde_json::from_str(&sim::publication::layout_json(&battle)).unwrap();
    let fields = layout["header"].as_array().unwrap();
    for (side, credits, pending) in [(Side::Blue, 800.0, 1.0), (Side::Red, 1000.0, 0.0)] {
        let mut values = Vec::new();
        let patch = sim::publication::GroundHeader {
            epoch: 0,
            side,
            base: 0,
            revision: 0,
            full: true,
            count: 0,
        };
        sim::publication::pack_logical(
            battle.observe(side),
            &patch,
            &sim::publication::FogPatch {
                full: true,
                base: 0,
                revision: 0,
                changed: &[],
            },
            std::iter::empty(),
            &mut values,
        )
        .unwrap();
        let read = |name: &str| {
            values[fields
                .iter()
                .position(|f| f.as_str() == Some(name))
                .expect("match header field")]
        };
        assert_eq!(read("credits"), credits);
        assert_eq!(read("pendingCount"), pending);
        assert_eq!(read("skirmishPhase"), 0.0);
    }
}

#[test]
fn a_purchase_preview_spreads_a_squad_round_its_destination_and_a_hull_stands_alone() {
    let mut rules = deck();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut battle = Battle::new(&match_on(rules), 1);
    let squad = battle
        .preview_purchase(Side::Blue, "test_rifle", [400.0, 300.0])
        .unwrap();
    let slots = battle
        .rules()
        .catalog
        .get(squad.kind)
        .slots()
        .unwrap()
        .len();
    assert!(slots > 1);
    assert_eq!(
        squad.spots.len(),
        slots,
        "one spot per soldier the squad fields"
    );
    assert!(squad
        .spots
        .iter()
        .all(|s| (s[0] - 400.0).hypot(s[1] - 300.0) < 15.0));
    let tank = battle
        .preview_purchase(Side::Blue, "test_tank", [400.0, 300.0])
        .unwrap();
    assert!(tank.spots.is_empty(), "a vehicle stands as one body");
}

#[test]
fn purchase_admission_refuses_unavailable_wrong_faction_and_invalid_destination_atomically() {
    let mut battle = Battle::new(&setup(), 1);
    for (seq, side, variant, destination, error) in [
        (
            1,
            Side::Blue,
            "missing-unit",
            [400.0, 300.0],
            contract::command::OrderError::UnitUnavailable,
        ),
        (
            1,
            Side::Red,
            "test_tank",
            [400.0, 300.0],
            contract::command::OrderError::WrongFaction,
        ),
        (
            2,
            Side::Blue,
            "test_tank",
            [-10.0, 300.0],
            contract::command::OrderError::OutOfBounds,
        ),
    ] {
        assert_eq!(
            battle
                .accept(command(
                    side,
                    seq,
                    Order::ConfirmPurchase {
                        variant: variant.into(),
                        destination
                    }
                ))
                .error,
            Some(error)
        );
    }
    battle.step();
    for side in Side::ALL {
        let status = battle.observe(side).skirmish.as_ref().unwrap();
        assert_eq!(status.credits, 1000.0);
        assert_eq!(status.occupied_slots, 0);
        assert!(status.pending.is_empty());
    }
}

#[test]
fn thirty_pending_units_fill_the_cap_even_when_credits_remain() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.credits_per_minute = 2000;
    let mut battle = Battle::new(&setup, 1);
    for seq in 1..=30 {
        assert!(battle
            .accept(command(
                Side::Blue,
                seq,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination: [400.0, 300.0]
                }
            ))
            .error
            .is_none());
    }
    // A tick on, as a player's next click would land: the rate allows one
    // more order, and the unit cap is what refuses it.
    battle.step();
    assert_eq!(
        battle
            .accept(command(
                Side::Blue,
                31,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination: [400.0, 300.0]
                }
            ))
            .error,
        Some(contract::command::OrderError::UnitLimitReached)
    );
    battle.step();
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.credits, 4000.0);
    assert_eq!(status.occupied_slots, 30);
    assert_eq!(status.pending.len(), 30);
}

#[test]
fn purchase_commands_have_a_paired_native_browser_record() {
    let saved: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-purchases.json"
    ))
    .unwrap();
    let setup = serde_json::from_value(saved["scenario"].clone()).unwrap();
    let commands = [
        command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 300.0],
            },
        ),
        command(Side::Blue, 2, Order::Ready),
        command(Side::Red, 1, Order::Ready),
    ];
    let mut battle = Battle::new(&setup, 1);
    for command in &commands {
        assert!(battle.accept(command.clone()).error.is_none());
    }
    for _ in 0..120 {
        battle.step();
    }
    let record = json!({ "scenario": setup, "seed": 1, "commands": commands, "ticks": 120, "digest": format!("{:016x}", battle.digest()) });
    if crate::common::bless_parity("skirmish-purchases.json", &record) {
        return;
    }
    assert_eq!(
        serde_json::to_string(&record).unwrap(),
        include_str!("../../../fixtures/parity/skirmish-purchases.json").trim()
    );
}

#[test]
fn preparation_timeout_starts_combat_without_granting_preparation_income() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 2;
    let mut battle = Battle::new(&setup, 1);
    let hz = battle.rules().tick_hz;
    for _ in 0..2 * hz - 1 {
        battle.step();
    }
    assert_eq!(
        battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase,
        contract::skirmish::Phase::Preparation
    );
    battle.step();
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.phase, contract::skirmish::Phase::Active);
    assert_eq!(status.credits, 1000.0);
    for _ in 0..3 * hz {
        battle.step();
    }
    assert_eq!(
        battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .unwrap()
            .credits,
        1010.0
    );
}

/// A match against the basic opponent on red, whose deck holds squads, a
/// launcher team and a truck.
fn against_the_basic_opponent() -> contract::scenario::ScenarioDefinition {
    let mut rules = deck();
    for (id, category) in [
        ("test_rifle", "inf"),
        ("test_at", "inf"),
        ("test_supply", "sup"),
    ] {
        rules["catalog"].as_array_mut().unwrap().push(json!({ "units": { format!("red_{id}"): {
            "extends": id,
            "roster": { "factions": ["eastern"], "category": category, "family_name": "Test", "variant": "Test" }
        } } }));
    }
    let mut setup = match_on(rules);
    setup.skirmish.as_mut().unwrap().ai_side = Some(Side::Red);
    setup
}

#[test]
fn the_player_readying_starts_the_battle_against_the_basic_opponent_at_once() {
    let mut battle = Battle::new(&against_the_basic_opponent(), 3);
    battle.step();
    assert!(battle
        .accept(command(Side::Blue, 1, Order::Ready))
        .error
        .is_none());
    battle.step();
    battle.step();
    let red = battle.observe(Side::Red).skirmish.clone().unwrap();
    assert_eq!(red.phase, contract::skirmish::Phase::Active);
    assert!(
        red.pending.len() > 1,
        "the opponent bought its whole opening before readying"
    );
}

#[test]
fn the_basic_opponent_buys_from_zero_through_recorded_commands_and_replays_every_tick() {
    let setup = against_the_basic_opponent();
    let mut live = Battle::new(&setup, 3);
    let mut digests = Vec::new();
    for _ in 0..45 * live.rules().tick_hz {
        live.step();
        digests.push(live.digest());
    }
    let red = live.observe(Side::Red).skirmish.as_ref().unwrap();
    assert!(
        red.credits < 1000.0,
        "the AI spends its ordinary opening wallet"
    );
    assert!(red.occupied_slots > 0 && red.occupied_slots <= 30);
    let record = live.replay();
    assert!(record
        .accepted
        .iter()
        .any(|(_, c)| c.side == Side::Red && matches!(c.order, Order::ConfirmPurchase { .. })));
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest, "tick {}", replay.tick());
    }
}

#[test]
fn a_refund_orders_a_vulnerable_return_and_pays_only_at_base() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 150.0]
            }
        ))
        .error
        .is_none());
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let before = battle.observe(Side::Blue).clone();
    let unit = before.own[0].id;
    assert!(before.own[0].position[1] > 80.0);
    let start = battle.tick();
    assert!(battle
        .accept(command(Side::Blue, 3, Order::Refund { units: vec![unit] }))
        .error
        .is_none());
    battle.step();
    assert_eq!(
        battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .unwrap()
            .occupied_slots,
        1
    );
    assert!(
        (battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .unwrap()
            .credits
            - before.skirmish.as_ref().unwrap().credits)
            < 1.0
    );
    for _ in 0..1000 {
        if battle.observe(Side::Blue).own.is_empty() {
            break;
        }
        battle.step();
    }
    let returned = battle.observe(Side::Blue).clone();
    assert!(returned.own.is_empty(), "the unit physically reached base");
    assert_eq!(returned.skirmish.as_ref().unwrap().occupied_slots, 0);
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    let refund = returned.skirmish.as_ref().unwrap().credits
        - before.skirmish.as_ref().unwrap().credits
        - passive;
    let expected = 200.0
        * sim::withdrawal::fraction(
            1.0,
            1.0,
            (battle.tick() - 1) as f64 / f64::from(battle.rules().tick_hz),
        );
    assert!(
        (refund - expected).abs() < 0.00001,
        "refund {refund}, expected {expected}"
    );
    assert!(returned.corpses.is_empty(), "retirement leaves no casualty");
    let retired_credits = returned.skirmish.as_ref().unwrap().credits;
    for _ in 0..300 {
        battle.step();
    }
    assert!(
        (battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .unwrap()
            .credits
            - retired_credits
            - 200.0 / 6.0)
            .abs()
            < 0.00001,
        "retirement pays once"
    );
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}

#[test]
fn a_new_order_cancels_withdrawal_without_a_refund() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 150.0],
        },
    ));
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    battle.accept(command(Side::Blue, 3, Order::Refund { units: vec![id] }));
    battle.step();
    assert!(battle.observe(Side::Blue).own[0].withdrawing);
    let credits = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    battle.accept(command(Side::Blue, 4, Order::Stop { units: vec![id] }));
    for _ in 0..450 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(!status.own[0].withdrawing);
    assert_eq!(status.skirmish.as_ref().unwrap().occupied_slots, 1);
    assert!((status.skirmish.as_ref().unwrap().credits - credits - 50.0).abs() < 0.00001);
}

#[test]
fn a_destroyed_withdrawing_unit_never_refunds_or_retires() {
    let mut setup = setup();
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost":200,"roster":{"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"},
            "body":{"hull":{"hp":0.01}},"sensors":{"ground_m":500},"mobility":{"tracked":{"offroad_kmh":5,"road_kmh":5}}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.skirmish.as_mut().unwrap().sites.entries[1].center = [400.0, 110.0];
    let mut battle = Battle::new(&setup, 7);
    for (side, destination) in [(Side::Blue, [400.0, 150.0]), (Side::Red, [400.0, 110.0])] {
        assert!(battle
            .accept(command(
                side,
                1,
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    destination
                }
            ))
            .error
            .is_none());
        battle.accept(command(side, 2, Order::Ready));
    }
    battle.step();
    for side in Side::ALL {
        let id = battle.observe(side).own[0].id;
        battle.accept(command(
            side,
            3,
            Order::SetEngagement {
                units: vec![id],
                policy: contract::command::Engagement::ReturnFireOnly,
            },
        ));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    let enemy = battle.observe(Side::Red).own[0].id;
    let before = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    let start = battle.tick();
    battle.accept(command(Side::Blue, 4, Order::Refund { units: vec![id] }));
    battle.accept(command(
        Side::Red,
        4,
        Order::Attack {
            units: vec![enemy],
            target: contract::command::TargetRef::Identified {
                id: battle.observe(Side::Red).identified[0].id,
            },
        },
    ));
    for _ in 0..1000 {
        battle.step();
    }
    assert!(!battle.unit(id).unwrap().alive());
    assert!(
        !battle.unit(id).unwrap().retired,
        "combat destruction must not become a retirement"
    );
    let after = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!(
        (after - before - passive).abs() < 0.00001,
        "no refund for a destroyed unit"
    );
}

#[test]
fn refund_condition_counts_casualties_lost_ammunition_carriers_and_empty_trucks() {
    let mut setup = crate::common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([{"side":"blue","kind":"test_rifle","position":[100,200]}, {"side":"blue","kind":"test_supply","position":[200,200]}]),
        json!([]),
        json!([]),
    );
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_grenadier",
        json!({"mounts":[{"id":"grenade launcher","weapons":["grenade"],"special":false}]}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let battle = Battle::new(&setup, 1);
    let mut squad = battle.unit(contract::ids::UnitId(0)).unwrap().clone();
    let mut truck = battle.unit(contract::ids::UnitId(1)).unwrap().clone();
    let condition = |unit: &sim::units::Unit| {
        sim::withdrawal::condition(unit, battle.rules(), battle.arsenal())
    };
    assert_eq!(condition(&squad), (1.0, 1.0));
    squad.members[0].hp = 0.0;
    assert_eq!(condition(&squad),(7.0/8.0,0.0),"dead grenadier retains neither original health nor usable finite grenades; unlimited rifles do not dilute ammo");
    truck.hp = truck.max_hp(battle.rules()) / 2.0;
    truck.stock = Some(0);
    assert_eq!(condition(&truck), (0.5, 0.0));
}

#[test]
fn changing_the_return_route_cancels_withdrawal_without_a_refund() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 150.0],
        },
    ));
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    battle.accept(command(Side::Blue, 3, Order::Refund { units: vec![id] }));
    battle.step();
    assert!(battle.observe(Side::Blue).own[0].withdrawing);
    let credits = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    battle.accept(command(
        Side::Blue,
        4,
        Order::UpgradeMove {
            gesture: 0,
            route: contract::command::RoutePolicy::Shortest,
        },
    ));
    for _ in 0..450 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(!status.own[0].withdrawing);
    assert_eq!(status.skirmish.as_ref().unwrap().occupied_slots, 1);
    assert!((status.skirmish.as_ref().unwrap().credits - credits - 50.0).abs() < 0.00001);
}

#[test]
fn an_occupied_base_keeps_the_returning_unit_and_its_slot_without_payment() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 150.0],
        },
    ));
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let returning = battle.observe(Side::Blue).own[0].id;
    battle.accept(command(
        Side::Blue,
        3,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 10.0],
        },
    ));
    for _ in 0..60 {
        battle.step();
    }
    assert_eq!(battle.observe(Side::Blue).own.len(), 2);
    let before = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    let start = battle.tick();
    battle.accept(command(
        Side::Blue,
        4,
        Order::Refund {
            units: vec![returning],
        },
    ));
    for _ in 0..1500 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(
        status
            .own
            .iter()
            .any(|unit| unit.id == returning && unit.withdrawing),
        "occupied physical entry must keep the return pending"
    );
    assert_eq!(status.skirmish.as_ref().unwrap().occupied_slots, 2);
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!((status.skirmish.as_ref().unwrap().credits - before - passive).abs() < 0.00001);
}
