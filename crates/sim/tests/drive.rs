//! Vehicle drive (Q29–Q31): tracks pivot, wheels hold their
//! turning radius, reverse is slower and holds the facing, and a reverse
//! order changes the deterministic battle state.
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
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
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
fn a_reverse_order_repeats_from_its_seed_and_changes_the_digest() {
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
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] },
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

/// Metres a vehicle covers in one second at cruising speed along a straight
/// 12 m strip of `surface` (open ground when `None`).
fn cruise(kind: &str, surface: Option<&str>) -> f64 {
    let surfaces = surface.map_or(json!([]), |kind| {
        json!([{ "kind": kind, "shape": { "kind": "stroke", "points": [[0, 60], [700, 60]], "width_m": 12 } }])
    });
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [700, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "surfaces": surfaces },
        "rules": rules(),
        "units": [{ "side": "blue", "kind": kind, "position": [20, 60], "yaw": 0.0,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": [680, 60], "route": "fastest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let poses = drive(&mut b, 22.0);
    (poses[21 * 30].0 - poses[20 * 30].0).length()
}

/// Q-G4: each road kind carries its own data-driven speed. A country road is
/// today's road, a dirt track is slower but still beats the field beside it,
/// and a sidewalk is no road at all.
#[test]
fn each_road_kind_carries_its_own_speed() {
    let rules: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let factor = |kind| rules.surfaces[&kind].speed_factor;
    for kind in ["jeep", "tank"] {
        let (open, road) = (cruise(kind, None), cruise(kind, Some("road")));
        assert!(
            road > open * 1.5,
            "{kind}: road {road:.2} vs open {open:.2}"
        );
        let country = cruise(kind, Some("country_road"));
        assert!(
            (country - road).abs() < 1e-9,
            "{kind}: country road {country} vs {road}"
        );
        let dirt = cruise(kind, Some("dirt_track"));
        let expected = road * factor(contract::map::SurfaceKind::DirtTrack);
        assert!(
            (dirt - expected).abs() < 1e-6,
            "{kind}: dirt {dirt} vs {expected}"
        );
        assert!(
            dirt > open && dirt < road,
            "{kind}: dirt {dirt} between {open} and {road}"
        );
        let sidewalk = cruise(kind, Some("sidewalk"));
        assert!(
            (sidewalk - open).abs() < 1e-9,
            "{kind}: sidewalk {sidewalk} vs {open}"
        );
    }
}

/// A surface never makes a mover slower than the open ground beside it: a
/// squad on foot gains nothing from a dirt track, and loses nothing.
#[test]
fn a_slow_surface_is_never_slower_than_open_ground() {
    let rules: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let foot = sim::units::mobility(rules.catalog.by_id("rifle"), &rules);
    let dirt = rules.surfaces[&contract::map::SurfaceKind::DirtTrack].speed_factor;
    assert!(
        foot.road_mps * dirt < foot.off_road_mps,
        "the case this test needs"
    );
    assert_eq!(foot.speed(dirt, false, 0.0), foot.off_road_mps);
    assert_eq!(foot.speed(1.0, false, 0.0), foot.road_mps);
    assert_eq!(foot.speed(0.0, false, 0.0), foot.off_road_mps);
}

/// The rules name every surface kind, with a factor a route's best-case
/// estimate can trust (never faster than the unit's road speed).
#[test]
fn the_surface_table_must_cover_every_kind_within_bounds() {
    let load = |edit: &dyn Fn(&mut Value)| {
        let mut raw = rules();
        edit(&mut raw);
        serde_json::from_value::<contract::scenario::Rules>(raw)
    };
    assert!(load(&|_| {}).is_ok());
    assert!(load(&|r| {
        r["surfaces"].as_object_mut().unwrap().remove("dirt_track");
    })
    .is_err());
    for bad in [-0.1, 1.1] {
        assert!(
            load(&|r| r["surfaces"]["dirt_track"]["speed_factor"] = json!(bad)).is_err(),
            "{bad}"
        );
    }
}
