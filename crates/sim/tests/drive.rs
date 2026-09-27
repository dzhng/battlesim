//! Vehicle drive (slice 39, Q29–Q31): tracks pivot, wheels hold their
//! turning radius, reverse is slower and holds the facing, and a reverse
//! order is part of the replayed state.
use contract::ids::UnitId;
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::wrap_angle;

fn rules() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/village.json")).unwrap()
}

/// One vehicle of `kind` on open flat ground at `at` facing `yaw`, moved to
/// `goal` at tick 1 in `direction`, with optional `props`.
fn battle(
    kind: &str,
    at: [f64; 2],
    yaw: f64,
    goal: [f64; 2],
    direction: &str,
    props: Value,
) -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": rules(),
        "units": [{ "side": "blue", "kind": kind, "position": at, "yaw": yaw,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": goal, "route": "shortest", "direction": direction } }],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn body(kind: &str, column: &str) -> f64 {
    rules()["bodies"][kind][column].as_f64().unwrap()
}

/// Every tick's (position, yaw, reversing) until the unit is idle.
fn drive(b: &mut Battle, seconds: f64) -> Vec<(sim::math::V2, f64, bool)> {
    let mut poses = Vec::new();
    for _ in 0..(seconds * 30.0) as usize {
        b.step();
        let u = b.unit(UnitId(0)).unwrap();
        poses.push((u.position.xy(), u.yaw, u.reversing));
        if b.tick() > 2 && u.state == MoveState::Idle {
            break;
        }
    }
    poses
}

/// No tick turns the hull more than the distance it rolled allows: at most
/// `1 / radius` per metre, and not at all standing still.
fn assert_within_radius(poses: &[(sim::math::V2, f64, bool)], radius: f64) {
    for (k, w) in poses.windows(2).enumerate() {
        let rolled = (w[1].0 - w[0].0).length();
        let turned = wrap_angle(w[1].1 - w[0].1).abs();
        assert!(
            turned <= rolled / radius * 1.001 + 1e-9,
            "tick {k}: turned {turned:.4} rad over {rolled:.4} m (radius {radius} m)"
        );
    }
}

#[test]
fn a_truck_turning_round_in_the_open_never_turns_tighter_than_its_radius() {
    let mut b = battle(
        "supply",
        [80.0, 60.0],
        0.0,
        [30.0, 60.0],
        "forward",
        json!([]),
    );
    let poses = drive(&mut b, 60.0);
    assert_eq!(
        b.unit(UnitId(0)).unwrap().state,
        MoveState::Idle,
        "it arrives"
    );
    assert_within_radius(&poses, body("supply", "turning_radius_m"));
    let (end, yaw, _) = *poses.last().unwrap();
    assert!((end - sim::math::v2(30.0, 60.0)).length() < 1.5);
    assert!(
        wrap_angle(yaw - std::f64::consts::PI).abs() < 0.5,
        "it turned round: yaw {yaw}"
    );
}

#[test]
fn a_truck_turning_round_in_a_lane_never_turns_tighter_than_its_radius() {
    let walls = json!([
        { "kind": "wall", "center": [70, 24.5], "yaw": 0, "half_extents": [40, 0.5, 1.5] },
        { "kind": "wall", "center": [70, 35.5], "yaw": 0, "half_extents": [40, 0.5, 1.5] },
    ]);
    let mut b = battle("supply", [70.0, 30.0], 0.0, [40.0, 30.0], "forward", walls);
    let poses = drive(&mut b, 90.0);
    assert_eq!(
        b.unit(UnitId(0)).unwrap().state,
        MoveState::Idle,
        "it arrives"
    );
    assert_within_radius(&poses, body("supply", "turning_radius_m"));
    assert!(poses.iter().any(|p| p.2), "a three-point turn backs up");
}

#[test]
fn a_tank_pivots_on_the_spot() {
    let mut b = battle(
        "tank",
        [80.0, 60.0],
        0.0,
        [40.0, 60.0],
        "forward",
        json!([]),
    );
    let poses = drive(&mut b, 4.0);
    let start = sim::math::v2(80.0, 60.0);
    // It has turned more than 90 degrees before it has moved half a metre.
    let turned_first = poses
        .iter()
        .take_while(|p| (p.0 - start).length() < 0.5)
        .map(|p| wrap_angle(p.1).abs())
        .fold(0.0, f64::max);
    assert!(
        turned_first > std::f64::consts::FRAC_PI_2,
        "turned {turned_first:.2} rad in place"
    );
}

/// Metres covered over the second second of a straight drive.
fn pace(kind: &str, yaw: f64, direction: &str) -> f64 {
    let mut b = battle(kind, [20.0, 60.0], yaw, [140.0, 60.0], direction, json!([]));
    let poses = drive(&mut b, 3.0);
    (poses[60].0 - poses[30].0).length()
}

#[test]
fn reverse_speed_is_the_fraction_of_forward() {
    for kind in ["tank", "supply", "jeep"] {
        let forward = pace(kind, 0.0, "forward");
        let reverse = pace(kind, std::f64::consts::PI, "reverse");
        let fraction = body(kind, "reverse_speed_fraction");
        assert!(
            (reverse / forward - fraction).abs() < 1e-6,
            "{kind}: {reverse:.3} m against {forward:.3} m, expected {fraction}"
        );
    }
}

#[test]
fn a_reverse_order_holds_the_facing_and_drives_backwards() {
    for kind in ["tank", "supply"] {
        let yaw = std::f64::consts::PI;
        let mut b = battle(kind, [20.0, 60.0], yaw, [60.0, 60.0], "reverse", json!([]));
        let poses = drive(&mut b, 40.0);
        assert_eq!(
            b.unit(UnitId(0)).unwrap().state,
            MoveState::Idle,
            "{kind} arrives"
        );
        assert!(
            poses.iter().all(|p| wrap_angle(p.1 - yaw).abs() < 1e-9),
            "{kind} holds its facing"
        );
        assert!(poses.iter().filter(|p| p.2).count() > 30, "{kind} reverses");
    }
}

#[test]
fn a_reverse_order_replays_and_is_in_the_digest() {
    let run = |direction: &str| {
        let mut b = battle(
            "tank",
            [20.0, 60.0],
            0.0,
            [60.0, 60.0],
            direction,
            json!([]),
        );
        for _ in 0..90 {
            b.step();
        }
        b.digest()
    };
    assert_eq!(run("reverse"), run("reverse"));
    assert_ne!(run("reverse"), run("forward"));
}
