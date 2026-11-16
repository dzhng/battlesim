//! Sight through a garrison's windows: an eye at every window of the
//! building, manned or not, each outside every shell of the building, and an
//! enemy that sees it there learns which building it holds.
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;

use crate::buildings::compound_setup;

#[test]
fn stepped_window_eyes_stay_outside_every_shell_and_fire_past_the_whole_owner() {
    let mut setup = compound_setup(json!([]));
    setup.units=serde_json::from_value(json!([{"side":"blue","kind":"test_rifle","position":[425,301],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::OccupyBuilding {
            units: vec![contract::ids::UnitId(0)],
            building: 1,
            gesture: 0,
        },
    );
    for _ in 0..600 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    let unit = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(unit.garrisoned());
    let observed = b.observe(Side::Blue).own[0].hold.as_ref().unwrap();
    assert_eq!(
        (match observed.host {
            contract::observation::HostRef::Building { center, half, .. } => (center, half),
            _ => unreachable!(),
        }),
        ([402.0, 300.0], [6.0, 3.0]),
        "the delivered footprint must include every current physical part"
    );
    let eyes = sim::sensing::eyes(unit, b.windows(), &setup.rules);
    for eye in &eyes {
        assert!(
            b.world()
                .props()
                .filter(|p| b.world().building_of(p.id) == Some(0))
                .all(|p| !p.footprint().contains(eye.xy(), 0.0)),
            "eye {eye:?} must stand outside every physical part"
        );
    }
    let east: Vec<_> = eyes.iter().filter(|e| e.x > 404.0 && e.y < 303.0).collect();
    assert!(!east.is_empty(), "the east facade has windows: {eyes:?}");
    for eye in east {
        assert!(b
            .world()
            .sight_clear(*eye, sim::math::v3(430.0, eye.y, eye.z)));
    }
    assert!(b
        .world()
        .raycast_past(
            sim::math::v3(390.0, 301.0, 2.0),
            sim::math::v3(1.0, 0.0, 0.0),
            30.0,
            Some(0)
        )
        .is_none());
    assert!(b.world().segment_clear_except(
        sim::math::v3(390.0, 301.0, 2.0),
        sim::math::v3(420.0, 301.0, 2.0),
        Some(1)
    ));
}

#[test]
fn a_garrison_sees_from_every_window_of_its_building_manned_or_not() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].geometry.floor_z = vec![0.0, 3.0, 6.0];
    setup.units = serde_json::from_value(json!([{"side":"blue","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only"}])).unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::OccupyBuilding {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
            gesture: 0,
        },
    );
    for _ in 0..1000 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    let u = b.unit(contract::ids::UnitId(0)).unwrap();
    assert!(u.garrisoned());
    let plan = b.windows().plan(0);
    let soldiers = u.members.iter().filter(|m| m.alive()).count();
    assert!(
        plan.len() > soldiers,
        "the building has windows nobody mans: {} for {soldiers}",
        plan.len()
    );
    // Every window looks out: the eye at it, or at the one above it, which
    // sees all it would.
    let lift = setup.rules.physics.infantry_eye_m;
    let eyes = sim::sensing::eyes(u, b.windows(), &setup.rules);
    for window in plan {
        assert!(
            eyes.iter()
                .any(|e| e.xy() == window.position.xy() && e.z >= window.position.z + lift),
            "window {window:?} has no eye: {eyes:?}"
        );
    }
    for eye in &eyes {
        assert!(
            plan.iter()
                .any(|w| w.position.xy() == eye.xy() && w.position.z + lift == eye.z),
            "eye {eye:?} is at no window"
        );
    }
}

#[test]
fn a_third_floor_garrison_sees_over_a_two_storey_obstacle() {
    let mut setup = compound_setup(json!([]));
    setup.map.buildings[0].geometry.floor_z = vec![0.0, 3.0, 6.0];
    setup.map.props.push(
        serde_json::from_value(
            json!({"id":3,"kind":"wall","center":[440,301],"yaw":0,"half_extents":[3,20,2.5]}),
        )
        .unwrap(),
    );
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only"},
        {"side":"red","kind":"test_tank","position":[580,301],"engagement":"return_fire_only"}
    ]))
    .unwrap();
    let mut b = Battle::new(&setup, 11);
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::OccupyBuilding {
            units: vec![contract::ids::UnitId(0)],
            building: 0,
            gesture: 0,
        },
    );
    for _ in 0..1000 {
        b.step();
        if b.unit(contract::ids::UnitId(0)).unwrap().garrisoned() {
            break;
        }
    }
    for _ in 0..30 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrisoned());
    assert_eq!(
        b.observe(Side::Blue).identified.len(),
        1,
        "highest occupied facade sees the tank over the intervening five-metre shop"
    );
    crate::common::order(
        &mut b,
        Side::Blue,
        2,
        contract::command::Order::Attack {
            units: vec![contract::ids::UnitId(0)],
            target: contract::command::TargetRef::Ground {
                point: [580.0, 301.0, 0.0],
            },
        },
    );
    let mut fired = false;
    for _ in 0..400 {
        b.step();
        if b.rounds().any(|(flight, round)| {
            round.unit == contract::ids::UnitId(0)
                && b.arsenal().weapons[round.weapon].id == "rifle"
                && flight.position.z > 6.0
        }) {
            assert_eq!(
                b.world().prop(3).unwrap().top_z(),
                5.0,
                "the shop still stands when the upper-floor rifle fires"
            );
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "a gun held on floor three fires over the shop from its real muzzle"
    );
}

#[test]
fn a_garrison_seen_at_its_windows_gives_away_its_building() {
    let mut setup = compound_setup(json!([]));
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only"},
        {"side":"red","kind":"test_rifle","position":[330,300],"engagement":"return_fire_only"}
    ]))
    .unwrap();
    let mut b = Battle::new(&setup, 11);
    // Nominated by its wing: any part names the building.
    crate::common::order(
        &mut b,
        Side::Blue,
        1,
        contract::command::Order::OccupyBuilding {
            units: vec![contract::ids::UnitId(0)],
            building: 1,
            gesture: 0,
        },
    );
    let held = |b: &Battle| b.observe(Side::Red).identified.first().map(|e| e.building);
    let mut entering_seen = false;
    for _ in 0..1000 {
        b.step();
        let blue = b.unit(contract::ids::UnitId(0)).unwrap();
        if blue.garrisoned() {
            break;
        }
        if blue.hold.is_some() {
            if let Some(building) = held(&b) {
                entering_seen = true;
                assert_eq!(
                    building, None,
                    "a squad still outside, entering, holds nothing yet"
                );
            }
        }
    }
    assert!(
        entering_seen,
        "red watched the squad stand outside to enter"
    );
    for _ in 0..30 {
        b.step();
    }
    assert!(b.unit(contract::ids::UnitId(0)).unwrap().garrisoned());
    assert!(
        matches!(held(&b), Some(Some(0 | 1))),
        "seen at its windows, the garrison names one of its building's parts: {:?}",
        held(&b)
    );
}
