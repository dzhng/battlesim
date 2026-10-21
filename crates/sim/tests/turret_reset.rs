use contract::command::Order;
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

use crate::common::{self, Commander};

fn map() -> String {
    json!({"size":[1200,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]}).to_string()
}

#[test]
fn an_idle_turret_returns_to_hull_heading_after_fifteen_seconds_without_an_enemy_lock() {
    let mut setup = common::scenario_with(
        &map(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [300, 300], "yaw": 0.0 },
            { "side": "red", "kind": "test_supply", "position": [300, 440], "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    for weapon in setup.rules.weapons.values_mut() {
        weapon.damage = 0.0;
    }
    let mut b = Battle::new(&setup, 5);
    let mut turned = false;
    for _ in 0..120 {
        b.step();
        turned |= b.unit(UnitId(0)).unwrap().mounts[0].bearing.abs() > 0.2;
    }
    assert!(turned, "the cannon turned toward the enemy");

    Commander::new().ok(
        &mut b,
        Side::Red,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [1100.0, 500.0],
            facing: None,
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
        },
    );
    for _ in 0..4000 {
        b.step();
    }
    let blue = b.unit(UnitId(0)).unwrap();
    let bearing = sim::math::wrap_angle(blue.mounts[0].bearing - blue.yaw);
    assert!(
        bearing.abs() < 0.05,
        "the idle cannon should face the hull: {bearing}"
    );
}
