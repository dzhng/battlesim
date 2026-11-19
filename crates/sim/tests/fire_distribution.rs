//! Functional tests for automatic target distribution.

use contract::command::{
    CommandEnvelope, Engagement, MoveDirection, Order, RoutePolicy, TargetRef,
};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, MoveState};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::weapons::Target;

use crate::common;

fn map() -> Value {
    json!({
        "size": [1200, 600],
        "fog_cell_m": 8,
        "height_grid_m": 4,
        "slope_cutoff_deg": 35
    })
}

fn rules() -> Value {
    let mut rules = common::scenario_rules();
    // The spread tests own their weights: one engaging unit outweighs the
    // widest seeded draw, so the expected choices hold for any seed.
    rules["targeting"] = json!({"engaged_distance_penalty": 0.5, "distance_spread": 0.1});
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({"sensors": {"ground_m": 800, "low_air_m": 800, "high_air_m": 800}}),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({"body": {"hull": {"hp": 1.0e9}}}),
    );
    rules
}

fn battle(units: Value, rules: Value, seed: u64) -> Battle {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map(), "rules": rules, "units": units, "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, seed)
}

fn lock_target(battle: &Battle, unit: u32) -> Option<UnitId> {
    battle.unit(UnitId(unit)).and_then(|u| {
        u.mounts.iter().find_map(|m| match m.lock.as_ref()?.target {
            Target::Unit(id) => Some(id),
            _ => None,
        })
    })
}

fn order(battle: &mut Battle, side: Side, seq: u64, order: Order) {
    let ack = battle.accept(CommandEnvelope {
        side,
        player: None,
        seq,
        queued: false,
        order,
    });
    assert_eq!(ack.error, None);
}

fn fanout(seed: u64) -> Battle {
    battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 302]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "red", "kind": "test_supply", "position": [350, 280]},
            {"side": "red", "kind": "test_supply", "position": [350, 320]}
        ]),
        rules(),
        seed,
    )
}

#[test]
fn identical_shooters_fan_out_across_nearly_equal_targets() {
    for seed in 1..8 {
        let mut battle = fanout(seed);
        battle.step();
        let first = lock_target(&battle, 0).expect("the first shooter locks a target");
        let second = lock_target(&battle, 1).expect("the second shooter locks a target");
        assert_ne!(first, second, "seed {seed}: both shooters took one target");
    }
}

#[test]
fn an_engaged_nearer_target_loses_to_a_bit_farther_one() {
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 302]},
            {"side": "red", "kind": "test_supply", "position": [350, 280]},
            {"side": "red", "kind": "test_supply", "position": [400, 320]}
        ]),
        rules(),
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(2)));
    assert_eq!(lock_target(&battle, 1), Some(UnitId(3)));
}

#[test]
fn a_far_target_still_loses_when_distance_exceeds_penalty_and_spread() {
    let mut rules = rules();
    common::json_ground_reach(&mut rules["weapons"]["hmg"], 800.0);
    let penalty = rules["targeting"]["engaged_distance_penalty"]
        .as_f64()
        .unwrap();
    let spread = rules["targeting"]["distance_spread"].as_f64().unwrap();
    let near = 250.0;
    let far = near * (1.0 + penalty) * (1.0 + spread) / (1.0 - spread) + 1.0;
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "blue", "kind": "test_jeep", "position": [100, 320]},
            {"side": "red", "kind": "test_supply", "position": [350, 300]},
            {"side": "red", "kind": "test_supply", "position": [100.0 + far, 300]}
        ]),
        rules,
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(2)));
    assert_eq!(lock_target(&battle, 1), Some(UnitId(2)));
}

#[test]
fn purchase_cost_does_not_out_rank_nearer_distance() {
    let mut rules = rules();
    rules["catalog"].as_array_mut().unwrap().push(json!({
        "units": {"expensive_target": {"extends": "test_supply", "cost": 100000}}
    }));
    let mut battle = battle(
        json!([
            {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
            {"side": "red", "kind": "test_supply", "position": [350, 300]},
            {"side": "red", "kind": "expensive_target", "position": [500, 300]}
        ]),
        rules,
        7,
    );
    battle.step();
    assert_eq!(lock_target(&battle, 0), Some(UnitId(1)));
}

#[test]
fn a_fresh_choice_is_stable_across_replays_and_ticks() {
    let units = json!([
        {"side": "blue", "kind": "test_jeep", "position": [100, 300]},
        {"side": "red", "kind": "test_supply", "position": [350, 280]},
        {"side": "red", "kind": "test_supply", "position": [350, 320]}
    ]);
    let mut first = battle(units.clone(), rules(), 19);
    let mut second = battle(units, rules(), 19);
    let mut first_targets = Vec::new();
    let mut second_targets = Vec::new();
    for _ in 0..20 {
        first.step();
        second.step();
        first_targets.push(lock_target(&first, 0));
        second_targets.push(lock_target(&second, 0));
    }
    assert_eq!(first_targets, second_targets);
    assert!(first_targets
        .iter()
        .all(|target| *target == first_targets[0]));
}

// Isolate distribution from scatter, armour faces, retaliation and the
// secondary guns. Distances increase in id order so earlier claims arrive first.
fn coordination_rules(row: &str) -> Value {
    let mut rules = rules();
    rules["weapons"][row]["avoids_overkill"] = json!(true);
    rules["weapons"][row]["aim_s"] = json!(1.0);
    rules["weapons"][row]["targets"]["ground"]["scatter_mrad"] = json!(0.0);
    rules["targeting"] = json!({"engaged_distance_penalty": 0, "distance_spread": 0});
    for gun in ["rifle", "hmg"] {
        common::json_ground_reach(&mut rules["weapons"][gun], 50.0);
    }
    let max_hp = damage(&rules, row) * 10.0;
    for kind in ["test_tank", "test_jeep", "test_supply"] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"body": {"hull": {"hp": max_hp}}}),
        );
    }
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "sensors": {"ground_m": 900, "low_air_m": 900, "high_air_m": 900},
            "body": {"hull": {"armor": {
                "front": 1, "side": 1, "rear": 1, "roof": 1,
                "ricochet": {"front": 0, "side": 0, "rear": 0, "roof": 0}
            }}}
        }),
    );
    rules
}

fn damage(rules: &Value, row: &str) -> f64 {
    rules["weapons"][row]["damage"].as_f64().unwrap()
}

fn mount<'a>(battle: &'a Battle, unit: u32, row: &str) -> &'a sim::weapons::Mount {
    let unit = battle.unit(UnitId(unit)).unwrap();
    unit.mounts
        .iter()
        .find(|mount| {
            battle.arsenal().specs(unit.kind)[mount.spec]
                .kinds
                .iter()
                .any(|&k| battle.arsenal().weapons[k].id == row)
        })
        .expect("unit carries the weapon")
}

fn until(battle: &mut Battle, what: &str, predicate: impl Fn(&Battle) -> bool) {
    let end = battle.tick() + 300;
    while !predicate(battle) && battle.tick() < end {
        battle.step();
    }
    assert!(predicate(battle), "{what}, tick {}", battle.tick());
}

fn assert_held(battle: &Battle, unit: u32, row: &str, target: u32) {
    let mount = mount(battle, unit, row);
    assert_eq!(mount.reason, ActionReason::HoldingFire);
    let lock = mount.lock.as_ref().expect("held mount retains its lock");
    assert_eq!(lock.target, Target::Unit(UnitId(target)));
    assert!(lock.engaging);
    let aim = battle.rules().weapons[row].aim_s;
    assert!(mount.cycles.iter().any(|cycle| cycle.aim >= aim));
}

#[test]
fn teams_hold_then_switch_to_a_second_tank_and_account_for_trophy() {
    for (trophy, second) in [(false, false), (false, true), (true, false)] {
        let mut rules = coordination_rules("atgm");
        let hp = damage(&rules, "atgm") * if trophy { 0.75 } else { 1.5 };
        if trophy {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                "test_tank",
                json!({
                    "capabilities": {"active_protection": {
                        "name": "Trophy", "icon": "trophy", "capacity": 1,
                        "cooldown_s": 3, "standoff_m": 8, "service_s": 10,
                        "stock_per_charge": 20
                    }}
                }),
            );
        }
        let mut units = json!([
            {"side": "blue", "kind": "test_at", "position": [200, 280]},
            {"side": "blue", "kind": "test_at", "position": [150, 320]},
            {"side": "blue", "kind": "test_at", "position": [100, 360]},
            {"side": "red", "kind": "test_tank", "position": [600, 300],
             "engagement": "return_fire_only", "condition": {"hp": hp}}
        ]);
        if second {
            // Farther than the first tank for every team at initial acquisition.
            units.as_array_mut().unwrap().push(json!({
                "side": "red", "kind": "test_tank", "position": [750, 500],
                "engagement": "return_fire_only", "condition": {"hp": hp}
            }));
        }
        let mut b = battle(units, rules, 31);
        if second {
            until(&mut b, "third team launches at second tank", |b| {
                b.rounds()
                    .any(|(_, r)| r.unit == UnitId(2) && r.aimed_at == Some(UnitId(4)))
            });
            let claims: Vec<_> = b
                .rounds()
                .filter(|(_, r)| b.arsenal().weapons[r.weapon].id == "atgm")
                .map(|(_, r)| (r.unit, r.aimed_at))
                .collect();
            assert_eq!(
                claims,
                vec![
                    (UnitId(0), Some(UnitId(3))),
                    (UnitId(1), Some(UnitId(3))),
                    (UnitId(2), Some(UnitId(4))),
                ]
            );
        } else {
            until(&mut b, "third team holds", |b| {
                mount(b, 2, "atgm").reason == ActionReason::HoldingFire
            });
            assert_eq!(mount(&b, 0, "atgm").shots, 1);
            assert_eq!(mount(&b, 1, "atgm").shots, 1);
            assert_eq!(mount(&b, 2, "atgm").shots, 0);
            assert_held(&b, 2, "atgm", 3);
            let team = b.unit(UnitId(2)).unwrap();
            assert!(sim::weapons::engaged(team));
            let held_mount = mount(&b, 2, "atgm");
            let gunner = team
                .members
                .iter()
                .find(|s| Some(s.id) == held_mount.operator)
                .unwrap();
            assert_eq!(gunner.active_mount, Some(held_mount.spec));
            // Staying on the lock keeps the AT gun out, without a posture reset.
            for _ in 0..5 {
                b.step();
                assert_held(&b, 2, "atgm", 3);
            }
        }
    }
}

#[test]
fn a_held_attack_move_keeps_aim_and_follows_up_when_guidance_is_released() {
    let rules = coordination_rules("atgm");
    let hp = damage(&rules, "atgm") * 0.75;
    let mut b = battle(
        json!([
            {"side": "blue", "kind": "test_at", "position": [200, 280]},
            {"side": "blue", "kind": "test_at", "position": [100, 340]},
            {"side": "red", "kind": "test_jeep", "position": [600, 300],
             "engagement": "return_fire_only", "condition": {"hp": hp}}
        ]),
        rules,
        43,
    );
    until(&mut b, "second launcher holds with full aim", |b| {
        mount(b, 1, "atgm").reason == ActionReason::HoldingFire
    });
    assert_held(&b, 1, "atgm", 2);
    assert_eq!(mount(&b, 0, "atgm").shots, 1);
    assert_eq!(mount(&b, 1, "atgm").shots, 0);
    let missile = mount(&b, 0, "atgm").support.unwrap().projectile;
    order(
        &mut b,
        Side::Blue,
        1,
        Order::AttackMove {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [950.0, 340.0],
        },
    );
    // Planning is also stationary; once admitted the move reports its halt.
    until(&mut b, "attack-move halts on the held engagement", |b| {
        b.observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(1))
            .unwrap()
            .state
            == MoveState::Halted
    });
    let position = b.unit(UnitId(1)).unwrap().position;
    for _ in 0..5 {
        b.step();
        assert_held(&b, 1, "atgm", 2);
        assert_eq!(b.unit(UnitId(1)).unwrap().position, position);
        let own = b
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(1))
            .unwrap();
        assert_eq!(own.state, MoveState::Halted);
    }
    order(
        &mut b,
        Side::Blue,
        2,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [200.0, 200.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
    until(
        &mut b,
        "moving launcher releases its live wire-guided missile",
        |b| {
            b.rounds()
                .any(|(p, _)| p.id == missile && p.guidance.is_some_and(|g| !g.supported))
        },
    );
    let released = b.tick();
    until(&mut b, "held team launches the follow-up", |b| {
        mount(b, 1, "atgm").shots == 1
    });
    let aim_ticks = b.rules().weapons["atgm"].aim_s * b.rules().tick_hz as f64;
    assert!(((b.tick() - released) as f64) < aim_ticks);
    assert_eq!(
        mount(&b, 1, "atgm").lock.as_ref().unwrap().target,
        Target::Unit(UnitId(2))
    );
}

#[test]
fn same_tick_ap_claims_hold_without_swapping_he_but_an_order_fires() {
    let mut rules = coordination_rules("tank_ap");
    rules["weapons"]["tank_ap"]["speed_mps"] = json!(150.0);
    rules["weapons"]["tank_ap"]["gravity_scale"] = json!(0.1);
    let hp = damage(&rules, "tank_ap") * 1.5;
    let spent = json!({
        "tank_ap": rules["weapons"]["tank_ap"]["ammo"],
        "tank_he": rules["weapons"]["tank_he"]["ammo"]
    });
    let mut b = battle(
        json!([
            {"side": "blue", "kind": "test_tank", "position": [200, 260]},
            {"side": "blue", "kind": "test_tank", "position": [150, 300]},
            {"side": "blue", "kind": "test_tank", "position": [100, 340]},
            {"side": "red", "kind": "test_tank", "position": [600, 300],
             "engagement": "return_fire_only", "condition": {"hp": hp, "spent": spent}}
        ]),
        rules,
        41,
    );
    let mut fired = Vec::new();
    for _ in 0..300 {
        let before = [0, 1, 2].map(|unit| mount(&b, unit, "tank_ap").shots);
        b.step();
        for (unit, shots) in before.into_iter().enumerate() {
            if mount(&b, unit as u32, "tank_ap").shots > shots {
                fired.push((b.tick(), UnitId(unit as u32)));
            }
        }
        if !fired.is_empty() {
            break;
        }
    }
    let launch_tick = b.tick();
    assert_eq!(
        fired,
        vec![(launch_tick, UnitId(0)), (launch_tick, UnitId(1))]
    );
    assert_eq!(mount(&b, 0, "tank_ap").shots, 1);
    assert_eq!(mount(&b, 1, "tank_ap").shots, 1);
    assert_eq!(mount(&b, 2, "tank_ap").shots, 0);
    let launches: Vec<_> = b.rounds().map(|(_, r)| (r.unit, r.aimed_at)).collect();
    assert_eq!(
        launches,
        vec![(UnitId(0), Some(UnitId(3))), (UnitId(1), Some(UnitId(3)))]
    );
    assert_held(&b, 2, "tank_ap", 3);
    let ammo = mount(&b, 2, "tank_ap").ammo.clone();
    for _ in 0..5 {
        b.step();
        assert_held(&b, 2, "tank_ap", 3);
        assert_eq!(
            mount(&b, 2, "tank_ap").ammo,
            ammo,
            "neither AP nor HE spent during hold"
        );
        assert_eq!(mount(&b, 2, "tank_ap").shots, 0);
    }
    let observed = b.observe(Side::Blue).identified.first().unwrap().id;
    order(
        &mut b,
        Side::Blue,
        1,
        Order::Attack {
            units: vec![UnitId(2)],
            target: TargetRef::Identified { id: observed },
        },
    );
    b.step();
    assert_eq!(b.tick(), launch_tick + 6);
    assert_eq!(
        mount(&b, 2, "tank_ap").shots,
        1,
        "explicit order bypasses the hold"
    );
    assert!(b.rounds().any(|(_, r)| r.unit == UnitId(2)
        && r.aimed_at == Some(UnitId(3))
        && b.arsenal().weapons[r.weapon].id == "tank_ap"));
}

#[test]
fn a_missed_shell_passing_its_arrival_releases_the_held_follow_up() {
    let mut rules = coordination_rules("tank_ap");
    rules["weapons"]["tank_ap"]["speed_mps"] = json!(150.0);
    rules["weapons"]["tank_ap"]["gravity_scale"] = json!(0.1);
    rules["weapons"]["tank_ap"]["reload_s"] = json!(0.1);
    let hp = damage(&rules, "tank_ap") * 0.75;
    let mut b = battle(
        json!([
            {"side": "blue", "kind": "test_tank", "position": [100, 300]},
            {"side": "red", "kind": "test_jeep", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2,
             "engagement": "return_fire_only", "condition": {"hp": hp}}
        ]),
        rules,
        47,
    );
    until(&mut b, "first AP shell launches", |b| {
        mount(b, 0, "tank_ap").shots == 1
    });
    let (shell, expected) = b
        .rounds()
        .find(|(_, r)| r.unit == UnitId(0))
        .map(|(p, r)| (p.id, r.expected_arrival_tick))
        .unwrap();
    order(
        &mut b,
        Side::Red,
        1,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [600.0, 540.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
    until(&mut b, "reloaded gun holds behind its shell", |b| {
        mount(b, 0, "tank_ap").reason == ActionReason::HoldingFire
    });
    assert_held(&b, 0, "tank_ap", 1);
    while (b.tick() + 1) as f64 <= expected {
        b.step();
        assert_eq!(mount(&b, 0, "tank_ap").shots, 1);
        assert_held(&b, 0, "tank_ap", 1);
    }
    b.step();
    assert!(b.tick() as f64 > expected);
    assert_eq!(
        mount(&b, 0, "tank_ap").shots,
        2,
        "full aim permits the immediate follow-up"
    );
    assert_eq!(
        b.unit(UnitId(1)).unwrap().hp,
        hp,
        "moving target survived the first shot"
    );
    assert!(
        b.rounds().any(|(p, _)| p.id == shell),
        "the missed shell still flies; expiration of the claim freed the gun"
    );
}

#[test]
fn unflagged_guns_keep_firing_and_a_default_gun_keeps_plinking() {
    for (kind, row, target_kind) in [
        ("test_gun_jeep", "autocannon", "test_supply"),
        ("test_jeep", "hmg", "test_supply"),
        ("test_tank", "tank_he", "test_supply"),
        ("test_rifle", "rifle", "test_supply"),
        ("test_jeep", "hmg", "test_tank"),
    ] {
        let mut rules = coordination_rules("atgm");
        common::json_ground_reach(&mut rules["weapons"][row], 800.0);
        rules["weapons"][row]["avoids_overkill"] = json!(false);
        rules["weapons"][row]["aim_s"] = json!(0.05);
        if row == "rifle" {
            // Give the AT guards short-range carbines so only the section
            // under test carries these long-range rifles.
            sim::fixtures::patch_catalog(
                &mut rules,
                "soldiers",
                "test_at_rifleman",
                json!({
                    "mounts": [{"id": "rifles", "weapons": ["carbine"]}]
                }),
            );
        }
        rules["weapons"][row]["magazine"] = json!({
            "rounds": 30, "shot_interval_s": 0.05, "burst": null
        });
        // HE owns this tank's cannon, so the test exercises HE rather than AP.
        if row == "tank_he" {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                "test_tank",
                json!({
                    "mounts": [{"id": "cannon", "weapons": ["tank_he"]}]
                }),
            );
        }
        if target_kind == "test_tank" {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                "test_tank",
                json!({
                    "body": {"hull": {"armor": {"front": 100, "side": 100, "rear": 100, "roof": 100}}}
                }),
            );
        }
        let hp = damage(&rules, "atgm") * 0.75;
        let spent = json!({
            "tank_ap": rules["weapons"]["tank_ap"]["ammo"],
            "tank_he": rules["weapons"]["tank_he"]["ammo"]
        });
        let mut b = battle(
            json!([
                {"side": "blue", "kind": "test_at", "position": [200, 280]},
                {"side": "blue", "kind": "test_at", "position": [150, 320]},
                {"side": "blue", "kind": kind, "position": [100, 380], "engagement": "return_fire_only"},
                {"side": "red", "kind": target_kind, "position": [600, 300],
                 "engagement": "return_fire_only", "condition": {"hp": hp, "spent": spent}}
            ]),
            rules,
            59,
        );
        until(
            &mut b,
            &format!("second flagged launcher holds before {row} vs {target_kind}"),
            |b| mount(b, 1, "atgm").reason == ActionReason::HoldingFire,
        );
        assert_eq!(mount(&b, 0, "atgm").shots, 1);
        assert_eq!(mount(&b, 1, "atgm").shots, 0);
        let (missile, arrival) = b
            .rounds()
            .find(|(_, r)| r.unit == UnitId(0))
            .map(|(p, r)| (p.id, r.expected_arrival_tick))
            .unwrap();
        // Start the gun late enough that the existing claim lands before its
        // own shot. This also rules out merely testing the fast-shot exemption.
        while (b.tick() as f64) < arrival - 12.0 {
            b.step();
        }
        assert_held(&b, 1, "atgm", 3);
        assert_eq!(mount(&b, 2, row).shots, 0);
        if target_kind == "test_tank" {
            assert!(!sim::weapons::can_damage(
                &b.rules().weapons[row],
                b.unit(UnitId(3)).unwrap().armor(b.rules()),
                b.rules()
            ));
        }
        order(
            &mut b,
            Side::Blue,
            1,
            Order::SetEngagement {
                units: vec![UnitId(2)],
                policy: Engagement::FireAtWill,
            },
        );
        until(
            &mut b,
            "unflagged gun fires while flagged missile still flies",
            |b| mount(b, 2, row).shots > 0,
        );
        assert!(
            b.rounds().any(|(p, _)| p.id == missile),
            "{row}: missile is still in flight"
        );
        assert!(
            b.rounds().any(|(_, r)| r.unit == UnitId(2)
                && b.arsenal().weapons[r.weapon].id == row
                && r.expected_arrival_tick >= arrival),
            "{row}: the committed missile arrives before this gun's shot"
        );
        let shots = mount(&b, 2, row).shots;
        until(&mut b, "unflagged gun keeps firing", |b| {
            mount(b, 2, row).shots > shots
        });
        assert!(
            b.rounds().any(|(p, _)| p.id == missile),
            "{row}: continued fire precedes the impact"
        );
        assert_eq!(
            mount(&b, 2, row).lock.as_ref().unwrap().target,
            Target::Unit(UnitId(3))
        );
    }
}

#[test]
fn a_squad_takes_one_counted_hit_per_soldier() {
    // Two soldiers, each well short of one AP round's damage, together short
    // of one round too: a direct hit strikes one soldier, so the squad needs
    // two rounds and only the third tank holds.
    let mut rules = coordination_rules("tank_ap");
    rules["weapons"]["tank_ap"]["speed_mps"] = json!(150.0);
    rules["weapons"]["tank_ap"]["gravity_scale"] = json!(0.1);
    let soldier_hp = damage(&rules, "tank_ap") * 0.4;
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({"body": {"squad": {"slots": ["test_rifleman", "test_rifleman"]}}}),
    );
    let gunner = json!({"spent": {"tank_he": rules["weapons"]["tank_he"]["ammo"]}});
    let mut b = battle(
        json!([
            {"side": "blue", "kind": "test_tank", "position": [200, 260], "condition": gunner},
            {"side": "blue", "kind": "test_tank", "position": [150, 300], "condition": gunner},
            {"side": "blue", "kind": "test_tank", "position": [100, 340], "condition": gunner},
            {"side": "red", "kind": "test_rifle", "position": [500, 300],
             "engagement": "return_fire_only",
             "condition": {"soldier_hp": soldier_hp}}
        ]),
        rules,
        47,
    );
    until(&mut b, "third tank holds", |b| {
        mount(b, 2, "tank_ap").reason == ActionReason::HoldingFire
    });
    assert_eq!(mount(&b, 0, "tank_ap").shots, 1);
    assert_eq!(mount(&b, 1, "tank_ap").shots, 1);
    assert_eq!(mount(&b, 2, "tank_ap").shots, 0);
    assert_held(&b, 2, "tank_ap", 3);
}
