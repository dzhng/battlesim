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
