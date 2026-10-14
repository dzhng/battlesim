//! A helicopter's missiles and rockets (D9, D38): it fires both on the move,
//! and its missile keeps guiding as it flies, because its row says it guides
//! on the move. A ground crew's missile still lets go as its launcher moves
//! (`guidance::moving_releases_at_once_and_frees_the_crew`). Rockets never
//! go up at aircraft.

use crate::common;
use crate::common::jet::JetField;
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{
    advance_projectiles, fire_round, Aim, Body, BodyId, FlightEvent, ImpactContext, ImpactDecision,
    ImpactResolver, Launch, Pose, Projectiles, Shape, Struck,
};
use sim::math::v3;

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
        slot: None,
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

struct TestFuse {
    target: Option<UnitId>,
}

impl ImpactResolver for TestFuse {
    fn resolve(&mut self, impact: &ImpactContext) -> ImpactDecision {
        match impact.struck {
            Struck::Body(_) => ImpactDecision::Detonate,
            _ => ImpactDecision::Stop,
        }
    }

    fn aircraft_fuse(&self, _projectile: sim::flight::ProjectileId) -> Option<(UnitId, f64)> {
        self.target.map(|target| (target, 3.0))
    }
}

#[test]
fn a_missile_bursts_within_its_blast_radius_of_a_passing_jet() {
    let battle = JetField::new(3000.0)
        .jet("test_jet", [800.0, 300.0], 0.0)
        .battle();
    let jet = battle.unit(UnitId(0)).unwrap();
    let half = jet.hull.unwrap();
    let body = Body {
        id: BodyId(sim::weapons::VEHICLE_BODY_BASE),
        unit: UnitId(0),
        shape: Shape::Box { half },
        from: Pose {
            base: jet.position,
            yaw: jet.yaw,
        },
        to: Pose {
            base: jet.position,
            yaw: jet.yaw,
        },
    };
    let mut projectiles = Projectiles::new(common::config());
    let _projectile = projectiles.launch(Launch {
        origin: v3(
            700.0,
            jet.position.y + half.y + 2.5,
            jet.position.z + half.z,
        ),
        carrier: v3(0.0, 0.0, 0.0),
        velocity: v3(290.0, 0.0, 0.0),
        gravity_scale: 0.0,
        lifetime_s: 1.0,
        suppression_radius_m: 0.0,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    });
    let mut events = Vec::new();
    let mut resolver = TestFuse {
        target: Some(UnitId(0)),
    };
    for _ in 0..20 {
        advance_projectiles(
            &mut projectiles,
            battle.world(),
            &[body],
            &mut events,
            &mut resolver,
        );
        if !projectiles.active().is_empty() {
            continue;
        }
        break;
    }
    let impact = events.iter().find_map(|event| match event {
        FlightEvent::Impact(impact) => Some(impact),
        _ => None,
    });
    let impact = impact.expect("the passing missile burst");
    assert!(impact.detonated);
    let lateral_surface_distance = (impact.point.y - jet.position.y).abs() - half.y;
    assert!(
        lateral_surface_distance <= 3.0 + 1e-6,
        "burst was {lateral_surface_distance:.3} m from the hull"
    );
}

#[test]
fn no_proximity_fuse_against_ground() {
    let battle = battle(json!([
        { "side": "red", "kind": "test_tank", "position": [800, 300], "yaw": 0.0 }
    ]));
    let tank = battle.unit(UnitId(0)).unwrap();
    let half = tank.hull.unwrap();
    let body = Body {
        id: BodyId(sim::weapons::VEHICLE_BODY_BASE),
        unit: UnitId(0),
        shape: Shape::Box { half },
        from: Pose {
            base: tank.position,
            yaw: tank.yaw,
        },
        to: Pose {
            base: tank.position,
            yaw: tank.yaw,
        },
    };
    let mut projectiles = Projectiles::new(common::config());
    projectiles.launch(Launch {
        origin: v3(
            700.0,
            tank.position.y + half.y + 2.5,
            tank.position.z + half.z,
        ),
        carrier: v3(0.0, 0.0, 0.0),
        velocity: v3(290.0, 0.0, 0.0),
        gravity_scale: 0.0,
        lifetime_s: 1.0,
        suppression_radius_m: 0.0,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    });
    let mut events = Vec::new();
    let mut resolver = TestFuse { target: None };
    advance_projectiles(
        &mut projectiles,
        battle.world(),
        &[body],
        &mut events,
        &mut resolver,
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, FlightEvent::Impact(_))),
        "a ground target got a proximity fuse"
    );
}

#[test]
fn an_air_missile_ends_at_its_lifetime() {
    let world = common::flat([10_000.0, 600.0], "");
    let profile = common::profile("air_missile");
    let mut projectiles = Projectiles::new(common::config());
    projectiles.launch(Launch {
        origin: v3(100.0, 300.0, 250.0),
        carrier: v3(0.0, 0.0, 0.0),
        velocity: v3(profile.speed_mps, 0.0, 0.0),
        gravity_scale: profile.gravity_scale,
        lifetime_s: profile.lifetime_s,
        suppression_radius_m: profile.suppression_radius_m,
        shooter: None,
        guidance: Some(sim::flight::Guidance {
            point: v3(10_000.0, 300.0, 250.0),
            turn_rad_s: profile.turn_rad_s.unwrap(),
            supported: false,
            loft: profile.loft,
        }),
        motor: profile.motor,
        fall: None,
    });
    let events = common::fly(&mut projectiles, &world, 400, |_| Vec::new());
    let expiry = events.iter().find_map(|(_, event)| match event {
        FlightEvent::Expired(expiry) => Some(expiry),
        _ => None,
    });
    let expiry = expiry.expect("the missile expired");
    assert_eq!(expiry.cause, sim::flight::Expiry::Lifetime);
    assert!(
        (expiry.time - 1.0).abs() < 1e-9,
        "expiry fraction was {}",
        expiry.time
    );
}

#[test]
fn a_seeker_steers_at_the_box_centre() {
    let world = common::flat([3000.0, 1000.0], "");
    let profile = common::profile("air_missile");
    let mut rng = contract::random::Rng::new(1);
    let (_, solution) = fire_round(
        &world,
        &common::config(),
        &profile,
        &Aim {
            origin: v3(300.0, 500.0, 250.0),
            target: v3(1000.0, 500.0, 251.6),
            target_velocity: v3(0.0, 0.0, 0.0),
            carrier: v3(0.0, 0.0, 0.0),
        },
        0.0,
        &mut rng,
        None,
        |_| false,
        |_| false,
    )
    .unwrap()
    .expect("the seeker solution");
    assert!(solution.intercept.z > 250.0, "the seeker aimed at the base");
}

#[test]
fn a_missile_engages_a_helicopter() {
    let row = sim::fixtures::test_game()["weapons"]["air_missile"].clone();
    assert!(row["targets"]["low_air"].is_object());
}

#[test]
fn a_climbing_targets_vertical_velocity_is_led() {
    let world = common::flat([3000.0, 1000.0], "");
    let profile = common::profile("air_missile");
    let mut rng = contract::random::Rng::new(2);
    let (_, solution) = fire_round(
        &world,
        &common::config(),
        &profile,
        &Aim {
            origin: v3(300.0, 500.0, 250.0),
            target: v3(1000.0, 500.0, 251.6),
            target_velocity: v3(0.0, 0.0, 30.0),
            carrier: v3(0.0, 0.0, 0.0),
        },
        0.0,
        &mut rng,
        None,
        |_| false,
        |_| false,
    )
    .unwrap()
    .expect("the climbing intercept");
    assert!(
        solution.intercept.z > 251.6,
        "vertical target motion was discarded"
    );
}
