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
    sim::fixtures::game()
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
    let mut fixture = rules();
    fixture["movement"]["drive"]["acceleration_s"] = json!(4.5);
    fixture["movement"]["drive"]["braking_s"] = json!(1.5);
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "supply",
        json!({
            "capabilities": { "deploy": { "pack_seconds": 1 } }
        }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": fixture,
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

/// Metres covered at cruise, after the fixed acceleration and packing windows.
fn pace(kind: &str, yaw: f64, direction: &str) -> f64 {
    let mut b = battle(kind, [20.0, 60.0], yaw, [140.0, 60.0], direction, json!([]));
    let poses = drive(&mut b, 8.0);
    (poses[210].0 - poses[180].0).length()
}

#[test]
fn wheeled_vehicles_turn_back_within_fourteen_metres_of_the_starting_line() {
    for kind in ["jeep", "supply"] {
        let mut b = battle(kind, [80.0, 60.0], 0.0, [40.0, 60.0], "forward", json!([]));
        let poses = drive(&mut b, 60.0);
        assert_eq!(
            b.unit(UnitId(0)).unwrap().state,
            MoveState::Idle,
            "{kind} arrives"
        );
        let widest = poses
            .iter()
            .map(|p| (p.0.y - 60.0).abs())
            .fold(0.0, f64::max);
        assert!(
            widest <= 14.0,
            "{kind} turn needs {widest:.2} m beside its starting line"
        );
        assert!(
            (b.unit(UnitId(0)).unwrap().position.xy() - sim::math::v2(40.0, 60.0)).length() < 1.0,
            "{kind} reaches the destination behind it"
        );
    }
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
/// 12 m strip of `surface` (open ground when `None`): the 21st second of a
/// 860 m move, well clear of where it joins and leaves its side of a road.
fn cruise(kind: &str, surface: Option<&str>) -> f64 {
    let surfaces = surface.map_or(json!([]), |kind| {
        json!([{ "kind": kind, "shape": { "kind": "stroke", "points": [[0, 60], [900, 60]], "width_m": 12 } }])
    });
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [900, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "surfaces": surfaces },
        "rules": rules(),
        "units": [{ "side": "blue", "kind": kind, "position": [20, 60], "yaw": 0.0,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": [880, 60], "route": "fastest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let poses = drive(&mut b, 22.0);
    (poses[21 * 30].0 - poses[20 * 30].0).length()
}

/// Q-G4: each road kind carries its own data-driven speed. A country road is
/// today's road, a dirt track is slower but still beats the field beside it,
/// and paving (a yard, a court) is no road at all: hard ground, no way through.
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
        let paving = cruise(kind, Some("paving"));
        assert!(
            (paving - open).abs() < 1e-9,
            "{kind}: paving {paving} vs {open}"
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

/// The moment: a truck has nosed half into a gap in a row of parked cars,
/// a yard's wall just ahead of it, and its way on lies in the street behind
/// it. It backs out however the gap behind it lets it (turning toward its
/// way where it can, else straight), and drives on. (Market Town's supply
/// truck, as it stood.)
#[test]
fn a_truck_nosed_into_a_kerb_gap_backs_out_to_the_street_behind_it() {
    // A row of cars and the yard's wall behind it, along 101 degrees.
    let row = 1.77;
    let body = |kind: &str, at: [f64; 2], yaw: f64, half: [f64; 3]| json!({ "kind": kind, "center": at, "yaw": yaw, "half_extents": half });
    let car = |at| body("parked_car", at, row, [2.1, 0.9, 0.75]);
    let wall = |at, long| body("courtyard_wall", at, row, [long, 0.125, 1.0]);
    let props = json!([
        car([74.0, 54.69]),
        car([71.73, 66.12]),
        car([74.92, 50.08]),
        car([70.81, 70.73]),
        body("bins", [71.38, 58.03], row, [0.6, 0.4, 0.6]),
        wall([70.43, 54.44], 1.5),
        wall([69.85, 57.38], 1.5),
        wall([67.82, 67.58], 1.63),
        body("street_tree", [69.91, 63.72], row, [0.35, 0.35, 4.0]),
    ]);
    let mut b = battle(
        "supply",
        [72.98, 60.04],
        2.470,
        [84.0, 63.0],
        "forward",
        props,
    );
    drive(&mut b, 60.0);
    let truck = b.unit(UnitId(0)).unwrap();
    assert_eq!(
        truck.state,
        MoveState::Idle,
        "it is still {:?} at {:?}",
        truck.state,
        truck.position.xy()
    );
    assert!(
        (truck.position.xy() - sim::math::v2(84.0, 63.0)).length() < 1.0,
        "it stopped at {:?}, short of the street",
        truck.position.xy()
    );
}
