//! A skirmish against an AI player: it takes a slot over mid-battle, buys
//! from nothing through recorded commands, and sends what it buys off its
//! entry.
use crate::common;
use crate::common::command;
use crate::common::Seat;
use crate::skirmish::{deck, match_on, setup};
use contract::command::Order;
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;

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

/// A seat weighs a move by having the way worked out, and a way that is not
/// there is searched for as far as a search may go. Having found none for a
/// unit, it does not have the same way worked out again at its next
/// decisions: a wall does not open in five seconds.
#[test]
fn a_seat_that_found_no_way_does_not_ask_again_at_its_next_decision() {
    // A scout squad to buy: a seat asks the way to a scout's post before
    // sending it.
    let mut rules = deck();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_recon",
        json!({ "roster": { "factions": ["us"], "category": "rec", "family_name": "Test scouts", "variant": "Test" } }),
    );
    let mut setup = match_on(rules);
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 0;
    setup.players[0].controller = contract::players::ControllerKind::ActiveAi;
    // A wall from edge to edge, close in front of the U.S. entry: between
    // it and the scouts' post, and every objective.
    setup.map.props.push(
        serde_json::from_value(json!({ "kind": "wall", "center": [400, 18], "yaw": 0,
            "half_extents": [400, 0.5, 3] }))
        .unwrap(),
    );
    let mut battle = Battle::new(&setup, 23);
    let scouts = battle.rules().catalog.index("test_recon").unwrap();
    battle.script_purchases("blue-1", vec![scouts], None);
    let hz = u64::from(battle.rules().tick_hz);
    // Until the scouts are in and the seat has asked their way.
    while battle.ai_estimates() == 0 {
        battle.step();
        assert!(battle.tick() < 120 * hz, "the seat never asked a way");
    }
    let asked = battle.ai_estimates();
    for _ in 0..12 * hz {
        battle.step();
    }
    assert_eq!(
        battle.ai_estimates(),
        asked,
        "the seat had a way it knows is not there worked out again"
    );
    let scouts = &battle.observe(Side::Blue).own[0];
    assert!(scouts.position[1] < 18.0, "the scouts are past the wall");
}

#[test]
fn the_skirmish_ai_procures_artillery_and_replays_its_named_catalog_cutover() {
    let setup = against_an_opponent_with_artillery();
    let mut live = Battle::new(&setup, 3);
    assert!(live
        .accept(command(
            Side::Blue,
            1,
            Order::purchase("test_tank", [400.0, 300.0]),
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
