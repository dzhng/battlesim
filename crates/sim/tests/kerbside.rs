//! A street with cars parked beside it. A road journey's lane is the free
//! width of the road: a vehicle moves over toward the middle past a body
//! beside its lane and comes back after it. It does not refuse the street,
//! does not shove a car it has room to pass, and two vehicles sent through
//! from opposite ends both get through.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, V2};

use crate::common;

/// The street's middle and half its length.
const MIDDLE: [f64; 2] = [400.0, 400.0];
const HALF_LENGTH_M: f64 = 300.0;
/// From the street's middle to the terraces' walls either side.
const WALL_M: f64 = 7.5;
/// From the street's middle to the near side of a parked car: a metre off
/// the kerb of a 7 m street.
const PARKED_M: f64 = 4.55;
/// A parked car's half length and half width.
const CAR: [f64; 2] = [2.1, 0.9];
/// Vehicles start and end this far from the middle of the street.
const END_M: f64 = 260.0;
/// The cross streets are this far from the middle of the street, and the
/// street round the back of the block this far to its left.
const CROSS_M: f64 = 280.0;
const BLOCK_M: f64 = 80.0;

/// The point `along` metres down a street of `bearing` from its middle and
/// `right` metres to the right of that direction.
fn at(bearing: f64, along: f64, right: f64) -> [f64; 2] {
    let (sin, cos) = bearing.sin_cos();
    [
        MIDDLE[0] + cos * along + sin * right,
        MIDDLE[1] + sin * along - cos * right,
    ]
}

/// How far `p` lies to the right of the middle of a street of `bearing`.
fn across(bearing: f64, p: [f64; 3]) -> f64 {
    let (sin, cos) = bearing.sin_cos();
    (p[0] - MIDDLE[0]) * sin - (p[1] - MIDDLE[1]) * cos
}

/// A 7 m street 600 m long on the heading `bearing`, between two rows of
/// terraces, and a way round the block on its left: a cross street near
/// each end and a street parallel to it 80 m off. With `cars`, runs of four
/// parked cars stand bumper to bumper beside its right-hand kerb (as one
/// drives along `bearing`), a gap between runs.
fn street(bearing: f64, cars: bool) -> Value {
    // The terraces stop short of the cross streets.
    let wall = |side: f64| {
        json!({ "kind": "wall", "center": at(bearing, 0.0, side * (WALL_M + 0.5)),
            "yaw": bearing, "half_extents": [CROSS_M - 12.0, 0.5, 3] })
    };
    let road = |from: [f64; 2], to: [f64; 2]| {
        json!({ "kind": "road", "shape": { "kind": "stroke", "points": [from, to], "width_m": 7 } })
    };
    let mut props = vec![wall(1.0), wall(-1.0)];
    if cars {
        // Six runs of four, a run every 60 m.
        for run in 0..6 {
            for k in 0..4 {
                let along = -170.0 + 60.0 * run as f64 + 4.7 * k as f64;
                props.push(json!({ "kind": "parked_car",
                    "center": at(bearing, along, PARKED_M + CAR[1]),
                    "yaw": bearing, "half_extents": [CAR[0], CAR[1], 0.75] }));
            }
        }
    }
    json!({ "size": [800, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": props,
        "surfaces": [
            road(at(bearing, -HALF_LENGTH_M, 0.0), at(bearing, HALF_LENGTH_M, 0.0)),
            road(at(bearing, -CROSS_M, 0.0), at(bearing, -CROSS_M, -BLOCK_M)),
            road(at(bearing, CROSS_M, 0.0), at(bearing, CROSS_M, -BLOCK_M)),
            road(at(bearing, -CROSS_M, -BLOCK_M), at(bearing, CROSS_M, -BLOCK_M)),
        ] })
}

fn scenario(map: Value, units: Value) -> ScenarioDefinition {
    serde_json::from_value(json!({
        "map": map, "rules": common::scenario_rules(), "units": units, "events": [], "scripts": [],
    }))
    .unwrap()
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

/// Send `unit` to `goal` by road.
fn send(b: &mut Battle, seq: u64, unit: u32, goal: [f64; 2]) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq,
        order: Order::Move {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

/// Where every parked car stands.
fn parked(b: &Battle) -> Vec<V2> {
    let car = common::kind("parked_car");
    b.world()
        .props()
        .filter(|prop| prop.kind == car)
        .map(|prop| prop.center)
        .collect()
}

fn near(b: &Battle, unit: u32, goal: [f64; 2]) -> bool {
    let p = own(b, unit).position;
    (v2(p[0], p[1]) - v2(goal[0], goal[1])).length() < 3.0
}

/// One vehicle of `kind` driven the length of the street, with the cars on
/// its right (`forward`) or on its left: the seconds it took.
fn drive(kind: &str, bearing: f64, cars: bool, forward: bool) -> f64 {
    let way = if forward { 1.0 } else { -1.0 };
    let (from, goal) = (
        at(bearing, -way * END_M, 0.0),
        at(bearing, way * END_M, 0.0),
    );
    let yaw = if forward {
        bearing
    } else {
        bearing + std::f64::consts::PI
    };
    let units = json!([{ "side": "blue", "kind": kind, "position": from, "yaw": yaw }]);
    let mut b = Battle::new(&scenario(street(bearing, cars), units), 1);
    let before = parked(&b);
    send(&mut b, 1, 0, goal);
    let hz = b.rules().tick_hz as u64;
    let what = format!(
        "a {kind} on a street at {:.0} degrees, cars {}",
        bearing.to_degrees(),
        match (cars, forward) {
            (false, _) => "nowhere",
            (true, true) => "on its right",
            (true, false) => "on its left",
        }
    );
    for _ in 0..600 * hz {
        b.step();
        let unit = own(&b, 0);
        let off = across(bearing, unit.position);
        assert!(
            off.abs() < 3.5,
            "{what}: it left the carriageway, {off:.1} m right of the street's middle at {:?}",
            unit.position
        );
        assert_ne!(
            unit.state,
            MoveState::RouteBlocked,
            "{what}: the street is refused"
        );
        if unit.state == MoveState::Idle {
            break;
        }
    }
    assert!(
        near(&b, 0, goal),
        "{what}: it never arrived: {:?} at {:?}",
        own(&b, 0).state,
        own(&b, 0).position
    );
    for (was, now) in before.iter().zip(parked(&b)) {
        assert!(
            (*was - now).length() < 0.01,
            "{what}: the car at {was:?} was shoved to {now:?}"
        );
    }
    b.tick() as f64 / hz as f64
}

/// The moment: a tank, a truck and a jeep each come down a 7 m street
/// between terraces with cars parked along one kerb. Each drives the length
/// of it, either way: it does not go round the block instead, it passes
/// every car without touching it, and it takes little longer than on the
/// empty street.
#[test]
fn every_hull_drives_a_parked_street_end_to_end_without_shoving_a_car() {
    // A street along the grid, and two that cut across it.
    for bearing in [0.0f64, 30.0, 45.0].map(f64::to_radians) {
        for kind in ["tank", "supply", "jeep"] {
            let bare = drive(kind, bearing, false, true);
            for forward in [true, false] {
                let took = drive(kind, bearing, true, forward);
                assert!(
                    took < 1.5 * bare + 5.0,
                    "a {kind} took {took:.0} s down the parked street at {:.0} degrees, {bare:.0} s down the empty one",
                    bearing.to_degrees()
                );
            }
        }
    }
}

/// The moment: two vehicles are sent through the same parked street from
/// opposite ends and meet in it. Both get through: neither waits for ever,
/// and their hulls never overlap.
#[test]
fn two_vehicles_sent_through_a_parked_street_from_opposite_ends_both_arrive() {
    for bearing in [0.0f64, 30.0].map(f64::to_radians) {
        for (east, west) in [("tank", "supply"), ("supply", "jeep"), ("tank", "tank")] {
            let ends = [at(bearing, -END_M, 0.0), at(bearing, END_M, 0.0)];
            let units = json!([
                { "side": "blue", "kind": east, "position": ends[0], "yaw": bearing },
                { "side": "blue", "kind": west, "position": ends[1],
                  "yaw": bearing + std::f64::consts::PI },
            ]);
            let mut b = Battle::new(&scenario(street(bearing, true), units), 1);
            // Each to 60 m short of where the other starts.
            let goals = [
                at(bearing, END_M - 60.0, 0.0),
                at(bearing, 60.0 - END_M, 0.0),
            ];
            send(&mut b, 1, 0, goals[0]);
            send(&mut b, 2, 1, goals[1]);
            let what = format!(
                "a {east} and a {west} meeting on a street at {:.0} degrees",
                bearing.to_degrees()
            );
            let hz = b.rules().tick_hz as u64;
            for _ in 0..300 * hz {
                b.step();
                let hull = |id| b.unit(UnitId(id)).unwrap().hull_box().unwrap();
                assert!(
                    !hull(0).overlaps(&hull(1)),
                    "{what}: their hulls overlap at tick {}",
                    b.tick()
                );
                if [0, 1]
                    .iter()
                    .all(|id| own(&b, *id).state == MoveState::Idle)
                {
                    break;
                }
            }
            for (id, goal) in [(0, goals[0]), (1, goals[1])] {
                assert!(
                    near(&b, id, goal),
                    "{what}: unit {id} never arrived: {:?} at {:?}, after {} s",
                    own(&b, id).state,
                    own(&b, id).position,
                    b.tick() / hz
                );
            }
        }
    }
}
