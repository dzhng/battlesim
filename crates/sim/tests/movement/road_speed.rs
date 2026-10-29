//! Vehicles building road speed and braking for a bend.
use super::*;

#[test]
fn a_supply_truck_builds_road_speed_instead_of_jumping_to_it() {
    let map = serde_json::json!({
        "size": [1000, 200], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "surfaces": [{ "kind": "road", "shape": { "kind": "stroke",
            "points": [[200, 100], [950, 100]], "width_m": 20 } }]
    })
    .to_string();
    let mut setup = common::scenario(
        &map,
        serde_json::json!([
            { "side": "blue", "kind": "test_supply", "position": [40, 100], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut rules = common::scenario_rules();
    // Its hull is past what the game admits.
    sim::fixtures::lift_hull_limits(&mut rules);
    rules["movement"]["drive"]["acceleration_s"] = serde_json::json!(4.5);
    rules["movement"]["drive"]["braking_s"] = serde_json::json!(1.5);
    rules["movement"]["speed_factors"]["wheeled"]["surfaces"]["road"] = serde_json::json!(1.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        serde_json::json!({
            "mobility": { "wheeled": { "offroad_kmh": 25, "road_kmh": 76,
                "turn_deg_s": 40, "turning_radius_m": 9, "reverse_speed_factor": 0.35 } },
            "capabilities": { "deploy": { "seconds": 3, "pack_seconds": 1 } }
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let hz = setup.rules.tick_hz;
    let mut b = Battle::new(&setup, 1);
    let mut orders = Orders { seq: 0 };
    orders.go(
        &mut b,
        &[0],
        [850.0, 100.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    let mut was_road = false;
    let mut entered = None;
    let mut road_speeds = Vec::new();
    for _ in 0..45 * hz {
        let before = b.unit(UnitId(0)).unwrap().position.xy();
        let road = b
            .world()
            .surface_at(before.x, before.y)
            .unwrap()
            .paved
            .is_some_and(|kind| kind.is_road());
        b.step();
        let after = b.unit(UnitId(0)).unwrap().position.xy();
        let speed = (after - before).length() * hz as f64;
        if road && !was_road {
            entered = Some(b.tick());
        }
        if entered.is_some() {
            road_speeds.push(speed);
            if road_speeds.len() >= (3.2 * hz as f64) as usize {
                break;
            }
        }
        was_road = road;
    }
    let full = 76.0 / 3.6;
    assert!(entered.is_some(), "truck reaches the road");
    assert!(
        road_speeds[0] < full * 0.8,
        "road entry jumped to {} m/s",
        road_speeds[0]
    );
    assert!(road_speeds[hz as usize] > 25.0 / 3.6 && road_speeds[hz as usize] < full * 0.9);
    assert!(
        road_speeds.last().unwrap() >= &(full * 0.99),
        "truck reaches road speed after about three seconds"
    );
}

#[test]
fn a_jeep_accelerates_from_rest_and_brakes_before_a_road_bend() {
    let map = serde_json::json!({
        "size": [500, 500], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "props": [{ "kind": "wall", "center": [350, 422.8], "yaw": 0,
            "half_extents": [60, 0.5, 1.5] }],
        "surfaces": [{ "kind": "road", "shape": { "kind": "stroke",
            "points": [[0, 50], [350, 50], [350, 450]], "width_m": 20 } }]
    })
    .to_string();
    let mut setup = common::scenario(
        &map,
        serde_json::json!([
            { "side": "blue", "kind": "test_jeep", "position": [20, 50], "yaw": 0 }
        ]),
        serde_json::json!([]),
    );
    let mut rules = common::scenario_rules();
    rules["movement"]["drive"]["acceleration_s"] = serde_json::json!(4.5);
    rules["movement"]["drive"]["braking_s"] = serde_json::json!(1.5);
    rules["movement"]["speed_factors"]["wheeled"]["surfaces"]["road"] = serde_json::json!(1.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        serde_json::json!({
            "body": { "hull": { "half_extents_m": [2.2, 1.0, 0.95] } },
            "mobility": { "wheeled": { "offroad_kmh": 32, "road_kmh": 110,
                "turn_deg_s": 60, "turning_radius_m": 6, "reverse_speed_factor": 0.5 } }
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let hz = setup.rules.tick_hz;
    let mut b = Battle::new(&setup, 1);
    let mut orders = Orders { seq: 0 };
    orders.go(&mut b, &[0], [350.0, 420.0], 1, RoutePolicy::Fastest, false);
    let full = 110.0 / 3.6;
    let mut peak: f64 = 0.0;
    let mut previous: f64 = 0.0;
    let mut braked_before_turn = false;
    let mut first = None;
    for _ in 0..35 * hz {
        let before = b.unit(UnitId(0)).unwrap().position.xy();
        b.step();
        let u = own(&b, 0);
        let speed = dist([before.x, before.y], xy(&u)) * hz as f64;
        first.get_or_insert(speed);
        if u.position[0] < 330.0 && u.yaw.abs() < 5.0f64.to_radians() {
            assert!(
                (speed - previous).abs() <= full / 1.5 / hz as f64 + 1e-6,
                "speed changes gradually on the incoming leg: {previous} -> {speed}"
            );
            peak = peak.max(speed);
            braked_before_turn |=
                u.position[0] > 250.0 && peak > full * 0.95 && speed < full * 0.85;
        }
        previous = speed;
        if u.state == MoveState::Idle {
            break;
        }
    }
    assert!(
        first.unwrap() < full * 0.1,
        "start from rest rather than road speed"
    );
    assert!(
        peak > full * 0.95,
        "straight road still reaches full speed: {peak}"
    );
    assert!(
        braked_before_turn,
        "brake while still facing along the incoming road"
    );
    let u = own(&b, 0);
    assert!(
        u.state == MoveState::Idle && dist(xy(&u), [350.0, 420.0]) < 2.0,
        "complete the turn and arrive: {:?} {:?}",
        u.state,
        u.position
    );
}
