#![allow(clippy::approx_constant)]

use crate::common;
use crate::common::Seat;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures::{self, test_rules};

fn command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        player: Some(
            match side {
                Side::Blue => "blue-1",
                Side::Red => "red-1",
                Side::Neutral => "neutral",
            }
            .into(),
        ),
        seq,
        order,
        queued: false,
    }
}

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
            order: Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination,
            },
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
        order: Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [100.0, 50.0],
        },
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
fn deck() -> serde_json::Value {
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
fn match_on(rules: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "players": crate::common::human_duel(), "skirmish": {"rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 60, "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
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

fn test_jet_rules(fuel_capacity_s: f64) -> serde_json::Value {
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
    assert_eq!(status.players[0].credits, 0.0);
    assert_eq!(status.players[0].unit_count, 5);
    assert_eq!(status.players[0].pending.len(), 5);
    assert!(status
        .players
        .iter()
        .flat_map(|player| player.pending.iter())
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
            common::unit_kind("test_tank"),
            [400.0, 300.0],
            200,
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
    let report = skirmish.view(Side::Red, 2, 30, &[0, 0]).report.unwrap();
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
            common::unit_kind("test_tank"),
            [400.0, 300.0],
            200,
            (sim::entries::EntryIndex(0), 7),
            4,
        )
        .unwrap();

    let view = skirmish.view(Side::Blue, 0, 30, &[3, 4, 4, 6]);
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
    assert_eq!(status.players[0].unit_count, 2);
    assert_eq!(status.players[0].pending.len(), 1);
    assert!(status.players[0].pending[0].blocked);
    assert_eq!(status.players[0].pending[0].id.0, 1);
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
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 300.0],
        },
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
        assert_eq!(read("playersCount"), players);
        assert_eq!(read("pendingCount"), pending);
        assert_eq!(read("skirmishPhase"), 0.0);
    }
}

#[test]
fn a_purchase_preview_spreads_a_squad_round_its_destination_and_a_hull_stands_alone() {
    let mut rules = deck();
    fixtures::patch_catalog(
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
fn a_purchase_ghost_keeps_its_shape_as_the_cursor_moves_over_open_ground() {
    let mut rules = deck();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut battle = Battle::new(&match_on(rules), 1);
    let shape = |battle: &mut Battle, at: [f64; 2]| -> Vec<(i64, i64)> {
        battle
            .preview_purchase(Side::Blue, "test_rifle", at)
            .unwrap()
            .spots
            .iter()
            .map(|s| (((s[0] - at[0]) * 1e6) as i64, ((s[1] - at[1]) * 1e6) as i64))
            .collect()
    };
    let first = shape(&mut battle, [400.0, 300.0]);
    for at in [[400.5, 300.0], [403.0, 298.0], [410.0, 305.0]] {
        assert_eq!(
            shape(&mut battle, at),
            first,
            "the ghost slides with the cursor at {at:?}"
        );
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
            Order::ConfirmPurchase {
                variant: variant.into(),
                destination: [400.0, 300.0],
            },
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
fn a_human_skirmish_slot_taken_over_mid_battle_starts_its_policy() {
    let mut setup = setup();
    setup.players[0].controller = contract::players::ControllerKind::Human;
    let mut battle = Battle::new(&setup, 97);
    battle.set_controllers(vec![(
        "blue-1".into(),
        contract::players::ControllerKind::ActiveAi,
    )]);
    battle.step();
    assert!(
        battle
            .ai_receipts()
            .iter()
            .any(|receipt| receipt.player == "blue-1"),
        "takeover did not start AI"
    );
}

#[test]
fn a_takeover_keeps_the_scripted_purchase_cursor() {
    let mut setup = setup();
    setup.players[0].controller = contract::players::ControllerKind::Human;
    let mut battle = Battle::new(&setup, 98);
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    let rifle = battle.rules().catalog.index("test_rifle").unwrap();
    battle.script_purchases("blue-1", vec![tank, rifle], None);
    battle.set_controllers(vec![(
        "blue-1".into(),
        contract::players::ControllerKind::ActiveAi,
    )]);
    battle.step();
    battle.set_controllers(vec![(
        "blue-1".into(),
        contract::players::ControllerKind::Human,
    )]);
    for _ in 0..battle.rules().tick_hz * 5 {
        battle.step();
    }
    battle.set_controllers(vec![(
        "blue-1".into(),
        contract::players::ControllerKind::ActiveAi,
    )]);
    for _ in 0..battle.rules().tick_hz * 5 {
        battle.step();
    }
    let purchases: Vec<_> = battle
        .replay()
        .accepted
        .into_iter()
        .filter_map(|(_, command)| match command.order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .collect();
    assert_eq!(purchases, vec!["test_tank", "test_rifle", "test_rifle"]);
}

/// A match against the basic opponent on red, whose deck holds squads, a
/// launcher team and a truck.
fn against_the_basic_opponent() -> contract::scenario::ScenarioDefinition {
    against_an_opponent_with(json!({ "units": {} }))
}

/// The basic opponent, its deck also holding the catalog document `extra`.
fn against_an_opponent_with(extra: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    let mut rules = deck();
    rules["catalog"].as_array_mut().unwrap().push(extra);
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
    setup.players[1].controller = contract::players::ControllerKind::ActiveAi;
    setup
}

fn against_an_opponent_with_a_helicopter() -> contract::scenario::ScenarioDefinition {
    against_an_opponent_with(json!({ "units": { "red_test_heli": {
        "extends": "test_heli",
        "roles": ["helicopter"],
        "roster": { "factions": ["eastern"], "category": "hel", "family_name": "Test", "variant": "Test" }
    } } }))
}

/// The opponent whose admitted deck includes a physical indirect-fire card.
fn against_an_opponent_with_artillery() -> contract::scenario::ScenarioDefinition {
    let mut rules = deck();
    rules["weapons"]["hmg"]["trajectory"] = json!("indirect");
    rules["weapons"]["hmg"]["min_range_m"] = json!(50.0);
    common::json_ground_reach(&mut rules["weapons"]["hmg"], 500.0);
    rules["catalog"].as_array_mut().unwrap().push(json!({ "units": { "red_test_jeep": {
        "extends": "test_jeep",
        "roles": ["artillery"],
        "roster": { "factions": ["eastern"], "category": "sup", "family_name": "Test artillery", "variant": "Test" }
    } } }));
    let mut setup = match_on(rules);
    setup.players[1].controller = contract::players::ControllerKind::ActiveAi;
    setup
}

/// The opponent buys a helicopter in its reinforcements; it enters at its
/// base already at cruise height, never waiting for clear ground, and sets off
/// for an objective.
#[test]
fn the_skirmish_ai_buys_and_flies_a_helicopter() {
    let setup = against_an_opponent_with_a_helicopter();
    let mut battle = Battle::new(&setup, 3);
    battle.accept(command(Side::Blue, 1, Order::Ready));
    let heli = battle.rules().catalog.index("red_test_heli").unwrap();
    let cruise = battle.rules().air.low_air.cruise_agl_m;
    let mut flown = false;
    for _ in 0..600 * battle.rules().tick_hz {
        battle.step();
        let red = battle.observe(Side::Red);
        if let Some(u) = red.own.iter().find(|u| u.kind == heli) {
            let ground = battle
                .world()
                .height_at(u.position[0], u.position[1])
                .unwrap();
            assert!(
                u.position[2] >= ground + cruise - 1e-6,
                "it entered on the ground"
            );
            if battle.unit(u.id).unwrap().route.is_some() {
                flown = true;
                break;
            }
        }
    }
    assert!(flown, "the opponent never bought and flew a helicopter");
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
        red.players[0].pending.len() > 1,
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
        red.players[0].credits < 1000.0,
        "the AI spends its ordinary opening wallet"
    );
    assert!(red.players[0].unit_count > 0 && red.players[0].unit_count <= 30);
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
fn ai_lane_arrivals_drive_off_the_entry_without_waiting_for_a_decision() {
    let setup = against_the_basic_opponent();
    let red_entry = setup.skirmish.as_ref().unwrap().sites.entries[1].clone();
    let red_entry_index = 1usize;
    let mut battle = Battle::new(&setup, 19);
    let hz = u64::from(battle.rules().tick_hz);
    assert!(battle
        .accept(command(Side::Blue, 1, Order::Ready))
        .error
        .is_none());

    let mut arrivals = Vec::new();
    let mut known = std::collections::BTreeSet::new();
    for _ in 0..180 * hz {
        battle.step();
        let red = battle.observe(Side::Red);
        for unit in &red.own {
            if known.insert(unit.id) {
                // It arrives already driving up its lane: the next arrival
                // waits only for this hull to clear the entry, never for the
                // AI's next decision.
                let goal = unit.goal.expect("an AI arrival carries its lane goal");
                assert!(
                    unit.state == contract::observation::MoveState::Moving
                        && (goal[0] - red_entry.center[0]).hypot(goal[1] - red_entry.center[1])
                            > 10.0,
                    "AI arrival did not drive off the entry: {unit:?}"
                );
                arrivals.push((battle.tick(), unit.id));
            }
        }
        if arrivals.len() >= 3 {
            break;
        }
    }
    assert!(
        arrivals.len() >= 3,
        "AI did not dispatch several units: {arrivals:?}"
    );
    for (_, id) in &arrivals {
        for _ in 0..5 * hz {
            battle.step();
        }
        if let Some(unit) = battle
            .observe(Side::Red)
            .own
            .iter()
            .find(|unit| unit.id == *id)
        {
            let distance = (unit.position[0] - red_entry.center[0])
                .hypot(unit.position[1] - red_entry.center[1]);
            assert!(
                distance > 10.0,
                "AI arrival remained parked on entry: {unit:?}"
            );
        }
    }
    assert!(
        battle
            .observe(Side::Red)
            .seat()
            .pending
            .iter()
            .all(|purchase| purchase.entry == red_entry_index),
        "AI purchase left its assigned lane entry"
    );
}

#[test]
fn a_point_a_move_would_refuse_has_no_route_time() {
    // The AI picks goals by asking for a route time; a goal off the map must
    // read as unreachable, or the AI issues a move admission refuses.
    let setup = against_the_basic_opponent();
    let mut battle = Battle::new(&setup, 19);
    assert!(battle
        .accept(command(Side::Blue, 1, Order::Ready))
        .error
        .is_none());
    let unit = loop {
        battle.step();
        if let Some(unit) = battle.observe(Side::Red).own.first().cloned() {
            break unit;
        }
        assert!(battle.tick() < 600 * 30, "the AI never fielded a unit");
    };
    let here = [unit.position[0], unit.position[1]];
    let near = [here[0], here[1] - 10.0];
    // Just past the edge: the refused AI goals lay about 2 m out.
    let off_map = [-2.0, here[1]];
    assert!(battle.route_time(Side::Red, unit.id, near).is_some());
    assert_eq!(battle.route_time(Side::Red, unit.id, off_map), None);
}

#[test]
fn the_skirmish_ai_procures_artillery_and_replays_its_named_catalog_cutover() {
    let setup = against_an_opponent_with_artillery();
    let mut live = Battle::new(&setup, 3);
    assert!(live
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [400.0, 300.0],
            },
        ))
        .error
        .is_none());
    assert!(live
        .accept(command(Side::Blue, 2, Order::Ready))
        .error
        .is_none());
    let mut digests = Vec::new();
    for _ in 0..45 * live.rules().tick_hz {
        live.step();
        digests.push(live.digest());
    }
    let artillery = live.rules().catalog.index("red_test_jeep").unwrap();
    let artillery_ids: std::collections::BTreeSet<_> = live
        .observe(Side::Red)
        .own
        .iter()
        .filter(|unit| unit.kind == artillery)
        .map(|unit| unit.id)
        .collect();
    let red = live.observe(Side::Red);
    assert!(
        red.own.iter().any(|unit| unit.kind == artillery)
            || red
                .seat()
                .pending
                .iter()
                .any(|pending| pending.kind == artillery),
        "the AI did not field or reserve its admitted artillery card"
    );
    assert!(live.replay().accepted.iter().any(|(_, command)| {
        command.side == Side::Red
            && matches!(
                &command.order,
                Order::ConfirmPurchase { variant, .. } if variant == "red_test_jeep"
            )
    }));
    assert!(live.replay().accepted.iter().any(|(_, command)| {
        command.side == Side::Red
            && matches!(&command.order, Order::Attack { units, target: contract::command::TargetRef::Identified { .. } } if units.iter().any(|id| artillery_ids.contains(id)))
    }), "artillery must use the ordinary identified-target attack order");
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest, "tick {}", replay.tick());
    }
    let mut without_artillery_row = setup.clone();
    without_artillery_row
        .rules
        .weapons
        .remove("artillery_mortar");
    assert_eq!(
        Battle::from_replay(&without_artillery_row, &live.replay()).err(),
        Some(sim::battle::ReplayError::ConfigMismatch),
        "a replay with the artillery row removed must use the named hard cutover"
    );
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
    assert_eq!(battle.observe(Side::Blue).seat().unit_count, 1);
    assert!((battle.observe(Side::Blue).seat().credits - before.seat().credits) < 1.0);
    for _ in 0..1000 {
        if battle.observe(Side::Blue).own.is_empty() {
            break;
        }
        battle.step();
    }
    let returned = battle.observe(Side::Blue).clone();
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
    let credits = battle.observe(Side::Blue).seat().credits;
    battle.accept(command(Side::Blue, 4, Order::Stop { units: vec![id] }));
    for _ in 0..450 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(!status.own[0].withdrawing);
    assert_eq!(status.seat().unit_count, 1);
    assert!((status.seat().credits - credits - 50.0).abs() < 0.00001);
}

#[test]
fn a_destroyed_withdrawing_unit_never_refunds_or_retires() {
    let mut setup = setup();
    let mut rules = test_rules();
    fixtures::patch_catalog(
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
    for side in Side::COMBATANTS {
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
    let before = battle.observe(Side::Blue).seat().credits;
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
    let after = battle.observe(Side::Blue).seat().credits;
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
    let mut rules = test_rules();
    fixtures::patch_catalog(
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
    let credits = battle.observe(Side::Blue).seat().credits;
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
    assert_eq!(status.seat().unit_count, 1);
    assert!((status.seat().credits - credits - 50.0).abs() < 0.00001);
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
    assert!(
        status
            .own
            .iter()
            .any(|unit| unit.id == returning && unit.withdrawing),
        "occupied physical entry must keep the return pending"
    );
    assert_eq!(status.seat().unit_count, 2);
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!((status.seat().credits - before - passive).abs() < 0.00001);
}

/// The player buys a helicopter as a reinforcement: the preview places it,
/// it enters at its base, and the battle plays on while it flies to the
/// destination the player chose.
/// The test deck with `test_jet` and `test_heli` on the U.S. roster, the
/// jet waiting `arrival` off the map (null: none).
fn air_deck(arrival: serde_json::Value) -> serde_json::Value {
    let mut rules = deck();
    for (id, category, family) in [
        ("test_jet", "veh", "Test jet"),
        ("test_heli", "hel", "Test heli"),
    ] {
        fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({
                "roster": { "factions": ["us"], "category": category, "family_name": family, "variant": "Test" }
            }),
        );
    }
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mobility": { "air": { "arrival": arrival } } }),
    );
    rules
}

/// Blue buys `purchases` in order and both sides ready up; the battle runs
/// until it is active. Returns the tick combat began.
fn bought(battle: &mut Battle, purchases: &[(&str, [f64; 2])]) -> u64 {
    for (seq, (variant, destination)) in purchases.iter().enumerate() {
        let ack = battle.accept(command(
            Side::Blue,
            seq as u64 + 1,
            Order::ConfirmPurchase {
                variant: (*variant).into(),
                destination: *destination,
            },
        ));
        assert!(ack.error.is_none(), "{variant}: {ack:?}");
    }
    battle.accept(command(Side::Blue, 100, Order::Ready));
    battle.accept(command(Side::Red, 100, Order::Ready));
    loop {
        battle.step();
        let phase = battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase;
        if phase == contract::skirmish::Phase::Active {
            return battle.tick();
        }
    }
}

/// The tick each of Blue's units first appears, by unit id, and how it
/// looked then, stepping for `seconds`.
fn arrivals(battle: &mut Battle, seconds: u32) -> Vec<(u64, contract::observation::OwnUnit)> {
    let mut seen: Vec<(u64, contract::observation::OwnUnit)> = Vec::new();
    for _ in 0..seconds * battle.rules().tick_hz {
        battle.step();
        for unit in &battle.observe(Side::Blue).own {
            if !seen.iter().any(|(_, u)| u.id == unit.id) {
                seen.push((battle.tick(), unit.clone()));
            }
        }
    }
    seen
}

/// D41: an aircraft's wait is its own. A tank bought behind a jet enters at
/// once, and two jets bought together wait side by side, each arriving when
/// its pending record and its approach said it would.
#[test]
fn a_waiting_aircraft_leaves_the_queue_and_waits_on_its_own_clock() {
    // Flying its own cruise height, which its approach is drawn at too.
    let mut rules = air_deck(json!({ "delay_s": [20, 20] }));
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mobility": { "air": { "cruise_agl_m": 180 } } }),
    );
    let mut battle = Battle::new(&match_on(rules), 6);
    let start = bought(
        &mut battle,
        &[
            ("test_jet", [300.0, 300.0]),
            ("test_jet", [500.0, 300.0]),
            ("test_tank", [400.0, 300.0]),
        ],
    );
    let hz = u64::from(battle.rules().tick_hz);
    let due = start + 20 * hz;
    battle.step();
    let frame = battle.observe(Side::Blue);
    let pending = &frame.seat().pending;
    let jet = battle.rules().catalog.index("test_jet").unwrap();
    let waiting: Vec<_> = pending.iter().filter(|p| p.kind == jet).collect();
    assert_eq!(waiting.len(), 2, "both jets stay pending: {pending:?}");
    for p in &waiting {
        assert_eq!(p.arrival_tick, Some(due), "{p:?}");
    }
    let approaches: Vec<_> = frame.air_transits.iter().map(|t| t.edge_tick).collect();
    assert_eq!(
        approaches,
        vec![due, due],
        "each approach meets its arrival"
    );
    // The packed publication carries the same tick to the pending cards.
    let layout = sim::publication::layout_json(&battle);
    let mut reader = sim::publication::reader::Reader::new(&layout).unwrap();
    let words: Vec<u32> = sim::publication::Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|w| w.to_bits())
        .collect();
    let published = reader.decode(&words).unwrap().unwrap();
    let cards: Vec<f64> = published
        .skirmish
        .unwrap()
        .players
        .iter()
        .flat_map(|player| player.pending.iter())
        .filter(|p| &*p.kind == "test_jet")
        .map(|p| p.arrival_tick)
        .collect();
    assert_eq!(cards, vec![due as f64, due as f64]);

    let seen = arrivals(&mut battle, 25);
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    let tank_at = seen
        .iter()
        .find(|(_, u)| u.kind == tank)
        .expect("the tank entered")
        .0;
    assert!(
        tank_at <= start + 2 * hz,
        "the tank waited for the jets: entered {}s in",
        (tank_at - start) / hz
    );
    let jets: Vec<u64> = seen
        .iter()
        .filter(|(_, u)| u.kind == jet)
        .map(|(t, _)| *t)
        .collect();
    for (_, u) in seen.iter().filter(|(_, u)| u.kind == jet) {
        assert!(
            (u.position[2] - 180.0).abs() < 1.0,
            "arrived at {:?}",
            u.position
        );
    }
    assert_eq!(jets, vec![due, due], "the jets arrive together, on time");
}

/// D38, D39: every aircraft enters at its side's back-edge point nearest
/// the side's entry site, whatever its destination, already flying at its
/// cruise height and pointed at its destination. A row with no `arrival`
/// is there at once and flies straight in.
#[test]
fn any_aircraft_enters_flying_at_the_edge_point_nearest_its_entry_site() {
    let mut battle = Battle::new(&match_on(air_deck(serde_json::Value::Null)), 6);
    let destination = [100.0, 300.0];
    let start = bought(
        &mut battle,
        &[("test_jet", destination), ("test_heli", destination)],
    );
    let hz = u64::from(battle.rules().tick_hz);
    let seen = arrivals(&mut battle, 3);
    assert_eq!(seen.len(), 2, "both aircraft are on the map: {seen:?}");
    // Blue's entry site is at (400, 10): its back edge is y = 0.
    let edge = [400.0, 0.0];
    let bearing = libm::atan2(destination[1] - edge[1], destination[0] - edge[0]);
    for (tick, unit) in &seen {
        let id = battle.rules().catalog.id(unit.kind);
        let cruise = battle
            .rules()
            .air
            .cruise_agl_m(&battle.rules().catalog.get(unit.kind).mobility);
        assert!(
            *tick <= start + 2 * hz,
            "{id} waited {}s",
            (tick - start) / hz
        );
        let [x, y, z] = unit.position;
        let dt = 1.0 / hz as f64;
        let speed = battle
            .rules()
            .catalog
            .get(unit.kind)
            .mobility
            .speeds_mps()
            .0;
        assert!(
            (x - edge[0]).hypot(y - edge[1]) < speed * dt * 2.0 + 0.5,
            "{id} entered at ({x:.1}, {y:.1}), not at the edge point nearest the entry"
        );
        assert!(
            (z - cruise).abs() < 1.0,
            "{id} entered at {z:.1} m, not cruising at {cruise}"
        );
        let off = sim::math::wrap_angle(unit.yaw - bearing).abs();
        assert!(
            off < 0.1,
            "{id} entered facing {:.2}, not its destination {bearing:.2}",
            unit.yaw
        );
    }
    let now = battle.observe(Side::Blue);
    let heli = battle.rules().catalog.index("test_heli").unwrap();
    let flown = now.own.iter().find(|u| u.kind == heli).unwrap().position;
    assert!(
        (flown[0] - edge[0]).hypot(flown[1] - edge[1]) > 50.0,
        "the helicopter is flying in, not held at the edge: {flown:?}"
    );
}

#[test]
fn a_helicopter_the_player_buys_enters_and_flies_to_its_destination() {
    let mut rules = deck();
    rules["catalog"].as_array_mut().unwrap().push(json!({ "units": { "blue_test_heli": {
        "extends": "test_heli",
        "roster": { "factions": ["us"], "category": "hel", "family_name": "Test", "variant": "Test" }
    } } }));
    let mut battle = Battle::new(&match_on(rules), 1);
    battle
        .preview_purchase(Side::Blue, "blue_test_heli", [400.0, 300.0])
        .unwrap();
    let ack = battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "blue_test_heli".into(),
            destination: [400.0, 300.0],
        },
    ));
    assert!(ack.error.is_none(), "{ack:?}");
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..60 * battle.rules().tick_hz {
        battle.step();
        for side in Side::COMBATANTS {
            battle.observe(side);
        }
    }
    let frame = battle.observe(Side::Blue);
    assert_eq!(frame.own.len(), 1);
    let p = frame.own[0].position;
    assert!(
        (p[0] - 400.0).hypot(p[1] - 300.0) < 5.0,
        "it ended at {p:?}"
    );
}

#[test]
fn inbound_transit_is_published_to_its_own_side_only() {
    let rules = test_jet_rules(180.0);
    let mut battle = Battle::new(&match_on(rules), 6);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_jet".into(),
            destination: [300.0, 300.0],
        },
    ));
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..1000 {
        battle.step();
        if !battle.observe(Side::Blue).air_transits.is_empty() {
            assert!(!battle
                .observe(Side::Red)
                .air_transits
                .iter()
                .any(|t| t.kind == battle.observe(Side::Blue).air_transits[0].kind));
            return;
        }
    }
    panic!("no transit published");
}

#[test]
fn an_inbound_transit_is_published_through_the_whole_delay() {
    let rules = test_jet_rules(180.0);
    let mut battle = Battle::new(&match_on(rules), 6);
    battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_jet".into(),
            destination: [300.0, 300.0],
        },
    ));
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));

    let mut edge_tick = None;
    let mut previous_tick = None;
    let mut spawned = false;
    for _ in 0..1000 {
        battle.step();
        let frame = battle.observe(Side::Blue);
        let inbound = frame
            .air_transits
            .iter()
            .find(|t| matches!(t.dir, contract::observation::AirTransitDirection::In));
        if let Some(transit) = inbound {
            let edge = *edge_tick.get_or_insert(transit.edge_tick);
            assert_eq!(transit.edge_tick, edge);
            assert!(
                battle.tick() < edge,
                "an inbound transit must precede spawn"
            );
            if let Some(previous) = previous_tick {
                assert_eq!(
                    battle.tick(),
                    previous + 1,
                    "the delay must have no publication gap"
                );
            }
            previous_tick = Some(battle.tick());
        } else if edge_tick.is_some() {
            spawned = frame
                .own
                .iter()
                .any(|u| battle.rules().catalog.id(u.kind) == "test_jet");
            if spawned {
                break;
            }
            panic!("inbound transit disappeared before the jet spawned");
        }
    }
    assert!(edge_tick.is_some(), "no inbound transit was published");
    assert!(spawned, "the jet did not spawn after its transit");
}

#[test]
fn nothing_senses_or_targets_a_jet_before_it_arrives() {
    let mut battle = Battle::new(&match_on(test_jet_rules(180.0)), 6);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_jet".into(),
                destination: [300.0, 300.0],
            },
        ))
        .error
        .is_none());
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    for _ in 0..(14 * battle.rules().tick_hz) {
        battle.step();
        assert!(battle.observe(Side::Blue).own.is_empty());
        assert!(battle.observe(Side::Red).identified.is_empty());
        assert!(battle
            .accept(command(
                Side::Red,
                1000 + battle.tick(),
                Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Identified {
                        id: contract::observation::ObservedTargetId(0),
                    },
                },
            ))
            .error
            .is_some());
    }
}

#[test]
fn withdrawal_retires_and_refunds_on_crossing_the_nearest_back_edge_point() {
    let mut battle = Battle::new(&match_on(test_jet_rules(2.0)), 6);
    assert!(battle
        .accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_jet".into(),
                destination: [300.0, 300.0],
            },
        ))
        .error
        .is_none());
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    let mut saw_out = false;
    for _ in 0..(90 * battle.rules().tick_hz) {
        battle.step();
        saw_out |= battle
            .observe(Side::Blue)
            .air_transits
            .iter()
            .any(|t| matches!(t.dir, contract::observation::AirTransitDirection::Out));
        if battle.observe(Side::Blue).own.is_empty() && saw_out {
            break;
        }
    }
    assert!(
        saw_out,
        "the departing jet did not publish an Out transit; own={:?} tick={}",
        battle.observe(Side::Blue).own,
        battle.tick()
    );
    assert!(battle.observe(Side::Blue).own.is_empty());
    assert!(battle.observe(Side::Blue).seat().credits > 700.0);
}

#[test]
fn fuel_return_uses_the_edge_distance() {
    fn withdrawal_tick(destination: [f64; 2]) -> (u64, f64) {
        let mut battle = Battle::new(&match_on(test_jet_rules(60.0)), 6);
        battle.accept(command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_jet".into(),
                destination,
            },
        ));
        battle.accept(command(Side::Blue, 2, Order::Ready));
        battle.accept(command(Side::Red, 1, Order::Ready));
        for _ in 0..(120 * battle.rules().tick_hz) {
            battle.step();
            if battle.observe(Side::Blue).own.iter().any(|u| u.withdrawing) {
                return (battle.tick(), battle.observe(Side::Blue).own[0].position[1]);
            }
        }
        panic!("fuel withdrawal never triggered");
    }
    let near = withdrawal_tick([400.0, 50.0]);
    let far = withdrawal_tick([400.0, 550.0]);
    assert!(
        far.0 <= near.0,
        "far-edge return should not trigger later: near {}, far {}",
        near.0,
        far.0
    );
    assert!(
        far.1 >= near.1,
        "far withdrawal starts farther from the back edge: near {}, far {}",
        near.1,
        far.1
    );
}

#[test]
fn air_ingress_jet_returns_through_west_edge_and_refunds_its_owner() {
    let rules = fixtures::rules();
    let map = serde_json::to_value(sim::maps::load("air-jet").unwrap().definition).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [], "events": [], "scripts": [],
        "players": [
            { "id": "blue-1", "side": "blue", "faction": "us", "controller": "human", "human_capable": true },
            { "id": "red-1", "side": "red", "faction": "eastern", "controller": "active_ai", "human_capable": false }
        ],
        "skirmish": {
            "rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 0,
                "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1,
                "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": {
                "entries": [
                    { "side": "blue", "center": [10, 1500], "yaw": 0 },
                    { "side": "red", "center": [2990, 1500], "yaw": 3.141592653589793 }
                ],
                "objectives": [
                    { "id": "center", "center": [1500, 1500], "radius_m": 40, "kind": "junction", "counterpart": null },
                    { "id": "south", "center": [1500, 450], "radius_m": 40, "kind": "field", "counterpart": "north" },
                    { "id": "north", "center": [1500, 2550], "radius_m": 40, "kind": "field", "counterpart": "south" }
                ]
            }
        }
    })).unwrap();
    let mut battle = Battle::new(&setup, 6);
    assert!(battle
        .accept(command(Side::Blue, 1, Order::Ready))
        .error
        .is_none());
    battle.step();
    battle.step();
    for (seq, destination) in [(2, [1500.0, 1500.0]), (3, [2500.0, 2400.0])] {
        assert_eq!(
            battle
                .accept(command(
                    Side::Blue,
                    seq,
                    Order::ConfirmPurchase {
                        variant: "f_16c_block_50".into(),
                        destination
                    },
                ))
                .error,
            None
        );
    }
    battle.step();

    let mut saw_withdrawal = false;
    let mut saw_westbound_transit = false;
    let mut blue_credits = battle.observe(Side::Blue).seat().credits;
    let mut retirement_refund = None;
    let mut outbound_tick = None;
    let mut retired_tick = None;
    let mut first_id = None;
    let mut withdrawing = false;
    for _ in 0..(300 * battle.rules().tick_hz) {
        let steps = if withdrawing { 1 } else { 30 };
        for _ in 0..steps {
            battle.step();
        }
        let frame = battle.observe(Side::Blue);
        let blue = frame
            .skirmish
            .as_ref()
            .unwrap()
            .players
            .iter()
            .find(|player| player.id == "blue-1")
            .unwrap();
        if first_id.is_none() {
            first_id = frame
                .own
                .iter()
                .find(|unit| battle.rules().catalog.id(unit.kind) == "f_16c_block_50")
                .map(|unit| unit.id);
        }
        let jet = first_id.and_then(|id| frame.own.iter().find(|unit| unit.id == id));
        if jet.is_some_and(|unit| unit.withdrawing) {
            saw_withdrawal = true;
            withdrawing = true;
        }
        saw_westbound_transit |= frame.air_transits.iter().any(|transit| {
            matches!(transit.dir, contract::observation::AirTransitDirection::Out)
                && transit.point[0] <= 10.0
        });
        if saw_westbound_transit && outbound_tick.is_none() {
            outbound_tick = Some(battle.tick());
        }
        if first_id.is_some() && jet.is_none() {
            retirement_refund = Some(blue.credits - blue_credits);
            retired_tick = Some(battle.tick());
            break;
        }
        blue_credits = blue.credits;
        saw_withdrawal |= frame.own.iter().any(|unit| unit.withdrawing);
    }
    assert!(saw_withdrawal, "the F-16 never turned for home");
    assert!(
        saw_westbound_transit,
        "the outbound transit did not use blue's west edge"
    );
    assert!(
        matches!((outbound_tick, retired_tick), (Some(out), Some(retired)) if out <= retired),
        "the jet retired before its outbound transit"
    );
    assert!(
        retirement_refund.unwrap_or_default() > 350.0 * 0.15,
        "the retired jet did not refund its owner: credits={:?}",
        retirement_refund
    );
}
