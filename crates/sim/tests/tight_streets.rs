//! A tank in walled streets barely wider than itself: it gets where it is
//! sent by a way its hull can really drive, and never wedges itself across
//! the street or against a wall on the way.
use crate::common;

use contract::ids::UnitId;
use contract::observation::MoveState;
use contract::scenario::WeightClass;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, wrap_angle, V2};

const TANK: &str = "test_tank";

/// Half the tank's length and half its width.
fn hull() -> V2 {
    hull_of(TANK)
}

fn hull_of(kind: &str) -> V2 {
    let half = common::hull(kind).half_extents_m;
    v2(half[0], half[1])
}

fn wall(x: [f64; 2], y: [f64; 2]) -> Value {
    json!({ "kind": "wall", "center": [(x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0], "yaw": 0,
        "half_extents": [(x[1] - x[0]) / 2.0, (y[1] - y[0]) / 2.0, 1.5] })
}

fn map(size: [f64; 2], props: Vec<Value>) -> String {
    json!({ "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

/// A battle with the tank at `at` facing east, sent to `goal`.
fn sent(map: String, at: [f64; 2], goal: [f64; 2]) -> Battle {
    sent_as(TANK, map, at, goal)
}

/// A battle with a `kind` at `at` facing east, sent to `goal`.
fn sent_as(kind: &str, map: String, at: [f64; 2], goal: [f64; 2]) -> Battle {
    let setup = common::scenario_with(
        &map,
        json!([{ "side": "blue", "kind": kind, "position": at, "yaw": 0.0,
            "engagement": "return_fire_only" }]),
        json!([]),
        json!([{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": goal, "route": "shortest" } }]),
    );
    Battle::new(&setup, 1)
}

/// Step for `seconds`, calling `each` with the tank after every tick.
fn drive(b: &mut Battle, seconds: f64, mut each: impl FnMut(&sim::units::Unit)) {
    for _ in 0..(seconds * f64::from(b.rules().tick_hz)) as u64 {
        b.step();
        each(b.unit(UnitId(0)).unwrap());
    }
}

fn arrived(b: &Battle, goal: [f64; 2]) -> bool {
    let tank = b.unit(UnitId(0)).unwrap();
    tank.state == MoveState::Idle && (tank.position.xy() - v2(goal[0], goal[1])).length() < 1.0
}

/// Two streets as wide as the tank is long meet at a right angle, and the
/// tank is sent from one into the other. The straight line to its goal cuts
/// across the block, but it parks in the far street the way it drives in,
/// along it: it goes there, and is not sent somewhere else for want of room
/// to stand across the street.
#[test]
fn a_tank_sent_round_a_street_corner_goes_there() {
    let w = hull().x;
    let (corner, far) = (v2(80.0, 30.0), [80.0, 105.0]);
    let mut b = sent(
        map(
            [140.0, 120.0],
            vec![
                wall([0.0, 140.0], [0.0, corner.y - w]),
                wall([corner.x + w, 140.0], [corner.y - w, 120.0]),
                wall([0.0, corner.x - w], [corner.y + w, 120.0]),
            ],
        ),
        [15.0, corner.y],
        far,
    );
    drive(&mut b, 90.0, |_| {});
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        arrived(&b, far),
        "it ended {:?} at {:?}",
        tank.state,
        tank.position.xy()
    );
}

/// A street the tank is too long to turn round in, and a goal behind it.
/// It backs down the street to it: it never starts a turn it cannot finish
/// and wedges itself across the street.
#[test]
fn a_tank_sent_back_down_a_street_too_narrow_to_turn_in_backs_out() {
    let half = hull();
    // Narrower than the hull is long from corner to corner.
    let w = half.x + 0.3;
    assert!(w < half.length());
    let (y, goal) = (30.0, [40.0, 30.0]);
    let mut b = sent(
        map(
            [160.0, 60.0],
            vec![
                wall([20.0, 120.0], [0.0, y - w]),
                wall([20.0, 120.0], [y + w, 60.0]),
            ],
        ),
        [100.0, y],
        goal,
    );
    let (mut swung, mut backed) = (0.0_f64, false);
    drive(&mut b, 90.0, |tank| {
        swung = swung.max(wrap_angle(tank.yaw).abs().to_degrees());
        backed |= tank.reversing;
    });
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        arrived(&b, goal),
        "it ended {:?} at {:?}",
        tank.state,
        tank.position.xy()
    );
    assert!(backed, "it never backed");
    assert!(
        swung < 20.0,
        "it swung {swung:.0} degrees across the street"
    );
}

/// A truck in a street too narrow for it to turn round in, sent back the
/// way it came. It does not stand at the street's mouth with its order,
/// working out again and again a manoeuvre whose first step the wall
/// refuses: it comes round and arrives.
#[test]
fn a_truck_sent_back_down_a_street_too_narrow_to_turn_in_gets_there() {
    let truck = "test_supply";
    let w = hull_of(truck).x + 0.3;
    let (y, goal) = (30.0, [40.0, 30.0]);
    let mut b = sent_as(
        truck,
        map(
            [160.0, 60.0],
            vec![
                wall([20.0, 120.0], [0.0, y - w]),
                wall([20.0, 120.0], [y + w, 60.0]),
            ],
        ),
        [100.0, y],
        goal,
    );
    drive(&mut b, 60.0, |_| {});
    let unit = b.unit(UnitId(0)).unwrap();
    assert!(
        arrived(&b, goal),
        "it ended {:?} at {:?}",
        unit.state,
        unit.position.xy()
    );
}

/// A walled street with a body the tank cannot shove parked in it, leaving
/// `spare` metres more than the tank's width between it and the far wall.
/// How the tank, sent down the street from a little off its middle, ends.
fn past_a_parked_body(spare: f64) -> (Battle, [f64; 2], f64) {
    let half = hull();
    let (y, street, goal) = (30.0, half.y + 2.6, [105.0, 30.0]);
    let (body, body_half) = (60.0, v2(2.1, 0.9));
    // The body's far side stands `2 * half.y + spare` short of the north wall.
    let body_y = y + street - 2.0 * half.y - spare - body_half.y;
    let mut setup = common::scenario_with(
        &map(
            [120.0, 60.0],
            vec![
                wall([0.0, 120.0], [0.0, y - street]),
                wall([0.0, 120.0], [y + street, 60.0]),
                json!({ "kind": "crate", "center": [body, body_y], "yaw": 0,
                    "half_extents": [body_half.x, body_half.y, 0.75] }),
            ],
        ),
        json!([{ "side": "blue", "kind": TANK, "position": [15.0, y], "yaw": 0.0,
            "engagement": "return_fire_only" }]),
        json!([]),
        json!([{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": goal, "route": "shortest" } }]),
    );
    let weight = WeightClass::Heavy;
    assert!(!common::hull(TANK).shoves(weight));
    setup.rules = setup
        .rules
        .overridden(
            json!({ "props": { "crate": { "body": { "weight_class": weight, "topples": false } } } })
                .as_object()
                .unwrap(),
        )
        .unwrap();
    (Battle::new(&setup, 1), goal, body - body_half.x)
}

/// The gap beside the body has room to spare each side of the hull: the
/// tank comes into it lined up with it, though it drove up at a slant, and
/// passes without touching the wall.
#[test]
fn a_tank_passes_a_parked_body_where_the_gap_leaves_it_room() {
    let (mut b, goal, _) = past_a_parked_body(1.2);
    drive(&mut b, 60.0, |_| {});
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        arrived(&b, goal),
        "it ended {:?} at {:?}",
        tank.state,
        tank.position.xy()
    );
}

/// The gap is wider than the hull by less than the room a driver keeps each
/// side: it is no way through, and the tank does not drive into it and jam
/// there with its route "blocked".
#[test]
fn a_tank_does_not_squeeze_into_a_gap_with_no_room_to_spare() {
    // Half the spare room the rules have a route keep, a margin each side.
    let spare = common::rules().navigation.hull_margin_m;
    let (mut b, _, body_near_end) = past_a_parked_body(spare);
    let mut blocked = false;
    drive(&mut b, 60.0, |tank| {
        blocked |= tank.state == MoveState::RouteBlocked
    });
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(!blocked, "it reported its route blocked");
    assert!(
        tank.position.x + hull().x < body_near_end,
        "it drove into the gap: it ended at {:?}",
        tank.position.xy()
    );
}
