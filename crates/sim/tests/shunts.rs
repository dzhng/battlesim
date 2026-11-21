//! A wheeled vehicle in a tight spot works its way out as a driver would:
//! it looks at the room around it and drives the forward and reverse legs
//! that get it onto its way, never a fixed turn that wedges it.
use contract::ids::UnitId;
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use std::f64::consts::{FRAC_PI_2, PI};

/// A residential courtyard as a played battle's air-defence launcher met
/// it (metro, seed 3), shifted to this map: planters round its trees,
/// concrete ping-pong tables, the yard's wall, and the blocks closing it to
/// the east and south. Its way out is to the west; open ground beyond.
fn courtyard() -> Value {
    let body = |kind: &str, at: [f64; 2], yaw: f64, half: [f64; 3]| json!({ "kind": kind, "center": [at[0] - 1200.0, at[1] - 700.0], "yaw": yaw, "half_extents": half });
    let planter = |at, yaw| body("planter", at, yaw, [0.8, 0.4, 0.4]);
    let tree = |at| body("street_tree", at, FRAC_PI_2, [0.35, 0.35, 5.0]);
    let table = |at| body("pingpong_table", at, FRAC_PI_2, [1.37, 0.76, 0.45]);
    let wall = |at| body("courtyard_wall", at, -0.1, [5.29, 0.12, 1.0]);
    let block = |at, half: [f64; 2]| body("wall", at, 0.0, [half[0], half[1], 6.0]);
    json!([
        table([1269.0, 768.9]),
        table([1269.0, 762.9]),
        planter([1260.8, 776.0], -PI),
        planter([1261.2, 777.2], FRAC_PI_2),
        planter([1259.6, 776.4], FRAC_PI_2),
        planter([1260.0, 777.6], -PI),
        tree([1260.3, 776.8]),
        planter([1276.4, 768.5], 0.0),
        planter([1276.0, 767.3], -FRAC_PI_2),
        planter([1277.6, 768.1], -FRAC_PI_2),
        planter([1277.2, 766.9], 0.0),
        tree([1276.8, 767.7]),
        tree([1276.0, 776.4]),
        tree([1256.8, 769.0]),
        tree([1252.0, 780.0]),
        tree([1249.3, 772.9]),
        tree([1284.3, 768.5]),
        tree([1284.3, 764.1]),
        tree([1280.5, 758.2]),
        wall([1267.2, 785.1]),
        wall([1277.7, 784.1]),
        wall([1256.7, 786.2]),
        block([1292.0, 770.0], [2.0, 20.0]),
        block([1270.0, 752.0], [20.0, 2.0]),
    ])
}

/// A truck as long, wide and wide-turning as the launcher, standing at
/// `at` (game coordinates) facing `yaw`, sent out of the courtyard to the
/// open ground south-west of it.
fn launcher(at: [f64; 2], yaw: f64) -> Battle {
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({
            "body": { "hull": { "half_extents_m": [5.168, 1.25, 1.8] } },
            "mobility": { "wheeled": { "turning_radius_m": 8 } },
        }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": courtyard() },
        "rules": rules,
        "units": [{ "side": "blue", "kind": "test_supply", "position": [at[0] - 1200.0, at[1] - 700.0],
                    "yaw": yaw, "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
            "gesture": 1, "goal": GOAL, "route": "shortest", "direction": "forward" } }],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

const GOAL: [f64; 2] = [32.0, 45.0];

/// How a drive went: whether it reversed on the way, and the seconds it
/// took to finish.
struct Drove {
    reversed: bool,
    seconds: f64,
}

fn run(b: &mut Battle, seconds: f64) -> Drove {
    let hz = b.rules().tick_hz as u64;
    let mut drove = Drove {
        reversed: false,
        seconds: f64::INFINITY,
    };
    for tick in 1..=(seconds * hz as f64) as u64 {
        b.step();
        let unit = b.unit(UnitId(0)).unwrap();
        drove.reversed |= unit.reversing;
        if tick > 2 && unit.state == MoveState::Idle {
            drove.seconds = tick as f64 / hz as f64;
            break;
        }
    }
    drove
}

fn at_goal(b: &Battle) -> f64 {
    (b.unit(UnitId(0)).unwrap().position.xy() - sim::math::v2(GOAL[0], GOAL[1])).length()
}

/// The moment the audit found: the launcher has swung round against a
/// ping-pong table, a planter at its tail, its way out behind it. Backing
/// or turning toward its way meets one or the other within a metre, but
/// the courtyard ahead and to its left is open. A driver pulls forward
/// into the open and backs out the way he came; before, it stood there
/// "moving" for the rest of the battle.
#[test]
fn a_launcher_wedged_against_a_table_pulls_forward_and_backs_out() {
    let mut b = launcher([1267.276, 773.425], -0.5102);
    let drove = run(&mut b, 90.0);
    assert!(
        at_goal(&b) < 2.0,
        "it never got out: {:?} at {:?}",
        b.unit(UnitId(0)).unwrap().state,
        b.unit(UnitId(0)).unwrap().position.xy()
    );
    assert!(drove.reversed, "it got out without backing");
    assert!(drove.seconds < 60.0, "it took {} s", drove.seconds);
}

/// The launcher parked nose-in where it set up, the courtyard's way out
/// behind it and no room for a U-turn. A driver shunts out, backing where
/// he must, and drives off; in the battle it pulled forward, backed round
/// and wedged itself against the table for the rest of the game.
#[test]
fn a_launcher_parked_nose_in_in_a_courtyard_shunts_out() {
    let mut b = launcher([1266.7, 771.7], 0.31);
    let drove = run(&mut b, 90.0);
    assert!(
        at_goal(&b) < 2.0,
        "it never got out: {:?} at {:?}",
        b.unit(UnitId(0)).unwrap().state,
        b.unit(UnitId(0)).unwrap().position.xy()
    );
    assert!(drove.reversed, "it came round in the yard without backing");
    assert!(drove.seconds < 45.0, "it took {} s", drove.seconds);
}

/// Two trucks meet nose to nose in a street, each sent past the other.
/// Each waits a moment for the other; when neither gives way, both work
/// their way round, and both get where they were sent. Neither waits on
/// the other for ever.
#[test]
fn trucks_meeting_nose_to_nose_in_a_street_get_past_each_other() {
    let wall = |y: f64| json!({ "kind": "wall", "center": [80, y], "yaw": 0, "half_extents": [60, 0.5, 3] });
    let truck = |x: f64, yaw: f64| json!({ "side": "blue", "kind": "test_supply", "position": [x, 60], "yaw": yaw, "engagement": "return_fire_only" });
    let order = |unit: u32, goal: [f64; 2]| {
        json!({ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [unit],
        "gesture": unit + 1, "goal": goal, "route": "shortest", "direction": "forward" } })
    };
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [wall(51.5), wall(68.5)] },
        "rules": sim::fixtures::test_rules(),
        "units": [truck(74.0, 0.0), truck(82.0, PI)],
        "events": [],
        "scripts": [order(0, [130.0, 60.0]), order(1, [30.0, 60.0])],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..90 * hz {
        b.step();
    }
    for (unit, goal) in [(0, [130.0, 60.0]), (1, [30.0, 60.0])] {
        let truck = b.unit(UnitId(unit)).unwrap();
        assert!(
            (truck.position.xy() - sim::math::v2(goal[0], goal[1])).length() < 2.0,
            "truck {unit} is {:?} at {:?}",
            truck.state,
            truck.position.xy()
        );
    }
}

/// A truck pulled up nose to tail behind a parked one, closer than traffic
/// keeps, and sent on past it. It waits a moment, then works its way round
/// the parked truck and gets where it was sent; its way round never starts
/// with a step the traffic rule refuses, which left it waiting for minutes.
#[test]
fn a_truck_close_behind_a_parked_one_works_its_way_round_it() {
    let truck = |x: f64| json!({ "side": "blue", "kind": "test_supply", "position": [x, 60], "yaw": 0, "engagement": "return_fire_only" });
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": sim::fixtures::test_rules(),
        "units": [truck(80.0), truck(73.8)],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [1],
            "gesture": 1, "goal": [130, 60], "route": "shortest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..60 * hz {
        b.step();
    }
    let truck = b.unit(UnitId(1)).unwrap();
    assert!(
        (truck.position.xy() - sim::math::v2(130.0, 60.0)).length() < 2.0,
        "it is {:?} at {:?}",
        truck.state,
        truck.position.xy()
    );
}

/// The same truck under a commander who keeps changing his mind: sent on
/// past the parked truck to one spot and then another every second and a
/// half. A parked vehicle is going nowhere, so it does not wait on one at
/// all; it goes round at once, whatever the next order says.
#[test]
fn a_truck_reordered_again_and_again_still_goes_round_a_parked_one() {
    let truck = |x: f64| json!({ "side": "blue", "kind": "test_supply", "position": [x, 60], "yaw": 0, "engagement": "return_fire_only" });
    let scripts: Vec<Value> = (0..30)
        .map(|k| json!({ "tick": 1 + 45 * k, "side": "blue", "order": { "kind": "move", "units": [1],
            "gesture": k + 1, "goal": [130, if k % 2 == 0 { 30 } else { 90 }], "route": "shortest", "direction": "forward" } }))
        .collect();
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": sim::fixtures::test_rules(),
        "units": [truck(80.0), truck(73.8)],
        "events": [],
        "scripts": scripts,
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..40 * hz {
        b.step();
    }
    let truck = b.unit(UnitId(1)).unwrap();
    assert!(
        truck.position.x > 95.0,
        "it is {:?} at {:?}, behind the parked truck",
        truck.state,
        truck.position.xy()
    );
}

/// A truck behind a parked one in a street with no room to pass. It looks
/// for a way round and finds none. Nothing has changed for another look to
/// find, so it does not look again along the same way each time its
/// patience with the truck ahead runs out: only in the search's own time.
#[test]
fn a_truck_with_no_way_round_a_parked_one_does_not_keep_looking_for_one() {
    let wall = |y: f64| json!({ "kind": "wall", "center": [80, y], "yaw": 0, "half_extents": [60, 0.5, 3] });
    let truck = |x: f64| json!({ "side": "blue", "kind": "test_supply", "position": [x, 60], "yaw": 0, "engagement": "return_fire_only" });
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [wall(57.3), wall(62.7)] },
        "rules": sim::fixtures::test_rules(),
        "units": [truck(80.0), truck(70.0)],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [1],
            "gesture": 1, "goal": [130, 60], "route": "shortest", "direction": "forward" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    let mut looked: Vec<(u64, Option<Vec<sim::math::V2>>)> = Vec::new();
    for _ in 0..30 * hz {
        b.step();
        let unit = b.unit(UnitId(1)).unwrap();
        if let Some(shunt) = unit.shunt.as_ref().filter(|s| s.legs.is_empty()) {
            if looked.last().is_none_or(|(tick, _)| *tick != shunt.tick) {
                looked.push((shunt.tick, unit.route.clone()));
            }
        }
    }
    assert!(!looked.is_empty(), "it never looked for a way round");
    for pair in looked.windows(2) {
        let again = pair[1].0 - pair[0].0;
        assert!(
            pair[0].1 != pair[1].1 || again > 3 * hz,
            "it looked again along the same way after {again} ticks"
        );
    }
}
