//! Supported AT guidance: shared identification with operator LOS, immediate
//! release on move/Stop/lost sight, no reacquisition (released missiles:
//! `guidance_release`).
use contract::command::{Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, GuidedMissile, OwnUnit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::v3;

use crate::common::{self as c, Commander};

fn battle(props: Value, units: Value, seed: u64) -> Battle {
    let map =
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
            .to_string();
    Battle::new(&c::scenario_with(&map, units, json!([]), json!([])), seed)
}

pub(crate) fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

pub(crate) fn missile(b: &Battle) -> Option<GuidedMissile> {
    b.observe(Side::Blue).guided.first().cloned()
}

/// Step until blue has a guided missile in flight; returns the tick.
pub(crate) fn until_launch(b: &mut Battle) -> u64 {
    for _ in 0..600 {
        b.step();
        if missile(b).is_some() {
            return b.tick();
        }
    }
    panic!("no launch");
}

#[test]
fn identical_launchers_have_their_own_rounds_reload_and_guidance() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["ammo"] = json!(2);
            // Both launchers fire at one tank: this is about each keeping its
            // own rounds and guidance, not about holding a covered kill.
            r["weapons"]["atgm"]["avoids_overkill"] = json!(false);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_at",
                json!({
                    "body": { "squad": { "slots": ["test_atgm_gunner", "test_atgm_gunner", "test_at_rifleman"] } }
                }),
            );
        },
    );
    let ammo: Vec<_> = own(&b, Side::Blue, 0)
        .unwrap()
        .mounts
        .iter()
        .map(|m| m.ammo.clone())
        .collect();
    assert_eq!(ammo, [vec![None], vec![Some(2)], vec![Some(2)]]);
    until_launch(&mut b);
    let launchers = own(&b, Side::Blue, 0).unwrap().mounts[1..].to_vec();
    assert!(launchers.iter().all(|m| m.ammo == [Some(1)] && m.guiding));
    assert_eq!(
        b.observe(Side::Blue)
            .guided
            .iter()
            .filter(|m| m.supported)
            .count(),
        2
    );
    b.step();
    assert!(own(&b, Side::Blue, 0).unwrap().mounts[1..]
        .iter()
        .all(|m| { m.loaded.is_none() && m.reloading == Some(0) && m.reload > 0.0 }));
}

#[test]
fn identical_launchers_can_hold_different_targets_from_their_own_muzzles() {
    for seed in 1..=3 {
        let team = json!({ "side": "blue", "kind": "test_at", "position": [200, 300] });
        let paired = |r: &mut Value| {
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_at",
                json!({
                    "body": { "squad": { "slots": ["test_atgm_gunner", "test_atgm_gunner", "test_at_rifleman"] } }
                }),
            )
        };
        // Place one target in each gunner's reach and outside the other's.
        // Read their public starting positions rather than pinning a seeded
        // formation sample; both battles use the same blue unit and seed.
        let probe = quick(json!([]), json!([team.clone()]), seed, paired);
        let members = own(&probe, Side::Blue, 0).unwrap().members;
        let a = v3(members[0][0], members[0][1], 0.0);
        let c = v3(members[1][0], members[1][1], 0.0);
        let gap = (a - c).length();
        let dir = (a - c) * (1.0 / gap);
        let range = gap + 20.0;
        let margin = gap.min(2.0) / 4.0;
        let targets = [a + dir * (range - margin), c - dir * (range - margin)];
        let mut b = quick(
            json!([]),
            json!([
                team,
                { "side": "red", "kind": "test_tank", "position": [targets[0].x, targets[0].y], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_tank", "position": [targets[1].x, targets[1].y], "engagement": "return_fire_only" },
            ]),
            seed,
            |r| {
                paired(r);
                c::json_ground_reach(&mut r["weapons"]["atgm"], range);
            },
        );
        until_launch(&mut b);
        let view = b.observe(Side::Blue);
        let target_refs: Vec<_> = targets
            .iter()
            .map(|p| {
                let target = view
                    .identified
                    .iter()
                    .find(|t| {
                        (t.position[0] - p.x).abs() < 0.01 && (t.position[1] - p.y).abs() < 0.01
                    })
                    .expect("both tanks are identified");
                Some(contract::command::TargetRef::Identified { id: target.id })
            })
            .collect();
        let launchers = own(&b, Side::Blue, 0).unwrap().mounts[1..].to_vec();
        assert_eq!(
            launchers.iter().map(|m| m.target).collect::<Vec<_>>(),
            target_refs
        );
        assert!(
            launchers.iter().all(|m| m.guiding),
            "seed {seed}: both independent guns launched"
        );
    }
}

#[test]
fn one_survivor_operates_one_launcher_then_takes_up_a_stocked_spare() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300], "condition": { "casualties": 2 } },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["ammo"] = json!(1);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_at",
                json!({
                    "body": { "squad": { "slots": ["test_at_rifleman", "test_atgm_gunner", "test_atgm_gunner"] } }
                }),
            );
            sim::fixtures::patch_catalog(r, "soldiers", "test_at_rifleman", json!({ "hp": 1.0e6 }));
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_tank",
                json!({ "body": { "hull": { "hp": 1.0e6 } } }),
            );
        },
    );
    until_launch(&mut b);
    let first = own(&b, Side::Blue, 0).unwrap();
    assert_eq!(first.member_slots, [0]);
    assert_eq!(first.mounts[1].ammo, [Some(0)]);
    assert_eq!(
        first.mounts[2].ammo,
        [Some(1)],
        "the spare did not fire concurrently"
    );
    assert_eq!(
        b.observe(Side::Blue)
            .guided
            .iter()
            .filter(|m| m.supported)
            .count(),
        1
    );
    for _ in 0..900 {
        b.step();
        let mounts = own(&b, Side::Blue, 0).unwrap().mounts;
        if mounts[2].ammo == [Some(0)] {
            assert_eq!(mounts[1].ammo, [Some(0)]);
            assert!(mounts[2].guiding, "the stocked spare actually launched");
            return;
        }
    }
    panic!("the survivor never took up his stocked spare");
}

#[test]
fn a_recovered_spare_pauses_its_existing_reload_without_losing_progress() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 330], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["reload_s"] = json!(30);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_at",
                json!({
                    "body": { "squad": { "slots": ["test_atgm_gunner", "test_atgm_gunner", "test_at_rifleman"] } }
                }),
            );
            sim::fixtures::patch_catalog(r, "soldiers", "test_at_rifleman", json!({ "hp": 1.0e6 }));
            sim::fixtures::patch_catalog(r, "soldiers", "test_atgm_gunner", json!({ "hp": 1 }));
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_tank",
                json!({ "body": { "hull": { "hp": 1.0e6 } } }),
            );
        },
    );
    until_launch(&mut b);
    Commander::new().ok(
        &mut b,
        Side::Red,
        Order::SetEngagement {
            units: vec![UnitId(2)],
            policy: contract::command::Engagement::FireAtWill,
        },
    );
    for _ in 0..600 {
        b.step();
        if own(&b, Side::Blue, 0).unwrap().member_slots == [2] {
            b.step(); // assign the sole survivor before the next guidance pass
            let before = own(&b, Side::Blue, 0).unwrap();
            let spare = before
                .mounts
                .iter()
                .find(|m| m.mount > 0 && m.reason == ActionReason::NoCompatibleTarget)
                .expect("one weapon remains unoperated");
            assert_eq!(
                spare.reloading,
                Some(0),
                "the spare retained its interrupted reload"
            );
            assert!(spare.reload > 0.0);
            assert_eq!(spare.ammo, [Some(3)]);
            let index = spare.mount as usize;
            for _ in 0..15 {
                b.step();
            }
            let after = own(&b, Side::Blue, 0).unwrap().mounts[index].clone();
            assert_eq!(
                after.reload, spare.reload,
                "unoperated reload progress is frozen"
            );
            assert_eq!(after.ammo, spare.ammo);
            return;
        }
    }
    panic!("the two original gunners never fell");
}

/// Blue's AT team 500 m from a red tank, a wall just north of the line of fire.
pub(crate) fn ambush(seed: u64) -> Battle {
    let map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [
            { "kind": "wall", "center": [560, 336], "yaw": 0, "half_extents": [0.5, 30, 4] }
        ] })
    .to_string();
    let mut setup = c::scenario_with(
        &map,
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300], "engagement": "fire_at_will" },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    // Guidance needs a controlled LOS crossing, independent of vehicle buildup.
    setup.rules.movement.drive.acceleration_s = 1e-6;
    setup.rules.movement.drive.braking_s = 1e-6;
    Battle::new(&setup, seed)
}

pub(crate) fn move_to(unit: u32, goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(unit)],
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}

/// The game rules with a quick ATGM (0.5 s aim, 1 s reload) so a next
/// round is ready while one is still in flight, and any `tweak` applied.
pub(crate) fn quick(props: Value, units: Value, seed: u64, tweak: impl Fn(&mut Value)) -> Battle {
    let mut rules = c::game();
    rules["weapons"]["atgm"]["aim_s"] = json!(0.5);
    rules["weapons"]["atgm"]["reload_s"] = json!(1.0);
    tweak(&mut rules);
    let map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props });
    let setup = serde_json::from_value(
        json!({ "map": map, "rules": rules, "units": units, "events": [], "scripts": [] }),
    )
    .unwrap();
    Battle::new(&setup, seed)
}

/// An AT team (eyes out to 2 km) launches at a still red tank `range_m` due
/// east, which then drives north across its line of fire. Returns the
/// tank's hp once the first missile has ended, and whether the missile
/// was guided all the way (never released).
fn crossing_shot(range_m: f64, seed: u64) -> (f64, bool) {
    let mut rules = c::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_at",
        json!({ "sensors": { "ground_m": 2000 } }),
    );
    let map = json!({ "size": [2200, 1400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let units = json!([
        { "side": "blue", "kind": "test_at", "position": [100, 500], "engagement": "fire_at_will" },
        { "side": "red", "kind": "test_tank", "position": [100.0 + range_m, 400], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
    ]);
    let setup = serde_json::from_value(
        json!({ "map": map, "rules": rules, "units": units, "events": [], "scripts": [] }),
    )
    .unwrap();
    let mut b = Battle::new(&setup, seed);
    until_launch(&mut b);
    // Once the missile is away the still tank drives off north across its
    // line: the launch led nothing, and only steering brings it on.
    Commander::new().ok(&mut b, Side::Red, move_to(1, [100.0 + range_m, 1300.0]));
    let mut guided = true;
    while let Some(m) = missile(&b) {
        guided &= m.supported;
        b.step();
    }
    (own(&b, Side::Red, 1).map_or(0.0, |u| u.hp), guided)
}

#[test]
fn a_missile_hits_a_tank_driving_across_its_line_near_and_at_full_range() {
    // Faster, accelerating missiles still steer onto a tank that moves off
    // after the launch, close in and near its full range (100 m short of it,
    // since the tank also stands 100 m off the launcher's line). (With
    // steering off, the missile misses.)
    let full = c::hull("test_tank").hp;
    let damage = c::game()["weapons"]["atgm"]["damage"].as_f64().unwrap();
    let far = c::weapon("atgm").range_m - 100.0;
    for range_m in [300.0, far] {
        for seed in 1..=3 {
            let (hp, guided) = crossing_shot(range_m, seed);
            assert!(guided, "{range_m} m, seed {seed}: guided to the end");
            assert!(
                (hp - (full - damage)).abs() < 1e-9,
                "{range_m} m, seed {seed}: struck, hp {hp}"
            );
        }
    }
}

#[test]
fn a_launcher_fires_and_guides_on_its_own_sight() {
    let mut b = ambush(1);
    until_launch(&mut b);
    let at = own(&b, Side::Blue, 0).unwrap();
    assert!(at.mounts.iter().any(|m| m.guiding), "the launcher guides");
    assert!(missile(&b).unwrap().supported);
}

#[test]
fn a_ready_next_round_waits_while_one_is_guided_and_fires_once_released() {
    // 560 m: about 1.8 s of flight, while the quick reload takes 1 s. The aim
    // after a Stop is shorter than a released missile's coast, so the crew's
    // next launch overlaps the first missile's last half second.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| r["weapons"]["atgm"]["aim_s"] = json!(0.2),
    );
    until_launch(&mut b);
    let mut waited = false;
    for _ in 0..45 {
        b.step();
        let atgm = own(&b, Side::Blue, 0).unwrap().mounts[1].clone();
        assert!(
            b.observe(Side::Blue).guided.len() <= 1,
            "one missile at a time"
        );
        if atgm.loaded.is_some() && atgm.aim >= 1.0 {
            assert_eq!(atgm.reason, ActionReason::Guiding);
            waited = true;
        }
    }
    assert!(waited, "a ready round waited on the guided one");
    // Stop releases; the crew launches again while the first still flies.
    let mut o = Commander::new();
    o.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    let mut both = false;
    for _ in 0..40 {
        b.step();
        let g = &b.observe(Side::Blue).guided;
        both |= g.len() == 2 && g.iter().filter(|m| m.supported).count() == 1;
    }
    assert!(both, "a second missile while the released first still flew");
}

#[test]
fn the_launcher_dying_releases_its_missile() {
    // A single fragile launcher; a red scout squad nearby shoots it.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 330] },
        ]),
        2,
        |r| {
            let at = json!({ "body": { "squad": { "slots": ["test_atgm_gunner"] } } });
            sim::fixtures::patch_catalog(r, "units", "test_at", at);
            sim::fixtures::patch_catalog(r, "soldiers", "test_rifleman", json!({ "hp": 1 }));
        },
    );
    until_launch(&mut b);
    let mut released = None;
    for _ in 0..120 {
        b.step();
        if own(&b, Side::Blue, 0).is_none() {
            released = missile(&b);
            break;
        }
    }
    let m = released.expect("the launcher died while its missile flew");
    b.step();
    let m2 = missile(&b).unwrap_or(m.clone());
    assert!(!m2.supported, "death releases support");
    assert!(m2.point[2].abs() < 1e-6, "to a point on the ground");
}

#[test]
fn shared_identification_supports_a_missile_beyond_the_launchers_sensor_range() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300] },
            { "side": "blue", "kind": "test_recon", "position": [450, 320] },
            { "side": "red", "kind": "test_tank", "position": [800, 300], "engagement": "return_fire_only" },
        ]),
        2,
        |r| {
            sim::fixtures::patch_catalog(
                r,
                "units",
                "test_at",
                json!({ "sensors": { "ground_m": 100 } }),
            );
            c::json_ground_reach(&mut r["weapons"]["atgm"], 900.0);
            for w in r["weapons"].as_object_mut().unwrap().values_mut() {
                w["damage"] = json!(0);
            }
        },
    );
    b.step();
    assert!(own(&b, Side::Blue, 0).unwrap().sees.is_empty());
    assert!(!b.observe(Side::Blue).identified.is_empty());
    until_launch(&mut b);
    let mut flight_ticks = 0;
    while let Some(m) = missile(&b) {
        assert!(m.supported, "clear physical LOS supports the full flight");
        b.step();
        flight_ticks += 1;
        assert!(flight_ticks < 400, "flight terminates");
    }
    assert!(
        flight_ticks > 30,
        "guidance lasted beyond the acquisition tick"
    );
}

#[test]
fn shared_spotting_cannot_launch_through_an_obstruction() {
    // The AT team is walled off from the tank; a scout sees it for the team.
    let mut b = battle(
        json!([{ "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 40, 5] }]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300] },
            { "side": "blue", "kind": "test_recon", "position": [160, 380] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        2,
    );
    for _ in 0..300 {
        b.step();
        assert!(missile(&b).is_none(), "never launched through the wall");
    }
    assert!(
        !b.observe(Side::Blue).identified.is_empty(),
        "the team does see it"
    );
    let atgm = &own(&b, Side::Blue, 0).unwrap().mounts[1];
    assert_eq!(atgm.reason, ActionReason::NoOwnSight);
}

#[test]
fn steering_is_bounded_by_the_turn_rate() {
    let v = v3(180.0, 0.0, 0.0);
    let max = 60f64.to_radians() / 30.0;
    let turned = v.turned_toward(v3(0.0, 1.0, 0.0), max);
    let angle = (turned.dot(v) / (turned.length() * v.length())).acos();
    assert!((angle - max).abs() < 1e-9 && (turned.length() - 180.0).abs() < 1e-9);
    // Within the limit it points straight at the goal.
    let small = v.turned_toward(v3(1.0, 0.001, 0.0), max);
    assert!((small.y / small.x - 0.001).abs() < 1e-9);
}

#[test]
fn a_guided_missile_ends_at_its_lifetime() {
    // Launch solving refuses a target beyond a missile's lifetime, so a guided
    // round ends (strikes or expires) within it; expiry itself is the shared
    // flight rule pinned in slice 07.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        3,
        |r| {
            r["weapons"]["atgm"]["lifetime_s"] = json!(4.0);
            c::json_ground_reach(&mut r["weapons"]["atgm"], 1400.0);
            r["weapons"]["atgm"]["speed_mps"] = json!(150.0);
        },
    );
    let launched = until_launch(&mut b);
    let mut ended = None;
    for _ in 0..200 {
        b.step();
        if missile(&b).is_none() {
            ended = Some(b.tick() - launched);
            break;
        }
    }
    let ended = ended.expect("the missile ended");
    assert!(ended <= 4 * 30 + 1, "within its lifetime: {ended} ticks");
}

#[test]
fn guided_flight_repeats_from_its_seed() {
    let (mut a, mut c) = (ambush(11), ambush(11));
    for _ in 0..300 {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}

#[test]
fn test_a_guided_missile_aims_at_an_aircrafts_centre_and_high_on_a_ground_hull() {
    let aim = |target: &str| {
        let mut b = quick(
            json!([]),
            json!([
                { "side": "blue", "kind": "test_at", "position": [40, 300] },
                { "side": "red", "kind": target, "position": [500, 300], "engagement": "return_fire_only" }
            ]),
            7,
            |r| {
                r["physics"]["vehicle_aim_height_fraction"] = json!(0.75);
                r["weapons"]["atgm"]["targets"] = json!({
                    "ground": { "classes": ["vehicle"], "range_m": 650, "scatter_mrad": 0.5 },
                    "low_air": { "classes": ["air_rotor"], "range_m": 650, "scatter_mrad": 0.5 }
                });
                sim::fixtures::patch_catalog(
                    r,
                    "units",
                    target,
                    json!({ "body": { "hull": { "half_extents_m": [3.0, 1.2, 1.2] } } }),
                );
            },
        );
        until_launch(&mut b);
        b.step();
        let point = missile(&b).expect("the missile is guided").point;
        point[2] - b.unit(UnitId(1)).unwrap().position.z
    };
    let air = aim("test_heli");
    assert!(
        (air - 1.2).abs() < 0.05,
        "aimed {air} m up a 2.4 m airframe"
    );
    let ground = aim("test_jeep");
    assert!(
        (ground - 1.8).abs() < 0.05,
        "aimed {ground} m up a 2.4 m hull"
    );
}

#[test]
fn seeker_missile_is_fire_and_forget_after_launch() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" }
        ]),
        7,
        |r| {
            r["weapons"]["atgm"]["seeker"] = json!("infrared");
            r["weapons"]["atgm"]["seeker_fov_deg"] = json!(180.0);
        },
    );
    until_launch(&mut b);
    let guided = missile(&b).expect("the seeker launched");
    assert!(
        !guided.supported,
        "a seeker is independent of launcher support"
    );
}
