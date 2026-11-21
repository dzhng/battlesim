//! Aircraft in a skirmish: bought, they wait off the map on their own
//! clocks, enter flying at the map edge nearest their side's entry, are
//! seen by nobody until they arrive, and leave by the edge they came from.
use crate::common::command;
use crate::common::Seat;
use crate::skirmish::{deck, match_on, test_jet_rules};
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures;

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
            Order::purchase(*variant, *destination),
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

/// The player buys a helicopter as a reinforcement: the preview places it,
/// it enters at its base, and the battle plays on while it flies to the
/// destination the player chose.
#[test]
fn a_helicopter_the_player_buys_enters_and_flies_to_its_destination() {
    let mut rules = deck();
    rules["catalog"].as_array_mut().unwrap().push(json!({ "units": { "blue_test_heli": {
        "extends": "test_heli",
        "roster": { "factions": ["us"], "category": "hel", "family_name": "Test", "variant": "Test" }
    } } }));
    let mut battle = Battle::new(&match_on(rules), 1);
    battle
        .preview_purchase(Side::Blue, "blue_test_heli", None, [400.0, 300.0])
        .unwrap();
    let ack = battle.accept(command(
        Side::Blue,
        1,
        Order::purchase("blue_test_heli", [400.0, 300.0]),
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
        Order::purchase("test_jet", [300.0, 300.0]),
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
        Order::purchase("test_jet", [300.0, 300.0]),
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
            Order::purchase("test_jet", [300.0, 300.0]),
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
            Order::purchase("test_jet", [300.0, 300.0]),
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
            Order::purchase("test_jet", destination),
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
            { "id": "red-1", "side": "red", "faction": "eastern", "controller": "passive_ai" }
        ],
        "skirmish": {
            "rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 0,
                "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1,
                "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": {
                "entries": [
                    { "side": "blue", "center": [10, 1500], "yaw": 0 },
                    { "side": "red", "center": [2990, 1500], "yaw": std::f64::consts::PI }
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
                    Order::purchase("f_16c_block_50", destination),
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
