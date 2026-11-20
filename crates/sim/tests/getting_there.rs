//! A unit with somewhere to go gets there, or as near as it can, and says
//! honestly what it is doing on the way: never "moving" without moving,
//! never "route blocked" for good, and never holding to work out a new way
//! while the way it has still serves.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

fn battle(map: Value, units: Value, events: Value, rules: Value) -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": events, "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn open(size: [u32; 2]) -> Value {
    json!({ "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

fn go(b: &mut Battle, seq: u64, unit: u32, goal: [f64; 2]) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
}

fn own(b: &Battle, unit: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(unit))
        .unwrap()
        .clone()
}

fn dist(a: [f64; 3], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The moment the audit kept finding: an air-defence launcher sent to the
/// spot its side's mortar carrier is parked on. It drives up, waits a while
/// for the carrier to move, and when it does not, parks beside it: the
/// order is done, as near as the carrier lets it. It never says its route
/// is blocked, and never sits "moving" ten metres short for the rest of
/// the battle.
#[test]
fn a_vehicle_whose_marker_a_parked_vehicle_holds_parks_beside_it() {
    let marker = [150.0, 100.0];
    let mut b = battle(
        open([300, 200]),
        json!([
            { "side": "blue", "kind": "test_supply", "position": [40, 100], "yaw": 0 },
            { "side": "blue", "kind": "test_jeep", "position": marker, "yaw": 1.2 },
        ]),
        json!([]),
        common::scenario_rules(),
    );
    go(&mut b, 1, 0, marker);
    let hz = b.rules().tick_hz as u64;
    let mut parked = None;
    for tick in 1..=120 * hz {
        b.step();
        let truck = own(&b, 0);
        assert_ne!(
            truck.state,
            MoveState::RouteBlocked,
            "at {:?}",
            truck.position
        );
        if truck.state == MoveState::Idle {
            parked = Some((tick as f64 / hz as f64, truck.position));
            break;
        }
    }
    let (seconds, at) = parked.expect("it never finished its order");
    assert!(
        dist(at, marker) < 12.0 && seconds < 75.0,
        "parked at {at:?} after {seconds} s"
    );
    assert!(b.unit(UnitId(0)).unwrap().orders.is_empty());
}

/// The entry the audit kept finding: squad after squad sent to the one
/// spot, where the earlier ones still stand. The newcomer walks up and its
/// men stand at the crowd's edge, as near as the men already there let
/// them: the order is done. It never sits "moving" with every man still.
#[test]
fn a_squad_sent_onto_standing_squads_stops_at_their_edge() {
    let marker = [150.0, 100.0];
    let offsets = [
        [0.0, 0.0],
        [3.0, 1.0],
        [-2.0, 2.0],
        [1.0, -3.0],
        [5.0, -1.0],
    ];
    let units: Vec<Value> = offsets
        .iter()
        .map(|[dx, dy]| [marker[0] + dx, marker[1] + dy])
        .chain([[150.0, 40.0]])
        .map(|at| json!({ "side": "blue", "kind": "test_rifle", "position": at, "yaw": 1.5 }))
        .collect();
    let newcomer = offsets.len() as u32;
    let mut b = battle(
        open([300, 200]),
        json!(units),
        json!([]),
        common::scenario_rules(),
    );
    go(&mut b, 1, newcomer, marker);
    let hz = b.rules().tick_hz as u64;
    let mut still_since = None;
    for tick in 1..=90 * hz {
        b.step();
        let squad = b.unit(UnitId(newcomer)).unwrap();
        if squad.orders.is_empty() {
            assert!(dist(own(&b, newcomer).position, marker) < 12.0);
            return;
        }
        let walking = squad
            .members
            .iter()
            .any(|s| s.alive() && s.velocity.length() > 1e-6);
        let since = *still_since.get_or_insert(tick);
        if walking {
            still_since = None;
        }
        assert!(
            tick - since < 5 * hz,
            "still {:?} at {:?} with every man standing for 5 s, {} s in",
            squad.state,
            squad.position,
            tick / hz
        );
    }
    panic!("it never finished its order");
}

/// A truck crossing open ground with a busy planner. Halfway, its side sees
/// a wall go up across its way far ahead, and then it is sent on to a spot
/// ten metres from where it was going. Each time it works out a new way it
/// drives on along the one it has, as a driver would: it never stops to
/// hold for the planner, and it gets where it was last sent.
#[test]
fn a_vehicle_drives_on_while_it_works_out_a_new_way_to_the_same_place() {
    let mut rules = common::scenario_rules();
    // A planner busy with the rest of a battle: a search takes many ticks.
    rules["navigation"]["work_per_tick"] = json!(60);
    let mut b = battle(
        open([900, 300]),
        json!([{ "side": "blue", "kind": "test_supply", "position": [40, 150], "yaw": 0 }]),
        json!([{ "tick": 600, "add_prop": { "kind": "wall", "center": [600, 150], "yaw": 0, "half_extents": [0.4, 40, 2.0] } }]),
        rules,
    );
    go(&mut b, 1, 0, [860.0, 150.0]);
    let hz = b.rules().tick_hz as u64;
    let goal = [860.0, 160.0];
    let mut set_off = false;
    let mut held = Vec::new();
    let mut arrived = false;
    for tick in 1..=200 * hz {
        b.step();
        if tick == 900 {
            go(&mut b, 2, 0, goal);
        }
        let truck = own(&b, 0);
        set_off |= truck.state == MoveState::Moving;
        if set_off && truck.state == MoveState::Planning {
            held.push(tick);
        }
        if set_off && truck.state == MoveState::Idle {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "it never got there: {:?}", own(&b, 0).position);
    assert!(
        held.is_empty(),
        "it stopped to plan at ticks {:?}",
        &held[..held.len().min(5)]
    );
    assert!(dist(own(&b, 0).position, goal) < 3.0);
}

/// A truck boxed in where it stands, walls on every side: there is no way
/// to where it is sent, and nothing it learns will open one. It says so
/// ("route blocked") and tries again now and then, but not for ever: after
/// its third try it gives the order up and stands idle, ready for another.
#[test]
fn a_vehicle_that_cannot_get_there_gives_the_order_up_after_trying_again() {
    let wall = |center: [f64; 2], half: [f64; 2]| json!({ "kind": "wall", "center": center, "yaw": 0, "half_extents": [half[0], half[1], 4.0] });
    let mut map = open([300, 200]);
    map["props"] = json!([
        wall([100.0, 95.0], [6.0, 0.5]),
        wall([100.0, 105.0], [6.0, 0.5]),
        wall([93.5, 100.0], [0.5, 5.5]),
        wall([106.5, 100.0], [0.5, 5.5]),
    ]);
    let mut b = battle(
        map,
        json!([{ "side": "blue", "kind": "test_supply", "position": [100, 100], "yaw": 0 }]),
        json!([]),
        common::scenario_rules(),
    );
    go(&mut b, 1, 0, [250.0, 100.0]);
    let hz = b.rules().tick_hz as u64;
    let mut blocked = false;
    let mut gave_up = None;
    for tick in 1..=300 * hz {
        b.step();
        let truck = own(&b, 0);
        assert_ne!(truck.state, MoveState::Moving, "it shows moving, boxed in");
        blocked |= truck.state == MoveState::RouteBlocked;
        if blocked && truck.state == MoveState::Idle {
            gave_up = Some(tick as f64 / hz as f64);
            break;
        }
    }
    let seconds = gave_up.expect("it never gave the order up");
    assert!(seconds < 150.0, "it gave up after {seconds} s");
    assert!(b.unit(UnitId(0)).unwrap().orders.is_empty());
}
