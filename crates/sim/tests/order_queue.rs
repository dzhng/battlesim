//! The order queue: what a unit does after its current order, and when
//! that order ends. Driven through real scenarios and commands.
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::units::UnitOrder;

use crate::common::{self, Commander};

const BUILDING: u32 = 0;

fn town(units: Value) -> Battle {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, "china-apartment-point-20x20-7f", [400.0, 300.0], 0.0)] })
    .to_string();
    Battle::new(&common::scenario(&map, units, json!([])), 1)
}

fn ticks(seconds: f64) -> u64 {
    (seconds * f64::from(common::tick_hz())).round() as u64
}

fn until(b: &mut Battle, limit: u64, what: &str, mut test: impl FnMut(&Battle) -> bool) {
    for _ in 0..limit {
        if test(b) {
            return;
        }
        b.step();
    }
    panic!("{what} within {limit} ticks");
}

fn alive(b: &Battle, id: u32) -> bool {
    b.unit(UnitId(id)).is_some_and(|u| u.alive())
}

/// A garrisoned squad told to attack an enemy, then to leave, leaves once
/// the enemy is dead: the attack ends from inside the building as it does
/// anywhere else.
#[test]
fn a_garrisoned_attack_ends_when_its_target_dies() {
    let mut b = town(json!([
        { "side": "blue", "kind": "test_rifle", "position": [350.0, 300.0], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_rifle", "position": [470.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::OccupyBuilding {
            units: vec![UnitId(0)],
            building: BUILDING,
            gesture: 0,
        },
    );
    until(&mut b, ticks(90.0), "the squad inside", |b| {
        b.unit(UnitId(0)).unwrap().garrisoned()
    });
    until(&mut b, ticks(30.0), "the enemy seen", |b| {
        !b.observe(Side::Blue).identified.is_empty()
    });
    let enemy = b.observe(Side::Blue).identified[0].id;
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: enemy },
        },
    );
    let exit = Order::ExitBuilding {
        units: vec![UnitId(0)],
    };
    assert_eq!(c.send(&mut b, Side::Blue, exit, true), None);
    until(&mut b, ticks(240.0), "the enemy dead", |b| !alive(b, 1));
    until(&mut b, ticks(5.0), "the attack over", |b| {
        !matches!(
            b.unit(UnitId(0)).unwrap().orders.front(),
            Some(UnitOrder::Attack { .. })
        )
    });
    until(&mut b, ticks(60.0), "the squad out", |b| {
        !b.unit(UnitId(0)).unwrap().garrisoned()
    });
}

/// A unit driving home keeps going home when an order is queued behind
/// it: a queued order waits its turn and never cancels what the unit is
/// doing now, and nothing follows a unit that leaves.
#[test]
fn a_queued_order_does_not_cancel_a_drive_home() {
    use crate::transport::{side_command, transport_skirmish_setup};
    use contract::command::CommandEnvelope;
    let mut b = Battle::new(&transport_skirmish_setup(), 41);
    let buy = side_command(Side::Blue, 1, Order::purchase("test_apc", [300.0, 100.0]));
    assert_eq!(b.accept(buy).error, None);
    b.accept(side_command(Side::Blue, 2, Order::Ready));
    b.accept(side_command(Side::Red, 1, Order::Ready));
    until(&mut b, ticks(30.0), "the carrier arrived", |b| {
        b.unit(UnitId(0)).is_some()
    });
    let out = [300.0, 300.0];
    assert_eq!(
        b.accept(side_command(Side::Blue, 3, drive(out))).error,
        None
    );
    until(&mut b, ticks(60.0), "the carrier out on the field", |b| {
        let at = b.unit(UnitId(0)).unwrap().position.xy();
        (at - sim::math::v2(out[0], out[1])).length() < 10.0
    });
    let home = Order::Refund {
        units: vec![UnitId(0)],
    };
    assert_eq!(b.accept(side_command(Side::Blue, 4, home)).error, None);
    b.step();
    let onward = CommandEnvelope {
        queued: true,
        ..side_command(Side::Blue, 5, drive(out))
    };
    assert_eq!(b.accept(onward).error, None);
    until(&mut b, ticks(120.0), "the carrier gone home", |b| {
        b.unit(UnitId(0)).unwrap().retired
    });
}

fn drive(goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal,
        route: contract::command::RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}
