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

fn setup() -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "tank",
        json!({
            "cost": 200,
            "roster": { "factions": ["us"], "category": "veh", "family_name": "Test tank", "variant": "Test" }
        }),
    );
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
                variant: "tank".into(),
                destination: [400.0, 300.0],
            },
        ));
        assert!(ack.error.is_none(), "{ack:?}");
    }
    let ack = battle.accept(command(
        Side::Blue,
        6,
        Order::ConfirmPurchase {
            variant: "tank".into(),
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
                variant: "tank".into(),
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
                variant: "tank".into(),
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
                    variant: "tank".into(),
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
            variant: "tank".into(),
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
            "tank",
            [400.0, 300.0],
            contract::command::OrderError::WrongFaction,
        ),
        (
            2,
            Side::Blue,
            "tank",
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
                    variant: "tank".into(),
                    destination: [400.0, 300.0]
                }
            ))
            .error
            .is_none());
    }
    assert_eq!(
        battle
            .accept(command(
                Side::Blue,
                31,
                Order::ConfirmPurchase {
                    variant: "tank".into(),
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
    let setup = setup();
    let commands = [
        command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "tank".into(),
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
    let saved: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-purchases.json"
    ))
    .unwrap();
    assert_eq!(record, saved);
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
