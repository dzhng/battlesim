use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use sim::battle::Battle;

fn command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        seq,
        order,
        queued: false,
    }
}

#[test]
fn an_uncontested_ground_unit_captures_after_twenty_seconds_and_scores_without_credit_income() {
    let mut setup = super::skirmish::setup();
    setup.skirmish.as_mut().unwrap().sites.objectives[0].center = [400.0, 10.0];
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 10.0],
        },
    ));
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    let hz = battle.rules().tick_hz;
    for _ in 0..20 * hz - 1 {
        battle.step();
    }
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.objectives[0].owner, None);
    assert_eq!(status.scores, [0.0, 0.0]);
    battle.step();
    assert_eq!(
        battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .unwrap()
            .objectives[0]
            .owner,
        Some(Side::Blue)
    );
    for _ in 0..36 * hz {
        battle.step();
    }
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert!((status.scores[0] - 10.0).abs() < 1e-9);
    assert_eq!(status.scores[1], 0.0);
    assert!((status.credits - (800.0 + (56.0 - 1.0 / hz as f64) * 200.0 / 60.0)).abs() < 1e-6);
}

#[test]
fn a_resupply_truck_cannot_capture_or_contest_a_flag() {
    let mut setup = super::skirmish::setup();
    setup.skirmish.as_mut().unwrap().sites.objectives[0].center = [400.0, 10.0];
    let mut battle = Battle::new(&setup, 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_supply".into(),
                destination: [400.0, 10.0]
            }
        ))
        .error
        .is_none());
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..30 * battle.rules().tick_hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    let flag = &frame.skirmish.as_ref().unwrap().objectives[0];
    assert_eq!(flag.owner, None);
    assert_eq!(flag.capturing, None);
    assert_eq!(flag.capture_progress, 0.0);
    assert!(!flag.contested);
}

#[test]
fn capturing_all_flags_finishes_and_freezes_the_authority() {
    let mut setup = super::skirmish::setup();
    // This analytic map isolates the all-owned referee condition from travel.
    for flag in &mut setup.skirmish.as_mut().unwrap().sites.objectives {
        flag.center = [400.0, 10.0];
    }
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 10.0],
        },
    ));
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..20 * battle.rules().tick_hz {
        battle.step();
    }
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.phase, contract::skirmish::Phase::Finished);
    assert_eq!(
        status.result,
        Some(contract::skirmish::MatchResult::Winner { side: Side::Blue })
    );
    let digest = battle.digest();
    let tick = battle.tick();
    battle.step();
    assert_eq!(battle.tick(), tick);
    assert_eq!(battle.digest(), digest);
}

#[test]
fn objective_transitions_replay_and_have_a_paired_browser_record() {
    let saved: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let setup = serde_json::from_value(saved["scenario"].clone()).unwrap();
    let commands = [
        command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 10.0],
            },
        ),
        command(Side::Blue, 2, Order::Ready),
        command(Side::Red, 1, Order::Ready),
    ];
    let mut battle = Battle::new(&setup, 1);
    for c in &commands {
        assert!(battle.accept(c.clone()).error.is_none());
    }
    let mut rows = Vec::new();
    for target in [599, 600, 1680] {
        while battle.tick() < target {
            battle.step();
        }
        let view = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
        rows.push(serde_json::json!({ "tick": target, "digest": format!("{:016x}", battle.digest()), "objectives": view.objectives, "scores": view.scores, "result": view.result }));
    }
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
    let record =
        serde_json::json!({ "scenario": setup, "seed": 1, "commands": commands, "rows": rows });
    if crate::common::bless_parity("skirmish-objectives.json", &record) {
        return;
    }
    assert_eq!(
        serde_json::to_string(&record).unwrap(),
        include_str!("../../../fixtures/parity/skirmish-objectives.json").trim()
    );
}

/// A helicopter over a flag neither captures nor contests it (D20).
#[test]
fn a_helicopter_cannot_capture_or_contest_a_flag() {
    let mut rules = sim::fixtures::test_game();
    rules["catalog"].as_array_mut().unwrap().push(serde_json::json!({ "units": { "blue_test_heli": {
        "extends": "test_heli",
        "roles": ["helicopter"],
        "roster": { "factions": ["us"], "category": "hel", "family_name": "Test", "variant": "Test" }
    } } }));
    let mut setup = super::skirmish::setup();
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.skirmish.as_mut().unwrap().sites.objectives[0].center = [400.0, 10.0];
    let mut battle = Battle::new(&setup, 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "blue_test_heli".into(),
                destination: [400.0, 10.0]
            }
        ))
        .error
        .is_none());
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..40 * battle.rules().tick_hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    let flag = &frame.skirmish.as_ref().unwrap().objectives[0];
    assert_eq!(flag.owner, None);
    assert_eq!(flag.capturing, None);
    assert!(!flag.contested);
}
