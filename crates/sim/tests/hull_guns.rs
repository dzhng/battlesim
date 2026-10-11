//! A gun fixed in its hull bears where the body faces (D8): it fires only
//! once the body has turned onto its target. A helicopter turns its airframe
//! to aim while it flies on; tracks pivot it at rest; wheels, which cannot
//! turn on the spot, hold fire. A turret keeps traversing on its own.

use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use std::f64::consts::PI;

/// The test rules plus hull-gun variants: a tank whose cannon is fixed in its
/// hull, a jeep whose autocannon is, and a helicopter with a fixed autocannon.
fn fixture() -> Value {
    let mut fixture = sim::fixtures::test_game();
    let fixed = |id: &str, weapons: &str| {
        json!([{ "id": id, "name": "Fixed gun", "weapons": [weapons], "turret": false,
            "pivot_m": [0, 0, 1.2], "muzzle_m": [3.0, 0, 0] }])
    };
    // It sees all round, so what it may not do is turn, not spot.
    let all_round =
        json!({ "ground_m": 700, "sight_shape": { "front": 1.0, "side": 1.0, "rear": 1.0 } });
    fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "units": {
            "test_hull_gun_tank": { "extends": "test_tank", "name": "Assault Gun",
                "description": "Test only: the tank's cannon fixed in its hull.",
                "sensors": all_round, "mounts": fixed("gun", "tank_he") },
            "test_hull_gun_jeep": { "extends": "test_jeep", "name": "Gun Truck",
                "description": "Test only: an autocannon fixed facing forward.",
                "sensors": all_round, "mounts": fixed("gun", "autocannon") },
            "test_hull_gun_heli": { "extends": "test_heli", "name": "Gunship",
                "description": "Test only: an autocannon fixed under the nose.",
                "mounts": fixed("gun", "autocannon") }
        }}));
    fixture
}

fn battle(units: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture(), "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Rounds the shooter's fixed gun has fired (its parent's turret guns,
/// merged in by id, are not under test).
fn gun_shots(b: &Battle) -> u32 {
    let u = b.unit(UnitId(0)).unwrap();
    let specs = b.rules().catalog.mounts(u.kind);
    (u.mounts.iter().zip(specs))
        .filter(|(_, s)| s.def.id == "gun")
        .map(|(m, _)| m.shots)
        .sum()
}

/// Each tick for `seconds`: the shooter's yaw, and whether its fixed gun fired.
fn watch(b: &mut Battle, seconds: u32) -> Vec<(f64, bool)> {
    let mut out = Vec::new();
    let mut shots = 0;
    for _ in 0..seconds * b.rules().tick_hz {
        b.step();
        let now = gun_shots(b);
        out.push((b.unit(UnitId(0)).unwrap().yaw, now > shots));
        shots = now;
    }
    out
}

fn off(yaw: f64, bearing: f64) -> f64 {
    sim::math::wrap_angle(yaw - bearing).abs()
}

/// A tolerance for "facing": the rules' bearing tolerance, and a hair.
fn tolerance(b: &Battle) -> f64 {
    b.rules().movement.bearing_tolerance_deg.to_radians() + 1e-6
}

#[test]
fn a_hull_gun_holds_fire_until_the_body_faces_its_target() {
    // The target is due west; the assault gun faces east.
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_hull_gun_tank", "position": [400, 300], "yaw": 0.0 },
        { "side": "red", "kind": "test_tank", "position": [150, 300], "yaw": 0.0,
          "engagement": "return_fire_only" },
    ]));
    let tol = tolerance(&b);
    let track = watch(&mut b, 30);
    assert!(track.iter().any(|&(_, fired)| fired), "it never fired");
    for &(yaw, fired) in &track {
        if fired {
            assert!(off(yaw, PI) <= tol, "fired facing {yaw:.2}, not west");
        }
    }
}

#[test]
fn a_wheeled_hull_gun_facing_away_holds_fire() {
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_hull_gun_jeep", "position": [400, 300], "yaw": 0.0 },
        { "side": "red", "kind": "test_jeep", "position": [150, 300], "yaw": 0.0,
          "engagement": "return_fire_only" },
    ]));
    let track = watch(&mut b, 20);
    assert!(track.iter().all(|&(_, fired)| !fired), "it fired backwards");
    assert!(
        track.iter().all(|&(yaw, _)| yaw == 0.0),
        "wheels turned it on the spot"
    );
}

#[test]
fn a_tracked_hull_gun_on_the_move_never_turns_aside_or_fires_backwards() {
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_hull_gun_tank", "position": [300, 300], "yaw": 0.0 },
        { "side": "red", "kind": "test_tank", "position": [100, 300], "yaw": 0.0,
          "engagement": "return_fire_only" },
    ]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [700.0, 300.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let tol = tolerance(&b);
    let track = watch(&mut b, 8);
    for &(yaw, fired) in &track {
        assert!(
            off(yaw, 0.0) < 0.3,
            "it turned aside to {yaw:.2} on the move"
        );
        if fired {
            assert!(off(yaw, PI) <= tol, "it fired backwards");
        }
    }
}

#[test]
fn a_helicopter_turns_its_airframe_to_aim_while_it_flies_on() {
    // Flying north; the target lies due east.
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_hull_gun_heli", "position": [300, 100], "yaw": std::f64::consts::FRAC_PI_2 },
        { "side": "red", "kind": "test_jeep", "position": [550, 250], "yaw": 0.0,
          "engagement": "return_fire_only" },
    ]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [300.0, 500.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let tol = tolerance(&b);
    let mut fired_flying = false;
    for _ in 0..12 * b.rules().tick_hz {
        let before = gun_shots(&b);
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        let after = gun_shots(&b);
        let v = heli.air.unwrap().velocity;
        if after > before && v.length() > 5.0 {
            let to = sim::math::v2(550.0, 250.0) - heli.position.xy();
            let bearing = libm::atan2(to.y, to.x);
            assert!(off(heli.yaw, bearing) <= tol, "fired off its nose");
            // Flying one way, facing another.
            let travel = libm::atan2(v.y, v.x);
            fired_flying |= off(travel, bearing) > 0.5;
        }
    }
    assert!(fired_flying, "it never fired while flying a different way");
}

/// Once on target, a gunship flying past keeps its fixed gun on it through
/// every aim, burst and reload: it never swings back to its heading between
/// shots.
#[test]
fn a_gunship_flying_past_holds_its_aim_through_its_bursts_and_reloads() {
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_hull_gun_heli", "position": [300, 100], "yaw": std::f64::consts::FRAC_PI_2 },
        { "side": "red", "kind": "test_tank", "position": [550, 250], "yaw": 0.0,
          "engagement": "return_fire_only" },
    ]));
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [300.0, 500.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let tol = tolerance(&b);
    let mut on_target = false;
    let mut checked = 0;
    for _ in 0..12 * b.rules().tick_hz {
        let before = gun_shots(&b);
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        if !b.unit(UnitId(1)).unwrap().alive() {
            break;
        }
        on_target |= gun_shots(&b) > before;
        // Only on the way past: hovering, it keeps whatever heading it has.
        if heli.air.unwrap().velocity.length() < 5.0 {
            continue;
        }
        if on_target {
            let to = sim::math::v2(550.0, 250.0) - heli.position.xy();
            let bearing = libm::atan2(to.y, to.x);
            // The target's bearing drifts as it flies; one tick's turn more.
            assert!(
                off(heli.yaw, bearing) <= tol + 0.05,
                "it swung {:.3} rad off its target between shots",
                off(heli.yaw, bearing)
            );
            checked += 1;
        }
    }
    assert!(checked > 10, "it held its aim only {checked} ticks");
}
