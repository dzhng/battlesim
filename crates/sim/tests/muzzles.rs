//! Weapon muzzle placement, friendly clearance and replay contracts.

use std::collections::BTreeSet;

use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ActionReason;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::ProjectileId;

use crate::common::{self, Commander};
use crate::weapons::{battle, map, mount, own, rules, run, scenario_with, ticks, weapon_name};

#[test]
fn a_tank_round_leaves_the_muzzle_past_the_hull_front_on_the_turret_bearing() {
    // The cannon is a realistic gun: its muzzle sits past the hull's front
    // face (about 5.9 m ahead of the hull centre at 2 m: its `mounts` row),
    // and the turret carries it round its ring.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [300, 300] },
            { "side": "red", "kind": "test_tank", "position": [500, 450], "yaw": std::f64::consts::PI },
        ]),
        json!([]),
        json!([]),
    );
    let hull_front = common::hull("test_tank").half_extents_m[0];
    let (origin, bearings) = first_launch(&mut b, 0, "tank_ap", 20.0, |_| true);
    let (dx, dy) = (origin[0] - 300.0, origin[1] - 300.0);
    let reach = dx.hypot(dy);
    let bearing = dy.atan2(dx);
    assert!(
        reach > hull_front + 1.0,
        "the muzzle is well past the hull front: reach {reach:.2} m, front {hull_front} m"
    );
    assert!((reach - 5.9).abs() < 0.2, "reach {reach:.2} m");
    assert!((origin[2] - 2.0).abs() < 0.2, "height {:.2} m", origin[2]);
    assert!(
        (bearing - 150f64.atan2(200.0)).abs() < 0.02,
        "the gun points at the target: bearing {bearing:.3}"
    );
    // Its row: a pivot on the hull's turret axis, the muzzle on its own bearing.
    let row = &tank_mount(0);
    let expected = mount_muzzle(row, [300.0, 300.0], 0.0, bearings[0]);
    let off = (0..3)
        .map(|i| (origin[i] - expected[i]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        off < 0.05,
        "launched at {origin:?}, its row puts the muzzle at {expected:?}"
    );
}

#[test]
fn a_grounded_launcher_fires_from_its_operator_and_declared_bore() {
    let mut game = rules();
    game["weapons"]["atgm"]["damage"] = json!(0.0);
    game["catalog"].as_array_mut().unwrap().push(json!({
        "soldiers":{"tripod_gunner":{"extends":"test_atgm_gunner", "mounts":[{
            "id":"ATGM launcher", "weapons":["atgm"], "special":true,
            "pivot_m":[0.2,-0.1,0.75], "muzzle_m":[0.6,0,0]
        }]}},
        "units":{"tripod_team":{"extends":"test_at", "body":{"squad":{"slots":["test_at_rifleman","tripod_gunner","test_at_rifleman"]}}}}
    }));
    let setup = serde_json::from_value(json!({
        "map":serde_json::from_str::<Value>(&map(json!([]))).unwrap(), "rules":game,
        "units":[{"side":"blue","kind":"tripod_team","position":[100,300]},
            {"side":"red","kind":"test_tank","position":[500,450],"engagement":"return_fire_only"}],
        "events":[], "scripts":[]
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 5);
    let (origin, bearings) = first_launch(&mut b, 0, "atgm", 20.0, |_| true);
    let unit = b.unit(UnitId(0)).unwrap();
    let operator = unit.mounts[1].operator.unwrap();
    let at = unit
        .members
        .iter()
        .find(|s| s.id == operator)
        .unwrap()
        .position;
    let bearing = bearings[1];
    let expected = [
        at.x + 0.8 * bearing.cos() + 0.1 * bearing.sin(),
        at.y + 0.8 * bearing.sin() - 0.1 * bearing.cos(),
        at.z + 0.75,
    ];
    for i in 0..3 {
        assert!(
            (origin[i] - expected[i]).abs() < 0.02,
            "origin {origin:?}, operator bore {expected:?}"
        );
    }
}

/// The first round `unit` launches from `weapon` within `seconds` while its
/// weapon bearings pass `when`: where it left (backed out of its flight:
/// p = o + v0·t − ½g·t², v = v0 − g·t) and those bearings.
fn first_launch(
    b: &mut Battle,
    unit: u32,
    weapon: &str,
    seconds: f64,
    when: impl Fn(&[f64]) -> bool,
) -> ([f64; 3], Vec<f64>) {
    let g = common::game()["physics"]["gravity_mps2"].as_f64().unwrap();
    let mut seen: BTreeSet<ProjectileId> = b.rounds().map(|(p, _)| p.id).collect();
    for _ in 0..ticks(seconds) {
        b.step();
        let mut launched = None;
        for (p, r) in b.rounds() {
            if seen.insert(p.id)
                && r.unit.0 == unit
                && b.arsenal().weapons[r.weapon].id == weapon
                && p.age_s < 0.1
            {
                launched = Some((p.position, p.velocity, p.age_s));
            }
        }
        let Some((at, v, age)) = launched else {
            continue;
        };
        let origin = [
            at.x - v.x * age,
            at.y - v.y * age,
            at.z - v.z * age - 0.5 * g * age * age,
        ];
        let bearings: Vec<f64> = own(b, Side::Blue, unit)
            .weapon_poses
            .iter()
            .map(|p| p.bearing)
            .collect();
        if when(&bearings) {
            return (origin, bearings);
        }
    }
    panic!("unit {unit} never fired its {weapon}");
}

#[test]
fn a_tank_roof_hmg_fires_from_its_own_muzzle_whatever_its_bearing_to_the_turret() {
    // The tank's HMG is its own weapon on the turret roof: its rounds leave
    // the short gun the viewer sees firing, above the turret, however far it
    // has turned from the cannon (sideways, or backwards over the engine
    // deck), never from the cannon's tip or a point in mid-air out to the
    // side. The cannon is ordered onto a tank ahead; the HMG, which can't
    // hurt it, takes a squad at the side or behind.
    let half = common::hull("test_tank").half_extents_m;
    let (hull_front, hull_top) = (half[0], 2.0 * half[2]);
    for deg in [90.0f64, 180.0, -90.0] {
        let a = deg.to_radians();
        // Inside the tank's rear sight (`sensors.sight_shape`).
        let squad = [300.0 + 80.0 * a.cos(), 300.0 + 80.0 * a.sin()];
        let mut b = battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "test_tank", "position": [300, 300] },
                { "side": "red", "kind": "test_tank", "position": [520, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": squad, "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        run(&mut b, 1);
        let tank = b
            .observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind("test_tank"))
            .expect("the tank ahead is seen")
            .id;
        Commander::new().ok(
            &mut b,
            Side::Blue,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Identified { id: tank },
            },
        );
        // Its first burst at the squad (it may rake the tank first).
        let turned = |b: &[f64]| wrap_deg((b[1] - b[0]).to_degrees());
        let (origin, bearings) = first_launch(&mut b, 0, "hmg", 30.0, |b| {
            wrap_deg(turned(b) - deg).abs() < 10.0
        });
        let turned = turned(&bearings);
        let reach = (origin[0] - 300.0).hypot(origin[1] - 300.0);
        assert!(
            reach < hull_front && origin[2] > hull_top,
            "{deg}°: the HMG fires from on the turret roof: reach {reach:.2} m (hull front {hull_front} m), height {:.2} m (hull top {hull_top} m), turned {turned:.0}° from the cannon",
            origin[2]
        );
        // Exactly where its row puts it: the pivot on the turret, turned
        // with the cannon, and the muzzle turned with its own bearing.
        let row = &tank_mount(1);
        assert_eq!(row["on"], "cannon");
        let expected = mount_muzzle(row, [300.0, 300.0], bearings[0], bearings[1]);
        let off = (0..3)
            .map(|i| (origin[i] - expected[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            off < 0.05,
            "{deg}°: launched at {origin:?}, its row puts the muzzle at {expected:?}"
        );
    }
}

#[test]
fn a_roof_hmg_on_an_off_centre_turret_rides_the_turret_round_its_ring() {
    // A turret whose ring sits a metre behind the hull's middle, as on a
    // front-engined vehicle: as it turns, the roof gun on it swings round
    // that ring with the turret, never round the hull's middle, so its
    // rounds still leave the gun the viewer sees on the turret roof.
    let mut authored = rules();
    sim::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_tank",
        json!({ "mounts": [
            { "id": "cannon", "pivot_m": [-1.0, 0, 1.45] },
            { "id": "HMG", "pivot_m": [-1.25, -0.58, 2.35] },
        ] }),
    );
    let rules: contract::scenario::Rules = serde_json::from_value(authored.clone()).unwrap();
    let (cannon, hmg) = (
        serde_json::to_value(&rules.catalog.by_id("test_tank").mounts[0]).unwrap(),
        serde_json::to_value(&rules.catalog.by_id("test_tank").mounts[1]).unwrap(),
    );
    // The turret turns onto a tank off to the left of the hull; the roof gun
    // takes a squad ahead, behind or to the right. The whole scene is laid
    // out for a hull heading east and again for one heading north, so the
    // ring is seen to turn with the hull too.
    let cases = [0.0f64, 90.0]
        .into_iter()
        .flat_map(|hull| [-90.0f64, 90.0, 180.0].map(|deg| (hull, deg)));
    for (hull, deg) in cases {
        let yaw = hull.to_radians();
        let at = |r: f64, rel_deg: f64| {
            let a = yaw + rel_deg.to_radians();
            [300.0 + r * a.cos(), 300.0 + r * a.sin()]
        };
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                { "side": "blue", "kind": "test_tank", "position": [300, 300], "yaw": yaw },
                { "side": "red", "kind": "test_tank", "position": at(140.0, 90.0), "yaw": yaw - std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": at(80.0, deg + 90.0), "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        setup.rules = serde_json::from_value(authored.clone()).unwrap();
        let mut b = Battle::new(&setup, 5);
        run(&mut b, 1);
        let tank = b
            .observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind("test_tank"))
            .expect("the tank ahead is seen")
            .id;
        Commander::new().ok(
            &mut b,
            Side::Blue,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Identified { id: tank },
            },
        );
        let turned = |b: &[f64]| wrap_deg((b[1] - b[0]).to_degrees());
        let (origin, bearings) = first_launch(&mut b, 0, "hmg", 30.0, |b| {
            wrap_deg(turned(b) - deg).abs() < 10.0
        });
        let expected = carried_muzzle(
            &cannon,
            &hmg,
            [300.0, 300.0],
            yaw,
            (bearings[0], bearings[1]),
        );
        let off = (0..3)
            .map(|i| (origin[i] - expected[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            off < 0.05,
            "hull {hull}°, {deg}°: launched at {origin:?}, riding the turret round its ring puts the muzzle at {expected:?}"
        );
    }
}

/// A carried mount's muzzle for a unit at `at` heading `yaw`: its carrier's
/// ring at the carrier's pivot (turned with the hull), its own pivot turned
/// round that ring with the carrier's bearing `carried`, its muzzle along
/// `bearing`.
fn carried_muzzle(
    carrier: &Value,
    row: &Value,
    at: [f64; 2],
    yaw: f64,
    (carried, bearing): (f64, f64),
) -> [f64; 3] {
    let v = |r: &Value, key: &str| -> Vec<f64> {
        r[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    let (ring, pivot, muzzle) = (v(carrier, "pivot_m"), v(row, "pivot_m"), v(row, "muzzle_m"));
    let turn = |x: f64, y: f64, by: f64| [x * by.cos() - y * by.sin(), x * by.sin() + y * by.cos()];
    let r = turn(ring[0], ring[1], yaw);
    let (a, b) = (
        turn(pivot[0] - ring[0], pivot[1] - ring[1], carried),
        turn(muzzle[0], muzzle[1], bearing),
    );
    [
        at[0] + r[0] + a[0] + b[0],
        at[1] + r[1] + a[1] + b[1],
        pivot[2] + muzzle[2],
    ]
}

/// A mount row's muzzle for a unit at `at` (hull yaw 0) whose carrier points
/// along `carried` and the mount along `bearing`.
fn mount_muzzle(row: &Value, at: [f64; 2], carried: f64, bearing: f64) -> [f64; 3] {
    let v = |key: &str| -> Vec<f64> {
        row[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    let (pivot, muzzle) = (v("pivot_m"), v("muzzle_m"));
    let turn = |p: &[f64], by: f64| {
        [
            p[0] * by.cos() - p[1] * by.sin(),
            p[0] * by.sin() + p[1] * by.cos(),
        ]
    };
    let (a, b) = (turn(&pivot, carried), turn(&muzzle, bearing));
    [
        at[0] + a[0] + b[0],
        at[1] + a[1] + b[1],
        pivot[2] + muzzle[2],
    ]
}

/// Degrees into (−180, 180].
fn wrap_deg(d: f64) -> f64 {
    (d + 540.0).rem_euclid(360.0) - 180.0
}

#[test]
fn a_tank_roof_hmg_holds_fire_over_a_friendly_jeep_parked_alongside() {
    // A jeep parks hard alongside the tank. A squad shows up out past it:
    // the roof gunner, whose gun sits over the turret, would be firing just
    // over the jeep's crew, so he holds (P11), judged from his own muzzle,
    // not from a point out beyond the jeep. Without the jeep he fires.
    for jeep in [true, false] {
        let mut units = vec![
            json!({ "side": "blue", "kind": "test_tank", "position": [300, 300] }),
            json!({ "side": "red", "kind": "test_rifle", "position": [300, 380], "engagement": "return_fire_only" }),
        ];
        if jeep {
            units.push(json!({ "side": "blue", "kind": "test_jeep", "position": [300, 303.5], "engagement": "return_fire_only" }));
        }
        let mut b = battle(json!([]), Value::Array(units), json!([]), json!([]));
        let mut held = false;
        let mut fired = false;
        let mut seen: BTreeSet<ProjectileId> = b.rounds().map(|(p, _)| p.id).collect();
        for _ in 0..ticks(15.0) {
            b.step();
            held |= mount(&b, Side::Blue, 0, 1).reason == ActionReason::FriendlyInLine;
            for (p, r) in b.rounds() {
                fired |= seen.insert(p.id) && r.unit.0 == 0 && weapon_name(&b, r.weapon) == "hmg";
            }
        }
        if jeep {
            assert!(
                held && !fired,
                "held {held}, fired {fired}: the HMG never fires over the jeep"
            );
        } else {
            assert!(fired, "with nothing alongside the HMG fires on the squad");
        }
    }
}

#[test]
fn a_tank_firing_both_mounts_apart_replays_to_the_same_digests() {
    // The cannon on a tank ahead, the roof HMG on a squad behind: each
    // mount's muzzle follows its own bearing, and the replay matches.
    let setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [300, 300] },
            { "side": "red", "kind": "test_tank", "position": [520, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [220, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut live = Battle::new(&setup, 11);
    live.step();
    let tank = live
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == common::unit_kind("test_tank"))
        .expect("the tank ahead is seen")
        .id;
    Commander::new().ok(
        &mut live,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    );
    let mut digests = vec![live.digest()];
    let mut apart = false;
    for _ in 0..ticks(12.0) {
        live.step();
        digests.push(live.digest());
        let poses = own(&live, Side::Blue, 0).weapon_poses;
        apart |= poses[1].shots > 0
            && wrap_deg((poses[1].bearing - poses[0].bearing).to_degrees()).abs() > 150.0;
    }
    assert!(apart, "the HMG fired turned away from the cannon");
    let json = serde_json::to_string(&live.replay()).unwrap();
    let mut replay = Battle::from_replay(&setup, &serde_json::from_str(&json).unwrap()).unwrap();
    replay.step();
    for (t, expected) in digests.iter().enumerate() {
        assert_eq!(
            replay.digest(),
            *expected,
            "first mismatch at tick {}",
            t + 1
        );
        replay.step();
    }
}

/// The shipped tank's `n`th mount row, as JSON.
fn tank_mount(n: usize) -> serde_json::Value {
    serde_json::to_value(&common::rules().catalog.by_id("test_tank").mounts[n]).unwrap()
}
