//! A door gun swings through its door, not through the cabin: a turret's arc
//! bounds where it can bear, and when none of a helicopter's guns that could
//! hurt a target bears on it, the airframe turns to bring one round.

use super::air::{battle_with, fixture, open_map};
use contract::ids::UnitId;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::wrap_angle;
use std::f64::consts::FRAC_PI_2;

/// The test helicopter with its door gun in the left door (an arc of 160°
/// on its left), and with `right`, a second one in the right door.
fn doors(right: bool) -> Value {
    let mut fixture = fixture();
    let mut mounts = vec![json!({ "id": "HMG", "arc": { "centre_deg": 90, "width_deg": 160 } })];
    if right {
        mounts.push(json!({
            "id": "HMG_R", "name": "Door HMG", "weapons": ["hmg"], "turret": true,
            "pivot_m": [1.6, -1.12, 1.25], "muzzle_m": [1.2, 0, 0],
            "arc": { "centre_deg": -90, "width_deg": 160 }
        }));
    }
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "mounts": mounts }),
    );
    fixture
}

/// The helicopter hovering at [200, 200] facing east, and a red jeep on its
/// right (south), in its guns' reach.
fn hovering(fixture: &Value) -> Battle {
    battle_with(
        fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [200, 200], "yaw": 0 },
            { "side": "red", "kind": "test_jeep", "position": [200, 60], "yaw": FRAC_PI_2 },
        ]),
    )
}

/// Each blue mount's rounds, by mount id.
fn shots(b: &Battle, id: &str) -> u32 {
    let heli = b.unit(UnitId(0)).unwrap();
    let specs = b.rules().catalog.mounts(heli.kind);
    (heli.mounts.iter().zip(specs))
        .filter(|(_, spec)| spec.def.id == id)
        .map(|(m, _)| m.shots)
        .sum()
}

#[test]
fn a_left_door_gun_never_swings_through_the_cabin_and_the_airframe_turns_it_onto_a_target() {
    let fixture = doors(false);
    let mut b = hovering(&fixture);
    let mut first_shot_yaw = None;
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        // Its bearing off the airframe's heading stays inside its door.
        let off = wrap_angle(heli.mounts[0].bearing - heli.yaw);
        assert!(
            (off - FRAC_PI_2).abs() <= 80f64.to_radians() + 1e-9,
            "the gun swung {:.0}° off the nose",
            off.to_degrees()
        );
        if first_shot_yaw.is_none() && shots(&b, "HMG") > 0 {
            first_shot_yaw = Some(heli.yaw);
        }
    }
    // It fired only once the airframe had turned the jeep (due south) into
    // its door: turned right, from east toward west.
    let yaw = first_shot_yaw.expect("the door gun never fired");
    assert!(
        wrap_angle(yaw).abs() > 10f64.to_radians(),
        "fired facing {:.0}°",
        yaw.to_degrees()
    );
}

#[test]
fn with_a_gun_in_each_door_the_one_facing_fires_and_the_airframe_holds_its_heading() {
    let fixture = doors(true);
    let mut b = hovering(&fixture);
    for _ in 0..15 * b.rules().tick_hz {
        b.step();
    }
    assert!(shots(&b, "HMG_R") > 0, "the right door gun never fired");
    assert_eq!(
        shots(&b, "HMG"),
        0,
        "the left door gun fired through the cabin"
    );
    let yaw = b.unit(UnitId(0)).unwrap().yaw;
    assert!(
        wrap_angle(yaw).abs() < 1e-6,
        "it turned to {:.1}°",
        yaw.to_degrees()
    );
}
