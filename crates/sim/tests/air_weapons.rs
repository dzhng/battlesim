//! Which weapons engage a helicopter (D1, D2): rifles, machine guns and
//! autocannons swing up at it; tank guns, grenades and ground missiles never
//! do. A rifle that cannot hurt it still fires at it, for the sparks.

use super::air::{battle_with, fixture, open_map, order};
use contract::ids::UnitId;
use serde_json::json;
use sim::battle::Battle;

/// A hovering helicopter at [200, 200] and one red `kind` at [450, 200],
/// facing it.
fn against(kind: &str) -> Battle {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [200, 200], "engagement": "return_fire_only" },
            { "side": "red", "kind": kind, "position": [450, 200], "yaw": std::f64::consts::PI },
        ]),
    );
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
    }
    b
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

#[test]
fn a_tank_gun_never_engages_a_helicopter_but_its_machine_gun_does() {
    let b = against("test_tank");
    assert_eq!(
        shots(&b, "cannon"),
        0,
        "the main gun fired at the helicopter"
    );
    assert!(shots(&b, "HMG") > 0, "the machine gun never fired");
}

#[test]
fn an_anti_tank_team_never_fires_its_missile_at_a_helicopter() {
    let b = against("test_at");
    assert_eq!(
        shots(&b, "ATGM launcher"),
        0,
        "the missile went up at the helicopter"
    );
}

#[test]
fn a_rifle_squad_keeps_plinking_a_helicopter_it_cannot_hurt() {
    let b = against("test_rifle");
    assert!(shots(&b, "rifles") > 0, "the rifles never fired");
    let heli = b.unit(UnitId(0)).unwrap();
    let full = b.rules().catalog.get(heli.kind).hull().unwrap().hp;
    assert_eq!(heli.hp, full, "rifle fire hurt the helicopter");
}

#[test]
fn an_autocannon_damages_a_helicopter() {
    let b = against("test_gun_jeep");
    let heli = b.unit(UnitId(0)).unwrap();
    let full = b.rules().catalog.get(heli.kind).hull().unwrap().hp;
    assert!(heli.hp < full, "the autocannon never hurt it");
}

#[test]
fn an_air_defence_team_brings_down_a_helicopter() {
    let b = against("test_aa");
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
