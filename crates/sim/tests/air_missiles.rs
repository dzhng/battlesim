//! A helicopter's missiles and rockets (D9, D38): it fires both on the move,
//! and its missile keeps guiding as it flies, because its row says it guides
//! on the move. A ground crew's missile still lets go as its launcher moves
//! (`guidance::moving_releases_at_once_and_frees_the_crew`). Rockets never
//! go up at aircraft.

use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

/// The test rules plus an attack helicopter: a chin autocannon on a turret,
/// and rockets and missiles fixed in its hull.
fn fixture() -> Value {
    let mut fixture = sim::fixtures::test_game();
    let fixed = |id: &str, weapons: &str| {
        json!({ "id": id, "name": "Fixed launcher", "weapons": [weapons], "turret": false,
            "pivot_m": [1.0, 0, 0.6], "muzzle_m": [1.5, 0, 0] })
    };
    fixture["catalog"].as_array_mut().unwrap().push(json!({ "units": {
        "test_attack_heli": { "extends": "test_heli", "name": "Attack Helicopter",
            "description": "Test only: chin gun, rockets and missiles.",
            "body": { "hull": { "hp": 250, "armor": { "front": 25, "side": 15, "rear": 15, "roof": 10 } } },
            "mounts": [
                { "id": "chin", "name": "Chin gun", "weapons": ["autocannon"], "turret": true,
                  "pivot_m": [3.0, 0, 0], "muzzle_m": [1.0, 0, 0] },
                fixed("rockets", "rocket_pod"),
                fixed("missiles", "heli_atgm")
            ] }
    }}));
    fixture
}

fn battle(units: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": { "size": [1600, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture(), "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn fly(b: &mut Battle, goal: [f64; 2]) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
}

/// Shots red's mount `id` fired: unit 0's.
fn shots(b: &Battle, unit: u32, id: &str) -> u32 {
    let u = b.unit(UnitId(unit)).unwrap();
    let specs = b.rules().catalog.mounts(u.kind);
    (u.mounts.iter().zip(specs))
        .filter(|(_, s)| s.def.id == id)
        .map(|(m, _)| m.shots)
        .sum()
}

#[test]
fn a_helicopter_guides_its_missile_while_it_flies_and_kills_a_tank() {
    // Flying north past a tank 600 m to the east, inside its sight.
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_attack_heli", "position": [300, 100], "yaw": std::f64::consts::FRAC_PI_2 },
        { "side": "red", "kind": "test_tank", "position": [900, 300], "yaw": std::f64::consts::PI,
          "engagement": "return_fire_only" },
    ]));
    fly(&mut b, [300.0, 500.0]);
    let mut guided_flying = 0;
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        let flying = heli.air.unwrap().velocity.length() > 5.0;
        let guiding = heli.mounts.iter().any(|m| m.support.is_some());
        guided_flying += usize::from(flying && guiding);
        if !b.unit(UnitId(1)).unwrap().alive() {
            break;
        }
    }
    assert!(shots(&b, 0, "missiles") > 0, "it never fired a missile");
    assert!(
        guided_flying > 30,
        "it guided only {guided_flying} ticks while flying"
    );
    assert!(!b.unit(UnitId(1)).unwrap().alive(), "the tank survived");
}

#[test]
fn its_rockets_never_go_up_at_an_aircraft() {
    let mut b = battle(json!([
        { "side": "blue", "kind": "test_attack_heli", "position": [300, 300], "yaw": 0.0 },
        { "side": "red", "kind": "test_heli", "position": [600, 300], "yaw": std::f64::consts::PI,
          "engagement": "return_fire_only" },
    ]));
    for _ in 0..20 * b.rules().tick_hz {
        b.step();
    }
    assert_eq!(
        shots(&b, 0, "rockets"),
        0,
        "rockets went up at a helicopter"
    );
    assert_eq!(
        shots(&b, 0, "missiles"),
        0,
        "a ground missile went up at a helicopter"
    );
    assert!(shots(&b, 0, "chin") > 0, "the chin gun never fired");
}

#[test]
fn a_guided_row_must_say_how_it_guides() {
    let mut fixture = sim::fixtures::game();
    fixture["weapons"]["atgm"]
        .as_object_mut()
        .unwrap()
        .remove("guidance");
    let error = serde_json::from_value::<Rules>(fixture).expect_err("guidance missing");
    assert!(error.to_string().contains("guidance"), "{error}");
}
