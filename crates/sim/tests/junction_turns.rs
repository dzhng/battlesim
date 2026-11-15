//! A wheeled vehicle turning at a town junction drives it the way a driver
//! would: it starts its turn before the corner and sweeps through, never
//! running past the turning and coming back, and never backing and filling
//! on a road it has the room to turn on.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

/// A generated town's T-junction, copied: a country road bending a little
/// at the junction, and a side road leaving it to the west from a point a
/// quarter metre off the bend, as the generator lays them. The far corner
/// of the turn is a pavement of street trees, bins and parked cars; the
/// near one is open apron in front of a block that rules out going across.
fn junction() -> Value {
    junction_with_bins([196.41, 184.93], 1.703145)
}

fn junction_with_bins(bins: [f64; 2], bins_yaw: f64) -> Value {
    let road = |points: Value| json!({ "kind": "country_road", "shape": { "kind": "stroke", "points": points, "width_m": 8 } });
    let prop = |kind: &str, center: [f64; 2], yaw: f64, half: [f64; 3]| json!({ "kind": kind, "center": center, "yaw": yaw, "half_extents": half });
    let (along, across) = (1.703145, -3.009271);
    let car = |c: [f64; 2], yaw: f64| prop("parked_car", c, yaw, [2.1, 0.9, 0.75]);
    let tree = |c: [f64; 2]| prop("street_tree", c, along, [0.35, 0.35, 5.0]);
    let mut props = vec![
        tree([195.68, 169.55]),
        tree([193.64, 184.91]),
        tree([212.2, 171.24]),
        tree([210.09, 187.1]),
        tree([205.87, 218.82]),
        tree([188.71, 190.12]),
        tree([177.55, 188.64]),
        tree([161.69, 186.53]),
        prop("bins", bins, bins_yaw, [0.6, 0.4, 0.6]),
        prop("bins", [199.08, 164.87], along, [0.6, 0.4, 0.6]),
        prop("planter", [177.62, 191.42], across, [0.8, 0.4, 0.4]),
        // The block in the near corner, square to the road.
        prop("wall", [122.6, 250.8], along, [45.0, 60.0, 8.0]),
    ];
    for c in [
        [197.65, 171.82],
        [197.03, 176.48],
        [196.41, 181.14],
        [210.43, 167.47],
    ] {
        props.push(car(c, along));
    }
    for c in [
        [209.48, 174.61],
        [208.76, 180.01],
        [208.14, 184.67],
        [203.81, 217.22],
    ] {
        props.push(car(c, along));
    }
    for c in [
        [186.75, 192.13],
        [182.09, 191.51],
        [173.72, 190.39],
        [169.06, 189.77],
    ] {
        props.push(car(c, across));
    }
    json!({ "size": [400, 450], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "surfaces": [
            road(json!([[220.25, 47.89], [200.0, 200.0], [168.93, 433.43]])),
            road(json!([[200.25, 200.03], [10.0, 174.71]])),
        ],
        "props": props })
}

fn turn_deg(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
    let cross = u[0] * v[1] - u[1] * v[0];
    let dot = u[0] * v[0] + u[1] * v[1];
    cross.atan2(dot).abs().to_degrees()
}

#[test]
fn a_truck_turning_off_a_town_road_sweeps_round_the_corner_without_stopping() {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": junction(), "rules": common::scenario_rules(), "events": [], "scripts": [],
        // Northbound in its lane, a hundred and thirty metres short of the turn.
        "units": [{ "side": "blue", "kind": "test_supply", "position": [181.0, 330.0], "yaw": -1.4385 }],
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let goal = [30.0, 177.4];
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    let hz = battle.rules().tick_hz as u64;
    let mut doubled_back = None;
    let mut reversed_at = None;
    let mut arrived = None;
    for tick in 1..=60 * hz {
        battle.step();
        let own = battle.observe(Side::Blue).own[0].clone();
        let here = [own.position[0], own.position[1]];
        let way = &own.route;
        if let Some(k) = (1..way.len().saturating_sub(1))
            .find(|&k| turn_deg(way[k - 1], way[k], way[k + 1]) > 120.0)
        {
            doubled_back.get_or_insert((tick, way[k]));
        }
        if battle.unit(UnitId(0)).unwrap().manoeuvre.is_some() {
            reversed_at.get_or_insert((tick, here));
        }
        if own.state == MoveState::Idle {
            arrived = Some((tick as f64 / hz as f64, here));
            break;
        }
    }
    assert_eq!(
        doubled_back, None,
        "the route ran past the turning and back"
    );
    assert_eq!(reversed_at, None, "it backed and filled at the junction");
    // About three hundred metres, most of it at road speed.
    let (seconds, at) = arrived.unwrap_or_else(|| {
        let own = &battle.observe(Side::Blue).own[0];
        panic!("it never arrived: {:?} at {:?}", own.state, own.position)
    });
    assert!(
        (at[0] - goal[0]).hypot(at[1] - goal[1]) < 3.0 && seconds < 40.0,
        "arrived at {at:?} after {seconds} s"
    );
}

/// The moment the generated map caught a Stryker in: a truck on the far
/// corner's pavement, nosed into bins it has shoved up against a parked
/// car's bumper, a street tree at its front corner, and sent on up the
/// road. The bins will not move, so it backs off the way it came and drives
/// on; it does not sit there for the rest of the battle.
#[test]
fn a_truck_nosed_into_bins_pinned_against_a_car_backs_off_and_drives_on() {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": junction_with_bins([196.50, 183.91], 1.8116), "rules": common::scenario_rules(),
        "events": [], "scripts": [],
        "units": [{ "side": "blue", "kind": "test_supply", "position": [195.42, 188.38], "yaw": -1.406 }],
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let goal = [212.0, 60.0];
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    let hz = battle.rules().tick_hz as u64;
    for _ in 0..60 * hz {
        battle.step();
        let own = &battle.observe(Side::Blue).own[0];
        assert_ne!(
            own.state,
            MoveState::RouteBlocked,
            "it gave up at {:?}",
            own.position
        );
        if own.state == MoveState::Idle {
            break;
        }
    }
    let own = &battle.observe(Side::Blue).own[0];
    assert!(
        own.state == MoveState::Idle
            && (own.position[0] - goal[0]).hypot(own.position[1] - goal[1]) < 3.0,
        "it never got away: {:?} at {:?}",
        own.state,
        own.position
    );
}
