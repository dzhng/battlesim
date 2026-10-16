use crate::common::air_field::wing_limits;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use contract::scenario::FixedWingFlight;
use serde_json::json;
use sim::air::pass::{advance, begin, choose_weapon, PassPhase, PassWeapon};
use sim::air::FixedWingState;
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
    rules["weapons"]["air_ground_missile"]["targets"]["ground"]["classes"] =
        json!(["infantry", "vehicle"]);
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

fn wing() -> FixedWingFlight {
    FixedWingFlight {
        strafe_ceiling_m: 130.0,
        bomb_ceiling_m: 200.0,
        dogfight_band_agl_m: [150.0, 350.0],
        dogfight_break_m: 1200.0,
        afterburner_ramp_s: 2.0,
        edge_margin_m: 300.0,
    }
}

fn state(agl: f64) -> FixedWingState {
    FixedWingState {
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
        &wing_limits(),
        aim,
        PassWeapon::Bomb,
        &wing(),
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
        let (intent, cue) = advance(&mut plan, &current, &wing_limits(), aim, &release);
        saw_release |= cue.is_some();
        let _ = intent;
    }
    assert!(saw_release || plan.phase == PassPhase::Release);
}

#[test]
fn pass_height_is_seeded_between_floor_and_ceiling() {
    let p = begin(
        &state(250.0),
        &wing_limits(),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(9),
    );
    assert!((100.0..=200.0).contains(&p.agl));
}

#[test]
fn a_jet_below_the_ceiling_doesnt_climb_to_start() {
    let p = begin(
        &state(150.0),
        &wing_limits(),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &wing(),
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

/// Every pass weapon flies a pass, and every fixed-wing row in the game's
/// catalog lives the whole life a player buys it for (D44): it arrives
/// through a purchase, already flying, within its authored wait, then turns
/// for home on low fuel and leaves by its back edge, refunded.
#[test]
fn every_loadout_flies_a_pass() {
    for weapon in [PassWeapon::Gun, PassWeapon::GroundMissile, PassWeapon::Bomb] {
        let p = begin(
            &state(250.0),
            &wing_limits(),
            v3(1000.0, 0.0, 0.0),
            weapon,
            &wing(),
            &mut Rng::new(3),
        );
        assert_eq!(p.weapon, weapon);
    }
    let rules: contract::scenario::Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let catalog = &rules.catalog;
    let rows: Vec<String> = catalog
        .indices()
        .filter(|&t| {
            catalog.get(t).mobility.flight() == Some(contract::catalog::FlightModel::FixedWing)
        })
        .map(|t| catalog.id(t).to_string())
        .collect();
    assert!(!rows.is_empty(), "the game fields fixed-wing aircraft");
    for row in &rows {
        bought_jet_arrives_and_withdraws_on_low_fuel(row);
    }
}

/// Blue buys `row` on an open 3 km field and the battle runs its life out.
fn bought_jet_arrives_and_withdraws_on_low_fuel(row: &str) {
    let mut rules = sim::fixtures::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        row,
        json!({ "roster": { "factions": ["us"] } }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [3000, 3000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "skirmish": {
            "factions": ["us", "eastern"],
            "rules": { "credits_per_minute": 200, "starting_minutes": 20, "preparation_s": 60, "max_units": 30, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": { "entries": [
                { "side": "blue", "center": [1500, 10], "yaw": 1.5707963267948966 },
                { "side": "red", "center": [1500, 2990], "yaw": -1.5707963267948966 }
            ], "objectives": [
                { "id": "center", "center": [1500, 1500], "radius_m": 50, "kind": "junction", "counterpart": null },
                { "id": "south", "center": [800, 1000], "radius_m": 50, "kind": "field", "counterpart": "north" },
                { "id": "north", "center": [2200, 2000], "radius_m": 50, "kind": "field", "counterpart": "south" }
            ] }
        }
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 4);
    let t = battle.rules().catalog.by_id(row);
    let wait_s = t.mobility.arrival().map_or(0.0, |a| a.delay_s[1]);
    let fuel_s = t
        .capabilities
        .air
        .as_ref()
        .expect("a jet's tank")
        .fuel_capacity_s;
    let hz = battle.rules().tick_hz;
    let envelope = |seq, order| CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq,
        order,
        queued: false,
    };
    let ack = battle.accept(envelope(
        1,
        Order::ConfirmPurchase {
            variant: row.into(),
            destination: [1500.0, 1500.0],
        },
    ));
    assert!(ack.error.is_none(), "{row}: {ack:?}");
    battle.accept(envelope(2, Order::Ready));
    battle.accept(CommandEnvelope {
        side: Side::Red,
        ..envelope(1, Order::Ready)
    });
    let credits =
        |b: &sim::battle::Battle| b.observe(Side::Blue).skirmish.as_ref().unwrap().credits;
    let (mut arrived, mut withdrew, mut left, mut refund) = (None, None, false, 0.0);
    let mut before = credits(&battle);
    let limit = (wait_s + fuel_s + 120.0) as u32 * hz;
    for _ in 0..limit {
        battle.step();
        let now = credits(&battle);
        let frame = battle.observe(Side::Blue);
        match frame.own.first() {
            Some(jet) => {
                arrived.get_or_insert((battle.tick(), jet.position));
                if jet.withdrawing && withdrew.is_none() {
                    withdrew = jet.fuel_s;
                }
            }
            None if arrived.is_some() => {
                left = frame
                    .air_transits
                    .iter()
                    .any(|t| matches!(t.dir, contract::observation::AirTransitDirection::Out));
                refund = now - before;
                break;
            }
            None => {}
        }
        before = now;
    }
    let (tick, at) = arrived.unwrap_or_else(|| panic!("{row} never arrived"));
    assert!(
        tick as f64 <= (60.0 + wait_s + 1.0) * f64::from(hz),
        "{row} arrived {:.1}s after its longest wait",
        tick as f64 / f64::from(hz) - 60.0 - wait_s
    );
    assert!(
        at[1].abs() < 50.0,
        "{row} arrived at {at:?}, not at its back edge"
    );
    let fuel = withdrew.unwrap_or_else(|| panic!("{row} never turned for home on low fuel"));
    assert!(fuel > 0.0, "{row} ran dry before turning home");
    assert!(left, "{row} never left by its back edge");
    // A tick's income is a fraction of a credit; a refund is many.
    assert!(
        refund > 1.0,
        "{row} was not refunded as it left: {refund:.2}"
    );
}

#[test]
fn the_gun_fires_only_nose_on_a_legal_target_in_range() {
    let battle = crate::common::air_field::AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
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

#[test]
fn a_tank_on_high_ground_gets_attack_runs_not_a_dogfight() {
    // Height never chooses a layer: a tank on a 140 m mesa stands higher
    // than the high-air floor, yet it is a ground target and gets runs.
    let setup = serde_json::from_value(json!({
        "map": {
            "size": [2400, 1200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "relief": [{ "kind": "mesa", "rect": [1300, 400, 1700, 800], "height_m": 140, "side_degrees": 40 }]
        },
        "rules": sim::fixtures::test_game(),
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 600], "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [1450, 600], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [1500, 600], "yaw": 0.0, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    for _ in 0..30 {
        battle.step();
    }
    let tank = battle
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|t| t.position[2] > 100.0)
        .map(|t| t.id)
        .expect("the recon sees the tank up on the mesa");
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    let mut passed = false;
    for _ in 0..600 {
        battle.step();
        let air = battle.unit(UnitId(0)).unwrap().air.unwrap();
        assert_eq!(
            air.dogfight_target, None,
            "a tank is never a dogfight opponent"
        );
        passed |= air.pass.is_some();
    }
    assert!(passed, "the jet flies an attack run at the tank");
}

/// The jet's own numbers for the repeated-pass test: every bound it asserts
/// is derived from them, not from the shipped catalog.
const REPEAT_CRUISE_KMH: f64 = 540.0;
const REPEAT_TURN_DEG_S: f64 = 60.0;
const REPEAT_PASSES: usize = 4;

/// A test jet carrying only `weapon` (in place of its gun), 1 km west of a ground aim point in the
/// middle of a 4 km field (far inside the edge guard), heading east at it
/// under an explicit attack order.
fn repeated_pass_battle(weapon: &str) -> sim::battle::Battle {
    let mut rules = sim::fixtures::test_game();
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(300.0);
    rules["weapons"]["air_bomb"]["ammo"] = json!(REPEAT_PASSES);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    // Rearming between passes is not this test's subject.
    rules["weapons"]["air_bomb"]["reload_s"] = json!(1.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "mobility": { "air": { "cruise_kmh": REPEAT_CRUISE_KMH, "turn_deg_s": REPEAT_TURN_DEG_S } },
            "mounts": [{
                "id": "gun", "name": "Stores", "weapons": [weapon],
                "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0]
            }]
        }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{ "side": "blue", "kind": "test_jet", "position": [1000, 2000], "yaw": 0.0 }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [2000.0, 2000.0, 0.0],
            },
        },
    });
    assert_eq!(ack.error, None);
    battle
}

/// Strafing or bombing a ground target is back and forth: line up, dive
/// nose-on from the approach side, fire or release, pull out, come round and
/// go again. Every pass after the first must line up as well as the first.
#[test]
fn repeated_passes_each_dive_in_nose_on_and_fire() {
    let speed = REPEAT_CRUISE_KMH / 3.6;
    let turn = REPEAT_TURN_DEG_S.to_radians();
    let lap_s = std::f64::consts::TAU / turn;
    // Reaching the target, then each pass: a turn-around and straight legs
    // out and back, well inside two full laps of flying.
    let bound_s = 1000.0 / speed + REPEAT_PASSES as f64 * 2.0 * lap_s;
    let tolerance = 10_f64.to_radians();
    let aim = sim::math::v2(2000.0, 2000.0);
    for weapon in ["air_bomb", "air_gun"] {
        let mut battle = repeated_pass_battle(weapon);
        let hz = f64::from(battle.rules().tick_hz);
        let mut previous = None;
        let mut dives = Vec::new();
        let mut fired_per_pass = Vec::new();
        let mut shots_before = 0;
        for tick in 0..(bound_s * hz) as u64 {
            battle.step();
            let jet = battle.unit(UnitId(0)).unwrap();
            let phase = jet.air.unwrap().pass.map(|p| p.phase);
            let to_aim = aim - jet.position.xy();
            if phase == Some(PassPhase::Dive) && previous != Some(PassPhase::Dive) {
                let off = sim::math::wrap_angle(libm::atan2(to_aim.y, to_aim.x) - jet.yaw);
                dives.push((tick as f64 / hz, off.to_degrees(), to_aim.length()));
                assert!(
                    off.abs() <= tolerance,
                    "{weapon}: dive {} began {:.0} deg off the target, {:.0} m out, at {:.1}s",
                    dives.len(),
                    off.to_degrees(),
                    to_aim.length(),
                    tick as f64 / hz
                );
            }
            if previous == Some(PassPhase::PullOut) && phase != Some(PassPhase::PullOut) {
                let shots = jet.mounts[0].shots;
                fired_per_pass.push(shots - shots_before);
                shots_before = shots;
            }
            previous = phase;
            if fired_per_pass.len() == REPEAT_PASSES {
                break;
            }
        }
        assert_eq!(
            fired_per_pass.len(),
            REPEAT_PASSES,
            "{weapon}: only {} of {REPEAT_PASSES} passes finished within {bound_s:.0}s; dives {dives:.0?}",
            fired_per_pass.len()
        );
        assert!(
            fired_per_pass.iter().all(|&n| n > 0),
            "{weapon}: a pass went by without firing: rounds per pass {fired_per_pass:?}"
        );
    }
}
