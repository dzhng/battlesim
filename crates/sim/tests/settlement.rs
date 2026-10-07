use crate::common;
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use serde_json::json;
use sim::battle::{Battle, Round};
use sim::damage::{self, DamageContext};
use sim::flight::{BodyId, FlightEvent, Impact, ProjectileId, Struck};
use sim::math::v3;
use std::collections::BTreeMap;

#[test]
fn lethal_damage_keeps_the_rounds_dead_shooter() {
    let setup = common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([
            {"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"test_tank","position":[200,300]}
        ]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let mut units = vec![
        battle.unit(UnitId(0)).unwrap().clone(),
        battle.unit(UnitId(1)).unwrap().clone(),
    ];
    units[0].hp = 0.0;
    units[1].hp = 1.0;
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "tank_ap")
        .unwrap();
    let projectile = ProjectileId(4);
    let events = vec![FlightEvent::Impact(Impact {
        projectile,
        struck: Struck::Body(BodyId(sim::weapons::VEHICLE_BODY_BASE + 1)),
        point: v3(200.0, 300.0, 1.0),
        normal: v3(-1.0, 0.0, 0.0),
        velocity: v3(100.0, 0.0, 0.0),
        time: 0.0,
        bounces: 0,
        pose: None,
        detonated: false,
    })];
    let rounds = BTreeMap::from([(
        projectile,
        Round {
            weapon,
            unit: UnitId(0),
            side: Side::Blue,
        },
    )]);
    let outcome = damage::resolve(
        &DamageContext {
            world: battle.world(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: battle.rules(),
            tick: 1,
        },
        &events,
        &rounds,
        &mut BTreeMap::new(),
        &mut units,
        &mut Rng::new(7),
    );
    assert!(!units[1].alive());
    assert_eq!(
        outcome.destroyed,
        vec![damage::UnitDeath {
            victim: UnitId(1),
            source: Some(damage::LethalSource {
                unit: UnitId(0),
                side: Side::Blue
            })
        }]
    );
}

#[test]
fn direct_and_blast_structural_damage_keep_the_same_round_source() {
    let setup = common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[
            {"kind":"sandbags","center":[200,300],"yaw":0,"half_extents":[0.5,2,0.5]},
            {"kind":"crate","center":[204,304],"yaw":0,"half_extents":[0.5,0.5,0.5]}
        ]})
        .to_string(),
        json!([{ "side":"blue","kind":"test_tank","position":[100,300]}]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let mut units = vec![battle.unit(UnitId(0)).unwrap().clone()];
    units[0].hp = 0.0;
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "tank_he")
        .unwrap();
    let projectile = ProjectileId(4);
    let events = [FlightEvent::Impact(Impact {
        projectile,
        struck: Struck::Prop(0),
        point: v3(199.5, 300.0, 0.5),
        normal: v3(-1.0, 0.0, 0.0),
        velocity: v3(100.0, 0.0, 0.0),
        time: 0.0,
        bounces: 0,
        pose: None,
        detonated: true,
    })];
    let rounds = BTreeMap::from([(
        projectile,
        Round {
            weapon,
            unit: UnitId(0),
            side: Side::Blue,
        },
    )]);
    let outcome = damage::resolve(
        &DamageContext {
            world: battle.world(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: battle.rules(),
            tick: 1,
        },
        &events,
        &rounds,
        &mut BTreeMap::new(),
        &mut units,
        &mut Rng::new(7),
    );
    let source = Some(damage::LethalSource {
        unit: UnitId(0),
        side: Side::Blue,
    });
    assert!(outcome
        .structural
        .iter()
        .any(|hit| hit.prop == 0 && hit.amount > 0.0));
    assert!(outcome
        .structural
        .iter()
        .any(|hit| hit.prop == 1 && hit.amount > 0.0));
    for hit in outcome.structural {
        assert_eq!(hit.source, source);
    }
}

fn economic(victim: u32, side: Side, price: u32) -> sim::settlement::EconomicDeath {
    sim::settlement::EconomicDeath {
        victim: UnitId(victim),
        side,
        price,
        source: Some(damage::LethalSource {
            unit: UnitId(99),
            side: if side == Side::Blue {
                Side::Red
            } else {
                Side::Blue
            },
        }),
        supply: false,
    }
}

#[test]
fn hostile_death_rewards_once_and_builds_only_original_price_bounty() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    let deaths = [economic(1, Side::Red, 200)];
    settlement.settle(&deaths, 1000, &mut wallets);
    settlement.settle(&deaths, 1000, &mut wallets);
    assert_eq!(wallets, [50_000_000, 0]);
    assert_eq!(settlement.pools(), [100_000_000, 0]);
}

#[test]
fn pass_through_structural_hit_keeps_round_source() {
    let setup = common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[{"kind":"sandbags","center":[200,300],"yaw":0,"half_extents":[0.5,2,0.5]}]})
        .to_string(),
        json!([{ "side":"blue","kind":"test_tank","position":[100,300]}]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "tank_he")
        .unwrap();
    let projectile = ProjectileId(4);
    let events = [FlightEvent::Pass(sim::flight::Pass {
        projectile,
        prop: 0,
        point: v3(200.0, 300.0, 0.5),
        along: v3(1.0, 0.0, 0.0),
        time: 0.0,
    })];
    let rounds = BTreeMap::from([(
        projectile,
        Round {
            weapon,
            unit: UnitId(0),
            side: Side::Blue,
        },
    )]);
    let outcome = damage::resolve(
        &DamageContext {
            world: battle.world(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: battle.rules(),
            tick: 1,
        },
        &events,
        &rounds,
        &mut BTreeMap::new(),
        &mut [],
        &mut Rng::new(7),
    );
    assert_eq!(outcome.structural.len(), 1);
    assert_eq!(
        outcome.structural[0].source,
        Some(damage::LethalSource {
            unit: UnitId(0),
            side: Side::Blue
        })
    );
}

#[test]
fn same_tick_trades_use_snapshot_pools_in_either_order() {
    let mut a = sim::settlement::Settlement::default();
    let mut b = sim::settlement::Settlement::default();
    let mut wa = [0; 2];
    let mut wb = [0; 2];
    let seed = [economic(1, Side::Red, 200), economic(2, Side::Blue, 200)];
    a.settle(&seed, 1000, &mut wa);
    b.settle(&seed, 1000, &mut wb);
    let deaths = [economic(3, Side::Red, 200), economic(4, Side::Blue, 400)];
    a.settle(&deaths, 1000, &mut wa);
    b.settle(
        &[economic(4, Side::Blue, 400), economic(3, Side::Red, 200)],
        1000,
        &mut wb,
    );
    assert_eq!(wa, [120_000_000, 190_000_000]);
    assert_eq!(wa, wb);
    assert_eq!(a.pools(), [100_000_000, 200_000_000]);
    assert_eq!(a.pools(), b.pools());
}

#[test]
fn friendly_and_source_free_deaths_preserve_bounty_and_pay_nothing() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    settlement.settle(&[economic(1, Side::Blue, 200)], 1000, &mut wallets);
    let before = wallets;
    let mut friendly = economic(2, Side::Red, 200);
    friendly.source.as_mut().unwrap().side = Side::Red;
    let mut free = economic(3, Side::Red, 200);
    free.source = None;
    settlement.settle(&[friendly, free], 1000, &mut wallets);
    assert_eq!(wallets, before);
    assert_eq!(settlement.pools(), [0, 100_000_000]);
}

#[test]
fn unseen_purchased_kill_changes_wallet_without_enemy_identity_and_replays() {
    use contract::command::{CommandEnvelope, Order, TargetRef};
    let mut setup = crate::skirmish::setup();
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost":200,
            "roster":{"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"},
            "body":{"hull":{"hp":0.01}}, "sensors":{"ground_m":1}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.skirmish.as_mut().unwrap().sites.entries[1].center = [400.0, 110.0];
    let mut battle = Battle::new(&setup, 7);
    let send = |battle: &mut Battle, side, seq, order| {
        let ack = battle.accept(CommandEnvelope {
            side,
            seq,
            order,
            queued: false,
        });
        assert!(ack.error.is_none(), "{ack:?}");
    };
    for (side, destination) in [(Side::Blue, [400.0, 10.0]), (Side::Red, [400.0, 110.0])] {
        send(
            &mut battle,
            side,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination,
            },
        );
        send(&mut battle, side, 2, Order::Ready);
    }
    let mut digests = Vec::new();
    for _ in 0..10 {
        battle.step();
        digests.push(battle.digest());
    }
    let shooter = battle.observe(Side::Blue).own[0].id;
    assert!(battle.observe(Side::Blue).identified.is_empty());
    let before = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    let start = battle.tick();
    send(
        &mut battle,
        Side::Blue,
        3,
        Order::Attack {
            units: vec![shooter],
            target: TargetRef::Ground {
                point: [400.0, 110.0, 0.0],
            },
        },
    );
    for _ in 0..1200 {
        battle.step();
        digests.push(battle.digest());
        if battle.observe(Side::Red).own.is_empty() {
            break;
        }
    }
    assert!(
        battle.observe(Side::Red).own.is_empty(),
        "blind ground fire killed the purchased tank"
    );
    let blue = battle.observe(Side::Blue);
    assert!(
        blue.identified.is_empty(),
        "payout grants no enemy identity"
    );
    assert!(
        blue.corpses.is_empty(),
        "payout reveals no hidden casualties"
    );
    assert!(
        blue.known_props.is_empty(),
        "payout reveals no hidden wreck"
    );
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!((blue.skirmish.as_ref().unwrap().credits - before - passive - 50.0).abs() < 0.00001);
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    for digest in digests {
        replay.step();
        assert_eq!(replay.digest(), digest, "tick {}", replay.tick());
    }
}

#[test]
fn aggregate_bonus_is_capped_at_snapshot_pool_and_supply_adds_once() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    settlement.settle(&[economic(1, Side::Blue, 200)], 1000, &mut wallets);
    let mut truck = economic(2, Side::Red, 1000);
    truck.supply = true;
    settlement.settle(&[truck, economic(3, Side::Red, 1000)], 1000, &mut wallets);
    assert_eq!(wallets, [850_000_000, 50_000_000]);
    assert_eq!(settlement.pools(), [1_000_000_000, 0]);
}

#[test]
fn fractional_bonus_carries_without_losing_sub_microcredits() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    for k in 0..7 {
        settlement.settle(&[economic(k * 2, Side::Blue, 1)], 7, &mut wallets);
        settlement.settle(&[economic(k * 2 + 1, Side::Red, 1)], 7, &mut wallets);
    }
    assert_eq!(wallets[0], 2_250_000);
}

#[test]
fn squad_emits_one_death_only_when_its_last_soldier_dies() {
    let setup = common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([
            {"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"test_rifle","position":[200,300]}
        ]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let mut units = vec![
        battle.unit(UnitId(0)).unwrap().clone(),
        battle.unit(UnitId(1)).unwrap().clone(),
    ];
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "tank_ap")
        .unwrap();
    let count = units[1].members.len();
    assert!(count > 1);
    let mut total = Vec::new();
    for k in 0..count {
        units[1].members[k].hp = 0.01;
        let projectile = ProjectileId(k as u64);
        let events = [FlightEvent::Impact(Impact {
            projectile,
            struck: Struck::Body(BodyId(units[1].members[k].id)),
            point: units[1].members[k].position,
            normal: v3(-1.0, 0.0, 0.0),
            velocity: v3(100.0, 0.0, 0.0),
            time: 0.0,
            bounces: 0,
            pose: None,
            detonated: false,
        })];
        let rounds = BTreeMap::from([(
            projectile,
            Round {
                weapon,
                unit: UnitId(0),
                side: Side::Blue,
            },
        )]);
        let outcome = damage::resolve(
            &DamageContext {
                world: battle.world(),
                ground: battle.ground(),
                arsenal: battle.arsenal(),
                rules: battle.rules(),
                tick: 1,
            },
            &events,
            &rounds,
            &mut BTreeMap::new(),
            &mut units,
            &mut Rng::new(7),
        );
        if k + 1 < count {
            assert!(outcome.destroyed.is_empty());
        }
        total.extend(outcome.destroyed);
    }
    assert_eq!(
        total,
        vec![damage::UnitDeath {
            victim: UnitId(1),
            source: Some(damage::LethalSource {
                unit: UnitId(0),
                side: Side::Blue
            })
        }]
    );
}

#[test]
fn one_blast_preserves_source_for_multiple_full_unit_deaths() {
    let setup = common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([
            {"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"test_jeep","position":[200,300]},
            {"side":"red","kind":"test_jeep","position":[204,300]}
        ]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let mut units: Vec<_> = (0..3)
        .map(|i| battle.unit(UnitId(i)).unwrap().clone())
        .collect();
    units[1].hp = 0.01;
    units[2].hp = 0.01;
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "tank_he")
        .unwrap();
    let projectile = ProjectileId(0);
    let events = [FlightEvent::Impact(Impact {
        projectile,
        struck: Struck::Terrain,
        point: v3(202.0, 300.0, 0.0),
        normal: v3(0.0, 0.0, 1.0),
        velocity: v3(100.0, 0.0, 0.0),
        time: 0.0,
        bounces: 0,
        pose: None,
        detonated: true,
    })];
    let rounds = BTreeMap::from([(
        projectile,
        Round {
            weapon,
            unit: UnitId(0),
            side: Side::Blue,
        },
    )]);
    let outcome = damage::resolve(
        &DamageContext {
            world: battle.world(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: battle.rules(),
            tick: 1,
        },
        &events,
        &rounds,
        &mut BTreeMap::new(),
        &mut units,
        &mut Rng::new(7),
    );
    let source = Some(damage::LethalSource {
        unit: UnitId(0),
        side: Side::Blue,
    });
    assert_eq!(
        outcome.destroyed,
        vec![
            damage::UnitDeath {
                victim: UnitId(1),
                source
            },
            damage::UnitDeath {
                victim: UnitId(2),
                source
            }
        ]
    );
}

#[test]
fn a_large_pool_still_caps_each_victim_bonus_at_half_original_cost() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    settlement.settle(&[economic(1, Side::Blue, 4000)], 1000, &mut wallets);
    settlement.settle(&[economic(2, Side::Red, 200)], 1000, &mut wallets);
    assert_eq!(wallets, [150_000_000, 1_000_000_000]);
    assert_eq!(settlement.pools(), [100_000_000, 1_800_000_000]);
}
