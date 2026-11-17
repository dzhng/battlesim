//! Magazine readiness, interruption and finite round accounting.
use super::*;

#[test]
fn engaged_rifles_and_hmgs_fire_bursts_then_reload_only_empty_magazines() {
    for (kind, weapon, seed) in [
        ("test_rifle", "rifle", 1),
        ("test_rifle", "rifle", 5),
        ("test_rifle", "rifle", 19),
        ("test_tank", "hmg", 1),
        ("test_tank", "hmg", 5),
        ("test_tank", "hmg", 19),
    ] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                { "side": "blue", "kind": kind, "position": [100, 300] },
                { "side": "red", "kind": "test_tank", "position": [1100, 550], "engagement": "return_fire_only" }
            ]),
            json!([]),
            json!([]),
        );
        let capacity = if kind == "test_rifle" { 30 } else { 8 };
        let def = setup.rules.weapons.get_mut(weapon).unwrap();
        def.magazine = Some(contract::weapons::Magazine {
            rounds: capacity,
            shot_interval_s: 0.1,
            burst: Some(contract::weapons::Burst {
                rounds: 3,
                aim_max_s: 1.0,
            }),
        });
        def.reload_s = 3.0;
        let aim = def.aim_s;
        let mut b = Battle::new(&setup, seed);
        common::order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [500.0, 300.0, 0.0],
                },
            },
        );
        let mut seen = BTreeSet::new();
        let mut by_soldier = std::collections::BTreeMap::<_, Vec<_>>::new();
        let mut digests = Vec::new();
        for _ in 0..1200 {
            b.step();
            digests.push(b.digest());
            for (p, r) in b.rounds() {
                if r.unit == UnitId(0) && weapon_name(&b, r.weapon) == weapon && seen.insert(p.id) {
                    by_soldier
                        .entry(p.shooter.unwrap().body.0)
                        .or_default()
                        .push(b.tick());
                }
            }
        }
        assert!(!by_soldier.is_empty(), "{weapon} fires");
        if kind == "test_rifle" {
            assert_eq!(by_soldier.len(), 8);
            let starts: BTreeSet<_> = by_soldier.values().map(|shots| shots[0]).collect();
            assert!(starts.len() > 1, "soldiers aim independently");
            assert!(
                *starts.last().unwrap() <= ticks(aim + 1.0) + 1,
                "initial aim is bounded"
            );
            assert!(
                starts.last().unwrap() - starts.first().unwrap() <= 30,
                "initial stagger stays within one second"
            );
        }
        for ticks in by_soldier.values() {
            let mut pauses = BTreeSet::new();
            assert!(
                ticks.len() >= 2 * capacity as usize,
                "two magazines fired: {ticks:?}"
            );
            for (i, gap) in ticks.windows(2).map(|w| w[1] - w[0]).enumerate() {
                if (i + 1).is_multiple_of(capacity as usize) {
                    assert!(
                        (90..=121).contains(&gap),
                        "magazine reload and next aim: {ticks:?}"
                    );
                } else if ((i % capacity as usize) + 1).is_multiple_of(3) {
                    assert!(
                        (3..=30).contains(&gap),
                        "bounded aim between bursts: {ticks:?}"
                    );
                    pauses.insert(gap);
                } else {
                    assert!(
                        (3..=4).contains(&gap),
                        "rapid shots within burst: {ticks:?}"
                    );
                }
            }
            assert!(
                pauses.len() > 3,
                "each gun samples fresh aim delays: {pauses:?}"
            );
        }
        let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
        for digest in digests {
            replay.step();
            assert_eq!(replay.digest(), digest);
        }
    }
}

#[test]
fn orders_preserve_partial_magazines_and_finite_ammo_counts_actual_rounds() {
    for (kind, weapon, index) in [("test_tank", "hmg", 1), ("test_rifle", "rifle", 0)] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                { "side": "blue", "kind": kind, "position": [100, 300] },
                { "side": "red", "kind": "test_tank", "position": [1100, 550], "engagement": "return_fire_only" }
            ]),
            json!([]),
            json!([]),
        );
        let def = setup.rules.weapons.get_mut(weapon).unwrap();
        def.magazine = Some(contract::weapons::Magazine {
            rounds: 3,
            shot_interval_s: 0.1,
            burst: None,
        });
        def.reload_s = 1.0;
        def.ammo = contract::weapons::AmmoCapacity::Rounds(5);
        def.round_cost = Some(1);
        def.resupply_rounds_per_s = Some(1.0);
        let mut b = Battle::new(&setup, 5);
        let attack = || Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [500.0, 300.0, 0.0],
            },
        };
        common::order(&mut b, Side::Blue, 1, attack());
        let mut seen = BTreeSet::new();
        let mut ticks = Vec::new();
        let mut interrupted = false;
        for _ in 0..300 {
            b.step();
            for (p, r) in b.rounds() {
                if r.unit == UnitId(0) && weapon_name(&b, r.weapon) == weapon && seen.insert(p.id) {
                    ticks.push(b.tick());
                }
            }
            if ticks.len() == 2 && !interrupted {
                interrupted = true;
                common::order(
                    &mut b,
                    Side::Blue,
                    2,
                    Order::Stop {
                        units: vec![UnitId(0)],
                    },
                );
                common::order(&mut b, Side::Blue, 3, attack());
            }
        }
        assert_eq!(
            ticks.len(),
            5,
            "finite ammunition is actual rounds: {ticks:?}"
        );
        if weapon == "hmg" {
            assert!(
                ticks[3] - ticks[2] >= 30,
                "orders cannot refill the partial magazine: {ticks:?}"
            );
            assert!(
                ticks[4] - ticks[3] <= 4,
                "next magazine fires rapidly: {ticks:?}"
            );
        }
        assert_eq!(mount(&b, Side::Blue, 0, index).ammo, vec![Some(0)]);
    }
}

#[test]
fn zero_reload_rifles_keep_loaded_readiness_across_magazines() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [1100, 550], "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    let weapon = setup.rules.weapons.get_mut("rifle").unwrap();
    weapon.reload_s = 0.0;
    weapon.magazine = Some(contract::weapons::Magazine {
        rounds: 3,
        shot_interval_s: 0.1,
        burst: Some(contract::weapons::Burst {
            rounds: 3,
            aim_max_s: 1.0,
        }),
    });
    let mut b = Battle::new(&setup, 5);
    common::order(
        &mut b,
        Side::Blue,
        1,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [500.0, 300.0, 0.0],
            },
        },
    );
    let mut seen = BTreeSet::new();
    let mut shots = std::collections::BTreeMap::<_, Vec<_>>::new();
    for _ in 0..ticks(15.0) {
        b.step();
        let unit = b.unit(UnitId(0)).unwrap();
        assert!(
            unit.mounts[0]
                .cycles
                .iter()
                .all(|c| c.loaded.is_some() && c.reload.is_none()),
            "every rifle stays loaded, including the magazine boundary"
        );
        for (p, r) in b.rounds() {
            if r.unit == UnitId(0) && weapon_name(&b, r.weapon) == "rifle" && seen.insert(p.id) {
                shots
                    .entry(p.shooter.unwrap().body.0)
                    .or_default()
                    .push(b.tick());
            }
        }
    }
    assert_eq!(shots.len(), 8);
    for fired in shots.values() {
        assert!(fired.len() > 30);
        assert!(
            fired.windows(2).all(|w| w[1] - w[0] <= ticks(1.0)),
            "no magazine reload pause"
        );
    }
}

#[test]
fn an_idle_gun_tops_up_its_partial_magazine_without_spending_ammunition() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([{ "side": "blue", "kind": "test_tank", "position": [100, 300] }]),
        json!([]),
        json!([]),
    );
    let gun = setup.rules.weapons.get_mut("hmg").unwrap();
    gun.magazine = Some(contract::weapons::Magazine {
        rounds: 5,
        shot_interval_s: 0.1,
        burst: None,
    });
    gun.reload_s = 2.0;
    gun.aim_s = 0.1;
    gun.ammo = contract::weapons::AmmoCapacity::Rounds(12);
    gun.round_cost = Some(1);
    gun.resupply_rounds_per_s = Some(1.0);
    let mut b = Battle::new(&setup, 5);
    let mut commander = Commander::new();
    let attack = || Order::Attack {
        units: vec![UnitId(0)],
        target: TargetRef::Ground {
            point: [500.0, 300.0, 0.0],
        },
    };
    commander.ok(&mut b, Side::Blue, attack());
    for _ in 0..ticks(2.0) {
        b.step();
        if own(&b, Side::Blue, 0).weapon_poses[1].shots == 3 {
            break;
        }
    }
    assert_eq!(own(&b, Side::Blue, 0).weapon_poses[1].shots, 3);
    commander.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    run(&mut b, ticks(0.5));
    let partial = mount(&b, Side::Blue, 0, 1);
    assert!(
        partial.reloading.is_some() && partial.reload > 0.0,
        "a partial magazine starts reloading during the lull: {partial:?}"
    );
    assert!(
        partial.loaded.is_none(),
        "a top-up publishes reload readiness"
    );
    assert_eq!(partial.ammo, vec![Some(9)], "topping up spends no rounds");
    let (reload_tick, reload_digest) = (b.tick(), b.digest());
    run(&mut b, ticks(2.0));
    assert!(mount(&b, Side::Blue, 0, 1).loaded.is_some());
    commander.ok(&mut b, Side::Blue, attack());
    let mut shots_before_reload = 0;
    for _ in 0..ticks(1.0) {
        b.step();
        shots_before_reload = own(&b, Side::Blue, 0).weapon_poses[1].shots - 3;
        if mount(&b, Side::Blue, 0, 1).reloading.is_some() {
            break;
        }
    }
    assert_eq!(
        shots_before_reload, 5,
        "the next engagement starts with a full magazine"
    );
    assert_eq!(mount(&b, Side::Blue, 0, 1).ammo, vec![Some(4)]);
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for _ in 0..b.tick() {
        replay.step();
        if replay.tick() == reload_tick {
            assert_eq!(
                replay.digest(),
                reload_digest,
                "replay preserves the partial reload"
            );
        }
    }
    assert_eq!(replay.digest(), b.digest());
}

#[test]
fn an_enemy_entering_range_cancels_a_top_up_but_not_an_empty_reload() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [430, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    for weapon in ["tank_ap", "tank_he"] {
        setup.rules.weapons.get_mut(weapon).unwrap().ammo =
            contract::weapons::AmmoCapacity::Rounds(0);
    }
    let gun = setup.rules.weapons.get_mut("hmg").unwrap();
    gun.magazine = Some(contract::weapons::Magazine {
        rounds: 5,
        shot_interval_s: 0.1,
        burst: None,
    });
    gun.reload_s = 10.0;
    gun.aim_s = 0.1;
    common::ground_reach_spread(gun, 300.0, 8.0);
    gun.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    let mut commander = Commander::new();
    commander.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [300.0, 300.0, 0.0],
            },
        },
    );
    for _ in 0..ticks(2.0) {
        b.step();
        if own(&b, Side::Blue, 0).weapon_poses[1].shots == 3 {
            break;
        }
    }
    assert_eq!(own(&b, Side::Blue, 0).weapon_poses[1].shots, 3);
    commander.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    run(&mut b, ticks(0.5));
    assert!(
        mount(&b, Side::Blue, 0, 1).reloading.is_some(),
        "the out-of-range enemy does not stop the top-up"
    );
    let began = b.tick();
    commander.ok(
        &mut b,
        Side::Red,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [350.0, 300.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    for _ in 0..ticks(8.0) {
        b.step();
        if own(&b, Side::Blue, 0).weapon_poses[1].shots > 3 {
            break;
        }
    }
    assert_eq!(
        own(&b, Side::Blue, 0).weapon_poses[1].shots,
        4,
        "the incoming enemy interrupts the reload before it completes"
    );
    let interrupted = mount(&b, Side::Blue, 0, 1);
    assert!(interrupted.loaded.is_some() && interrupted.reloading.is_none());
    run(&mut b, ticks(1.0));
    assert_eq!(
        own(&b, Side::Blue, 0).weapon_poses[1].shots,
        5,
        "canceling leaves just the two rounds from the old magazine"
    );
    assert!(mount(&b, Side::Blue, 0, 1).reloading.is_some());
    assert!(b.tick() - began < ticks(10.0));
    run(&mut b, ticks(10.0));
    assert!(
        own(&b, Side::Blue, 0).weapon_poses[1].shots > 5,
        "the empty reload completes while the enemy remains in range"
    );
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for _ in 0..b.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), b.digest());
}
