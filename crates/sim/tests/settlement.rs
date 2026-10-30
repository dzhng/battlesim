use crate::common;
use crate::common::Seat;
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use serde_json::json;
use sim::battle::{Battle, Round};
use sim::damage::{self, DamageContext};
use sim::flight::{BodyId, FlightEvent, Impact, ProjectileId, Struck};
use sim::math::v3;
use std::collections::BTreeMap;
use std::collections::VecDeque;

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
            seeker_target: None,
            fused_target: None,
            layer: contract::catalog::AltitudeLayer::Ground,
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
            seeker_target: None,
            fused_target: None,
            layer: contract::catalog::AltitudeLayer::Ground,
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
            unit: UnitId(victim + 1000),
            side: if side == Side::Blue {
                Side::Red
            } else {
                Side::Blue
            },
        }),
        supply: false,
    }
}

fn settle(
    settlement: &mut sim::settlement::Settlement,
    deaths: &[sim::settlement::EconomicDeath],
    starting_budget: u64,
    wallets: &mut [u64; 2],
) {
    let mut players = [
        sim::skirmish::PlayerState {
            id: "blue".into(),
            side: Side::Blue,
            faction: contract::catalog::Faction::Us,
            controller: contract::players::ControllerKind::ActiveAi,
            wallet: wallets[0],
            ready: false,
            reservations: VecDeque::new(),
            next_purchase: 0,
            totals: Default::default(),
        },
        sim::skirmish::PlayerState {
            id: "red".into(),
            side: Side::Red,
            faction: contract::catalog::Faction::Eastern,
            controller: contract::players::ControllerKind::ActiveAi,
            wallet: wallets[1],
            ready: false,
            reservations: VecDeque::new(),
            next_purchase: 0,
            totals: Default::default(),
        },
    ];
    let killer_players = deaths
        .iter()
        .filter_map(|death| {
            death
                .source
                .map(|source| (source.unit, if source.side == Side::Blue { 0 } else { 1 }))
        })
        .collect();
    settlement.settle_players(deaths, starting_budget, &killer_players, &mut players);
    wallets[0] = players[0].wallet;
    wallets[1] = players[1].wallet;
}

#[test]
fn hostile_death_rewards_once_and_builds_only_original_price_bounty() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    let deaths = [economic(1, Side::Red, 200)];
    settle(&mut settlement, &deaths, 1000, &mut wallets);
    settle(&mut settlement, &deaths, 1000, &mut wallets);
    assert_eq!(wallets, [50_000_000, 0]);
    assert_eq!(settlement.pools(), [100_000_000, 0]);
}

#[test]
fn player_bonus_split_conserves_pool_and_ignores_death_order() {
    fn players() -> Vec<sim::skirmish::PlayerState> {
        [
            ("blue-1", Side::Blue, contract::catalog::Faction::Us),
            ("blue-2", Side::Blue, contract::catalog::Faction::Us),
            ("red-1", Side::Red, contract::catalog::Faction::Eastern),
        ]
        .into_iter()
        .map(|(id, side, faction)| sim::skirmish::PlayerState {
            id: id.into(),
            side,
            faction,
            controller: contract::players::ControllerKind::ActiveAi,
            wallet: 0,
            ready: false,
            reservations: VecDeque::new(),
            next_purchase: 0,
            totals: Default::default(),
        })
        .collect()
    }

    let mut first = sim::settlement::Settlement::default();
    let mut second = sim::settlement::Settlement::default();
    let mut first_players = players();
    let mut second_players = players();
    let seed = economic(1, Side::Blue, 1000);
    let seed_killer = BTreeMap::from([(UnitId(1001), 2usize)]);
    first.settle_players(&[seed], 1000, &seed_killer, &mut first_players);
    second.settle_players(
        &[economic(1, Side::Blue, 1000)],
        1000,
        &seed_killer,
        &mut second_players,
    );
    let pool_before = first.pools()[Side::Red.index()];
    let deaths = [economic(2, Side::Red, 100), economic(3, Side::Red, 200)];
    let killers = BTreeMap::from([(UnitId(1002), 0usize), (UnitId(1003), 1usize)]);
    first.settle_players(&deaths, 1000, &killers, &mut first_players);
    second.settle_players(
        &[economic(3, Side::Red, 200), economic(2, Side::Red, 100)],
        1000,
        &killers,
        &mut second_players,
    );
    assert_eq!(
        first_players[0].wallet, second_players[0].wallet,
        "killer split must not depend on death order"
    );
    assert_eq!(first_players[1].wallet, second_players[1].wallet);
    let base_rewards = 75_000_000;
    let bonus_wallets = first_players[0].wallet + first_players[1].wallet - base_rewards;
    let growth_into_victim_pool = 0;
    let victim_losses = 150_000_000;
    assert_eq!(
        pool_before - first.pools()[Side::Red.index()] + growth_into_victim_pool - victim_losses,
        bonus_wallets
    );
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
            seeker_target: None,
            fused_target: None,
            layer: contract::catalog::AltitudeLayer::Ground,
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
    settle(&mut a, &seed, 1000, &mut wa);
    settle(&mut b, &seed, 1000, &mut wb);
    let deaths = [economic(3, Side::Red, 200), economic(4, Side::Blue, 400)];
    settle(&mut a, &deaths, 1000, &mut wa);
    settle(
        &mut b,
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
    settle(
        &mut settlement,
        &[economic(1, Side::Blue, 200)],
        1000,
        &mut wallets,
    );
    let before = wallets;
    let mut friendly = economic(2, Side::Red, 200);
    friendly.source.as_mut().unwrap().side = Side::Red;
    let mut free = economic(3, Side::Red, 200);
    free.source = None;
    settle(&mut settlement, &[friendly, free], 1000, &mut wallets);
    assert_eq!(wallets, before);
    assert_eq!(settlement.pools(), [0, 100_000_000]);
}

#[test]
fn unseen_purchased_kill_changes_wallet_without_enemy_identity_and_replays() {
    use contract::command::{CommandEnvelope, Order, TargetRef};
    let mut setup = crate::skirmish::setup();
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost":200,
            "roster":{"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"},
            // A shell anywhere on it kills it: the subject is the wallet.
            "body":{"hull":{"hp":0.01,"armor":{"front":1,"side":1,"rear":1,"roof":1}}},
            "sensors":{"ground_m":1}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.skirmish.as_mut().unwrap().sites.entries[1].center = [400.0, 110.0];
    let mut battle = Battle::new(&setup, 7);
    let send = |battle: &mut Battle, side, seq, order| {
        let ack = battle.accept(CommandEnvelope {
            side,
            player: Some(
                match side {
                    Side::Blue => "blue-1",
                    Side::Red => "red-1",
                    Side::Neutral => "neutral",
                }
                .into(),
            ),
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
    let before = battle.observe(Side::Blue).seat().credits;
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
    assert!((blue.seat().credits - before - passive - 50.0).abs() < 0.00001);
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
    settle(
        &mut settlement,
        &[economic(1, Side::Blue, 200)],
        1000,
        &mut wallets,
    );
    let mut truck = economic(2, Side::Red, 1000);
    truck.supply = true;
    settle(
        &mut settlement,
        &[truck, economic(3, Side::Red, 1000)],
        1000,
        &mut wallets,
    );
    assert_eq!(wallets, [850_000_000, 50_000_000]);
    assert_eq!(settlement.pools(), [1_000_000_000, 0]);
}

#[test]
fn fractional_bonus_carries_without_losing_sub_microcredits() {
    let mut settlement = sim::settlement::Settlement::default();
    let mut wallets = [0; 2];
    for k in 0..7 {
        settle(
            &mut settlement,
            &[economic(k * 2, Side::Blue, 1)],
            7,
            &mut wallets,
        );
        settle(
            &mut settlement,
            &[economic(k * 2 + 1, Side::Red, 1)],
            7,
            &mut wallets,
        );
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
                seeker_target: None,
                fused_target: None,
                layer: contract::catalog::AltitudeLayer::Ground,
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
            seeker_target: None,
            fused_target: None,
            layer: contract::catalog::AltitudeLayer::Ground,
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
    settle(
        &mut settlement,
        &[economic(1, Side::Blue, 4000)],
        1000,
        &mut wallets,
    );
    settle(
        &mut settlement,
        &[economic(2, Side::Red, 200)],
        1000,
        &mut wallets,
    );
    assert_eq!(wallets, [150_000_000, 1_000_000_000]);
    assert_eq!(settlement.pools(), [100_000_000, 1_800_000_000]);
}

/// The side-only settlement that shipped before players (main at slice 01),
/// kept as the oracle: with one player per side, paying players must pay
/// exactly what paying sides did, capped batches included.
#[derive(Default)]
struct SideOracle {
    pools: [u64; 2],
    remainder: [u64; 2],
    processed: std::collections::BTreeSet<UnitId>,
}

impl SideOracle {
    fn settle(
        &mut self,
        deaths: &[sim::settlement::EconomicDeath],
        budget: u64,
        wallets: &mut [u64; 2],
    ) {
        const SCALE: u64 = 1_000_000;
        let snapshot = self.pools;
        let (mut rewards, mut losses, mut growth) = ([0u64; 2], [0u64; 2], [0u64; 2]);
        let mut numer = [0u128; 2];
        for death in deaths {
            if !self.processed.insert(death.victim) {
                continue;
            }
            let Some(source) = death.source.filter(|s| s.side != death.side) else {
                continue;
            };
            let (victim, killer) = (death.side.index(), source.side.index());
            let cost = u64::from(death.price) * SCALE;
            rewards[killer] += cost / 4 * if death.supply { 2 } else { 1 };
            losses[victim] += cost / 2;
            growth[killer] += cost / 2;
            let d = u128::from(budget);
            numer[victim] += (u128::from(snapshot[victim]) * u128::from(death.price))
                .min(u128::from(snapshot[victim]) * d)
                .min(u128::from(cost / 2) * d);
        }
        for victim in 0..2 {
            let d = u128::from(budget);
            let numerator = numer[victim].min(u128::from(snapshot[victim]) * d)
                + u128::from(self.remainder[victim]);
            let paid = (numerator / d).min(u128::from(snapshot[victim]));
            self.remainder[victim] = (numerator - paid * d) as u64;
            rewards[1 - victim] += paid as u64;
            self.pools[victim] = snapshot[victim]
                .saturating_sub(paid as u64)
                .saturating_sub(losses[victim]);
        }
        for side in 0..2 {
            self.pools[side] += growth[side];
            wallets[side] += rewards[side];
        }
    }
}

#[test]
fn one_player_per_side_pays_exactly_what_side_settlement_paid() {
    // Capped batches first (the pool is smaller than the bonus owed), then
    // small fractional kills that depend on the carried remainder.
    let mut truck = economic(2, Side::Red, 1000);
    truck.supply = true;
    let batches: Vec<Vec<sim::settlement::EconomicDeath>> = vec![
        vec![economic(1, Side::Blue, 200)],
        vec![truck, economic(3, Side::Red, 1000)],
        vec![economic(4, Side::Blue, 900), economic(5, Side::Blue, 700)],
        vec![economic(6, Side::Red, 50)],
        vec![economic(7, Side::Red, 3), economic(8, Side::Blue, 1)],
        vec![economic(9, Side::Red, 1), economic(10, Side::Red, 1)],
        vec![economic(11, Side::Blue, 333)],
    ];
    for budget in [7, 1000] {
        let mut oracle = SideOracle::default();
        let mut settlement = sim::settlement::Settlement::default();
        let (mut expected, mut actual) = ([0u64; 2], [0u64; 2]);
        for batch in &batches {
            oracle.settle(batch, budget, &mut expected);
            settle(&mut settlement, batch, budget, &mut actual);
            assert_eq!(actual, expected, "wallets after a batch, budget {budget}");
            assert_eq!(
                settlement.pools(),
                oracle.pools,
                "pools after a batch, budget {budget}"
            );
        }
    }
}
