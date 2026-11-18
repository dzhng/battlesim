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
    for _ in 0..2 * b.rules().tick_hz {
        b.step();
    }
    let flare = b
        .observe(Side::Red)
        .flares
        .first()
        .cloned()
        .expect("the burning flare is still airborne");
    let dt = 1.0 / f64::from(b.rules().tick_hz);
    let speed = (v3(flare.to[0], flare.to[1], flare.to[2])
        - v3(flare.from[0], flare.from[1], flare.from[2]))
        * (1.0 / dt);
    assert!(
        speed.xy().length() < 5.0,
        "drag arrests the flare's ejection: {speed:?}"
    );
    assert!(
        speed.z < 0.0 && speed.z > -5.0,
        "a missile follows a slowly settling flare, not a fast falling shell: {speed:?}"
    );
    for _ in 0..4 * b.rules().tick_hz {
        b.step();
    }
    assert!(
        b.observe(Side::Red).flares.is_empty(),
        "a flare burns out in the air rather than glowing until it reaches the ground"
    );
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

#[test]
fn a_fast_flare_cannot_fuse_a_missile_at_a_point_it_never_reaches() {
    use sim::flight::{self, ImpactContext, ImpactDecision, ImpactResolver, Launch, Projectiles};
    struct Stops;
    impl ImpactResolver for Stops {
        fn resolve(&mut self, _: &ImpactContext) -> ImpactDecision {
            ImpactDecision::Stop
        }
    }
    let world = crate::common::flat([1200.0, 400.0], "");
    let mut store = Projectiles::new(crate::common::config());
    let launch = Launch {
        origin: v3(510.0, 200.0, 100.0),
        carrier: v3(0.0, 0.0, 0.0),
        velocity: v3(0.0, 0.0, 0.0),
        gravity_scale: 0.0,
        lifetime_s: 1.0,
        suppression_radius_m: 0.0,
        shooter: None,
        motor: None,
        fall: None,
        guidance: Some(flight::Guidance {
            point: v3(500.0, 200.0, 100.0),
            turn_rad_s: 1.0,
            supported: false,
            loft: None,
        }),
    };
    let id = store.launch(launch);
    let tracking = store.launch(Launch {
        origin: v3(400.0, 200.0, 100.0),
        velocity: v3(100.0, 0.0, 0.0),
        ..launch
    });
    let decoy = flight::Decoy {
        position: v3(500.0, 200.0, 100.0),
        velocity: v3(300.0, 0.0, 0.0),
        meet_m: 0.01,
        drag_s: 0.85,
        fall_mps: 3.0,
        burn_s: 4.0,
    };
    for projectile in [id, tracking] {
        store.divert(projectile, decoy, contract::catalog::AltitudeLayer::HighAir);
    }
    let mut events = Vec::new();
    flight::advance_projectiles(&mut store, &world, &[], &mut events, &mut Stops);
    assert!(
        store.get(id).is_some(),
        "drag keeps the flare short of the stationary missile's tight fuse"
    );
    let (_, physical, _) = store.flares().next().unwrap();
    assert_eq!(
        store.get(tracking).unwrap().guidance.unwrap().point,
        physical,
        "guidance predicts the same endpoint the decoy physically reaches"
    );
}
