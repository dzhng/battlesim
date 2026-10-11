//! Which weapons engage a helicopter (D1, D2), and from how far: rifles,
//! machine guns and autocannons swing up at it, each at a much shorter reach
//! and a much wider spread than along the ground; tank guns, grenades and
//! ground missiles never do. A rifle that cannot hurt it still fires at it,
//! for the sparks.

use super::air::{battle_with, open_map};
use contract::ids::UnitId;
use serde_json::{json, Value};
use sim::battle::Battle;

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
        f["weapons"]["hmg"]["damage"] = json!(0.0);
        f
    };
    let mut ground_aim = counting();
    ground_aim["weapons"]["autocannon"]["targets"]["low_air"]["scatter_mrad"] =
        ground_aim["weapons"]["hmg"]["scatter_mrad"].clone();
    let air = against_at(&counting(), "test_gun_jeep", 250.0, 180);
    let ground = against_at(&ground_aim, "test_gun_jeep", 250.0, 180);
    assert!(
        heli_loss(&air) * 2.0 < heli_loss(&ground),
        "air spread {} vs ground spread {}",
        heli_loss(&air),
        heli_loss(&ground)
    );
}
