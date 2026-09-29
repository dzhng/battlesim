//! Vehicle drive (Q29–Q31): tracks pivot, wheels hold their
//! turning radius, reverse is slower and holds the facing, and a reverse
//! order is part of the replayed state.
use contract::ids::UnitId;
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::wrap_angle;

fn rules() -> Value {
    sim::fixtures::village()
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

fn mobility(kind: &str) -> contract::catalog::Mobility {
    let rules: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    rules.catalog.by_id(kind).mobility
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
        let fraction = match mobility(kind) {
            contract::catalog::Mobility::Tracked {
                reverse_fraction, ..
            }
            | contract::catalog::Mobility::Wheeled {
                reverse_fraction, ..
            } => reverse_fraction,
            contract::catalog::Mobility::Foot { .. } => panic!("{kind} is on foot"),
        };
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

/// A seen enemy vehicle's reverse is as plain as its position: the observing
/// side's identified entry carries `reversing` (the reverse whine's cue).
#[test]
fn a_seen_enemy_reversing_is_published_to_the_observer() {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] },
        "rules": rules(),
        "units": [
            { "side": "blue", "kind": "tank", "position": [20, 60], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [110, 60], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" }
        ],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": [60, 60], "route": "shortest", "direction": "reverse" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let mut seen_reversing = false;
    for _ in 0..(20 * 30) {
        b.step();
        let red = b.observe(contract::ids::Side::Red);
        seen_reversing |= red.identified.iter().any(|e| e.reversing);
        // Never claims a reverse the unit isn't driving.
        let truth = b.unit(UnitId(0)).unwrap().reversing;
        assert!(red.identified.iter().all(|e| !e.reversing || truth));
    }
    assert!(seen_reversing, "red saw blue's tank backing up");
}
