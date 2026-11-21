//! Detours round traffic: a vehicle that goes round another still finishes
//! the order it was given.
use super::*;

/// A tank driving through is halted on the jeep's marker before the jeep
/// arrives, so the jeep's detour round it cannot end there and is snapped
/// aside. When the tank drives on, the jeep finishes on its own marker, not
/// where the detour happened to end.
#[test]
fn a_detour_round_a_vehicle_on_the_marker_still_ends_on_the_marker() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "test_jeep", "position": [40, 100], "yaw": 0 },
        { "side": "blue", "kind": "test_tank", "position": [150, 70],
          "yaw": std::f64::consts::FRAC_PI_2 },
    ]);
    let map = r#"{"size":[300,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#;
    let mut b = Battle::new(&common::scenario(map, units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    let (marker, beyond) = ([150.0, 100.0], [150.0, 180.0]);
    o.go(&mut b, &[0], marker, 1, RoutePolicy::Shortest, false);
    o.go(&mut b, &[1], beyond, 2, RoutePolicy::Shortest, false);
    b.step();
    let (goal, _) = b.unit(UnitId(0)).unwrap().movement_goal().unwrap();
    assert!(dist([goal.x, goal.y], marker) < 1.5);
    let (mut waited, mut halted) = (false, false);
    for tick in 0..90 * 30 {
        b.step();
        waited |= own(&b, 0).state == MoveState::Waiting;
        if !halted && dist(xy(&own(&b, 1)), marker) < 1.0 {
            halted = true;
            let units = vec![UnitId(1)];
            o.send(&mut b, Order::Stop { units }, false);
        }
        if tick == 30 * 30 {
            assert!(halted, "the tank never came to the marker");
            o.go(&mut b, &[1], beyond, 3, RoutePolicy::Shortest, false);
        }
    }
    assert!(waited, "the jeep never met the tank");
    let jeep = own(&b, 0);
    assert_eq!(jeep.state, MoveState::Idle);
    assert!(
        dist(xy(&jeep), [goal.x, goal.y]) < 1.5,
        "the jeep stopped at {:?}, short of its marker {goal:?}",
        xy(&jeep)
    );
}
