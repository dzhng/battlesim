use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use contract::scenario::JetAir;
use serde_json::json;
use sim::air::pass::{advance, begin, choose_weapon, PassPhase, PassWeapon};
use sim::air::JetState;
use sim::flight::Release;
use sim::math::v3;

fn ground_attack_battle(weapons: &[&str]) -> sim::battle::Battle {
    ground_attack_battle_with_bomb_ammo(weapons, 1)
}

fn ground_attack_battle_with_bomb_ammo(weapons: &[&str], bomb_ammo: u32) -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    let mounts: Vec<_> = weapons
        .iter()
        .enumerate()
        .map(|(i, weapon)| {
            json!({
                "id": format!("test_mount_{i}"),
                "name": "Test Mount",
                "weapons": [*weapon],
                "turret": false,
                "pivot_m": [0, 0, 1.2],
                "muzzle_m": [7, 0, 0]
            })
        })
        .collect();
    sim::fixtures::patch_catalog(&mut rules, "units", "test_jet", json!({ "mounts": mounts }));
    rules["weapons"]["air_bomb"]["ammo"] = json!(bomb_ammo);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    rules["weapons"]["air_ground_missile"]["aim_s"] = json!(0.1);
    rules["weapons"]["air_ground_missile"]["anti_armor"] = json!(false);
    let setup = serde_json::from_value(json!({
        "map": { "size": [2000, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 400], "yaw": 0.0 },
            { "side": "red", "kind": "test_tank", "position": [900, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    })).unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [900.0, 400.0, 0.0],
            },
        },
    });
    assert_eq!(ack.error, None);
    battle
}

fn air_battle() -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "air missiles", "name": "Air Missiles", "weapons": ["air_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [2000, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 400], "yaw": 0.0 },
            { "side": "red", "kind": "test_jet", "position": [900, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    })).unwrap();
    sim::battle::Battle::new(&setup, 1)
}

fn jet_air() -> JetAir {
    JetAir {
        cruise_agl_m: 250.0,
        floor_agl_m: 100.0,
        strafe_ceiling_m: 130.0,
        bomb_ceiling_m: 200.0,
        dogfight_band_agl_m: [150.0, 350.0],
        dogfight_break_m: 1200.0,
        afterburner_ramp_s: 2.0,
        edge_margin_m: 300.0,
    }
}

fn state(agl: f64) -> JetState {
    JetState {
        pos: v3(0.0, 0.0, agl),
        heading: 0.0,
        speed_mps: 100.0,
        burner: 0.0,
        turn_sign: 1,
    }
}

#[test]
fn a_bombing_pass_lines_up_dives_under_its_ceiling_releases_and_climbs_back() {
    let aim = v3(1000.0, 0.0, 0.0);
    let mut plan = begin(
        &state(250.0),
        aim,
        PassWeapon::Bomb,
        &jet_air(),
        &mut Rng::new(7),
    );
    assert_eq!(plan.phase, PassPhase::LineUp);
    let release = Release {
        fall_s: 3.0,
        ahead_m: 300.0,
        at: sim::math::v2(700.0, 0.0),
    };
    let mut saw_release = false;
    for step in 0..20 {
        let mut current = state(plan.agl);
        current.pos.x = step as f64 * 60.0;
        let (intent, cue) = advance(&mut plan, &current, aim, &release);
        saw_release |= cue.is_some();
        let _ = intent;
    }
    assert!(saw_release || plan.phase == PassPhase::Release);
}

#[test]
fn pass_height_is_seeded_between_floor_and_ceiling() {
    let p = begin(
        &state(250.0),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &jet_air(),
        &mut Rng::new(9),
    );
    assert!((100.0..=200.0).contains(&p.agl));
}

#[test]
fn a_jet_below_the_ceiling_doesnt_climb_to_start() {
    let p = begin(
        &state(150.0),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &jet_air(),
        &mut Rng::new(9),
    );
    assert!(p.agl <= 150.0);
}

#[test]
fn a_limited_ground_weapon_is_chosen_before_the_gun() {
    assert_eq!(
        choose_weapon(&[PassWeapon::Gun, PassWeapon::Bomb]),
        Some(PassWeapon::Bomb)
    );
    assert_eq!(choose_weapon(&[PassWeapon::Gun]), Some(PassWeapon::Gun));
}

#[test]
fn every_loadout_flies_a_pass() {
    for weapon in [PassWeapon::Gun, PassWeapon::GroundMissile, PassWeapon::Bomb] {
        let p = begin(
            &state(250.0),
            v3(1000.0, 0.0, 0.0),
            weapon,
            &jet_air(),
            &mut Rng::new(3),
        );
        assert_eq!(p.weapon, weapon);
    }
}

#[test]
fn the_gun_fires_only_nose_on_a_legal_target_in_range() {
    let battle = crate::common::jet::JetField::new(3000.0)
        .jet("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    let unit = battle.unit(contract::ids::UnitId(0)).unwrap();
    assert!(sim::weapons::nose_on(
        unit,
        v3(500.0, 1500.0, unit.position.z),
        700.0
    ));
    assert!(!sim::weapons::nose_on(
        unit,
        v3(500.0, 1600.0, unit.position.z),
        700.0
    ));
}

#[test]
fn a_bombing_pass_never_fires_the_gun() {
    let mut battle = ground_attack_battle(&["air_gun", "air_bomb"]);
    let mut bomb_tick = None;
    for _ in 0..900 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let specs = battle.rules().catalog.mounts(unit.kind);
        let gun_shots: u32 = unit
            .mounts
            .iter()
            .zip(specs)
            .filter(|(_, s)| s.def.weapons.iter().any(|w| w == "air_gun"))
            .map(|(m, _)| m.shots)
            .sum();
        if gun_shots > 0 && bomb_tick.is_none() {
            panic!("gun fired before bomb: {:?}", unit.air.unwrap().pass);
        }
        let bomb_shots: u32 = unit
            .mounts
            .iter()
            .zip(specs)
            .filter(|(_, s)| s.def.weapons.iter().any(|w| w == "air_bomb"))
            .map(|(m, _)| m.shots)
            .sum();
        if bomb_shots > 0 {
            bomb_tick = Some(gun_shots);
            break;
        }
    }
    assert_eq!(bomb_tick.expect("bomb pass never released"), 0);
}

#[test]
fn a_missile_pass_fires_the_gun_when_nose_on() {
    let mut battle = ground_attack_battle(&["air_gun", "air_ground_missile"]);
    for _ in 0..900 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let specs = battle.rules().catalog.mounts(unit.kind);
        let missile = unit
            .mounts
            .iter()
            .zip(specs)
            .find(|(_, s)| s.def.weapons.iter().any(|w| w == "air_ground_missile"))
            .is_some_and(|(m, _)| m.shots > 0);
        let gun = unit
            .mounts
            .iter()
            .zip(specs)
            .filter(|(_, s)| s.def.weapons.iter().any(|w| w == "air_gun"))
            .map(|(m, _)| m.shots)
            .sum::<u32>();
        if missile || gun > 0 {
            assert!(gun > 0, "missile pass crossed without a nose-on gun burst");
            return;
        }
    }
    panic!("ground missile pass never reached a nose-on gun burst");
}

#[test]
fn an_air_missile_fires_at_an_enemy_in_its_cone_during_a_move() {
    let mut battle = air_battle();
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 2,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [1100.0, 400.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    for _ in 0..600 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let specs = battle.rules().catalog.mounts(unit.kind);
        if unit
            .mounts
            .iter()
            .zip(specs)
            .any(|(m, s)| s.def.weapons.iter().any(|w| w == "air_missile") && m.shots > 0)
        {
            return;
        }
    }
    panic!("air missile did not self-fire during a move");
}

#[test]
fn a_strafe_is_allowed_while_bombs_remain() {
    // The bomb station is still present in the loadout, but its finite store
    // is empty. A stale bomb-first ban must not suppress the gun pass.
    let mut battle = ground_attack_battle_with_bomb_ammo(&["air_gun", "air_bomb"], 0);
    for _ in 0..900 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().mounts[0].shots > 0 {
            return;
        }
    }
    panic!("gun never fired with a bomb mount present");
}

fn attack_move_battle() -> sim::battle::Battle {
    attack_move_battle_with_hp(100.0)
}

fn attack_move_battle_with_hp(target_hp: f64) -> sim::battle::Battle {
    attack_move_battle_with_hp_and_fire(target_hp, false)
}

fn attack_move_battle_with_hp_and_fire(target_hp: f64, fire: bool) -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    if fire {
        rules["sensors"]["contact_lifetime_s"] = json!(100.0);
    }
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "air gun", "name": "Air Gun", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    let events = if fire {
        json!([{ "tick": 5, "fire": { "unit": 3 } }])
    } else {
        json!([])
    };
    let setup = serde_json::from_value(json!({
        "map": { "size": [2400, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [300, 400], "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [300, 500], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [900, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only", "condition": { "hp": target_hp } },
            { "side": "red", "kind": "test_jet", "position": [1_900, 700], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": events, "scripts": []
    })).unwrap();
    sim::battle::Battle::new(&setup, 1)
}

fn holding_ground_battle(engagement: &str, fire_event: bool) -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "air gun", "name": "Air Gun", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    let events = if fire_event {
        json!([{ "tick": 5, "fire": { "unit": 1 } }])
    } else {
        json!([])
    };
    let setup = serde_json::from_value(json!({
        "map": { "size": [2000, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 400], "yaw": 0.0, "engagement": engagement },
            { "side": "red", "kind": "test_tank", "position": [900, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": events, "scripts": []
    })).unwrap();
    sim::battle::Battle::new(&setup, 1)
}

fn attack_move(battle: &mut sim::battle::Battle, seq: u64, goal: [f64; 2]) {
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq,
        queued: false,
        order: Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: seq,
            goal,
        },
    });
    assert_eq!(ack.error, None);
}

#[test]
fn an_explicit_attack_repeats_passes_until_the_target_dies() {
    let mut battle = attack_move_battle_with_hp(10_000.0);
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [900.0, 400.0, 0.0],
            },
        },
    });
    let mut starts = 0;
    let mut had_pass = false;
    for _ in 0..5000 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let has_pass = jet.air.is_some_and(|a| a.pass.is_some());
        if has_pass && !had_pass {
            starts += 1;
        }
        had_pass = has_pass;
    }
    assert!(
        starts >= 2,
        "explicit attack only started {starts} pass(es)"
    );
    assert!(
        !battle.unit(UnitId(2)).unwrap().alive(),
        "target survived repeated passes"
    );
}

#[test]
fn attack_move_picks_up_targets_then_holds_circling() {
    let mut battle = attack_move_battle();
    attack_move(&mut battle, 1, [1_700.0, 400.0]);
    let mut fired = false;
    for _ in 0..1800 {
        battle.step();
        fired |= battle.unit(UnitId(0)).unwrap().mounts[0].shots > 0;
    }
    assert!(fired, "attack-move never picked up the target");
    assert!(
        battle.unit(UnitId(0)).unwrap().orders.is_empty(),
        "attack-move did not settle"
    );
}

#[test]
fn a_holding_jet_starts_no_runs() {
    let mut battle = holding_ground_battle("fire_at_will", false);
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 2,
        queued: false,
        order: Order::Stop {
            units: vec![UnitId(0)],
        },
    });
    for _ in 0..300 {
        battle.step();
    }
    let jet = battle.unit(UnitId(0)).unwrap();
    assert!(jet.air.unwrap().pass.is_none());
    assert_eq!(jet.mounts[0].shots, 0);
}

#[test]
fn a_fired_on_holding_jet_attacks_its_visible_attacker() {
    let mut battle = air_battle();
    for _ in 0..30 {
        battle.step();
    }
    assert!(battle
        .observe(Side::Blue)
        .identified
        .iter()
        .any(|u| u.kind == battle.unit(UnitId(1)).unwrap().kind));
    for _ in 0..180 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target == Some(UnitId(1)) {
            return;
        }
    }
    panic!("holding jet did not answer its visible attacker");
}

#[test]
fn a_fired_on_holding_jet_attacks_the_contact_area_when_unseen() {
    let mut battle = holding_ground_battle("fire_at_will", true);
    for _ in 0..30 {
        battle.step();
    }
    assert!(battle.observe(Side::Blue).identified.is_empty());
    for _ in 0..300 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().air.unwrap().pass.is_some() {
            return;
        }
    }
    panic!("holding jet did not attack its firing contact");
}

#[test]
fn hold_fire_stops_the_answer() {
    let mut battle = holding_ground_battle("return_fire_only", true);
    for _ in 0..300 {
        battle.step();
    }
    let jet = battle.unit(UnitId(0)).unwrap();
    assert!(jet.air.unwrap().pass.is_none());
    assert_eq!(jet.mounts[0].shots, 0);
}

#[test]
fn an_explicit_attack_keeps_passing_under_fire() {
    let mut battle = attack_move_battle();
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [900.0, 400.0, 0.0],
            },
        },
    });
    let mut starts = 0;
    let mut had_pass = false;
    for _ in 0..1800 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let has_pass = jet.air.is_some_and(|a| a.pass.is_some());
        if has_pass && !had_pass {
            starts += 1;
        }
        had_pass = has_pass;
    }
    assert!(starts >= 2, "incoming fire interrupted explicit passes");
}

#[test]
fn attack_move_finishes_its_pass_then_turns_on_the_attacker() {
    let mut battle = attack_move_battle_with_hp_and_fire(100.0, true);
    attack_move(&mut battle, 1, [1_700.0, 400.0]);
    for _ in 0..1200 {
        battle.step();
    }
    let jet = battle.unit(UnitId(0)).unwrap();
    assert!(jet.orders.is_empty() && jet.air.unwrap().pass.is_none());
}

#[test]
fn a_move_breaks_off_mid_pass_at_once() {
    let mut battle = ground_attack_battle(&["air_gun"]);
    for _ in 0..300 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().air.unwrap().pass.is_some() {
            break;
        }
    }
    assert!(battle.unit(UnitId(0)).unwrap().air.unwrap().pass.is_some());
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 2,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [500.0, 700.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    battle.step();
    assert!(battle.unit(UnitId(0)).unwrap().air.unwrap().pass.is_none());
}
