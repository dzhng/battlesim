//! Flares an aircraft dispenses to decoy a missile flying at it.

use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::math::v3;

use crate::guidance::{missile, quick, until_launch};

#[test]
fn physical_flares_publish_their_falling_path_only_to_sides_that_see_them() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_heli", "position": [500, 300], "engagement": "return_fire_only" }
        ]),
        7,
        |r| {
            r["weapons"]["hmg"]["damage"] = json!(0);
            r["weapons"]["atgm"]["decoyable"] = json!(true);
            r["weapons"]["atgm"]["ammo"] = json!(1);
            r["weapons"]["atgm"]["targets"] = json!({"low_air": {
                "classes": ["air_rotor"], "range_m": 650, "scatter_mrad": 0.5
            }});
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_heli",
                json!({
                    "capabilities": {"flares": {"capacity": 2, "reload_s": 30}}
                }),
            );
        },
    );
    until_launch(&mut b);
    b.step();
    let aircraft = b.unit(UnitId(1)).unwrap().position;
    for side in [Side::Blue, Side::Red, Side::Neutral] {
        let frame = b.observe(side);
        if side == Side::Neutral {
            assert!(frame.flares.is_empty(), "no observer, no flare disclosure");
        } else {
            let flare = frame.flares.first().expect("the side sees the flare");
            assert_eq!(frame.flares.len(), 1);
            assert!(flare.to[2] < flare.from[2], "the flare falls");
            assert!(
                (v3(flare.to[0], flare.to[1], flare.to[2]) - aircraft).length() < 20.0,
                "the visible flare stays beside the aircraft"
            );
        }
        let words = sim::publication::Publisher::new()
            .publish(&b, side)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect::<Vec<_>>();
        let decoded = sim::publication::reader::Reader::new(&sim::publication::layout_json(&b))
            .unwrap()
            .decode(&words)
            .unwrap()
            .unwrap();
        let widened = frame
            .flares
            .iter()
            .map(|f| contract::observation::Flare {
                from: f.from.map(|v| f64::from(v as f32)),
                to: f.to.map(|v| f64::from(v as f32)),
            })
            .collect::<Vec<_>>();
        assert_eq!(decoded.flares, widened, "the client reads the same path");
    }
}

#[test]
fn a_flare_is_ejected_near_the_aircraft_before_the_seeker_arrives() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_heli", "position": [500, 300], "engagement": "return_fire_only" }
        ]),
        7,
        |r| {
            r["weapons"]["atgm"]["seeker"] = json!("infrared");
            r["weapons"]["atgm"]["seeker_fov_deg"] = json!(180);
            r["weapons"]["atgm"]["decoyable"] = json!(true);
            r["weapons"]["atgm"]["targets"] = json!({"low_air": {"classes": ["air_rotor", "air_fixed_wing"], "range_m": 650, "scatter_mrad": 0.5}});
            r["weapons"]["atgm"]["ammo"] = json!(1);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_heli",
                json!({
                    "capabilities": {"flares": {"capacity": 2, "reload_s": 30},
                        "air": {"fuel_capacity_s":180,
                        "refund":{"health":0.4,"ammunition":0.4,"fuel":0.2}}}
                }),
            );
        },
    );
    until_launch(&mut b);
    let hp = b.unit(UnitId(1)).unwrap().hp;
    b.step();
    let target = b.unit(UnitId(1)).unwrap();
    let flare = missile(&b).unwrap();
    assert_eq!(target.flares.unwrap().charges, 1, "flare reacts on lock");
    assert_eq!(target.hp, hp, "missile is still far from its target");
    assert!(
        (v3(flare.point[0], flare.point[1], flare.point[2]) - target.position).length() < 20.0,
        "flare begins beside the airframe, not at a teleported aim point: {:?}",
        flare.point
    );
    let first = flare.point;
    for _ in 0..10 {
        b.step();
    }
    let later = missile(&b).unwrap();
    assert_ne!(
        later.point, first,
        "ejected flare continues its physical flight"
    );
    assert_eq!(
        b.unit(UnitId(1)).unwrap().flares.unwrap().charges,
        1,
        "one locked missile consumes one flare"
    );
    let readiness = b.observe(Side::Red).own[0]
        .flares
        .expect("the aircraft publishes flare readiness");
    assert!(
        readiness.cooldown.is_some(),
        "spending a flare starts its reload"
    );
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
    }
    let reloaded = b.observe(Side::Red).own[0]
        .flares
        .expect("the aircraft still publishes flare readiness");
    assert_eq!(reloaded.charges, 1, "the flare spend is retained");
    assert!(reloaded.cooldown.is_none(), "reload completes");
}

/// A seeker diverted to a flare bursts on it once within the authored
/// meeting distance (`guided.flare_meet_m`): authored wider than the
/// field, it bursts as soon as it is diverted, far short of the airframe.
#[test]
fn a_diverted_seeker_bursts_within_the_authored_meeting_distance() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_heli", "position": [500, 300], "engagement": "return_fire_only" }
        ]),
        7,
        |r| {
            r["guided"]["flare_meet_m"] = json!(2000);
            r["weapons"]["atgm"]["seeker"] = json!("infrared");
            r["weapons"]["atgm"]["seeker_fov_deg"] = json!(180);
            r["weapons"]["atgm"]["decoyable"] = json!(true);
            r["weapons"]["atgm"]["targets"] =
                json!({"low_air": {"classes": ["air_rotor"], "range_m": 650, "scatter_mrad": 0.5}});
            r["weapons"]["atgm"]["ammo"] = json!(1);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_heli",
                json!({ "capabilities": {"flares": {"capacity": 2, "reload_s": 30}} }),
            );
        },
    );
    until_launch(&mut b);
    let hp = b.unit(UnitId(1)).unwrap().hp;
    for _ in 0..2 {
        b.step();
    }
    assert_eq!(
        b.unit(UnitId(1)).unwrap().flares.unwrap().charges,
        1,
        "the flare diverts the seeker"
    );
    assert!(missile(&b).is_none(), "the seeker bursts on its flare");
    let flare = b
        .observe(Side::Red)
        .flares
        .first()
        .cloned()
        .expect("the physical flare outlives the missile");
    b.step();
    let later = b
        .observe(Side::Red)
        .flares
        .first()
        .cloned()
        .expect("the flare keeps falling independently");
    assert!(later.to[2] < flare.to[2], "the flare continues falling");
    assert_eq!(b.unit(UnitId(1)).unwrap().hp, hp, "the airframe unhurt");
}

/// Flares are a unit's own capability, not an aircraft's: a jeep carrying
/// them decoys an infrared missile fired at it, and one without is hit.
#[test]
fn flares_decoy_a_seeker_fired_at_any_unit_that_carries_them() {
    let run = |flares: bool| {
        let mut b = quick(
            json!([]),
            json!([
                { "side": "blue", "kind": "test_at", "position": [40, 300] },
                { "side": "red", "kind": "test_jeep", "position": [500, 300], "engagement": "return_fire_only" }
            ]),
            7,
            |r| {
                r["weapons"]["atgm"]["seeker"] = json!("infrared");
                r["weapons"]["atgm"]["seeker_fov_deg"] = json!(180);
                r["weapons"]["atgm"]["decoyable"] = json!(true);
                r["weapons"]["atgm"]["ammo"] = json!(1);
                if flares {
                    sim::fixtures::patch_catalog(
                        r,
                        "units",
                        "test_jeep",
                        json!({ "capabilities": { "flares": { "capacity": 2, "reload_s": 30 } } }),
                    );
                }
            },
        );
        let full = b.unit(UnitId(1)).unwrap().hp;
        until_launch(&mut b);
        for _ in 0..10 * b.rules().tick_hz {
            b.step();
        }
        let charges = b
            .observe(Side::Red)
            .own
            .first()
            .and_then(|u| u.flares)
            .map(|f| f.charges);
        (full - b.unit(UnitId(1)).unwrap().hp, charges)
    };
    assert_eq!(
        run(true),
        (0.0, Some(1)),
        "one flare spent, the jeep unhurt"
    );
    let (lost, charges) = run(false);
    assert_eq!(charges, None);
    assert!(lost > 0.0, "without flares the missile hits");
}

/// A spoofed missile flies toward the flare ejected beside the aircraft,
/// independently of its launcher, rather than diving beside the shooter.
/// Without flares the helicopter is hit.
#[test]
fn test_flares_spoof_an_operator_guided_missile_at_a_helicopter() {
    let run = |flares: bool| {
        let mut b = quick(
            json!([]),
            json!([
                { "side": "blue", "kind": "test_at", "position": [40, 300] },
                { "side": "red", "kind": "test_heli", "position": [500, 300], "engagement": "return_fire_only" }
            ]),
            7,
            |r| {
                // Its door gun answers harmlessly: the operator lives.
                r["weapons"]["hmg"]["damage"] = json!(0.0);
                r["weapons"]["atgm"]["decoyable"] = json!(true);
                r["weapons"]["atgm"]["targets"] = json!({ "low_air": { "classes": ["air_rotor"], "range_m": 650, "scatter_mrad": 0.5 } });
                r["weapons"]["atgm"]["ammo"] = json!(1);
                if flares {
                    sim::fixtures::patch_catalog(
                        r,
                        "units",
                        "test_heli",
                        json!({ "capabilities": { "flares": { "capacity": 2, "reload_s": 30 } } }),
                    );
                }
            },
        );
        let full = b.unit(UnitId(1)).unwrap().hp;
        until_launch(&mut b);
        assert!(
            missile(&b).is_some_and(|m| m.supported),
            "the operator guides it"
        );
        b.step();
        if flares {
            let diverted = missile(&b).expect("the diverted missile keeps flying");
            let target = b.unit(UnitId(1)).unwrap();
            assert!(!diverted.supported, "the launcher has let go");
            assert!(
                (v3(diverted.point[0], diverted.point[1], diverted.point[2]) - target.position)
                    .length()
                    < 20.0,
                "the missile chases a flare beside the aircraft: {:?}",
                diverted.point
            );
        }
        for _ in 0..10 * b.rules().tick_hz {
            b.step();
        }
        let charges = b
            .observe(Side::Red)
            .own
            .first()
            .and_then(|u| u.flares)
            .map(|f| f.charges);
        (full - b.unit(UnitId(1)).unwrap().hp, charges)
    };
    assert_eq!(
        run(true),
        (0.0, Some(1)),
        "one flare spent, the helicopter unhurt"
    );
    let (lost, charges) = run(false);
    assert_eq!(charges, None);
    assert!(lost > 0.0, "without flares the missile hits");
}
