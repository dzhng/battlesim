//! Vehicle drive (Q29–Q31): tracks pivot, wheels hold their
//! turning radius, reverse is slower and holds the facing, and a reverse
//! order changes the deterministic battle state.
use contract::ids::UnitId;
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::wrap_angle;
use std::f64::consts::FRAC_PI_2;

fn rules() -> Value {
    sim::fixtures::test_rules()
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
        "test_supply",
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
    for kind in ["test_jeep", "test_supply"] {
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
    for kind in ["test_tank", "test_supply", "test_jeep"] {
        let forward = pace(kind, 0.0, "forward");
        let reverse = pace(kind, std::f64::consts::PI, "reverse");
        let fraction = match mobility(kind) {
            contract::catalog::Mobility::Tracked {
                reverse_speed_factor,
                ..
            }
            | contract::catalog::Mobility::Wheeled {
                reverse_speed_factor,
                ..
            } => reverse_speed_factor,
            contract::catalog::Mobility::Foot { .. } | contract::catalog::Mobility::Air { .. } => {
                panic!("{kind} is not a ground vehicle")
            }
        };
        assert!(
            (reverse / forward - fraction).abs() < 1e-6,
            "{kind}: {reverse:.3} m against {forward:.3} m, expected {fraction}"
        );
    }
}

#[test]
fn a_reverse_order_holds_the_facing_and_drives_backwards() {
    for kind in ["test_tank", "test_supply"] {
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
fn an_armoured_vehicle_runs_over_a_chainlink_fence() {
    let fixture = rules();
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                  "props": [{ "kind": "chainlink_fence", "center": [80.0, 60.0], "yaw": 0.0,
                               "half_extents": [0.15, 8.0, 1.0] }] },
        "rules": fixture,
        "units": [{ "side": "blue", "kind": "test_apc", "position": [20.0, 60.0], "yaw": 0.0,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": [140.0, 60.0], "route": "shortest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let poses = drive(&mut b, 30.0);
    let tank = b.unit(UnitId(0)).unwrap();
    assert_eq!(tank.state, MoveState::Idle, "it is still {:?}", tank.state);
    assert!((tank.position.xy() - sim::math::v2(140.0, 60.0)).length() < 1.0);
    assert!(poses
        .iter()
        .any(|(p, _, _)| p.x > 80.0 && (p.y - 60.0).abs() < 1.0));
    assert!(!b
        .world()
        .props()
        .any(|p| p.kind == crate::common::kind("chainlink_fence")));
}

#[test]
fn a_reverse_order_repeats_from_its_seed_and_changes_the_digest() {
    let run = |direction: &str| {
        let mut b = battle(
            "test_tank",
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
/// side's identified entry carries `reversing` (the reverse beeper's cue).
#[test]
fn a_seen_enemy_reversing_is_published_to_the_observer() {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] },
        "rules": rules(),
        "units": [
            { "side": "blue", "kind": "test_tank", "position": [20, 60], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [110, 60], "yaw": std::f64::consts::PI,
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
    for kind in ["test_jeep", "test_tank"] {
        let mobility = sim::units::ground_mobility(rules.catalog.by_id(kind), &rules).unwrap();
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
        let expected =
            road * mobility.road_factor([Some(contract::map::SurfaceKind::DirtTrack), None]);
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
    let foot = sim::units::ground_mobility(rules.catalog.by_id("test_rifle"), &rules).unwrap();
    let dirt = foot.road_factor([Some(contract::map::SurfaceKind::DirtTrack), None]);
    assert!(
        foot.road_mps * dirt < foot.off_road_mps,
        "the case this test needs"
    );
    let road = |kind| sim::navigation::Footing {
        paved: [Some(kind), None],
        forest: false,
        swamp: false,
        slope_deg: 0.0,
        crater_fill: 0.0,
    };
    use contract::map::SurfaceKind;
    assert_eq!(foot.speed(road(SurfaceKind::DirtTrack)), foot.off_road_mps);
    assert_eq!(foot.speed(road(SurfaceKind::Road)), foot.road_mps);
    assert_eq!(foot.speed(road(SurfaceKind::Paving)), foot.off_road_mps);
}

/// Every locomotion names every surface kind, with factors a route's
/// best-case estimate can trust (never faster than the unit's road speed),
/// and no forest stops a mover dead (it would freeze a unit inside one and
/// divide that estimate by zero): the rules are refused at load, by name.
#[test]
fn every_speed_factor_is_refused_at_load_outside_its_bounds() {
    let load = |edit: &dyn Fn(&mut Value)| {
        let mut raw = rules();
        edit(&mut raw["movement"]["speed_factors"]["wheeled"]);
        serde_json::from_value::<contract::scenario::Rules>(raw).map_err(|e| e.to_string())
    };
    assert!(load(&|_| {}).is_ok());
    assert!(load(&|w| {
        w["surfaces"].as_object_mut().unwrap().remove("dirt_track");
    })
    .is_err());
    for bad in [-0.1, 1.1] {
        let error = load(&|w| w["surfaces"]["dirt_track"] = json!(bad)).unwrap_err();
        assert!(
            error.contains("speed_factors.wheeled.surfaces.DirtTrack"),
            "{bad}: {error}"
        );
    }
    let error = load(&|w| w["forest"] = json!(0.0)).unwrap_err();
    assert!(error.contains("speed_factors.wheeled.forest"), "{error}");
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
        "test_supply",
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

/// The moment: a light tank under the trees, a trunk off its front right,
/// is told to come round to a spot 2 m behind its left shoulder and face
/// back the way it came. Pivoting on the spot swings its rear quarter into
/// the trunk whichever way it turns, so the crew rolls along its length
/// until the turn is clear, comes round, and finishes facing as ordered.
/// (A played battle's M10 Booker, its trunks and its order, as they stood;
/// before, it froze against the trunk for every later order.)
/// A long truck whose tail stands in the mouth of an alley cannot turn
/// toward its way where it stands: its tail would swing into the building
/// beside the alley, and there is no room to back. It rolls straight ahead
/// until the turn clears, rather than standing there for good.
#[test]
fn a_truck_with_its_tail_in_an_alley_rolls_ahead_until_it_can_turn() {
    // The street, the truck's pose and its next waypoint as a game truck met
    // them on a generated map, around the truck's spot.
    let origin = [80.0, 60.0];
    let props: Vec<Value> = [
        ("wall", 7.5453, 13.5108, -1.478504, 3.0000, 5.5000, 4.8750),
        ("wall", 8.0983, 7.5364, -1.478504, 3.0000, 5.5000, 4.8750),
        ("wall", 8.6513, 1.5619, -1.478504, 3.0000, 5.5000, 4.8750),
        ("wall", 10.5485, -8.0571, -1.478504, 3.7500, 6.5000, 5.3750),
        ("wall", 11.2397, -15.5251, -1.478504, 3.7500, 6.5000, 5.3750),
        (
            "parked_car",
            -12.2247,
            14.9908,
            -1.478504,
            2.1000,
            0.9000,
            0.7500,
        ),
        (
            "parked_car",
            -11.7847,
            10.3108,
            -1.478504,
            2.1000,
            0.9000,
            0.7500,
        ),
        (
            "parked_car",
            -11.3547,
            5.6308,
            -1.478504,
            2.1000,
            0.9000,
            0.7500,
        ),
        ("hydrant", 0.0553, 6.0108, -1.478504, 0.2000, 0.2000, 0.4000),
        (
            "scooter", -0.3147, 11.1508, -1.478504, 0.8000, 0.3000, 0.5500,
        ),
    ]
    .into_iter()
    .map(|(kind, x, y, yaw, hx, hy, hz)| {
        json!({ "kind": kind, "center": [origin[0] + x, origin[1] + y], "yaw": yaw,
                "half_extents": [hx, hy, hz] })
    })
    .collect();
    let goal = [origin[0] - 6.3047, origin[1] + 5.5108];
    // A truck as long and narrow as the game's.
    let mut fixture = rules();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_supply",
        json!({ "body": { "hull": { "half_extents_m": [5.2, 1.2, 1.8] } } }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": props },
        "rules": fixture,
        "units": [{ "side": "blue", "kind": "test_supply", "position": origin, "yaw": -4.104237,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": goal, "route": "shortest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    drive(&mut b, 40.0);
    let unit = b.unit(UnitId(0)).unwrap();
    assert!(
        (unit.position.xy() - sim::math::v2(goal[0], goal[1])).length() < 2.0,
        "it reaches {goal:?}: at {:?}, {:?}",
        unit.position.xy(),
        unit.state
    );
}

#[test]
fn a_tank_with_a_trunk_beside_it_rolls_clear_to_turn_and_faces_as_ordered() {
    let mut fixture = rules();
    // The test tank in the Booker's body: a medium hull, which a trunk
    // of the same weight class stops.
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_tank",
        json!({
            "body": { "hull": { "half_extents_m": [3.5, 1.7, 1.3],
                                "weight_class": "medium",
                                "wreck": "medium_wreck" } },
            "mobility": { "tracked": { "offroad_kmh": 30, "road_kmh": 64, "turn_deg_s": 50,
                                       "reverse_speed_factor": 0.4 } }
        }),
    );
    let at = [60.0, 40.0];
    let trunk = |rel: [f64; 2]| {
        let center = [at[0] + rel[0], at[1] + rel[1]];
        json!({ "kind": "trunk", "center": center, "yaw": 0, "half_extents": [0.35, 0.35, 5.0] })
    };
    let props: Vec<Value> = [
        [2.44, -1.37],
        [14.05, -1.21],
        [-7.52, 8.0],
        [5.23, 8.43],
        [9.51, 6.45],
        [18.84, 8.4],
        [-4.63, 19.24],
        [3.15, 18.0],
        [11.01, 20.43],
        [23.55, 16.28],
    ]
    .into_iter()
    .map(trunk)
    .collect();
    let goal = [at[0] - 2.137, at[1] - 0.847];
    let facing = -1.675;
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": fixture,
        "units": [{ "side": "blue", "kind": "test_tank", "position": at, "yaw": 0.3837,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": goal, "route": "shortest", "direction": "forward",
            "facing": facing } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    for _ in 0..30 * 30 {
        b.step();
        let tank = b.unit(UnitId(0)).unwrap();
        let hull = tank.ground_footprint().unwrap();
        assert!(
            !b.world()
                .props()
                .filter(|p| p.blocks(contract::map::MoverClass::Vehicle))
                .any(|p| hull.overlaps(&p.footprint())),
            "the hull is in a trunk at {:?}",
            hull.center
        );
        if b.tick() > 2 && tank.state == MoveState::Idle && tank.turn_to.is_none() {
            break;
        }
    }
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        tank.state == MoveState::Idle
            && (tank.position.xy() - sim::math::v2(goal[0], goal[1])).length() < 1.0,
        "it is {:?} at {:?}, not at its spot",
        tank.state,
        tank.position.xy()
    );
    assert!(
        wrap_angle(tank.yaw - facing).abs() < 2f64.to_radians(),
        "it faces {:.3}, not the ordered {facing}",
        tank.yaw
    );
}

/// The moment: a tank pulls up beside another parked half a metre off its
/// flank and is told to face toward it. Turning on the spot would swing its
/// corners through the other hull, so the crew gives the turn up rather
/// than drive into a friend.
#[test]
fn a_tank_at_rest_never_turns_into_the_tank_beside_it() {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules(),
        "units": [
            { "side": "blue", "kind": "test_tank", "position": [60.0, 40.0], "yaw": 0,
              "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_tank", "position": [70.0, 44.1], "yaw": 0,
              "engagement": "return_fire_only" },
        ],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": [70.0, 40.0], "route": "shortest", "direction": "forward",
            "facing": std::f64::consts::FRAC_PI_2 } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    for _ in 0..30 * 30 {
        b.step();
        let hull = |id| b.unit(UnitId(id)).unwrap().ground_footprint().unwrap();
        assert!(
            !hull(0).overlaps(&hull(1)),
            "the tank turned into its neighbour at {:?}",
            hull(0)
        );
        let tank = b.unit(UnitId(0)).unwrap();
        if b.tick() > 2 && tank.state == MoveState::Idle && tank.turn_to.is_none() {
            break;
        }
    }
    let tank = b.unit(UnitId(0)).unwrap();
    assert_eq!(tank.state, MoveState::Idle, "it is still {:?}", tank.state);
    assert!(tank.turn_to.is_none(), "it still means to turn");
}

/// Every tick's poses of a `kind` sent by road from `at` (facing `yaw`) to
/// `goal` over the road strokes `roads` (`[points, width]`).
fn by_road(
    kind: &str,
    roads: &[(Value, f64)],
    at: [f64; 2],
    yaw: f64,
    goal: [f64; 2],
) -> (Battle, Vec<(sim::math::V2, f64, bool)>) {
    let surfaces: Vec<Value> = roads
        .iter()
        .map(|(points, width)| json!({ "kind": "road", "shape": { "kind": "stroke", "points": points, "width_m": width } }))
        .collect();
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [320, 320], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "surfaces": surfaces },
        "rules": rules(),
        "units": [{ "side": "blue", "kind": kind, "position": at, "yaw": yaw,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": goal, "route": "fastest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let poses = drive(&mut b, 90.0);
    (b, poses)
}

/// A road that turns back on itself sharper than a right angle, on its own
/// bend or where one road meets another, is driven round forward: a truck
/// whose turning circle is wider than the corner swings through it and
/// never stops to back and fill.
#[test]
fn a_truck_drives_round_a_sharp_road_corner_without_reversing() {
    let corners = [
        // One road's own corner, its runs too short to round it wide.
        vec![(
            json!([[60, 10], [60, 160], [62, 190], [100, 182], [300, 140]]),
            10.0,
        )],
        // Two roads meeting at a corner, which no stroke rounds.
        vec![
            (json!([[60, 10], [60, 190]]), 10.0),
            (json!([[60, 190], [300, 140]]), 10.0),
        ],
    ];
    for (i, roads) in corners.iter().enumerate() {
        let (b, poses) = by_road(
            "test_supply",
            roads,
            [60.0, 20.0],
            FRAC_PI_2,
            [280.0, 144.0],
        );
        let truck = b.unit(UnitId(0)).unwrap();
        assert_eq!(
            truck.state,
            MoveState::Idle,
            "corner {i}: still {:?} at {:?}",
            truck.state,
            truck.position.xy()
        );
        assert!(
            (truck.position.xy() - sim::math::v2(280.0, 144.0)).length() < 2.0,
            "corner {i}: stopped at {:?}",
            truck.position.xy()
        );
        let backed: Vec<_> = poses.iter().filter(|p| p.2).map(|p| p.0).collect();
        assert!(
            backed.is_empty(),
            "corner {i}: backed up at {:?}",
            backed.first()
        );
    }
}

/// The moment: a tank sent by road comes to a junction where its way turns
/// a right angle. The crew slows and sweeps round the corner; it never
/// stops in the road to turn on the spot and set off again. It turns on
/// the spot only at the end, to the facing it was ordered.
#[test]
fn a_tank_drives_round_corners_and_pivots_only_to_its_final_facing() {
    let road = |points: Value| json!({ "kind": "road", "shape": { "kind": "stroke", "points": points, "width_m": 10 } });
    let (goal, facing) = ([280.0, 190.0], std::f64::consts::PI);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [320, 320], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "surfaces": [road(json!([[60, 10], [60, 190]])), road(json!([[60, 190], [300, 190]]))] },
        "rules": rules(),
        "units": [{ "side": "blue", "kind": "test_tank", "position": [60, 20], "yaw": FRAC_PI_2,
                    "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": goal, "route": "fastest", "direction": "forward",
            "facing": facing } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let mut was = b.unit(UnitId(0)).unwrap().clone();
    let mut pivots_on_the_way = Vec::new();
    for _ in 0..90 * 30 {
        b.step();
        let tank = b.unit(UnitId(0)).unwrap();
        let still = (tank.position.xy() - was.position.xy()).length() < 1e-9;
        if tank.route.is_some() && was.route.is_some() && still && tank.yaw != was.yaw {
            pivots_on_the_way.push(tank.position.xy());
        }
        was = tank.clone();
        if b.tick() > 2 && tank.state == MoveState::Idle && tank.turn_to.is_none() {
            break;
        }
    }
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        tank.state == MoveState::Idle
            && (tank.position.xy() - sim::math::v2(goal[0], goal[1])).length() < 1.0,
        "it is {:?} at {:?}, not at its spot",
        tank.state,
        tank.position.xy()
    );
    assert!(
        pivots_on_the_way.is_empty(),
        "it stopped to turn on the spot on its way, at {pivots_on_the_way:?}"
    );
    assert!(
        wrap_angle(tank.yaw - facing).abs() < 2f64.to_radians(),
        "it faces {:.3}, not the ordered {facing}",
        tank.yaw
    );
}
