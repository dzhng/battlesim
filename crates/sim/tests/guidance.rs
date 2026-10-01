//! Supported AT guidance (slice 10): own-sight launch and support, immediate
//! release on move/Stop/lost sight, no reacquisition. A released missile
//! coasts straight on, then goes to ground.
use contract::command::{Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, GuidedMissile, OwnUnit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::steer;
use sim::math::v3;

use crate::common::{self, Commander};

fn battle(props: Value, units: Value, seed: u64) -> Battle {
    let map =
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
            .to_string();
    Battle::new(
        &common::scenario_with(&map, units, json!([]), json!([])),
        seed,
    )
}

fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

fn missile(b: &Battle) -> Option<GuidedMissile> {
    b.observe(Side::Blue).guided.first().cloned()
}

/// Step until blue has a guided missile in flight; returns the tick.
fn until_launch(b: &mut Battle) -> u64 {
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
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["ammo"] = json!(2);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "at",
                json!({
                    "body": { "squad": { "slots": ["atgm_gunner", "atgm_gunner", "at_rifleman"] } }
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
        let team = json!({ "side": "blue", "kind": "at", "position": [200, 300] });
        let paired = |r: &mut Value| {
            sim::fixtures::patch_catalog(
                r,
                "units",
                "at",
                json!({
                    "body": { "squad": { "slots": ["atgm_gunner", "atgm_gunner", "at_rifleman"] } }
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
                { "side": "red", "kind": "tank", "position": [targets[0].x, targets[0].y], "engagement": "return_fire_only" },
                { "side": "red", "kind": "tank", "position": [targets[1].x, targets[1].y], "engagement": "return_fire_only" },
            ]),
            seed,
            |r| {
                paired(r);
                r["weapons"]["atgm"]["range_m"] = json!(range);
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
            { "side": "blue", "kind": "at", "position": [40, 300], "condition": { "casualties": 2 } },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["ammo"] = json!(1);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "at",
                json!({
                    "body": { "squad": { "slots": ["at_rifleman", "atgm_gunner", "atgm_gunner"] } }
                }),
            );
            sim::fixtures::patch_catalog(r, "soldiers", "at_rifleman", json!({ "hp": 1.0e6 }));
            sim::fixtures::patch_catalog(
                r,
                "units",
                "tank",
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
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [100, 330], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            r["weapons"]["atgm"]["reload_s"] = json!(30);
            sim::fixtures::patch_catalog(
                r,
                "units",
                "at",
                json!({
                    "body": { "squad": { "slots": ["atgm_gunner", "atgm_gunner", "at_rifleman"] } }
                }),
            );
            sim::fixtures::patch_catalog(r, "soldiers", "at_rifleman", json!({ "hp": 1.0e6 }));
            sim::fixtures::patch_catalog(r, "soldiers", "atgm_gunner", json!({ "hp": 1 }));
            sim::fixtures::patch_catalog(
                r,
                "units",
                "tank",
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
fn ambush(seed: u64) -> Battle {
    let map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [
            { "kind": "wall", "center": [560, 336], "yaw": 0, "half_extents": [0.5, 30, 4] }
        ] })
    .to_string();
    let mut setup = common::scenario_with(
        &map,
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300], "engagement": "fire_at_will" },
            { "side": "red", "kind": "tank", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    // Guidance needs a controlled LOS crossing, independent of vehicle buildup.
    setup.rules.movement.drive.acceleration_s = 1e-6;
    setup.rules.movement.drive.braking_s = 1e-6;
    Battle::new(&setup, seed)
}

fn move_to(unit: u32, goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(unit)],
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}

/// The village rules with a quick ATGM (0.5 s aim, 1 s reload) so a next
/// round is ready while one is still in flight, and any `tweak` applied.
fn quick(props: Value, units: Value, seed: u64, tweak: impl Fn(&mut Value)) -> Battle {
    let mut rules = common::village();
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
    let mut rules = common::village();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "at",
        json!({ "sensors": { "ground_m": 2000 } }),
    );
    let map = json!({ "size": [2200, 1400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let units = json!([
        { "side": "blue", "kind": "at", "position": [100, 500], "engagement": "fire_at_will" },
        { "side": "red", "kind": "tank", "position": [100.0 + range_m, 400], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
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
    let full = common::hull("tank").hp;
    let damage = common::village()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    let far = common::weapon("atgm").range_m - 100.0;
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
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
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
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [100, 330] },
        ]),
        2,
        |r| {
            let at = json!({ "body": { "squad": { "slots": ["atgm_gunner"] } } });
            sim::fixtures::patch_catalog(r, "units", "at", at);
            sim::fixtures::patch_catalog(r, "soldiers", "rifleman", json!({ "hp": 1 }));
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
fn team_identification_is_not_enough_to_launch() {
    // The AT team is walled off from the tank; a scout sees it for the team.
    let mut b = battle(
        json!([{ "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 40, 5] }]),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [160, 380] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        2,
    );
    for _ in 0..300 {
        b.step();
        assert!(missile(&b).is_none(), "never launched on shared sight");
    }
    assert!(
        !b.observe(Side::Blue).identified.is_empty(),
        "the team does see it"
    );
    let atgm = &own(&b, Side::Blue, 0).unwrap().mounts[1];
    assert_eq!(atgm.reason, ActionReason::NoOwnSight);
}

#[test]
fn moving_releases_at_once_and_frees_the_crew() {
    let mut b = ambush(3);
    until_launch(&mut b);
    let before = missile(&b).unwrap();
    let mut o = Commander::new();
    o.ok(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    b.step();
    let after = missile(&b).expect("still flying");
    assert!(!after.supported, "movement releases support");
    // The point is a coast straight ahead, on the ground: the tank 500 m off
    // no longer draws it.
    let speed = common::village()["weapons"]["atgm"]["speed_mps"]
        .as_f64()
        .unwrap();
    let coast = speed * coast_s();
    let ahead = horizontal(after.point, before.position);
    assert!(
        (ahead - coast).abs() <= speed / common::tick_hz() as f64 + 1.0,
        "{ahead:.1} m ahead of release, coast {coast:.1} m"
    );
    assert!(
        (after.point[1] - before.position[1]).abs() < 1.0,
        "straight on"
    );
    assert!(
        after.point[2].abs() < 1e-6,
        "on the ground: {:?}",
        after.point
    );
    let at0 = own(&b, Side::Blue, 0).unwrap().position;
    b.step();
    assert!(missile(&b).is_some(), "the missile flies on");
    assert_ne!(
        own(&b, Side::Blue, 0).unwrap().position,
        at0,
        "the crew is free to move"
    );
    // It dives into the ground at the point, never reaching the tank.
    let full = common::hull("tank").hp;
    let mut burst = None;
    for _ in 0..120 {
        b.step();
        burst = burst.or(b.observe(Side::Blue).blasts.first().map(|x| x.point));
    }
    let burst = burst.expect("it burst");
    assert!(
        horizontal(burst, after.point) < 1.0,
        "at its point: {burst:?}"
    );
    assert_eq!(
        own(&b, Side::Red, 1).unwrap().hp,
        full,
        "the tank is untouched"
    );
}

#[test]
fn stop_releases_and_the_point_never_moves_again() {
    let mut b = ambush(4);
    until_launch(&mut b);
    let mut o = Commander::new();
    o.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    b.step();
    let frozen = missile(&b).unwrap();
    assert!(!frozen.supported);
    // Red's tank drives off; the released missile never follows or reacquires.
    o.ok(&mut b, Side::Red, move_to(1, [600.0, 250.0]));
    while let Some(m) = missile(&b) {
        assert!(!m.supported, "never reacquires");
        assert_eq!(m.point, frozen.point, "the point stays put");
        b.step();
    }
}

#[test]
fn a_launcher_that_moves_with_its_missile_close_still_hits_a_still_target() {
    let mut b = ambush(5);
    until_launch(&mut b);
    // Release inside the coast distance of the tank's west face (x 598).
    while missile(&b).expect("in flight").position[0] < 540.0 {
        b.step();
    }
    let mut o = Commander::new();
    o.ok(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    let full = common::hull("tank").hp;
    let damage = common::village()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    b.step();
    assert!(!missile(&b).expect("still flying").supported, "released");
    for _ in 0..60 {
        b.step();
    }
    let tank = own(&b, Side::Red, 1).unwrap();
    assert!(
        (tank.hp - (full - damage)).abs() < 1e-9,
        "normal damage: {}",
        tank.hp
    );
}

/// The tank drives behind the wall `delay` ticks after the launch; returns its
/// hp and whether the missile was released. Along the way: once released it is
/// never supported again (even when the tank comes back into view), and it
/// never turns faster than its limit.
fn retreat(delay: u64, seed: u64, come_back: bool) -> (f64, bool) {
    let mut b = ambush(seed);
    until_launch(&mut b);
    let mut released = false;
    let mut o = Commander::new();
    let turn = common::village()["weapons"]["atgm"]["turn_deg_s"]
        .as_f64()
        .unwrap()
        .to_radians()
        / 30.0;
    let mut path: Vec<[f64; 3]> = Vec::new();
    for t in 0..240 {
        if t == delay {
            o.ok(&mut b, Side::Red, move_to(1, [600.0, 345.0]));
        }
        if come_back && t == delay + 60 {
            // Back out into view: a released missile does not reacquire.
            o.ok(&mut b, Side::Red, move_to(1, [600.0, 290.0]));
        }
        b.step();
        if let Some(m) = missile(&b) {
            assert!(!(released && m.supported), "never reacquires");
            released |= !m.supported;
            path.push(m.position);
        }
    }
    for w in path.windows(3) {
        let (a, c) = (sub(w[1], w[0]), sub(w[2], w[1]));
        let cos = dot(a, c) / (dot(a, a).sqrt() * dot(c, c).sqrt());
        assert!(
            cos.clamp(-1.0, 1.0).acos() <= turn + 1e-6,
            "turned faster than its limit"
        );
    }
    (own(&b, Side::Red, 1).map_or(0.0, |u| u.hp), released)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[test]
fn a_prompt_retreat_out_of_sight_escapes_and_a_late_one_does_not() {
    let full = common::hull("tank").hp;
    for seed in [6, 7, 8] {
        let (prompt, lost) = retreat(0, seed, false);
        assert!(lost, "the launcher lost sight");
        assert_eq!(prompt, full, "seed {seed}: prompt retreat escapes");
        let (late, _) = retreat(120, seed, false);
        assert!(late < full, "seed {seed}: a late retreat is hit");
    }
}

#[test]
fn a_released_missile_does_not_reacquire_a_target_back_in_view() {
    // `retreat` asserts it on every tick while the tank drives back out.
    for seed in [6, 7] {
        let (_, released) = retreat(0, seed, true);
        assert!(released);
    }
}

#[test]
fn steering_is_bounded_by_the_turn_rate() {
    let v = v3(180.0, 0.0, 0.0);
    let max = 60f64.to_radians() / 30.0;
    let turned = steer(v, v3(0.0, 1.0, 0.0), max);
    let angle = (turned.dot(v) / (turned.length() * v.length())).acos();
    assert!((angle - max).abs() < 1e-9 && (turned.length() - 180.0).abs() < 1e-9);
    // Within the limit it points straight at the goal.
    let small = steer(v, v3(1.0, 0.001, 0.0), max);
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
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        3,
        |r| {
            r["weapons"]["atgm"]["lifetime_s"] = json!(4.0);
            r["weapons"]["atgm"]["range_m"] = json!(1400);
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

/// What a release does to a missile in flight. The AT team 500 m
/// from a stationary red tank; `screen_after` ticks after launch a wall rises
/// just in front of the team (a stand-in for smoke), behind the missile, so
/// the launcher loses its own sighting while the tank stays put.
struct Screened {
    /// The missile's last supported position and its velocity there.
    released_at: [f64; 3],
    velocity: [f64; 3],
    /// Where the missile burst.
    impact: [f64; 3],
    tank_hp: f64,
}

/// The screened battle, before its first step, and its launch tick.
fn screened_battle(screen_after: u64, seed: u64) -> (Battle, u64) {
    let units = json!([
        { "side": "blue", "kind": "at", "position": [100, 300] },
        { "side": "red", "kind": "tank", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
    ]);
    let map =
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    let launch = until_launch(&mut Battle::new(
        &common::scenario_with(&map, units.clone(), json!([]), json!([])),
        seed,
    ));
    let screen = json!([{ "tick": launch + screen_after, "add_prop":
        { "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 60, 5] } }]);
    let b = Battle::new(&common::scenario_with(&map, units, screen, json!([])), seed);
    (b, launch)
}

fn screened(screen_after: u64, seed: u64) -> Screened {
    let (mut b, _) = screened_battle(screen_after, seed);
    let hz = common::tick_hz() as f64;
    let (mut prev, mut last): (Option<[f64; 3]>, Option<[f64; 3]>) = (None, None);
    let mut released = None;
    for _ in 0..600 {
        b.step();
        let obs = b.observe(Side::Blue);
        if let (Some(blast), Some((at, velocity))) = (obs.blasts.first(), released) {
            return Screened {
                released_at: at,
                velocity,
                impact: blast.point,
                tank_hp: own(&b, Side::Red, 1).map_or(0.0, |u| u.hp),
            };
        }
        match obs.guided.first() {
            Some(m) if m.supported => (prev, last) = (last, Some(m.position)),
            Some(_) if released.is_none() => {
                let (p, l) = (prev.expect("flew a tick"), last.unwrap());
                let v = [(l[0] - p[0]) * hz, (l[1] - p[1]) * hz, (l[2] - p[2]) * hz];
                released = Some((l, v));
            }
            _ => {}
        }
    }
    panic!("no release and burst");
}

fn coast_s() -> f64 {
    common::village()["guided"]["release_coast_s"]
        .as_f64()
        .unwrap()
}

fn horizontal(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn a_far_missile_that_loses_sight_coasts_and_goes_to_ground_short_of_a_still_target() {
    let full = common::hull("tank").hp;
    for seed in [21, 22] {
        let s = screened(15, seed);
        let speed = s.velocity[0].hypot(s.velocity[1]);
        let coast = speed * coast_s();
        let flew = horizontal(s.impact, s.released_at);
        // Within one tick of flight: the release is judged on the tick after
        // the sighting lapses.
        assert!(
            (flew - coast).abs() <= speed / common::tick_hz() as f64 + 1.0,
            "seed {seed}: burst {flew:.1} m past release, coast {coast:.1} m"
        );
        assert!(s.impact[2].abs() < 0.05, "on the ground: {:?}", s.impact);
        assert!(s.impact[0] < 500.0, "short of the tank: {:?}", s.impact);
        assert_eq!(s.tank_hp, full, "seed {seed}: the still tank is untouched");
    }
}

#[test]
fn a_close_missile_that_loses_sight_still_hits_a_still_target() {
    let full = common::hull("tank").hp;
    let damage = common::village()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    for seed in [21, 22] {
        // About 70 m short of the tank at release, inside the coast distance.
        let s = screened(46, seed);
        assert!(
            600.0 - s.released_at[0] < s.velocity[0] * coast_s(),
            "seed {seed}: released within a coast of the tank: {:?}",
            s.released_at
        );
        assert!(
            (s.tank_hp - (full - damage)).abs() < 1e-9,
            "seed {seed}: normal damage: {}",
            s.tank_hp
        );
    }
}

#[test]
fn a_screened_release_repeats_from_its_seed() {
    let (mut a, launch) = screened_battle(15, 21);
    let (mut c, _) = screened_battle(15, 21);
    for _ in 0..launch + 120 {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}

#[test]
fn a_survivor_keeps_guiding_when_the_original_gunner_falls() {
    // A fragile gunner guides at a far tank; once he launches, a red scout
    // team opens fire on his team, whose riflemen do not fall. Whenever he
    // falls with a missile in flight, a survivor takes up the same launcher
    // and keeps guiding that missile.
    let mut takeovers = 0;
    for seed in 1..=6 {
        let mut b = quick(
            json!([]),
            json!([
                { "side": "blue", "kind": "at", "position": [40, 300] },
                { "side": "red", "kind": "tank", "position": [630, 300], "engagement": "return_fire_only" },
                { "side": "red", "kind": "recon", "position": [100, 330], "engagement": "return_fire_only" },
            ]),
            seed,
            |r| {
                let hp = |hp| json!({ "hp": hp });
                sim::fixtures::patch_catalog(r, "soldiers", "at_rifleman", hp(100_000));
                sim::fixtures::patch_catalog(r, "soldiers", "atgm_gunner", hp(1));
            },
        );
        until_launch(&mut b);
        let open_fire = Order::SetEngagement {
            units: vec![UnitId(2)],
            policy: contract::command::Engagement::FireAtWill,
        };
        Commander::new().ok(&mut b, Side::Red, open_fire);
        for _ in 0..600 {
            let flying = missile(&b).filter(|m| m.supported);
            b.step();
            let gunner = own(&b, Side::Blue, 0).is_some_and(|u| u.member_slots.contains(&0));
            // His missile, if it is still in flight the tick he fell: one that
            // struck that same tick had nothing left to release.
            let still = |b: &Battle| {
                let id = flying.as_ref()?.id;
                b.observe(Side::Blue)
                    .guided
                    .iter()
                    .find(|m| m.id == id)
                    .cloned()
            };
            if let (Some(before), false, true) = (&flying, gunner, still(&b).is_some()) {
                // Guidance runs before damage: the next tick assigns a
                // survivor before steering the existing missile.
                b.step();
                let Some(m) = still(&b) else {
                    // It may hit during the handoff tick: there is no continuing
                    // guidance to inspect, but disappearance must be a real impact.
                    assert!(b.flight_events().iter().any(|event| matches!(
                        event, sim::flight::FlightEvent::Impact(i)
                            if i.projectile.0 == before.id && i.detonated
                    )));
                    break;
                };
                assert!(m.supported, "seed {seed}: a survivor keeps guiding");
                assert_eq!(m.id, before.id, "the same missile survives the handoff");
                assert!(
                    m.point[0] > 600.0 && m.point[2] > 0.0,
                    "still aiming at the tank"
                );
                takeovers += 1;
                break;
            }
            if !gunner {
                break;
            }
        }
    }
    assert!(takeovers > 0, "no seed saw the gunner fall mid-flight");
}

/// A living soldier: his slot and where he stands.
type Member = (u8, [f64; 3]);

/// Steps until blue unit 0 launches an ATGM: where it left (its muzzle) and
/// the launcher's living soldiers then, as (slot, position).
fn atgm_launch(b: &mut Battle, ticks: u64) -> Option<([f64; 3], Vec<Member>)> {
    let mut seen: std::collections::BTreeSet<_> = b.rounds().map(|(p, _)| p.id).collect();
    for _ in 0..ticks {
        b.step();
        let launched = b.rounds().find_map(|(p, r)| {
            let atgm = b.arsenal().weapons[r.weapon].id == "atgm";
            (seen.insert(p.id) && atgm && r.unit == UnitId(0))
                .then_some(p.position - p.velocity * p.age_s)
        });
        if let Some(at) = launched {
            let u = own(b, Side::Blue, 0).unwrap();
            let members = u.member_slots.iter().copied().zip(u.members).collect();
            return Some(([at.x, at.y, at.z], members));
        }
    }
    None
}

/// A round left from this soldier's own muzzle where he stands.
fn standing_at(origin: [f64; 3], soldier: [f64; 3]) -> bool {
    let muzzle = common::village()["physics"]["infantry_muzzle_m"]
        .as_f64()
        .unwrap();
    horizontal(origin, soldier) < 0.05 && (origin[2] - soldier[2] - muzzle).abs() < 0.05
}

#[test]
fn a_special_weapon_leaves_from_its_carrier_in_whatever_slot_he_stands() {
    // The gunner stands in the team's middle slot: the missile leaves from
    // his own muzzle, not the first soldier's and not the team's middle.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            let slots = ["at_rifleman", "atgm_gunner", "at_rifleman"];
            let at = json!({ "body": { "squad": { "slots": slots } } });
            sim::fixtures::patch_catalog(r, "units", "at", at);
        },
    );
    let (origin, members) = atgm_launch(&mut b, 600).expect("the team launched");
    let gunner = members
        .iter()
        .find(|m| m.0 == 1)
        .expect("the gunner lives")
        .1;
    assert!(
        standing_at(origin, gunner),
        "from the gunner at {gunner:?}, not {origin:?}"
    );
}

#[test]
fn a_fallen_gunners_launcher_fires_from_the_soldier_who_took_it_up() {
    // A fragile gunner with two riflemen; a red scout team shoots him. The
    // next missile leaves from the first living soldier's own muzzle.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [100, 330] },
        ]),
        2,
        |r| {
            let hp = |hp| json!({ "hp": hp });
            sim::fixtures::patch_catalog(r, "soldiers", "at_rifleman", hp(100_000));
            sim::fixtures::patch_catalog(r, "soldiers", "atgm_gunner", hp(1));
        },
    );
    let mut fallen = false;
    for _ in 0..900 {
        b.step();
        let Some(u) = own(&b, Side::Blue, 0) else {
            break;
        };
        if !u.member_slots.contains(&0) {
            fallen = true;
            break;
        }
    }
    assert!(fallen, "the gunner fell with his team alive");
    let (origin, members) = atgm_launch(&mut b, 900).unwrap_or_else(|| {
        panic!(
            "the team launched again: {:?} tank {:?}",
            own(&b, Side::Blue, 0).map(|u| (u.member_slots, u.mounts)),
            own(&b, Side::Red, 1).map(|u| u.hp)
        )
    });
    let first = members.iter().min_by_key(|m| m.0).unwrap();
    assert!(
        standing_at(origin, first.1),
        "from slot {} at {:?}, not {origin:?}",
        first.0,
        first.1
    );
}
