//! A helicopter's missiles and rockets (D9, D38): it fires both on the move,
//! and its missile keeps guiding as it flies, because its row says it guides
//! on the move. A ground crew's missile still lets go as its launcher moves
//! (`guidance::moving_releases_at_once_and_frees_the_crew`). Rockets never
//! go up at aircraft.

use crate::common;
use crate::common::air_field::AirField;
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
        player: None,
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
    let mut fixture = sim::fixtures::rules();
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

    fn target_fuse(&self, _projectile: sim::flight::ProjectileId) -> Option<(UnitId, f64)> {
        self.target.map(|target| (target, 3.0))
    }
}

#[test]
fn a_missile_bursts_within_its_blast_radius_of_a_passing_jet() {
    let battle = AirField::new(3000.0)
        .aircraft("test_jet", [800.0, 300.0], 0.0)
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

/// A test jet whose only store is one guided, interceptable ground-attack
/// missile, sent at a red tank, with a red jeep beside the tank mounting a
/// guided interceptor whose `missile` class sits in `intercepts_in` alone.
/// Red's tank and jeep see high air `red_high_air_m` far. Returns the tank's
/// hit points before and after, and the shots the jet and the jeep fired.
fn jet_missile_at_a_covered_tank(intercepts_in: &str, red_high_air_m: f64) -> (f64, f64, u32, u32) {
    let mut b = covered_tank_battle(intercepts_in, red_high_air_m, None);
    let before = b.unit(UnitId(1)).unwrap().hp;
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
    }
    (
        before,
        b.unit(UnitId(1)).unwrap().hp,
        shots(&b, 0, "gun"),
        shots(&b, 2, "HMG"),
    )
}

/// [`jet_missile_at_a_covered_tank`]'s battle as the jet takes its attack
/// order, with a red squad placed fifth at `spotter`'s point,
/// seeing high air as far as its distance.
fn covered_tank_battle(
    intercepts_in: &str,
    red_high_air_m: f64,
    spotter: Option<([f64; 2], f64)>,
) -> Battle {
    let mut rules = sim::fixtures::test_game();
    rules["weapons"]["test_agm"] = json!({
        "name": "Test AGM", "description": "A test row.", "icon": "atgm",
        "interceptable": true, "stationary": false, "guidance": "on_the_move",
        "range_m": 1500, "min_range_m": 5, "speed_mps": 50, "accel_mps2": 140,
        "top_speed_mps": 300, "aim_s": 0.1, "reload_s": 4, "ammo": 1,
        "penetration": 400, "partial_from": 0.08, "damage": 400,
        "turn_deg_s": 60, "lifetime_s": 15, "near_miss_suppression": 0,
        "suppression_radius_m": 0, "blast_radius_m": 4, "structural_damage": 20,
        "targets": { "ground": { "classes": ["vehicle"], "range_m": 1500, "scatter_mrad": 0.5 } }
    });
    rules["weapons"]["test_interceptor"] = json!({
        "name": "Test SAM", "description": "A test row.", "icon": "aa_missile",
        "stationary": true, "guidance": "stationary", "range_m": 1500, "min_range_m": 50,
        "speed_mps": 100, "accel_mps2": 95, "top_speed_mps": 290,
        "aim_s": 0.5, "reload_s": 4, "ammo": 6, "penetration": 300, "partial_from": 0.08,
        "damage": 100, "turn_deg_s": 180,
        "lifetime_s": 10, "near_miss_suppression": 0, "suppression_radius_m": 0,
        "blast_radius_m": 3, "proximity_fuse_m": 5, "structural_damage": 20,
        "targets": { intercepts_in: { "classes": ["missile"], "range_m": 1500, "scatter_mrad": 0.5 } }
    });
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(300.0);
    rules["weapons"]["test_agm"]["round_cost"] = json!(1);
    rules["weapons"]["test_interceptor"]["round_cost"] = json!(1);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "sensors": { "ground_m": 1500 },
            "mounts": [{ "id": "gun", "name": "Stores", "weapons": ["test_agm"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0] }] }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "sensors": { "high_air_m": red_high_air_m },
            "mounts": [{ "id": "HMG", "name": "AA", "weapons": ["test_interceptor"],
            "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }] }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "sensors": { "high_air_m": red_high_air_m } }),
    );
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_jet", "position": [600, 2000], "yaw": 0.0 }),
        json!({ "side": "red", "kind": "test_tank", "position": [2400, 2000], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" }),
        json!({ "side": "red", "kind": "test_jeep", "position": [2450, 2040], "yaw": std::f64::consts::PI }),
        json!({ "side": "blue", "kind": "test_recon", "position": [1800, 2000], "engagement": "return_fire_only" }),
    ];
    if let Some((at, high_air_m)) = spotter {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_rifle",
            json!({ "sensors": { "high_air_m": high_air_m } }),
        );
        units.push(json!({ "side": "red", "kind": "test_rifle", "position": at,
            "engagement": "return_fire_only" }));
    }
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": units,
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    for _ in 0..30 {
        b.step();
    }
    let tank = b
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|t| (t.position[0] - 2400.0).abs() < 5.0)
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: contract::command::TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    b
}

/// A guided interceptor whose high-air layer lists `missile` shoots down the
/// missile a jet fires at the tank it covers, and the tank is untouched. The
/// same interceptor listing `missile` only in low air lets the jet's round
/// (fired from high air) through, and it strikes the tank.
#[test]
fn a_high_air_interceptor_shoots_down_a_jets_missile_and_the_tank_survives() {
    let (before, after, fired, intercepted) = jet_missile_at_a_covered_tank("high_air", 2000.0);
    assert_eq!(fired, 1, "the jet fired its one missile");
    assert!(intercepted > 0, "the interceptor never launched");
    assert_eq!(after, before, "the covered tank was hit");

    let (before, after, fired, intercepted) = jet_missile_at_a_covered_tank("low_air", 2000.0);
    assert_eq!(fired, 1, "the jet fired its one missile");
    assert_eq!(
        intercepted, 0,
        "a low-air interceptor engaged a high-air round"
    );
    assert!(after < before, "the jet's missile missed the open tank");
}

/// An interceptor fires only at a round its side can see, as at any other
/// target: with neither the tank nor the jeep able to see high air, the
/// jet's missile flies in unseen and strikes the tank.
#[test]
fn an_interceptor_holds_for_a_round_its_side_cannot_see() {
    let (before, after, fired, intercepted) = jet_missile_at_a_covered_tank("high_air", 0.0);
    assert_eq!(fired, 1, "the jet fired its one missile");
    assert_eq!(
        intercepted, 0,
        "the interceptor fired at a round nobody saw"
    );
    assert!(after < before, "the jet's missile missed the tank");
}

/// A guided interceptor is steered only at a round its side sees, as at
/// a unit: the jeep can't see high air, and once the jet's missile flies
/// out of the one red spotter's sight, its launcher's line of sight alone
/// keeps no guidance and the interceptor lets go.
#[test]
fn a_guided_interceptor_lets_go_of_a_round_its_side_loses_sight_of() {
    const SPOTTER: [f64; 2] = [1300.0, 2000.0];
    const SPOTTER_HIGH_AIR_M: f64 = 350.0;
    let mut b = covered_tank_battle("high_air", 0.0, Some((SPOTTER, SPOTTER_HIGH_AIR_M)));
    let spotter = v3(SPOTTER[0], SPOTTER[1], 0.0);
    let (mut guided_while_seen, mut checked) = (false, false);
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
        let Some((missile, _)) = b.rounds().find(|(_, r)| r.unit == UnitId(0)) else {
            continue;
        };
        let Some((interceptor, _)) = b.rounds().find(|(_, r)| r.unit == UnitId(2)) else {
            continue;
        };
        let supported = interceptor.guidance.is_some_and(|g| g.supported);
        // Past its sight, with a tick's flight and the eye's height to
        // spare.
        if (missile.position - spotter).length() > SPOTTER_HIGH_AIR_M + 50.0 {
            assert!(
                !supported,
                "the interceptor was guided at a round nobody sees"
            );
            checked = true;
        } else {
            guided_while_seen |= supported;
        }
    }
    assert!(
        guided_while_seen,
        "the interceptor was never guided at the seen round"
    );
    assert!(
        checked,
        "the round never flew out of the spotter's sight under guidance"
    );
}

/// How far from a helicopter 400 m off its first missile bursts, when a jeep
/// fires a stinger with this proximity fuse at it (none: it bursts only on
/// what it strikes).
fn burst_from_the_helicopter(fuse_m: Option<f64>) -> f64 {
    let mut rules = sim::fixtures::test_game();
    match fuse_m {
        Some(m) => rules["weapons"]["stinger"]["proximity_fuse_m"] = json!(m),
        None => {
            rules["weapons"]["stinger"]
                .as_object_mut()
                .unwrap()
                .remove("proximity_fuse_m");
        }
    }
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "HMG", "weapons": ["stinger"] }] }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0.0 },
            { "side": "red", "kind": "test_heli", "position": [500, 200], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    for _ in 0..600 {
        b.step();
        if let Some(blast) = b.observe(Side::Blue).blasts.first() {
            let heli = b.unit(UnitId(1)).unwrap().position;
            let p = blast.point;
            return (v3(p[0], p[1], p[2]) - heli).length();
        }
    }
    panic!("the stinger never burst");
}

/// A fuse bursts near whatever its round was fired at, on the ground as in
/// the air: an ATGM given a 30 m fuse bursts 30 m off the jeep it was fired
/// at, not on its hull.
#[test]
fn test_a_proximity_fuse_bursts_near_a_ground_target_too() {
    let mut rules = sim::fixtures::test_game();
    rules["weapons"]["atgm"]["proximity_fuse_m"] = json!(30.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "HMG", "weapons": ["atgm"] }] }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0.0 },
            { "side": "red", "kind": "test_jeep", "position": [500, 200], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let target = b.unit(UnitId(1)).unwrap();
    let (centre, reach) = (target.position, target.hull.unwrap().length());
    for _ in 0..600 {
        b.step();
        if let Some(blast) = b.observe(Side::Blue).blasts.first() {
            let p = blast.point;
            let off = (v3(p[0], p[1], p[2]) - centre).length();
            assert!(
                (30.0..=30.0 + reach).contains(&off),
                "burst {off:.1} m off the jeep"
            );
            return;
        }
    }
    panic!("the missile never burst");
}

#[test]
fn a_proximity_fused_missile_bursts_as_it_nears_the_aircraft() {
    // A fuse reaching 30 m bursts it 30 m off the helicopter's hull (7 m
    // from its centre at the most); without one it flies on into the hull.
    let fused = burst_from_the_helicopter(Some(30.0));
    assert!((30.0..=37.5).contains(&fused), "{fused}");
    let unfused = burst_from_the_helicopter(None);
    assert!(unfused < 10.0, "{unfused}");
}
