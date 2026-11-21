use crate::common;
use crate::common::command;
use crate::common::Seat;
use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures::{self, test_rules};

/// A match on the test units, with a U.S. deck of a tank and a supply truck.
pub(crate) fn setup() -> contract::scenario::ScenarioDefinition {
    match_on(deck())
}

fn two_entry_setup() -> contract::scenario::ScenarioDefinition {
    let mut setup = setup();
    let mut blue_two = setup.players[0].clone();
    blue_two.id = "blue-2".into();
    let mut red_two = setup.players[1].clone();
    red_two.id = "red-2".into();
    setup.players.extend([blue_two, red_two]);
    let sites = &mut setup.skirmish.as_mut().unwrap().sites;
    sites.entries[0].center = [100.0, 10.0];
    sites.entries[1].center = [700.0, 590.0];
    sites.entries.extend([
        contract::encounter::EntrySite {
            side: Side::Blue,
            center: [700.0, 10.0],
            yaw: std::f64::consts::FRAC_PI_2,
        },
        contract::encounter::EntrySite {
            side: Side::Red,
            center: [100.0, 590.0],
            yaw: -std::f64::consts::FRAC_PI_2,
        },
    ]);
    setup
}

#[test]
fn multi_entry_purchases_use_their_route_owner() {
    let setup = two_entry_setup();
    let mut battle = Battle::new(&setup, 3);
    let mut buy = |side: Side, player: &str, seq: u64, destination: [f64; 2]| {
        battle.accept(CommandEnvelope {
            side,
            player: Some(player.into()),
            seq,
            order: Order::purchase("test_tank", destination),
            queued: false,
        })
    };
    assert!(buy(Side::Blue, "blue-1", 1, [700.0, 50.0]).error.is_none());
    assert!(buy(Side::Blue, "blue-2", 1, [100.0, 50.0]).error.is_none());
    for (side, player, seq) in [
        (Side::Blue, "blue-1", 2),
        (Side::Blue, "blue-2", 2),
        (Side::Red, "red-1", 1),
        (Side::Red, "red-2", 1),
    ] {
        assert!(battle
            .accept(CommandEnvelope {
                side,
                player: Some(player.into()),
                seq,
                order: Order::Ready,
                queued: false,
            })
            .error
            .is_none());
    }
    for _ in 0..(3 * battle.rules().tick_hz) {
        battle.step();
    }
    let own = &battle.observe(Side::Blue).own;
    assert!(own.iter().any(|unit| unit.position[0] > 650.0));
    assert!(own.iter().any(|unit| unit.position[0] < 150.0));
}

#[test]
fn a_returning_unit_reselects_the_nearest_entry_after_moving() {
    let mut battle = Battle::new(&two_entry_setup(), 4);
    let buy = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: Some("blue-2".into()),
        seq: 1,
        order: Order::purchase("test_tank", [100.0, 50.0]),
        queued: false,
    });
    assert!(buy.error.is_none());
    for (side, player, seq) in [
        (Side::Blue, "blue-1", 1),
        (Side::Blue, "blue-2", 2),
        (Side::Red, "red-1", 1),
        (Side::Red, "red-2", 1),
    ] {
        assert!(battle
            .accept(CommandEnvelope {
                side,
                player: Some(player.into()),
                seq,
                order: Order::Ready,
                queued: false,
            })
            .error
            .is_none());
    }
    for _ in 0..(3 * battle.rules().tick_hz) {
        battle.step();
    }
    let unit = battle
        .observe(Side::Blue)
        .own
        .iter()
        .find(|unit| unit.position[0] < 150.0)
        .expect("the purchase arrived at the left entry")
        .id;
    assert!(battle
        .accept(CommandEnvelope {
            side: Side::Blue,
            player: Some("blue-2".into()),
            seq: 3,
            order: Order::Move {
                units: vec![unit],
                gesture: 1,
                goal: [700.0, 300.0],
                route: contract::command::RoutePolicy::Fastest,
                direction: Default::default(),
                facing: None,
            },
            queued: false,
        })
        .error
        .is_none());
    for _ in 0..(180 * battle.rules().tick_hz) {
        battle.step();
        if battle
            .observe(Side::Blue)
            .own
            .iter()
            .any(|observed| observed.id == unit && observed.position[0] > 600.0)
        {
            break;
        }
    }
    assert!(
        battle
            .observe(Side::Blue)
            .own
            .iter()
            .any(|observed| observed.id == unit && observed.position[0] > 600.0),
        "unit did not reach the right side: {:?}",
        battle.observe(Side::Blue).own
    );
    assert!(battle
        .accept(CommandEnvelope {
            side: Side::Blue,
            player: Some("blue-2".into()),
            seq: 4,
            order: Order::Refund { units: vec![unit] },
            queued: false,
        })
        .error
        .is_none());
    for _ in 0..(60 * battle.rules().tick_hz) {
        battle.step();
        if battle
            .observe(Side::Blue)
            .own
            .iter()
            .all(|observed| observed.id != unit)
        {
            break;
        }
    }
    assert!(battle
        .observe(Side::Blue)
        .own
        .iter()
        .all(|observed| observed.id != unit));
    assert!(battle.observe(Side::Blue).seat().credits > 800.0);
}

/// The test units, with a U.S. deck of a tank and a supply truck.
pub(crate) fn deck() -> serde_json::Value {
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost": 200,
            "roster": { "factions": ["us"], "category": "veh", "family_name": "Test tank", "variant": "Test" }
        }),
    );
    fixtures::patch_catalog(
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
pub(crate) fn match_on(rules: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "players": crate::common::human_duel(), "skirmish": {"rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 60, "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": { "entries": [
                { "side": "blue", "center": [400, 10], "yaw": std::f64::consts::FRAC_PI_2 },
                { "side": "red", "center": [400, 590], "yaw": -std::f64::consts::FRAC_PI_2 }
            ], "objectives": [
                { "id": "center", "center": [400, 300], "radius_m": 50, "kind": "junction", "counterpart": null },
                { "id": "south", "center": [200, 200], "radius_m": 50, "kind": "field", "counterpart": "north" },
                { "id": "north", "center": [600, 400], "radius_m": 50, "kind": "field", "counterpart": "south" }
            ] }
        }
    })).unwrap()
}

pub(crate) fn test_jet_rules(fuel_capacity_s: f64) -> serde_json::Value {
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "roster": {"factions":["us","eastern"],"category":"veh","family_name":"Test jet","variant":"Test"},
            "capabilities": {"air": {"fuel_capacity_s": fuel_capacity_s}}
        }),
    );
    rules
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
    assert_eq!(status.players[0].credits, 1000.0);
    assert_eq!(status.players[0].unit_count, 0);
}

#[test]
fn both_ready_start_early_and_income_adds_exactly_two_hundred_per_minute() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    for side in Side::COMBATANTS {
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
    for side in Side::COMBATANTS {
        assert_eq!(battle.observe(side).seat().credits, 1200.0);
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
            Order::purchase("test_tank", [400.0, 300.0]),
        ));
        assert!(ack.error.is_none(), "{ack:?}");
    }
    let ack = battle.accept(command(
        Side::Blue,
        6,
        Order::purchase("test_tank", [400.0, 300.0]),
    ));
    assert_eq!(
        ack.error,
        Some(contract::command::OrderError::InsufficientCredits)
    );
    battle.step();
    let frame = battle.observe(Side::Blue);
    assert!(frame.own.is_empty());
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.players[0].credits, 0.0);
    assert_eq!(status.players[0].unit_count, 5);
    assert_eq!(status.players[0].pending.len(), 5);
    // Each tank is bound for room of its own by the point they were bought to.
    let bound: Vec<[f64; 2]> = (status.players.iter())
        .flat_map(|player| player.pending.iter().map(|p| p.destination))
        .collect();
    for (k, at) in bound.iter().enumerate() {
        let off = (at[0] - 400.0).hypot(at[1] - 300.0);
        assert!(off < 40.0, "a tank was sent {off:.0} m from the point");
        assert!(!bound[..k].contains(at), "two tanks were sent to {at:?}");
    }
}

#[test]
fn cancelling_a_pending_purchase_returns_the_price_and_slot_once() {
    let mut battle = Battle::new(&setup(), 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::purchase("test_tank", [400.0, 300.0])
        ))
        .error
        .is_none());
    battle.step();
    let id = battle.observe(Side::Blue).seat().pending[0].id;
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
    assert_eq!(status.players[0].credits, 1000.0);
    assert_eq!(status.players[0].unit_count, 0);
    assert!(status.players[0].pending.is_empty());
}

#[test]
fn paid_reinforcement_enters_at_base_once_and_moves_to_its_destination() {
    let mut battle = Battle::new(&setup(), 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::purchase("test_tank", [400.0, 300.0])
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
    assert_eq!(status.players[0].credits, 800.0);
    assert_eq!(status.players[0].unit_count, 1);
    assert!(status.players[0].pending.is_empty());
    for _ in 0..5 * battle.rules().tick_hz {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    assert!(frame.own[0].position[1] > 15.0);
}

#[test]
fn terminal_report_records_spawn_actions_and_spend_for_both_sides() {
    let setup = setup();
    let players = setup.players.clone();
    let mut skirmish = sim::skirmish::Skirmish::new(setup.skirmish.unwrap(), &players);
    assert!(skirmish.report.events.is_empty());
    skirmish
        .reserve_at(
            0,
            sim::skirmish::Purchase {
                kind: common::unit_kind("test_tank"),
                price: 200,
                carrier: None,
            },
            [400.0, 300.0],
            (sim::entries::EntryIndex(0), 1),
            0,
        )
        .unwrap();
    skirmish.record_spawn(
        0,
        contract::ids::UnitId(0),
        common::unit_kind("test_tank"),
        200,
        2,
    );
    skirmish.record_kill(
        contract::skirmish::MatchUnit {
            side: Side::Red,
            id: contract::ids::UnitId(1),
            kind: common::unit_kind("test_rifle"),
            owner: None,
        },
        110,
        Some(contract::skirmish::MatchUnit {
            side: Side::Blue,
            id: contract::ids::UnitId(0),
            kind: common::unit_kind("test_tank"),
            owner: None,
        }),
        Some(200),
        3,
        None,
    );
    skirmish.phase = contract::skirmish::Phase::Finished;
    let report = skirmish
        .view(Side::Red, 2, 30, &[Default::default(); 2])
        .report
        .unwrap();
    assert_eq!(report.side_totals(&players)[Side::Blue.index()].spent, 200);
    assert!(report.events.iter().any(|event| matches!(
        event,
        contract::skirmish::MatchEvent::Spawn { unit, .. } if unit.side == Side::Blue
    )));
    assert!(report.events.iter().any(|event| matches!(
        event,
        contract::skirmish::MatchEvent::Kill {
            victim_cost: 110,
            killer_cost: Some(200),
            ..
        }
    )));
}

#[test]
fn view_exposes_each_observed_players_economy_and_hides_the_enemy_side() {
    let mut setup = setup();
    setup.players.extend([
        contract::players::Player {
            id: "blue-2".into(),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: contract::players::ControllerKind::ActiveAi,
            human_capable: false,
        },
        contract::players::Player {
            id: "red-2".into(),
            side: Side::Red,
            faction: Some(contract::catalog::Faction::Eastern),
            controller: contract::players::ControllerKind::PassiveAi,
            human_capable: false,
        },
    ]);
    let players = setup.players.clone();
    let mut skirmish = sim::skirmish::Skirmish::new(setup.skirmish.take().unwrap(), &players);
    for (player, credits) in skirmish.players.iter_mut().zip([111_u64, 222, 333, 444]) {
        player.wallet = credits * sim::skirmish::CREDIT_SCALE;
    }
    skirmish
        .reserve_at(
            2,
            sim::skirmish::Purchase {
                kind: common::unit_kind("test_tank"),
                price: 200,
                carrier: None,
            },
            [400.0, 300.0],
            (sim::entries::EntryIndex(0), 7),
            4,
        )
        .unwrap();

    let view = skirmish.view(
        Side::Blue,
        0,
        30,
        &[3, 4, 4, 6].map(|units| sim::skirmish::Fielded { units, cost: 0 }),
    );
    assert_eq!(
        view.players
            .iter()
            .map(|player| player.id.as_str())
            .collect::<Vec<_>>(),
        vec!["blue-1", "blue-2"]
    );
    assert_eq!(view.players[0].index.0, 0);
    assert_eq!(view.players[0].credits, 111.0);
    assert_eq!(view.players[0].unit_count, 3);
    assert!(view.players[0].pending.is_empty());
    assert_eq!(view.players[1].index.0, 2);
    assert_eq!(view.players[1].credits, 133.0);
    assert_eq!(view.players[1].unit_count, 5);
    assert_eq!(view.players[1].pending.len(), 1);
    assert_eq!(view.players[1].pending[0].id.0, 0);
    assert!(view
        .players
        .iter()
        .all(|player| !player.id.starts_with("red-")));
}

#[test]
fn occupied_entry_relocates_the_next_paid_unit_without_overlap() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    for seq in 1..=2 {
        assert!(battle
            .accept(command(
                Side::Blue,
                seq,
                Order::purchase("test_tank", [400.0, 10.0])
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
    assert_eq!(frame.own.len(), 2);
    assert!(frame
        .own
        .iter()
        .any(|unit| unit.position[0] == 400.0 && (unit.position[1] - 10.0).abs() < 1.0));
    assert!(frame
        .own
        .iter()
        .any(|unit| unit.position[0] != 400.0 || (unit.position[1] - 10.0).abs() >= 1.0));
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.players[0].unit_count, 2);
    assert!(status.players[0].pending.is_empty());
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}

#[test]
fn packed_match_header_exposes_players_and_pending_counts() {
    let mut battle = Battle::new(&setup(), 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::purchase("test_tank", [400.0, 300.0]),
    ));
    battle.step();
    let layout: serde_json::Value =
        serde_json::from_str(&sim::publication::layout_json(&battle)).unwrap();
    let fields = layout["header"].as_array().unwrap();
    assert!(fields.iter().all(|field| {
        !matches!(
            field.as_str(),
            Some("credits" | "unitCount" | "maxUnits" | "readyBlue" | "readyRed")
        )
    }));
    for (side, players, pending) in [(Side::Blue, 1.0, 1.0), (Side::Red, 1.0, 0.0)] {
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
            &[],
            &patch,
            &crate::publication::full_fog(),
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
        assert_eq!(read("playersCount"), players);
        assert_eq!(read("pendingCount"), pending);
        assert_eq!(read("skirmishPhase"), 0.0);
    }
}

/// A purchase aimed at ground its unit can't stand on goes where a move
/// there would: to the nearest ground it can, not refused. Before, the
/// skirmish AI kept re-sending a purchase aimed at a wrecked objective and
/// bought nothing for minutes.
#[test]
fn a_purchase_aimed_at_a_blocked_point_is_sent_to_the_nearest_open_ground() {
    // A 6 m by 3 m tank, and a 20 m square block centred on the click.
    let (hull_half_length, hull_half_width) = (3.0, 1.5);
    let block_half = 10.0;
    let mut rules = deck();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "body": { "hull": { "half_extents_m": [hull_half_length, hull_half_width, 1.2] } } }),
    );
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut setup = serde_json::to_value(match_on(rules)).unwrap();
    setup["map"]["props"] = json!([{
        "kind": "wall", "center": [400, 300], "yaw": 0,
        "half_extents": [block_half, block_half, 2]
    }]);
    let mut battle = Battle::new(&serde_json::from_value(setup).unwrap(), 1);
    // How far outside the block a point lies (negative inside it).
    let outside = |[x, y]: [f64; 2]| (x - 400.0).abs().max((y - 300.0).abs()) - block_half;
    for (seq, variant) in [(1, "test_tank"), (2, "test_rifle")] {
        let ack = battle.accept(command(
            Side::Blue,
            seq,
            Order::purchase(variant, [400.0, 300.0]),
        ));
        assert_eq!(ack.error, None, "{variant} is accepted");
    }
    battle.step();
    let pending = &battle.observe(Side::Blue).seat().pending;
    assert_eq!(pending.len(), 2);
    let tank = pending[0].destination;
    // The hull stands clear of the block, on its nearest side: no farther
    // out than its own length and one 2 m navigation cell.
    assert!(
        (hull_half_width..=hull_half_length + 2.0).contains(&outside(tank)),
        "the tank is sent just outside the block, not to {tank:?}"
    );
    let squad = pending[1].destination;
    assert!(
        outside(squad) > 0.0,
        "the squad is sent outside the block, not to {squad:?}"
    );
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
                .accept(command(side, seq, Order::purchase(variant, destination)))
                .error,
            Some(error)
        );
    }
    battle.step();
    for side in Side::COMBATANTS {
        let status = battle.observe(side).skirmish.as_ref().unwrap();
        assert_eq!(status.players[0].credits, 1000.0);
        assert_eq!(status.players[0].unit_count, 0);
        assert!(status.players[0].pending.is_empty());
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
                Order::purchase("test_tank", [400.0, 300.0])
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
                Order::purchase("test_tank", [400.0, 300.0])
            ))
            .error,
        Some(contract::command::OrderError::UnitLimitReached)
    );
    battle.step();
    let status = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(status.players[0].credits, 4000.0);
    assert_eq!(status.players[0].unit_count, 30);
    assert_eq!(status.players[0].pending.len(), 30);
}

#[test]
fn purchase_commands_have_a_paired_native_browser_record() {
    let saved: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-purchases.json"
    ))
    .unwrap();
    let setup = serde_json::from_value(saved["scenario"].clone()).unwrap();
    let commands = [
        command(Side::Blue, 1, Order::purchase("test_tank", [400.0, 300.0])),
        command(Side::Blue, 2, Order::Ready),
        command(Side::Red, 1, Order::Ready),
    ];
    let mut battle = Battle::new(&setup, 1);
    common::hand_red_to_human(&mut battle);
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
    assert_eq!(status.players[0].credits, 1000.0);
    for _ in 0..3 * hz {
        battle.step();
    }
    assert_eq!(battle.observe(Side::Blue).seat().credits, 1010.0);
}

#[test]
fn a_refund_orders_a_vulnerable_return_and_pays_only_at_base() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::purchase("test_tank", [400.0, 150.0])
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
    assert_eq!(battle.observe(Side::Blue).seat().unit_count, 1);
    assert!((battle.observe(Side::Blue).seat().credits - before.seat().credits) < 1.0);
    let mut last_position = before.own[0].position;
    for _ in 0..1000 {
        if let Some(unit) = battle.observe(Side::Blue).own.first() {
            last_position = unit.position;
        } else {
            break;
        }
        battle.step();
    }
    let returned = battle.observe(Side::Blue).clone();
    assert!(last_position[1] > 20.0);
    assert!(returned.own.is_empty(), "the unit physically reached base");
    assert_eq!(returned.seat().unit_count, 0);
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    let refund = returned.seat().credits - before.seat().credits - passive;
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
    let retired_credits = returned.seat().credits;
    for _ in 0..300 {
        battle.step();
    }
    assert!(
        (battle.observe(Side::Blue).seat().credits - retired_credits - 200.0 / 6.0).abs() < 0.00001,
        "retirement pays once"
    );
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}

#[test]
fn an_occupied_entry_does_not_block_return() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::purchase("test_tank", [400.0, 150.0]),
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
        Order::purchase("test_tank", [400.0, 10.0]),
    ));
    for _ in 0..60 {
        battle.step();
    }
    assert_eq!(battle.observe(Side::Blue).own.len(), 2);
    let before = battle.observe(Side::Blue).seat().credits;
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
    assert!(!status.own.iter().any(|unit| unit.id == returning));
    assert_eq!(status.seat().unit_count, 1);
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!(status.seat().credits > before + passive);
}
