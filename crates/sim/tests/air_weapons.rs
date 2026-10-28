//! Which weapons engage a helicopter (D1, D2), and from how far: rifles,
//! machine guns and autocannons swing up at it, each at a much shorter reach
//! and a much wider spread than along the ground; tank guns, grenades and
//! ground missiles never do. A rifle that cannot hurt it still fires at it,
//! for the sparks.

use super::air::{battle_with, fixture, open_map, order};
use crate::common::{config, flat, profile};
use contract::ids::UnitId;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{fire_round, predicted_path, Aim, Projectiles};
use sim::math::v3;

/// A hovering helicopter at [200, 200] and one red `kind` `metres` east of
/// it, facing it, after `seconds`.
fn against_at(fixture: &Value, kind: &str, metres: f64, seconds: u64) -> Battle {
    let mut b = battle_with(
        fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [200, 200], "engagement": "return_fire_only" },
            { "side": "red", "kind": kind, "position": [200.0 + metres, 200.0], "yaw": std::f64::consts::PI },
        ]),
    );
    for _ in 0..seconds * b.rules().tick_hz as u64 {
        b.step();
    }
    b
}

fn against(kind: &str, metres: f64) -> Battle {
    against_at(&sim::fixtures::test_game(), kind, metres, 30)
}

/// Rounds red's mount `id` fired.
fn shots(b: &Battle, id: &str) -> u32 {
    let red = b.unit(UnitId(1)).unwrap();
    let specs = b.rules().catalog.mounts(red.kind);
    (red.mounts.iter().zip(specs))
        .filter(|(_, spec)| spec.def.id == id)
        .map(|(m, _)| m.shots)
        .sum()
}

/// Hit points the helicopter has lost.
fn heli_loss(b: &Battle) -> f64 {
    let heli = b.unit(UnitId(0)).unwrap();
    b.rules().catalog.get(heli.kind).hull().unwrap().hp - heli.hp
}

#[test]
fn a_round_fired_from_a_jet_starts_with_its_velocity() {
    let world = flat([1000.0, 400.0], "");
    let config = config();
    let profile = profile("hmg");
    let aim = Aim {
        origin: v3(100.0, 200.0, 20.0),
        target: v3(500.0, 200.0, 20.0),
        target_velocity: v3(0.0, 0.0, 0.0),
        carrier: v3(80.0, 0.0, 0.0),
    };
    let (launch, solution) = fire_round(
        &world,
        &config,
        &profile,
        &aim,
        0.0,
        &mut contract::random::Rng::new(1),
        None,
        |_| false,
        |_| false,
    )
    .unwrap()
    .expect("round launches");
    let mut projectiles = Projectiles::new(config.clone());
    let id = projectiles.launch(launch);
    assert_eq!(
        projectiles.get(id).unwrap().velocity,
        launch.velocity + aim.carrier
    );
    let endpoint = predicted_path(
        &config,
        &profile,
        launch.origin,
        launch.velocity + aim.carrier,
        solution.time_of_flight_s,
    )
    .last()
    .copied()
    .unwrap();
    assert!((endpoint - solution.intercept).length() < 1e-6);
}

#[test]
fn a_ground_shooters_round_is_unchanged() {
    let world = flat([1000.0, 400.0], "");
    let config = config();
    let profile = profile("hmg");
    let aim = Aim {
        origin: v3(100.0, 200.0, 1.5),
        target: v3(500.0, 200.0, 1.5),
        target_velocity: v3(0.0, 0.0, 0.0),
        carrier: v3(0.0, 0.0, 0.0),
    };
    let (launch, fired) = fire_round(
        &world,
        &config,
        &profile,
        &aim,
        0.0,
        &mut contract::random::Rng::new(1),
        None,
        |_| false,
        |_| false,
    )
    .unwrap()
    .expect("round launches");
    assert_eq!(launch.velocity, fired.velocity);
    assert_eq!(launch.carrier, v3(0.0, 0.0, 0.0));
}

#[test]
fn a_tank_gun_never_engages_a_helicopter_but_its_machine_gun_does() {
    let b = against("test_tank", 150.0);
    assert_eq!(
        shots(&b, "cannon"),
        0,
        "the main gun fired at the helicopter"
    );
    assert!(shots(&b, "HMG") > 0, "the machine gun never fired");
}

#[test]
fn a_machine_gun_holds_fire_at_a_helicopter_past_its_air_reach() {
    // 300 m is well inside the HMG's ground range: a gunner swinging the
    // barrel up at a helicopter that far off would only waste the belt.
    let b = against("test_tank", 300.0);
    assert_eq!(shots(&b, "HMG"), 0, "the machine gun fired at 300 m");
}

#[test]
fn an_anti_tank_team_never_fires_its_missile_at_a_helicopter() {
    let b = against("test_at", 150.0);
    assert_eq!(
        shots(&b, "ATGM launcher"),
        0,
        "the missile went up at the helicopter"
    );
}

#[test]
fn a_rifle_squad_keeps_plinking_a_helicopter_it_cannot_hurt() {
    let b = against("test_rifle", 80.0);
    assert!(shots(&b, "rifles") > 0, "the rifles never fired");
    assert_eq!(heli_loss(&b), 0.0, "rifle fire hurt the helicopter");
}

#[test]
fn a_rifle_squad_holds_fire_at_a_helicopter_past_its_air_reach() {
    let b = against("test_rifle", 250.0);
    assert_eq!(shots(&b, "rifles"), 0, "the rifles fired at 250 m");
}

#[test]
fn an_autocannon_damages_a_helicopter() {
    let b = against("test_gun_jeep", 250.0);
    assert!(heli_loss(&b) > 0.0, "the autocannon never hurt it");
}

#[test]
fn a_gun_scatters_wider_at_a_helicopter_than_along_the_ground() {
    // The same autocannon, once with its air spread and once with its
    // ground spread at the helicopter: swinging up at an aircraft costs it
    // most of its hits. One hit point a hit keeps the airframe up for
    // the whole run, so its loss counts the hits, and a harmless door gun
    // (the autocannon states its own damage) keeps the jeep firing.
    let counting = || {
        let mut f = sim::fixtures::test_game();
        f["weapons"]["autocannon"]["damage"] = json!(1.0);
        f["weapons"]["autocannon"]["targets"]["ground"]["scatter_mrad"] = json!(8.0);
        f["weapons"]["hmg"]["damage"] = json!(0.0);
        f
    };
    let mut ground_aim = counting();
    ground_aim["weapons"]["autocannon"]["targets"]["low_air"]["scatter_mrad"] =
        ground_aim["weapons"]["autocannon"]["targets"]["ground"]["scatter_mrad"].clone();
    let air = against_at(&counting(), "test_gun_jeep", 250.0, 180);
    let ground = against_at(&ground_aim, "test_gun_jeep", 250.0, 180);
    assert!(
        heli_loss(&air) * 2.0 < heli_loss(&ground),
        "air spread {} vs ground spread {}",
        heli_loss(&air),
        heli_loss(&ground)
    );
}

/// A Little Bird's thin skin lets part of a rifle round through, so rifles
/// can wear one down; a Wildcat, sharing its class, shrugs them off.
#[test]
fn rifles_can_hurt_a_little_bird_but_not_a_wildcat() {
    let rules: contract::scenario::Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let rifle = rules.weapons.get("rifle").unwrap();
    let armor = |id: &str| match &rules.catalog.by_id(id).body {
        contract::catalog::Body::Hull(hull) => hull.armor,
        _ => panic!("{id} has no hull"),
    };
    for little_bird in [
        "us_ah_6_mh_6_little_bird_ah_6m",
        "us_ah_6_mh_6_little_bird_mh_6m",
    ] {
        assert!(
            sim::weapons::can_damage(rifle, Some(&armor(little_bird)), &rules),
            "{little_bird}"
        );
    }
    assert!(!sim::weapons::can_damage(
        rifle,
        Some(&armor("europe_aw159_wildcat_ah1")),
        &rules
    ));
}

#[test]
fn an_air_defence_team_brings_down_a_helicopter() {
    let b = against("test_aa", 250.0);
    assert!(shots(&b, "AA launcher") > 0, "the missile never went up");
    assert!(
        !b.unit(UnitId(0)).unwrap().alive(),
        "the helicopter survived"
    );
}

#[test]
fn an_air_defence_team_never_fires_its_missile_at_ground_targets() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [200, 180], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_rifle", "position": [200, 220], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_aa", "position": [450, 200], "yaw": std::f64::consts::PI },
        ]),
    );
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
    }
    let red = b.unit(UnitId(2)).unwrap();
    let specs = b.rules().catalog.mounts(red.kind);
    let missiles: u32 = (red.mounts.iter().zip(specs))
        .filter(|(_, spec)| spec.def.id == "AA launcher")
        .map(|(m, _)| m.shots)
        .sum();
    let rifles: u32 = (red.mounts.iter().zip(specs))
        .filter(|(_, spec)| spec.def.id == "rifles")
        .map(|(m, _)| m.shots)
        .sum();
    assert_eq!(missiles, 0, "the missile went at a ground target");
    assert!(rifles > 0, "the riflemen never fired");
}

/// The missile chases a helicopter flying across the team's front: the
/// gunner tracks it and the missile turns onto it, rather than flying at
/// where it was.
#[test]
fn an_air_defence_missile_runs_down_a_helicopter_crossing_its_front() {
    let fixture = fixture();
    let map = json!({ "size": [1600, 1200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let mut b = battle_with(
        &fixture,
        map,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_aa", "position": [800, 700], "yaw": -std::f64::consts::FRAC_PI_2 },
        ]),
    );
    assert_eq!(b.accept(order(&[0], [1500.0, 300.0])).error, None);
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
    }
    assert!(shots(&b, "AA launcher") > 0, "the missile never went up");
    assert!(
        !b.unit(UnitId(0)).unwrap().alive(),
        "the crossing helicopter survived"
    );
}
