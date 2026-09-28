//! The one sight shape (slice 04): a unit sees farthest along its forward
//! direction, and spotting, the fog sweep and the publication read the same
//! shape. Vehicles look along the turret (the truck along its hull);
//! infantry see an even 360°.
use std::f64::consts::{FRAC_PI_2, PI};

use contract::ids::{Side, UnitId};
use contract::observation::OwnUnit;
use contract::scenario::Rules;
use serde_json::json;
use sim::battle::Battle;
use sim::sight::sight_range;

use crate::common;

/// Open, flat ground big enough for a recon's full range in every direction.
const FLAT: &str = r#"{ "size": [2600, 2600], "height_grid_m": 4, "slope_cutoff_deg": 35 }"#;

fn rules() -> Rules {
    serde_json::from_value(common::village()).unwrap()
}

fn battle(map: &str, units: serde_json::Value) -> Battle {
    Battle::new(&common::scenario(map, units, json!([])), 1)
}

fn own(b: &Battle, side: Side, id: u32) -> OwnUnit {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
        .unwrap()
}

fn base_range(kind: &str) -> f64 {
    common::rules().catalog.by_id(kind).sensors.ground_m
}

fn shape(kind: &str) -> [f64; 3] {
    let s = common::rules().catalog.by_id(kind).sensors.sight_shape;
    [s.front, s.side, s.rear]
}

#[test]
fn range_follows_the_vehicle_shape_at_front_side_rear_and_across_the_wrap() {
    let rules = rules();
    for (kind, yaw) in [("tank", 0.7), ("supply", -2.9)] {
        let b = battle(
            FLAT,
            json!([{ "side": "blue", "kind": kind, "position": [1300, 1300], "yaw": yaw }]),
        );
        let unit = b.unit(UnitId(0)).unwrap();
        let [front, side, rear] = shape(kind);
        let range = base_range(kind);
        let at = |rel: f64| sight_range(unit, &rules, yaw + rel);
        assert!((at(0.0) - range * front).abs() < 1e-9, "{kind} ahead");
        assert!((at(FRAC_PI_2) - range * side).abs() < 1e-9, "{kind} left");
        assert!((at(-FRAC_PI_2) - range * side).abs() < 1e-9, "{kind} right");
        assert!((at(PI) - range * rear).abs() < 1e-9, "{kind} astern");
        // The same bearing reached either way round, and no seam astern.
        assert!((at(PI - 1e-6) - at(-PI + 1e-6)).abs() < 1e-3, "{kind} wrap");
        assert!((at(0.3) - at(0.3 + 2.0 * PI)).abs() < 1e-9, "{kind} turn");
        // Continuous through the side boundary, and never rising away from
        // the front.
        assert!((at(FRAC_PI_2 - 1e-6) - at(FRAC_PI_2 + 1e-6)).abs() < 1e-3);
        let mut last = f64::INFINITY;
        for k in 0..=180 {
            let r = at((k as f64).to_radians());
            assert!(r <= last + 1e-9, "{kind} rises at {k}°");
            assert!(
                (r - at(-(k as f64).to_radians())).abs() < 1e-9,
                "{kind} mirror {k}°"
            );
            last = r;
        }
    }
}

#[test]
fn infantry_sight_is_isotropic_whatever_the_yaw() {
    let rules = rules();
    for kind in ["rifle", "recon", "at"] {
        let b = battle(
            FLAT,
            json!([{ "side": "blue", "kind": kind, "position": [1300, 1300], "yaw": 1.1 }]),
        );
        let unit = b.unit(UnitId(0)).unwrap();
        for k in 0..36 {
            let r = sight_range(unit, &rules, (k as f64 * 10.0).to_radians());
            assert!((r - base_range(kind)).abs() < 1e-9, "{kind} at {}°", k * 10);
        }
        let sight = own(&b, Side::Blue, 0).sight;
        assert_eq!(
            [sight.shape.front, sight.shape.side, sight.shape.rear],
            [1.0; 3]
        );
    }
    // A squad identifies a tank just inside its range behind it as well as ahead.
    let r = base_range("rifle");
    for dx in [r - 5.0, -(r - 5.0)] {
        let b = battle(
            FLAT,
            json!([
                { "side": "blue", "kind": "rifle", "position": [1300, 1300] },
                { "side": "red", "kind": "tank", "position": [1300.0 + dx, 1300] },
            ]),
        );
        assert_eq!(b.observe(Side::Blue).identified.len(), 1, "dx {dx}");
    }
}

#[test]
fn a_vehicle_looks_along_its_turret_and_a_truck_along_its_hull() {
    // The tank faces east. A squad 120 m north is inside its side reach, so
    // it locks on and the turret swings north; a second tank 300 m north is
    // outside the side reach and inside the front reach.
    let side_reach = base_range("tank") * shape("tank")[1];
    let front_reach = base_range("tank") * shape("tank")[0];
    assert!(side_reach < 300.0 && 300.0 < front_reach - 20.0);
    let mut b = battle(
        FLAT,
        json!([
            { "side": "blue", "kind": "tank", "position": [1300, 1300], "yaw": 0 },
            { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [1300, 1600], "engagement": "return_fire_only" },
        ]),
    );
    let far_seen = |b: &Battle| own(b, Side::Blue, 0).sees.len() == 2;
    assert!(
        !far_seen(&b),
        "the far tank is abeam of the hull-forward turret"
    );
    assert!(own(&b, Side::Blue, 0).sight.forward.abs() < 1e-9);
    let mut turned = None;
    for t in 0..600 {
        b.step();
        if far_seen(&b) {
            turned = Some(t);
            break;
        }
    }
    assert!(turned.is_some(), "the turret brings the far tank into view");
    let tank = own(&b, Side::Blue, 0);
    // Swung from east toward the squad in the north, not past it.
    assert!(
        0.5 < tank.sight.forward && tank.sight.forward < FRAC_PI_2 + 0.05,
        "{}",
        tank.sight.forward
    );
    assert!(tank.yaw.abs() < 1e-9, "the hull never turned");

    let truck = battle(
        FLAT,
        json!([{ "side": "blue", "kind": "supply", "position": [1300, 1300], "yaw": 2.0 }]),
    );
    assert!((own(&truck, Side::Blue, 0).sight.forward - 2.0).abs() < 1e-9);
}

/// Every fog cell centre around `eye` at `bearing`, `margin` metres inside and
/// outside the directional range, as (inside, outside) visibility.
fn fog_at(b: &Battle, centre: [f64; 2], bearing: f64, reach: f64, margin: f64) -> (bool, bool) {
    let fog = &b.observe(Side::Blue).ground_visibility;
    let at = |d: f64| fog.visible(centre[0] + bearing.cos() * d, centre[1] + bearing.sin() * d);
    (at(reach - margin), at(reach + margin))
}

#[test]
fn sight_shape_consumers_agree() {
    // One tank on open, flat ground; red tanks placed around it just inside
    // and just outside its directional range. Spotting and the fog sweep
    // follow the same shape, and the publication carries it.
    let rules = rules();
    let yaw = 0.4;
    let centre = [1300.0, 1300.0];
    let cell = common::village()["sensors"]["fog_cell_m"].as_f64().unwrap();
    let observer = battle(
        FLAT,
        json!([{ "side": "blue", "kind": "tank", "position": centre, "yaw": yaw }]),
    );
    let unit = observer.unit(UnitId(0)).unwrap();
    let published = own(&observer, Side::Blue, 0).sight;
    assert_eq!(published.forward, yaw);
    assert_eq!(published.range, base_range("tank"));
    let [front, side, rear] = shape("tank");
    assert_eq!(
        [
            published.shape.front,
            published.shape.side,
            published.shape.rear
        ],
        [front, side, rear]
    );
    for k in 0..16 {
        let bearing = yaw + k as f64 * PI / 8.0;
        let reach = sight_range(unit, &rules, bearing);
        for (d, seen) in [(reach - 3.0, true), (reach + 3.0, false)] {
            let at = [centre[0] + bearing.cos() * d, centre[1] + bearing.sin() * d];
            let b = battle(
                FLAT,
                json!([
                    { "side": "blue", "kind": "tank", "position": centre, "yaw": yaw },
                    { "side": "red", "kind": "tank", "position": at },
                ]),
            );
            assert_eq!(
                b.observe(Side::Blue).identified.len() == 1,
                seen,
                "spotting at {}°, {d:.0} m of {reach:.0}",
                k * 360 / 16
            );
        }
        let (inside, outside) = fog_at(&observer, centre, bearing, reach, 2.0 * cell);
        assert!(inside, "fog clear inside {reach:.0} m at {}°", k * 360 / 16);
        assert!(!outside, "fogged past {reach:.0} m at {}°", k * 360 / 16);
    }
}

#[test]
fn a_garrison_sees_from_one_eye_per_facade_it_holds() {
    // garrison-lab's one building: 24 m square round (360, 250), yaw 0, on
    // flat ground.
    let map = include_str!("../../../fixtures/garrison-lab.json");
    let mut b = battle(
        map,
        json!([
            { "side": "blue", "kind": "rifle", "position": [300, 250], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "tank", "position": [200, 250] },
        ]),
    );
    let eye = |key: &str| common::village()["physics"][key].as_f64().unwrap();
    let tank = own(&b, Side::Blue, 1);
    assert_eq!(
        tank.sight.eyes,
        vec![[
            tank.position[0],
            tank.position[1],
            tank.position[2] + common::hull("tank").eye_m
        ]]
    );
    let ack = b.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: serde_json::from_value(json!({ "kind": "garrison", "units": [0], "building": 0 }))
            .unwrap(),
        queued: false,
    });
    assert_eq!(ack.error, None);
    for _ in 0..1200 {
        b.step();
        if own(&b, Side::Blue, 0)
            .garrison
            .is_some_and(|g| g.phase == contract::observation::GarrisonPhase::Inside)
        {
            break;
        }
    }
    // Let the fog sweep run from inside.
    for _ in 0..30 {
        b.step();
    }
    let squad = own(&b, Side::Blue, 0);
    assert!(squad.garrison.is_some());
    let standoff = common::village()["garrison"]["slot_standoff_m"]
        .as_f64()
        .unwrap();
    let out = 12.0 + standoff;
    // Facades in order +x, +y, -x, -y: each one a soldier stands at gives
    // one eye, at its middle.
    let facades = [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]];
    let z = squad.members[0][2] + eye("infantry_eye_m");
    let expected: Vec<[f64; 3]> = facades
        .iter()
        .filter(|n| {
            squad
                .members
                .iter()
                .any(|m| (m[0] - 360.0) * n[0] + (m[1] - 250.0) * n[1] > out - 0.01)
        })
        .map(|n| [360.0 + n[0] * out, 250.0 + n[1] * out, z])
        .collect();
    assert_eq!(
        expected.len(),
        4,
        "the squad spreads round all four facades"
    );
    assert!(expected.len() < squad.members.len());
    assert_eq!(squad.sight.eyes.len(), expected.len());
    for (got, want) in squad.sight.eyes.iter().zip(&expected) {
        for i in 0..3 {
            assert!((got[i] - want[i]).abs() < 1e-6, "{got:?} vs {want:?}");
        }
    }
    assert_eq!(squad.sight.range, base_range("rifle"));
    // The side's fog is the union of those eyes: open ground 30 m straight
    // out from every facade is seen.
    let fog = &b.observe(Side::Blue).ground_visibility;
    for n in facades {
        let (x, y) = (360.0 + n[0] * (out + 30.0), 250.0 + n[1] * (out + 30.0));
        assert!(fog.visible(x, y), "seen 30 m out of facade {n:?}");
    }
}

#[test]
fn a_turning_turret_replays_to_identical_digests() {
    let setup = common::scenario(
        FLAT,
        json!([
            { "side": "blue", "kind": "tank", "position": [1300, 1300] },
            { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "supply", "position": [1200, 1200] },
        ]),
        json!([]),
    );
    let mut live = Battle::new(&setup, 9);
    let mut digests = Vec::new();
    for _ in 0..240 {
        live.step();
        digests.push(live.digest());
    }
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    for d in digests {
        replay.step();
        assert_eq!(replay.digest(), d);
    }
}

/// The optics' mount is data (`sensors.on`): the jeep's sight stays on its
/// hull as shipped, and turns with its pedestal HMG once its type says the
/// optics sit there.
#[test]
fn a_jeeps_sight_turns_with_its_hmg_only_when_its_sensors_sit_on_it() {
    let run = |on: Option<&str>| {
        let mut rules = common::village();
        if let Some(on) = on {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                "jeep",
                json!({ "sensors": { "on": on, "sight_shape": { "front": 1.0, "side": 0.6, "rear": 0.4 } } }),
            );
        }
        let map: serde_json::Value = serde_json::from_str(FLAT).unwrap();
        let setup = serde_json::from_value(json!({
            "map": map, "rules": rules, "events": [], "scripts": [],
            "units": [
                { "side": "blue", "kind": "jeep", "position": [1300, 1300], "yaw": 0 },
                { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            ],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        for _ in 0..90 {
            b.step();
        }
        let jeep = own(&b, Side::Blue, 0);
        (jeep.yaw, jeep.sight.forward)
    };
    let (yaw, forward) = run(None);
    assert!(
        (forward - yaw).abs() < 1e-9,
        "on its hull: {forward} vs {yaw}"
    );
    let (yaw, forward) = run(Some("HMG"));
    assert!(yaw.abs() < 1e-9, "the hull never turned");
    assert!(
        forward > 0.5,
        "the sight swung north with the HMG toward the squad: {forward}"
    );
}
