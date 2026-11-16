//! Where a gun aims at a squad: its middle, or a soldier its side saw.
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

use crate::common;

/// A red rifle squad stands bunched round a parked car 50 m from a blue rifle
/// squad, its soldiers in plain view either side of it, so the middle of
/// what blue sees lies on the car. No round can break the car, so a shot at
/// that middle meets the car; blue fires at the soldiers it sees instead.
#[test]
fn a_gun_fires_at_the_soldiers_it_sees_round_the_car_that_hides_their_middle() {
    const CAR: [f64; 2] = [300.0, 300.0];
    const HALF: [f64; 2] = [1.0, 2.2];
    let map = json!({ "size": [600, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "buildings": [],
        "props": [{ "kind": "parked_car", "center": CAR, "yaw": 0,
                    "half_extents": [HALF[0], HALF[1], 0.75] }] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [250, 300] },
        { "side": "red", "kind": "test_rifle", "position": CAR, "yaw": std::f64::consts::PI,
          "engagement": "return_fire_only" },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    // Rounds that can't wear the car down, so nothing fires into it.
    for w in setup.rules.weapons.values_mut() {
        w.structural_damage = 0.0;
    }
    let mut b = Battle::new(&setup, 5);
    assert!(
        b.world().prop(0).unwrap().body.stops_rounds,
        "the car stops rounds"
    );
    b.step();
    let seen = &b.observe(Side::Blue).identified;
    let [squad] = seen.as_slice() else {
        panic!("blue sees the red squad: {seen:?}")
    };
    let on_car = |p: [f64; 3]| (p[0] - CAR[0]).abs() < HALF[0] && (p[1] - CAR[1]).abs() < HALF[1];
    assert!(
        on_car(squad.position) && squad.members.len() > 1,
        "blue sees several soldiers, their middle on the car: {squad:?}"
    );
    for _ in 0..150 {
        b.step();
        if b.rounds().any(|(_, r)| r.unit == UnitId(0)) {
            return;
        }
    }
    panic!(
        "blue never fires at the soldiers it sees: {:?}",
        b.unit(UnitId(0)).unwrap().mounts[0].reason
    );
}
