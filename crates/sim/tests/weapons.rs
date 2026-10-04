//! Weapon control contracts (slice 08): selection, aim/reload overlap,
//! interruptions, policy and firing rules, driven through real scenarios.
use std::collections::BTreeSet;

use contract::command::{Engagement, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, MountReadiness, MoveState, OwnUnit, SuppressionTier};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, ProjectileId};

use crate::common::{self, Commander};

// Range controls isolate weapon control and timing from live balance tuning.
const RANGES: [(&str, f64); 6] = [
    ("rifle", 600.0),
    ("grenade", 450.0),
    ("tank_ap", 1400.0),
    ("tank_he", 1400.0),
    ("hmg", 800.0),
    ("atgm", 1800.0),
];
fn rules() -> Value {
    let mut rules = common::scenario_rules();
    for (id, range) in RANGES {
        rules["weapons"][id]["range_m"] = json!(range);
    }
    rules
}
fn scenario_with(
    map: &str,
    units: Value,
    events: Value,
    scripts: Value,
) -> contract::scenario::ScenarioDefinition {
    let mut setup = common::scenario_with(map, units, events, scripts);
    for (id, range) in RANGES {
        setup.rules.weapons.get_mut(id).unwrap().ballistics.range_m = range;
    }
    setup
}

/// A flat 1200 × 600 map plus extra props.
fn map(props: Value) -> String {
    json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

fn battle(props: Value, units: Value, events: Value, scripts: Value) -> Battle {
    Battle::new(&scenario_with(&map(props), units, events, scripts), 5)
}

fn own(b: &Battle, side: Side, id: u32) -> OwnUnit {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

fn mount(b: &Battle, side: Side, id: u32, m: usize) -> MountReadiness {
    own(b, side, id).mounts[m].clone()
}

fn weapon_name(b: &Battle, index: usize) -> String {
    b.arsenal().weapons[index].id.clone()
}

/// Steps `ticks`, recording every launch as (tick, unit, weapon name).
fn run(b: &mut Battle, ticks: u64) -> Vec<(u64, u32, String)> {
    let mut seen: BTreeSet<ProjectileId> = b.rounds().map(|(p, _)| p.id).collect();
    let mut shots = Vec::new();
    for _ in 0..ticks {
        b.step();
        let mut fired: Vec<(u32, String)> = Vec::new();
        for (p, r) in b.rounds() {
            if seen.insert(p.id) {
                fired.push((r.unit.0, weapon_name(b, r.weapon)));
            }
        }
        fired.sort();
        fired.dedup(); // a squad volley is one shot
        shots.extend(fired.into_iter().map(|(u, w)| (b.tick(), u, w)));
    }
    shots
}

fn shots_by(shots: &[(u64, u32, String)], unit: u32, weapon: &str) -> Vec<u64> {
    shots
        .iter()
        .filter(|s| s.1 == unit && s.2 == weapon)
        .map(|s| s.0)
        .collect()
}

fn ticks(seconds: f64) -> u64 {
    (seconds * 30.0).round() as u64
}

fn weapon(name: &str) -> Value {
    rules()["weapons"][name].clone()
}

#[test]
fn a_launcher_operator_uses_one_gun_while_his_guards_keep_firing() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [550, 300], "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    for weapon in setup.rules.weapons.values_mut() {
        weapon.damage = 0.0;
    }
    let mut b = Battle::new(&setup, 5);
    let operator = b.unit(UnitId(0)).unwrap().mounts[1].operator.unwrap();
    let mut seen = BTreeSet::new();
    let (mut launcher, mut guards) = (false, false);
    for _ in 0..180 {
        b.step();
        for (p, r) in b.rounds() {
            if r.unit != UnitId(0) || !seen.insert(p.id) {
                continue;
            }
            let shooter = p.shooter.unwrap().body.0;
            if weapon_name(&b, r.weapon) == "rifle" {
                assert_ne!(
                    shooter, operator,
                    "the launcher operator cannot also work his rifle"
                );
                guards = true;
            } else {
                assert_eq!(shooter, operator);
                launcher = true;
            }
        }
    }
    assert!(launcher && guards, "the launcher and its guards both fight");
}

#[test]
fn switching_to_a_rifle_pauses_reload_and_resuming_a_useful_launcher_finishes_it() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [450, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [250, 400], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [650, 200], "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    for weapon in setup.rules.weapons.values_mut() {
        weapon.damage = 0.0;
        weapon.near_miss_suppression = 0.0;
    }
    let launcher = setup.rules.weapons.get_mut("atgm").unwrap();
    launcher.aim_s = 0.2;
    launcher.reload_s = 10.0;
    launcher.damage = 1.0e6;
    launcher.ballistics.range_m = 500.0;
    let mut b = Battle::new(&setup, 5);
    let mut switched = false;
    for _ in 0..300 {
        b.step();
        let u = b.unit(UnitId(0)).unwrap();
        if !b.unit(UnitId(1)).unwrap().alive() && u.members[0].active_mount == Some(0) {
            switched = true;
            break;
        }
    }
    assert!(
        switched,
        "after killing the reachable tank the gunner uses his rifle"
    );
    let reload = mount(&b, Side::Blue, 0, 1).reload;
    assert!(
        reload > 0.0 && reload < 1.0,
        "the launcher has unfinished reload work"
    );
    let operator = b.unit(UnitId(0)).unwrap().members[0].id;
    let guard = b.unit(UnitId(0)).unwrap().members[1].id;
    let mut riflemen = BTreeSet::new();
    for _ in 0..60 {
        b.step();
        assert_eq!(
            mount(&b, Side::Blue, 0, 1).reload,
            reload,
            "the carried launcher's reload pauses"
        );
        riflemen.extend(
            b.rounds()
                .filter(|(_, r)| r.unit == UnitId(0) && weapon_name(&b, r.weapon) == "rifle")
                .map(|(p, _)| p.shooter.unwrap().body.0),
        );
    }
    assert!(
        riflemen.contains(&operator),
        "the gunner's inherited rifle remains usable"
    );
    assert!(
        riflemen.contains(&guard),
        "the guards keep fighting across his switch"
    );
    Commander::new().ok(
        &mut b,
        Side::Red,
        Order::Move {
            units: vec![UnitId(3)],
            gesture: 8100,
            goal: [450.0, 200.0],
            facing: None,
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
        },
    );
    let mut resumed = false;
    for _ in 0..600 {
        b.step();
        if b.unit(UnitId(0)).unwrap().members[0].active_mount == Some(1) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "a useful launcher resumes despite being unloaded");
    assert!(
        mount(&b, Side::Blue, 0, 1).reload > reload,
        "reload resumes its retained progress"
    );
    for _ in 0..60 {
        b.step();
        assert!(
            !b.rounds().any(|(p, r)| {
                p.age_s == 0.0
                    && r.unit == UnitId(0)
                    && weapon_name(&b, r.weapon) == "rifle"
                    && p.shooter.unwrap().body.0 == operator
            }),
            "the reloading gunner cannot also fire his rifle"
        );
    }
    assert!(
        run(&mut b, 360).iter().any(|s| s.1 == 0 && s.2 == "atgm"),
        "the resumed reload is not starved"
    );
}

#[test]
fn a_depleted_single_gun_returns_to_the_rifle_without_resetting_its_guards_aim() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "recon", "position": [220, 300], "engagement": "return_fire_only" }
        ]),
        json!([]),
        json!([]),
    );
    for weapon in setup.rules.weapons.values_mut() {
        weapon.damage = 0.0;
        weapon.near_miss_suppression = 0.0;
    }
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.aim_s = 2.0;
    rifle.magazine = None;
    let grenade = setup.rules.weapons.get_mut("grenade").unwrap();
    grenade.aim_s = 0.2;
    grenade.ammo = contract::weapons::AmmoCapacity::Rounds(1);
    let mut b = Battle::new(&setup, 5);
    let operator = b.unit(UnitId(0)).unwrap().members[0].id;
    let guard = b.unit(UnitId(0)).unwrap().members[1].id;
    let mut launch = None;
    for _ in 0..30 {
        b.step();
        if b.unit(UnitId(0)).unwrap().mounts[1].shots > 0 {
            launch = Some(b.tick());
            break;
        }
    }
    let launch = launch.expect("the grenadier launches his one round");
    let u = b.unit(UnitId(0)).unwrap();
    assert_eq!(u.members[0].active_mount, Some(1));
    assert_eq!(
        u.mounts[0].cycles[0].aim, 0.0,
        "his carried rifle earns no aim"
    );
    let guard_aim = u.mounts[0].cycles[1].aim;
    assert!(guard_aim > 0.0, "the guard has already begun aiming");
    b.step();
    let u = b.unit(UnitId(0)).unwrap();
    assert_eq!(
        u.members[0].active_mount,
        Some(0),
        "a depleted gun frees its soldier"
    );
    assert!(
        u.mounts[0].cycles[1].aim > guard_aim,
        "switching one soldier preserves the guard's aim"
    );
    let (mut gunner_first, mut guard_first) = (None, None);
    for _ in 0..90 {
        b.step();
        for (p, r) in b.rounds() {
            if r.unit != UnitId(0) || weapon_name(&b, r.weapon) != "rifle" {
                continue;
            }
            match p.shooter.unwrap().body.0 {
                id if id == operator => {
                    gunner_first.get_or_insert(b.tick());
                }
                id if id == guard => {
                    guard_first.get_or_insert(b.tick());
                }
                _ => {}
            }
        }
    }
    assert!(
        guard_first.expect("the guard fires") <= ticks(2.0) + 2,
        "the guard's original acquisition continues"
    );
    assert!(
        gunner_first.expect("the gunner fires his inherited rifle") >= launch + ticks(2.0),
        "the returning rifle needs its normal aim"
    );
}

/// A single launcher with a level aim point: only engagement distance varies.
fn minimum_range_setup(
    id: &str,
    distance: f64,
    muzzle_forward: f64,
    infantry: bool,
) -> contract::scenario::ScenarioDefinition {
    let mut game = rules();
    game["weapons"][id]["min_range_m"] = json!(5.0);
    game["weapons"][id]["aim_s"] = json!(0.0);
    game["weapons"][id]["ammo"] = json!(10);
    game["weapons"][id]["penetration"] = json!(1000.0);
    game["weapons"][id]["damage"] = json!(0.0);
    let base = common::rules();
    let target_height = 2.0
        * base.catalog.by_id("jeep").hull().unwrap().half_extents_m[2]
        * base.physics.vehicle_aim_height_fraction;
    // A small hull avoids squad spacing and collision at the boundary.
    let launcher = if infantry {
        json!({ "extends": "rifle", "body": { "squad": { "slots": ["grenadier"] } } })
    } else {
        json!({
            "extends": "jeep",
            "body": { "hull": { "half_extents_m": [0.25, 0.25, 0.25] } },
            "mounts": [{ "name": "HMG", "weapons": [id],
                "pivot_m": [0, 0, 0], "muzzle_m": [muzzle_forward, 0, target_height] }]
        })
    };
    game["catalog"].as_array_mut().unwrap().push(json!({
        "units": { "close_launcher": launcher }
    }));
    serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(&map(json!([]))).unwrap(),
        "rules": game,
        "units": [
            { "side": "blue", "kind": "close_launcher", "position": [100, 300] },
            { "side": "red", "kind": "jeep", "position": [100.0 + distance, 300], "engagement": "return_fire_only" }
        ], "events": [], "scripts": []
    })).unwrap()
}

#[test]
fn minimum_range_holds_close_shots_for_automatic_and_ordered_fire() {
    for id in ["grenade", "atgm"] {
        for ordered in [false, true] {
            let mut b = Battle::new(&minimum_range_setup(id, 4.0, 0.0, false), 5);
            run(&mut b, 3);
            let target = b
                .observe(Side::Blue)
                .identified
                .first()
                .expect("the nearby enemy is identified")
                .id;
            if ordered {
                Commander::new().ok(
                    &mut b,
                    Side::Blue,
                    Order::Attack {
                        units: vec![UnitId(0)],
                        target: TargetRef::Identified { id: target },
                    },
                );
            }
            let before = own(&b, Side::Blue, 0).position;
            run(&mut b, 60);
            assert_eq!(
                mount(&b, Side::Blue, 0, 0).ammo,
                vec![Some(10)],
                "{id}, ordered={ordered}: a too-close target must not consume rounds"
            );
            assert_eq!(
                mount(&b, Side::Blue, 0, 0).reason,
                ActionReason::HoldingFire
            );
            assert_eq!(
                own(&b, Side::Blue, 0).position,
                before,
                "a too-close attack must not advance farther into the target"
            );
            Commander::new().ok(
                &mut b,
                Side::Red,
                Order::Move {
                    units: vec![UnitId(1)],
                    gesture: 1,
                    goal: [120.0, 300.0],
                    route: contract::command::RoutePolicy::Shortest,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                },
            );
            run(&mut b, 120);
            assert!(own(&b, Side::Red, 1).position[0] > 105.0);
            assert!(
                own(&b, Side::Blue, 0).weapon_poses[0].shots > 0,
                "{id}, ordered={ordered}: fire resumes when the target leaves the minimum range"
            );
        }
    }
}

#[test]
fn minimum_range_includes_the_boundary_and_is_measured_from_the_muzzle() {
    for id in ["grenade", "atgm"] {
        for (distance, muzzle_forward, fires) in [
            (4.99, 0.0, false),
            (5.0, 0.0, true),
            (5.01, 0.0, true),
            (6.0, 2.0, false),
            (4.0, -2.0, true),
        ] {
            let mut b = Battle::new(&minimum_range_setup(id, distance, muzzle_forward, false), 5);
            run(&mut b, 30);
            assert_eq!(
                own(&b, Side::Blue, 0).weapon_poses[0].shots > 0,
                fires,
                "{id}: target distance {distance}, muzzle offset {muzzle_forward}"
            );
        }
    }
}

#[test]
fn minimum_range_tries_a_farther_visible_soldier_when_the_nearest_is_too_close() {
    for infantry in [false, true] {
        let mut setup = minimum_range_setup("grenade", 6.0, 0.0, infantry);
        setup.units[1].kind = "rifle".into();
        setup.rules.weapons.get_mut("rifle").unwrap().damage = 0.0;
        let mut b = Battle::new(&setup, 5);
        run(&mut b, 3);
        let view = b.observe(Side::Blue);
        let target = &view.identified[0];
        let from = if infantry {
            b.unit(UnitId(0)).unwrap().members[0].position
                + sim::math::v3(0.0, 0.0, setup.rules.physics.infantry_muzzle_m)
        } else {
            sim::math::v3(
                100.0,
                300.0,
                2.0 * setup
                    .rules
                    .catalog
                    .by_id("jeep")
                    .hull()
                    .unwrap()
                    .half_extents_m[2]
                    * setup.rules.physics.vehicle_aim_height_fraction,
            )
        };
        let distance = |p: &[f64; 3]| {
            (sim::math::v3(p[0], p[1], p[2] + setup.rules.physics.infantry_aim_m) - from).length()
        };
        assert!(
            distance(&target.members[0]) < 5.0,
            "first aim point is too close"
        );
        assert!(
            target.members.iter().any(|p| distance(p) >= 5.0),
            "another aim point is valid"
        );
        run(&mut b, 60);
        let launcher = if infantry { 1 } else { 0 };
        assert!(own(&b, Side::Blue, 0).weapon_poses[launcher].shots > 0,
            "infantry={infantry}: a soldier inside minimum range must not mask farther valid aim points");
    }
}

#[test]
fn every_mount_aims_and_reloads_independently_and_aims_once_per_target() {
    // A rifle squad against an enemy rifle squad that only answers, close
    // enough for the grenade's arc to reach: a pinned range does not make a
    // lobbed round fly farther than its speed and drop allow.
    let reach = common::scenario_rules()["weapons"]["grenade"]["range_m"]
        .as_f64()
        .unwrap();
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [100.0 + 0.8 * reach, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    // Isolate mount acquisition from magazine/burst timing, tested separately.
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.aim_s = 2.0;
    rifle.reload_s = 0.5;
    rifle.magazine = None;
    let (rifle_aim, rifle_reload) = (rifle.aim_s, rifle.reload_s);
    let mut b = Battle::new(&setup, 5);
    // Until red's answering fire first suppresses blue (a tier, which slows reloads).
    let mut shots = Vec::new();
    let mut calm_until = u64::MAX;
    for _ in 0..600 {
        shots.extend(run(&mut b, 1));
        if own(&b, Side::Blue, 0).suppression != SuppressionTier::None {
            calm_until = calm_until.min(b.tick());
        }
    }
    let rifles = shots_by(&shots, 0, "rifle");
    let grenades = shots_by(&shots, 0, "grenade");
    let calm: Vec<u64> = rifles
        .iter()
        .copied()
        .filter(|&t| t <= calm_until)
        .collect();
    assert!(
        !rifles.is_empty() && !grenades.is_empty(),
        "both mounts engage the same squad (W01)"
    );
    assert!(
        rifles[0] >= ticks(rifle_aim),
        "the first rifle shot waits for aim"
    );
    // Repeated shots at the uninterrupted target need only the reload (W02).
    let gaps: Vec<u64> = calm.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.len() >= 2 && gaps.iter().all(|&g| g <= ticks(rifle_reload) + 1),
        "gaps {gaps:?}"
    );
    let grenade_gap = grenades.windows(2).map(|w| w[1] - w[0]).min().unwrap();
    assert!(grenade_gap >= ticks(weapon("grenade")["reload_s"].as_f64().unwrap()) - 1);
}

#[test]
fn each_weapon_takes_the_costliest_target_it_can_damage() {
    // Blue tank faces a red tank and a red rifle squad.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 260] },
            { "side": "red", "kind": "rifle", "position": [300, 340] },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 5);
    let ids: Vec<_> = b
        .observe(Side::Blue)
        .identified
        .iter()
        .map(|e| (e.id, e.kind))
        .collect();
    let tank_id = ids
        .iter()
        .find(|(_, k)| *k == common::unit_kind("tank"))
        .unwrap()
        .0;
    let rifle_id = ids
        .iter()
        .find(|(_, k)| *k == common::unit_kind("rifle"))
        .unwrap()
        .0;
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: tank_id }),
        "cannon on the tank"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).target,
        Some(TargetRef::Identified { id: rifle_id }),
        "HMG cannot hurt the tank"
    );
}

#[test]
fn an_explicit_attack_focuses_compatible_weapons_and_frees_the_rest() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 260] },
            { "side": "red", "kind": "rifle", "position": [300, 340] },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 3);
    let rifle_id = b
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == common::unit_kind("rifle"))
        .unwrap()
        .id;
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: rifle_id },
        },
    );
    run(&mut b, 2);
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: rifle_id }),
        "cannon obeys at once"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).target,
        Some(TargetRef::Identified { id: rifle_id })
    );
    let tank_id = b
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == common::unit_kind("tank"))
        .unwrap()
        .id;
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank_id },
        },
    );
    run(&mut b, 2);
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: tank_id })
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).target,
        Some(TargetRef::Identified { id: rifle_id }),
        "the HMG engages what it can hurt (W08)"
    );
}

#[test]
fn the_default_gun_fires_at_what_it_cannot_hurt_but_specialists_do_not() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 300] },
        ]),
        json!([]),
        json!([]),
    );
    let shots = run(&mut b, 120);
    assert!(
        !shots_by(&shots, 0, "rifle").is_empty(),
        "rifles keep firing (W09)"
    );
    assert!(
        shots_by(&shots, 0, "grenade").is_empty(),
        "the grenade cannot hurt a tank"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 1).reason,
        ActionReason::NoCompatibleTarget
    );
}

#[test]
fn a_default_gun_with_a_finite_supply_keeps_it_for_what_it_can_hurt() {
    // The same squad and tank as above, but the rifles carry a counted supply.
    let mut rules = rules();
    rules["weapons"]["rifle"]["ammo"] = json!(200);
    rules["service"]["round_costs"]["rifle"] = json!(1);
    let map: Value = serde_json::from_str(&map(json!([]))).unwrap();
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 300] },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 5);
    let shots = run(&mut b, 120);
    assert!(
        shots_by(&shots, 0, "rifle").is_empty(),
        "no counted round spent on a tank it cannot hurt"
    );
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).reason,
        ActionReason::NoCompatibleTarget
    );
}

/// Blue's rifle squad; a red squad hidden behind a low building 250 m east
/// fires every second, leaving firing areas (never identified). From tick 30
/// a red tank drives in from beyond sight, to stand in the open 290 m off.
fn an_area_then_a_tank() -> Battle {
    let fire: Vec<Value> = (0..40)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 1 } }))
        .collect();
    let mut setup = scenario_with(
        &map(
            json!([{ "kind": "building", "center": [330, 300], "yaw": 0, "half_extents": [8, 20, 2] }]),
        ),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [350, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [760, 150], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([{ "tick": 30, "side": "red", "order":
            { "kind": "move", "units": [2], "gesture": 1, "goal": [350, 150], "route": "shortest" } }]),
    );
    // This selection experiment needs a lob over the building, independent of tuning.
    let grenade = &mut setup.rules.weapons.get_mut("grenade").unwrap().ballistics;
    (grenade.speed_mps, grenade.gravity_scale) = (24.0, 0.09);
    Battle::new(&setup, 5)
}

#[test]
fn an_area_draws_fire_only_while_no_identified_enemy_is_in_range() {
    let mut b = an_area_then_a_tank();
    let grenade_range = weapon("grenade")["range_m"].as_f64().unwrap();
    // Where the squad started: its soldiers shift a few metres into cover.
    let blue = sim::math::v2(100.0, 300.0);
    let mut grenade_at_area = false;
    let mut grenade_after_tank = Vec::new();
    let mut rifles_on_tank = false;
    for _ in 0..ticks(60.0) {
        let shots = run(&mut b, 1);
        let seen = b.observe(Side::Blue).identified.clone();
        let tank_in_reach = seen.iter().any(|e| {
            (sim::math::v2(e.position[0], e.position[1]) - blue).length() <= grenade_range - 10.0
        });
        let (rifles, grenade) = (mount(&b, Side::Blue, 0, 0), mount(&b, Side::Blue, 0, 1));
        if seen.is_empty() {
            grenade_at_area |= matches!(grenade.target, Some(TargetRef::Contact { .. }))
                && !shots_by(&shots, 0, "grenade").is_empty();
        }
        if tank_in_reach {
            grenade_after_tank.extend(shots_by(&shots, 0, "grenade"));
            for m in [&rifles, &grenade] {
                assert!(
                    !matches!(m.target, Some(TargetRef::Contact { .. })),
                    "tick {}: an area held with a tank in reach: {m:?}",
                    b.tick()
                );
            }
            rifles_on_tank |= rifles.target == Some(TargetRef::Identified { id: seen[0].id });
        }
    }
    assert!(
        grenade_at_area,
        "with no enemy in sight the grenade answers the area"
    );
    assert!(
        rifles_on_tank,
        "the rifles take the tank they cannot hurt over the area"
    );
    assert!(
        grenade_after_tank.is_empty(),
        "no grenade spent while a tank it cannot hurt is in reach: {grenade_after_tank:?}"
    );
}

#[test]
fn areas_draw_only_general_purpose_fire_and_never_ap() {
    // A red squad 700 m away, beyond every blue unit's optics, fires at
    // nothing; blue's tank and AT squad hold only its firing area.
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 2 } }))
        .collect();
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [120, 300] },
            { "side": "blue", "kind": "at", "position": [120, 250] },
            { "side": "red", "kind": "rifle", "position": [820, 300], "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([]),
    );
    let shots = run(&mut b, 600);
    assert!(b.observe(Side::Blue).identified.is_empty(), "never seen");
    assert!(
        !shots_by(&shots, 0, "tank_he").is_empty(),
        "the cannon answers the area with HE"
    );
    assert!(
        shots_by(&shots, 0, "tank_ap").is_empty(),
        "never AP at an area"
    );
    assert!(
        shots_by(&shots, 1, "atgm").is_empty(),
        "the ATGM is kept for identified armour"
    );
    // The AP round loaded at the start was swapped out, not lost.
    let cannon = mount(&b, Side::Blue, 0, 0);
    assert_eq!(
        cannon.ammo[0],
        Some(weapon("tank_ap")["ammo"].as_u64().unwrap() as u32)
    );
}

#[test]
fn a_stationary_weapon_loses_aim_and_unfinished_reload_when_the_unit_moves() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300] },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, ticks(1.5));
    let aiming = mount(&b, Side::Blue, 0, 1);
    assert!(
        aiming.aim > 0.3 && aiming.aim < 1.0,
        "part-aimed: {}",
        aiming.aim
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 200.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    run(&mut b, 3);
    let moving = mount(&b, Side::Blue, 0, 1);
    assert_eq!(moving.reason, ActionReason::MovingStationaryWeapon);
    assert_eq!(moving.aim, 0.0, "movement clears the ATGM's aim (W03)");
    assert_eq!(moving.loaded, Some(0), "the loaded missile stays loaded");
    // The rifles, a mobile weapon, keep working on the move.
    assert_ne!(
        mount(&b, Side::Blue, 0, 0).reason,
        ActionReason::MovingStationaryWeapon
    );
}

#[test]
fn stop_clears_aim_and_unfinished_reload_once_and_fire_resumes() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [400, 300] },
        ]),
        json!([]),
        json!([]),
    );
    let first = run(&mut b, ticks(3.0));
    assert!(!shots_by(&first, 0, "tank_ap").is_empty(), "fired once");
    run(&mut b, ticks(2.0)); // part-way through the 6 s reload
    let before = mount(&b, Side::Blue, 0, 0);
    assert!(before.reload > 0.2);
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    b.step();
    let after = mount(&b, Side::Blue, 0, 0);
    assert!(
        after.reload < before.reload,
        "the unfinished reload restarted (W15)"
    );
    assert!(after.aim < 0.1, "aim cleared once");
    let resumed = run(&mut b, ticks(8.5));
    assert!(
        !shots_by(&resumed, 0, "tank_ap").is_empty(),
        "automatic fire restarts"
    );
}

#[test]
fn a_brief_loss_of_sight_keeps_the_acquisition_and_its_aim() {
    // Red's tank crosses behind a short wall; blue's tank keeps its lock.
    // At 300 m (inside the tank's 350 m optics); the wall's 6 m shadow hides the
    // tank for about a second, less than the 1.5 s grace.
    let wall =
        json!([{ "kind": "wall", "center": [250, 300], "yaw": 0, "half_extents": [1, 1.5, 5] }]);
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [400, 340], "route": "shortest" } }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [400, 250], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    let mut lock = None;
    let mut saw_grace = false;
    for _ in 0..600 {
        b.step();
        let cannon = mount(&b, Side::Blue, 0, 0);
        if cannon.reason == ActionReason::TrackingLastSighting {
            saw_grace = true;
            assert_eq!(
                cannon.target, lock,
                "the same acquisition through the grace (V12)"
            );
            assert!(cannon.aim > 0.0, "aim continues toward the last sighting");
            // An automatic fallback must not steal the lock during the grace.
            assert!(!matches!(cannon.target, Some(TargetRef::Contact { .. })));
        } else if let Some(t @ TargetRef::Identified { id }) = cannon.target {
            if saw_grace {
                assert!(
                    b.observe(Side::Blue).identified.iter().any(|e| e.id == id),
                    "the tank is seen again"
                );
                assert_eq!(
                    Some(t),
                    lock,
                    "reidentified within the grace: same handle, same lock"
                );
                return;
            }
            lock = Some(t);
        }
    }
    panic!("the tank must enter grace and be reacquired; saw grace: {saw_grace}");
}

#[test]
fn return_fire_only_answers_only_its_own_attacker() {
    // Blue squad on Return fire only; a red squad shoots it, another red squad does not.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [250, 250], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [250, 350] },
        ]),
        json!([]),
        json!([]),
    );
    let mut shots = Vec::new();
    let mut answered_attacker = false;
    for _ in 0..200 {
        shots.extend(run(&mut b, 1));
        if let Some(target) = mount(&b, Side::Blue, 0, 0).target {
            let answered: BTreeSet<_> = b
                .observe(Side::Blue)
                .identified
                .iter()
                .filter(|e| e.kind == common::unit_kind("recon"))
                .map(|e| e.id)
                .collect();
            assert!(
                matches!(target, TargetRef::Identified { id } if answered.contains(&id)),
                "only at the attacker: {target:?}"
            );
            answered_attacker = true;
        }
    }
    assert!(
        shots_by(&shots, 1, "rifle").is_empty(),
        "the holding red squad stays silent"
    );
    let blue = shots_by(&shots, 0, "rifle");
    assert!(
        !blue.is_empty(),
        "blue answers the recon that attacks it (W13)"
    );
    assert!(answered_attacker, "blue acquired its attacker");
}

#[test]
fn attack_orders_switch_to_fire_at_will_and_moves_keep_policy() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [250, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 3);
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 320.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    run(&mut b, 2);
    assert_eq!(
        own(&b, Side::Blue, 0).engagement,
        Engagement::ReturnFireOnly
    );
    let id = b.observe(Side::Blue).identified[0].id;
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id },
        },
    );
    run(&mut b, 2);
    assert_eq!(
        own(&b, Side::Blue, 0).engagement,
        Engagement::FireAtWill,
        "W14"
    );
}

#[test]
fn automatic_targets_never_move_a_unit_but_explicit_attacks_pursue() {
    // A red squad 850 m away: the scout sees it, the blue rifles (600 m) cannot reach it.
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "recon", "position": [100, 300] },
            { "side": "blue", "kind": "rifle", "position": [100, 250] },
            { "side": "red", "kind": "rifle", "position": [950, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut optics = common::game();
    sim::fixtures::patch_catalog(
        &mut optics,
        "units",
        "recon",
        json!({"sensors":{"ground_m":1000}}),
    );
    setup.rules.catalog = serde_json::from_value::<contract::scenario::Rules>(optics)
        .unwrap()
        .catalog;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 60);
    assert_eq!(
        own(&b, Side::Blue, 1).position[0],
        100.0,
        "no automatic pursuit (W18)"
    );
    let observation = b.observe(Side::Blue);
    assert!(
        !observation.identified.is_empty(),
        "the scout identifies the distant target"
    );
    let id = observation.identified[0].id;
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(1)],
            target: TargetRef::Identified { id },
        },
    );
    run(&mut b, 90);
    assert!(
        own(&b, Side::Blue, 1).position[0] > 104.0,
        "an attack order closes to firing range (W17)"
    );
}

/// The identified enemy nearest `at`, as blue sees it.
fn seen_near(b: &Battle, at: [f64; 2]) -> TargetRef {
    let o = b.observe(Side::Blue);
    let e = o
        .identified
        .iter()
        .min_by(|a, b| {
            let d = |e: &&contract::observation::IdentifiedUnit| {
                (e.position[0] - at[0]).hypot(e.position[1] - at[1])
            };
            d(a).total_cmp(&d(b))
        })
        .expect("an identified enemy");
    TargetRef::Identified { id: e.id }
}

#[test]
fn an_attack_order_prioritises_its_target_and_fights_others_until_it_can_shoot_it() {
    // Blue's rifles are ordered onto a squad 850 m off (the scout sees it;
    // rifles reach 600 m) while another red squad stands in range. Walking up,
    // they fire at the near squad, and switch to the ordered one once it is in
    // reach: an order names a priority, never a weapon held silent.
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "recon", "position": [100, 300] },
            { "side": "blue", "kind": "rifle", "position": [100, 250] },
            { "side": "red", "kind": "rifle", "position": [950, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [400, 420], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut optics = common::game();
    sim::fixtures::patch_catalog(
        &mut optics,
        "units",
        "recon",
        json!({"sensors":{"ground_m":1000}}),
    );
    setup.rules.catalog = serde_json::from_value::<contract::scenario::Rules>(optics)
        .unwrap()
        .catalog;
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 30);
    let (far, near) = (seen_near(&b, [950.0, 300.0]), seen_near(&b, [400.0, 420.0]));
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(1)],
            target: far,
        },
    );
    let shots = run(&mut b, ticks(3.0));
    assert_eq!(mount(&b, Side::Blue, 1, 0).target, Some(near));
    assert!(
        !shots_by(&shots, 1, "rifle").is_empty(),
        "the rifles fire while out of the ordered target's reach"
    );
    assert!(own(&b, Side::Blue, 1).position[0] > 104.0, "still closing");
    for _ in 0..ticks(120.0) {
        b.step();
        if mount(&b, Side::Blue, 1, 0).target == Some(far) {
            let gap = 950.0 - own(&b, Side::Blue, 1).position[0];
            assert!(gap <= 620.0, "switched only once in reach: {gap} m");
            return;
        }
    }
    panic!("never switched to the ordered target");
}

#[test]
fn an_attack_that_loses_its_target_fights_on_the_way_to_the_last_report() {
    // Red's ordered squad steps behind a wall out of rifle range; another red
    // squad stands in range. Searching, blue halts to fight it, as an
    // attack-move would, instead of walking past it.
    let wall =
        json!([{ "kind": "wall", "center": [700, 400], "yaw": 0, "half_extents": [1, 70, 5] }]);
    let scripts = json!([{ "tick": 20, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [730, 380], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(wall),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [100, 340] },
            { "side": "red", "kind": "rifle", "position": [720, 322], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [350, 120], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 5);
    let (ordered, other) = (seen_near(&b, [720.0, 322.0]), seen_near(&b, [350.0, 120.0]));
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: ordered,
        },
    );
    let mut unseen_for = 0;
    for _ in 0..900 {
        b.step();
        let o = b.observe(Side::Blue);
        if o.identified
            .iter()
            .any(|e| TargetRef::Identified { id: e.id } == ordered)
        {
            unseen_for = 0;
            continue;
        }
        unseen_for += 1;
        if unseen_for == 90 {
            let blue = own(&b, Side::Blue, 0);
            assert!(blue.goal.is_some(), "still searching");
            assert_eq!(blue.state, MoveState::Halted, "halted to fight");
            assert_eq!(mount(&b, Side::Blue, 0, 0).target, Some(other));
            return;
        }
    }
    panic!("the ordered squad was never lost");
}

#[test]
fn attack_move_halts_to_engage_and_resumes() {
    // Red shows itself past the end of a short wall, then steps behind it;
    // once its last-seen area fades the attack-move carries on.
    let wall =
        json!([{ "kind": "wall", "center": [280, 430], "yaw": 0, "half_extents": [1, 30, 5] }]);
    let scripts = json!([{ "tick": 90, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [300, 430], "route": "shortest" } }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [290, 480], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [700.0, 300.0],
        },
    );
    let mut halted_at = None;
    let mut resumed = false;
    for _ in 0..1500 {
        let x = own(&b, Side::Blue, 0).position[0];
        b.step();
        let u = own(&b, Side::Blue, 0);
        if u.mounts.iter().any(|m| m.reason == ActionReason::Firing)
            && (u.position[0] - x).abs() < 1e-9
        {
            assert_eq!(u.state, MoveState::Halted, "a hold to engage says so");
            halted_at.get_or_insert(x);
        }
        if halted_at.is_some_and(|h| u.position[0] > h + 10.0) {
            assert_eq!(u.state, MoveState::Moving);
            resumed = true;
            break;
        }
    }
    assert!(halted_at.is_some(), "stopped while it could engage");
    assert!(resumed, "and kept going once nothing remained in reach");
}

#[test]
fn friendly_vehicles_in_the_line_withhold_fire_but_infantry_do_not() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "blue", "kind": "tank", "position": [200, 300] },
            { "side": "red", "kind": "tank", "position": [400, 300] },
        ]),
        json!([]),
        json!([]),
    );
    run(&mut b, 5);
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).reason,
        ActionReason::FriendlyInLine,
        "P11"
    );
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "blue", "kind": "rifle", "position": [200, 300] },
            { "side": "red", "kind": "tank", "position": [400, 300] },
        ]),
        json!([]),
        json!([]),
    );
    let shots = run(&mut b, ticks(4.0));
    assert!(
        !shots_by(&shots, 0, "tank_ap").is_empty(),
        "infantry in the line does not withhold"
    );
}

#[test]
fn rounds_hit_whatever_they_meet_including_friendly_soldiers() {
    // Near the target, the descending HMG rounds cross the friendly soldiers' height.
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "blue", "kind": "rifle", "position": [340, 300] },
            { "side": "red", "kind": "rifle", "position": [400, 300] },
        ]),
        json!([]),
        json!([]),
    );
    // This collision experiment fixes its firing cadence; changing burst aim
    // must not change which incidental trajectory crosses the friendly line.
    for (weapon, rounds, interval) in [("rifle", 30, 0.25), ("hmg", 50, 0.2)] {
        setup.rules.weapons.get_mut(weapon).unwrap().magazine = Some(contract::weapons::Magazine {
            rounds,
            shot_interval_s: interval,
            burst: None,
        });
    }
    let mut b = Battle::new(&setup, 5);
    let friendly: BTreeSet<_> = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .map(|s| s.id)
        .collect();
    let mut blue_rounds = BTreeSet::new();
    let mut friendly_hits = 0;
    for _ in 0..300 {
        b.step();
        for e in b.flight_events() {
            if let FlightEvent::Impact(i) = e {
                if let sim::flight::Struck::Body(body) = i.struck {
                    if friendly.contains(&body.0) && blue_rounds.contains(&i.projectile) {
                        friendly_hits += 1;
                    }
                }
            }
        }
        blue_rounds.extend(
            b.rounds()
                .filter(|(_, r)| r.unit == UnitId(0))
                .map(|(p, _)| p.id),
        );
    }
    assert!(friendly_hits > 0, "P09: collisions ignore allegiance");
}

#[test]
fn cannon_rounds_swap_by_target_without_losing_any() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [400, 300] },
        ]),
        json!([]),
        json!([]),
    );
    let shots = run(&mut b, ticks(8.0));
    assert!(
        shots_by(&shots, 0, "tank_ap").is_empty(),
        "HE preferred against infantry"
    );
    let he_shots = shots_by(&shots, 0, "tank_he");
    // The loaded AP round is set aside: HE starts its reload from zero (W05).
    let reload = weapon("tank_he")["reload_s"].as_f64().unwrap();
    assert!(
        he_shots.first().is_some_and(|&t| t >= ticks(reload)),
        "{he_shots:?}"
    );
    let cannon = mount(&b, Side::Blue, 0, 0);
    let (ap, he) = (
        weapon("tank_ap")["ammo"].as_u64().unwrap() as u32,
        weapon("tank_he")["ammo"].as_u64().unwrap() as u32,
    );
    assert_eq!(
        cannon.ammo[0],
        Some(ap),
        "the unloaded AP round is conserved"
    );
    assert_eq!(
        cannon.ammo[1],
        Some(he - shots_by(&shots, 0, "tank_he").len() as u32)
    );
}

#[test]
fn combat_replays_to_identical_digests() {
    let setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "tank", "position": [100, 250] },
            { "side": "red", "kind": "rifle", "position": [300, 300] },
            { "side": "red", "kind": "at", "position": [320, 260] },
        ]),
        json!([]),
        json!([]),
    );
    let mut a = Battle::new(&setup, 9);
    let mut digests = Vec::new();
    for _ in 0..300 {
        a.step();
        digests.push(a.digest());
    }
    let mut again = Battle::from_replay(&setup, &a.replay()).unwrap();
    for d in digests {
        again.step();
        assert_eq!(again.digest(), d);
    }
}

#[test]
fn enemy_rounds_are_drawn_only_over_seen_ground_and_own_rounds_whole() {
    // Red's squad fires from behind a wall's end at blue; blue sees little of red's side.
    let wall =
        json!([{ "kind": "wall", "center": [300, 300], "yaw": 0, "half_extents": [1, 80, 6] }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "recon", "position": [320, 140] },
        ]),
        json!([]),
        json!([]),
    );
    let (mut enemy, mut own) = (0, 0);
    for _ in 0..240 {
        b.step();
        let f = b.observe(Side::Blue);
        for s in &f.projectiles {
            if s.own {
                own += 1;
            } else {
                enemy += 1;
                assert!(
                    s.path
                        .iter()
                        .all(|p| f.ground_visibility.visible(p[0], p[1])),
                    "an enemy segment lies over seen ground at every point"
                );
            }
        }
    }
    assert!(own > 0 && enemy > 0, "own {own}, enemy {enemy}");
}

#[test]
fn a_loaded_weapon_drops_a_target_it_can_no_longer_reach() {
    // The costlier red squad walks out of rifle range but stays identified by
    // blue's scout; the rifles take the nearer scout-class target instead of
    // holding a loaded round on the unreachable one (W07).
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [1100, 300], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [100, 340] },
            { "side": "red", "kind": "rifle", "position": [690, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [400, 420], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    // The squad must live to walk out of range: this is about which target a
    // loaded weapon holds, not how fast rifles kill or pin at long range.
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    let handle = |b: &Battle, kind: &str| {
        b.observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind(kind))
            .map(|e| e.id)
    };
    run(&mut b, 5);
    let squad = handle(&b, "rifle").expect("the squad is identified");
    assert_eq!(
        mount(&b, Side::Blue, 0, 0).target,
        Some(TargetRef::Identified { id: squad })
    );
    let mut switched = false;
    for _ in 0..1200 {
        b.step();
        if mount(&b, Side::Blue, 0, 0).target
            == handle(&b, "recon").map(|id| TargetRef::Identified { id })
        {
            switched = handle(&b, "rifle") == Some(squad);
            break;
        }
    }
    assert!(
        switched,
        "replaced while the out-of-range squad was still identified"
    );
}

#[test]
fn a_weapon_keeps_its_target_while_it_can_still_shoot_it() {
    // Blue's rifles take the nearer of two red squads; it then walks back past
    // the other but stays in range and in sight. The rifles stay on it: a
    // squad that swaps targets every time the ranking shifts flickers on
    // screen and wastes its aim (W07).
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [550, 300], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [100, 340] },
            { "side": "red", "kind": "rifle", "position": [300, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [340, 380], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    // Nobody dies or is pinned: this is about which target the rifles hold.
    let rifle = setup.rules.weapons.get_mut("rifle").unwrap();
    rifle.damage = 0.0;
    rifle.near_miss_suppression = 0.0;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 5);
    let first = mount(&b, Side::Blue, 0, 0).target;
    assert!(first.is_some(), "the rifles take a target");
    let mut swaps = 0;
    for _ in 0..ticks(40.0) {
        b.step();
        if mount(&b, Side::Blue, 0, 0).target != first {
            swaps += 1;
        }
    }
    let at = |id: u32| b.unit(UnitId(id)).unwrap().position.xy();
    assert!(
        (at(2) - at(0)).length() > 300.0,
        "the first target walked back past the other"
    );
    assert_eq!(swaps, 0, "the rifles never left their first target");
}

/// Blue's tank watches red's tank cross behind a wall 300 m away; the wall's
/// shadow at red's track is four times `wall_half_y` wide.
fn crossing(wall_half_y: f64) -> Battle {
    let wall = json!([{ "kind": "wall", "center": [250, 300], "yaw": 0, "half_extents": [1, wall_half_y, 5] }]);
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [400, 360], "route": "shortest" } }]);
    battle(
        wall,
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [400, 250], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    )
}

#[test]
fn after_the_grace_the_acquisition_is_cleared_and_restarts_from_zero() {
    // A 20 m shadow: about three seconds out of sight.
    let mut b = crossing(5.0);
    let (mut before, mut expired, mut after) = (None, false, None);
    for _ in 0..900 {
        b.step();
        let cannon = mount(&b, Side::Blue, 0, 0);
        match (before, cannon.target) {
            (None, Some(t @ TargetRef::Identified { .. })) => before = Some(t),
            (Some(old), t) if !expired => expired = t != Some(old),
            (Some(old), Some(t @ TargetRef::Identified { .. })) if expired => {
                assert_ne!(t, old, "reidentified after the grace: a new handle");
                after = Some(cannon.aim);
                break;
            }
            _ => {}
        }
    }
    assert!(expired, "the lock outlasted the grace");
    assert!(
        after.is_some_and(|aim| aim < 1.0),
        "aim restarts from zero: {after:?}"
    );
}

#[test]
fn an_explicit_attack_on_the_area_replaces_the_retained_acquisition() {
    let mut b = crossing(1.5);
    let mut c = Commander::new();
    let mut ordered = false;
    for _ in 0..600 {
        b.step();
        let cannon = mount(&b, Side::Blue, 0, 0);
        if !ordered && cannon.reason == ActionReason::TrackingLastSighting {
            let area = b.observe(Side::Blue).contacts[0].id;
            c.ok(
                &mut b,
                Side::Blue,
                Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Contact { id: area },
                },
            );
            b.step();
            assert_eq!(
                mount(&b, Side::Blue, 0, 0).target,
                Some(TargetRef::Contact { id: area })
            );
            ordered = true;
        } else if ordered && matches!(cannon.target, Some(TargetRef::Identified { .. })) {
            // The abandoned aim is not restored on reidentification.
            assert!(cannon.aim < 1.0, "aim {}", cannon.aim);
            return;
        }
    }
    panic!("ordered {ordered}, never reacquired");
}

/// A blue squad 615 m from a red squad, just beyond its 600 m sight and its
/// rifles' reach; the red squad fires once and blue orders the squad onto the
/// report it left. `approach`: red then walks on, into blue's sight, before
/// the report fades.
fn attack_on_a_hidden_shooter(approach: bool) -> Battle {
    let scripts = if approach {
        json!([{ "tick": 10, "side": "red", "order":
            { "kind": "move", "units": [1], "gesture": 1, "goal": [550, 300], "route": "shortest" } }])
    } else {
        json!([])
    };
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [715, 300], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 5, "fire": { "unit": 1 } }]),
        scripts,
    );
    for _ in 0..6 {
        b.step();
    }
    assert!(b.observe(Side::Blue).identified.is_empty(), "red unseen");
    let area = b.observe(Side::Blue).contacts[0].id;
    Commander::new().ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id: area },
        },
    );
    b
}

fn attack_target(b: &Battle) -> Option<sim::weapons::Target> {
    match b.unit(UnitId(0)).unwrap().orders.front() {
        Some(sim::units::UnitOrder::Attack { target, .. }) => Some(*target),
        _ => None,
    }
}

#[test]
fn an_attack_on_a_contact_carries_over_to_its_cause_once_identified() {
    let mut b = attack_on_a_hidden_shooter(true);
    b.step();
    assert!(matches!(
        attack_target(&b),
        Some(sim::weapons::Target::Contact(_))
    ));
    for _ in 0..ticks(8.0) {
        b.step();
        if !b.observe(Side::Blue).identified.is_empty() {
            break;
        }
    }
    assert!(
        !b.observe(Side::Blue).identified.is_empty(),
        "red walked into sight"
    );
    assert_eq!(
        attack_target(&b),
        Some(sim::weapons::Target::Unit(UnitId(1))),
        "on the tick it is identified, the attack names the shooter"
    );
    b.step();
    let own = own(&b, Side::Blue, 0);
    let id = b.observe(Side::Blue).identified[0].id;
    assert!(
        own.mounts
            .iter()
            .any(|m| m.target == Some(TargetRef::Identified { id })),
        "and a weapon locks onto it"
    );
}

#[test]
fn an_attack_on_a_contact_that_expires_unidentified_ends() {
    let mut b = attack_on_a_hidden_shooter(false);
    let lifetime = common::game()["sensors"]["contact_lifetime_s"]
        .as_f64()
        .unwrap();
    for _ in 0..ticks(lifetime - 1.0) {
        b.step();
    }
    assert!(attack_target(&b).is_some(), "held while the area lasts");
    for _ in 0..ticks(2.0) {
        b.step();
    }
    assert!(
        b.observe(Side::Blue).contacts.is_empty(),
        "the area expired"
    );
    assert_eq!(attack_target(&b), None, "and the attack with it");
}

#[test]
fn fresh_firing_after_expiry_cannot_revive_the_previous_contact_attack() {
    let lifetime = ticks(
        common::game()["sensors"]["contact_lifetime_s"]
            .as_f64()
            .unwrap(),
    );
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [715, 300], "engagement": "return_fire_only" },
        ]),
        json!([
            { "tick": 5, "fire": { "unit": 1 } },
            { "tick": 6 + lifetime, "fire": { "unit": 1 } },
        ]),
        json!([]),
    );
    run(&mut b, 6);
    let id = b.observe(Side::Blue).contacts[0].id;
    Commander::new().ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id },
        },
    );
    run(&mut b, lifetime);
    let frame = b.observe(Side::Blue);
    assert!(frame.identified.is_empty(), "the cause stays hidden");
    assert_eq!(
        frame.contacts.len(),
        1,
        "the fresh shot supplies new evidence"
    );
    assert_eq!(frame.contacts[0].id, id, "reuse the one visual slot");
    assert_eq!(frame.contacts[0].evidence_tick, b.tick());
    assert_eq!(
        attack_target(&b),
        None,
        "the expired episode's intent ends despite ID reuse"
    );
}

#[test]
fn fresh_firing_after_expiry_does_not_restore_old_return_fire_permission() {
    // The supply truck cannot see the rifleman, but he sees and actually fires
    // at it. No responding weapon can keep the evidence alive accidentally.
    let start = |events| {
        let mut b = battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "supply", "position": [100, 300], "engagement": "return_fire_only" },
                { "side": "red", "kind": "rifle", "position": [600, 300], "engagement": "return_fire_only" },
            ]),
            events,
            json!([]),
        );
        run(&mut b, 3);
        let target = b.observe(Side::Red).identified[0].id;
        let mut commander = Commander::new();
        commander.ok(
            &mut b,
            Side::Red,
            Order::Attack {
                units: vec![UnitId(1)],
                target: TargetRef::Identified { id: target },
            },
        );
        for _ in 0..180 {
            b.step();
            if b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)) {
                commander.ok(
                    &mut b,
                    Side::Red,
                    Order::SetEngagement {
                        units: vec![UnitId(1)],
                        policy: Engagement::ReturnFireOnly,
                    },
                );
                commander.ok(
                    &mut b,
                    Side::Red,
                    Order::Stop {
                        units: vec![UnitId(1)],
                    },
                );
                b.step();
                assert!(
                    b.observe(Side::Blue).identified.is_empty(),
                    "the rifleman stays unseen"
                );
                assert_eq!(b.observe(Side::Blue).contacts.len(), 1);
                return b;
            }
        }
        panic!("the rifleman never fired at the truck");
    };
    // Discover the exact episode endpoint from its publication; the second
    // battle differs only by an authored non-attacking shot on that boundary.
    let control = start(json!([]));
    let first = control.observe(Side::Blue).contacts[0].clone();
    let fresh_tick = first.expires_tick + 1;
    let mut b = start(json!([{ "tick": fresh_tick, "fire": { "unit": 1 } }]));
    assert_eq!(b.observe(Side::Blue).contacts[0], first);
    let remaining = fresh_tick - b.tick();
    run(&mut b, remaining);
    let frame = b.observe(Side::Blue);
    assert_eq!(frame.contacts.len(), 1);
    assert_eq!(frame.contacts[0].id, first.id);
    assert_eq!(frame.contacts[0].evidence_tick, fresh_tick);
    assert!(
        !b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)),
        "hearing a new shot does not restore permission earned in the expired episode"
    );
}

#[test]
fn a_short_contact_lifetime_preserves_return_fire_during_identification_grace() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "supply", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [250, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    setup.rules.sensors.contact_lifetime_s = 0.1;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 3);
    let target = b.observe(Side::Red).identified[0].id;
    let mut commander = Commander::new();
    commander.ok(
        &mut b,
        Side::Red,
        Order::Attack {
            units: vec![UnitId(1)],
            target: TargetRef::Identified { id: target },
        },
    );
    for _ in 0..180 {
        b.step();
        if b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)) {
            break;
        }
    }
    assert!(
        b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)),
        "an actual shot grants permission"
    );
    commander.ok(
        &mut b,
        Side::Red,
        Order::SetEngagement {
            units: vec![UnitId(1)],
            policy: Engagement::ReturnFireOnly,
        },
    );
    commander.ok(
        &mut b,
        Side::Red,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [500.0, 300.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    let mut lost = None;
    for _ in 0..3000 {
        b.step();
        let frame = b.observe(Side::Blue);
        if let Some(report) = frame.contacts.first() {
            lost.get_or_insert(report.expires_tick);
        }
        if lost.is_some_and(|expiry| b.tick() == expiry + 1) {
            assert!(frame.contacts.is_empty(), "the short area has expired");
            assert!(
                b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)),
                "the independently retained identification grace still owns permission"
            );
            run(&mut b, ticks(2.0));
            assert!(
                !b.unit(UnitId(0)).unwrap().attackers.contains(&UnitId(1)),
                "permission ends once neither identification nor contact remains"
            );
            return;
        }
    }
    panic!("the rifleman never left sight and expired its short report");
}

#[test]
fn renewed_contact_starts_a_new_acquisition_after_positive_aim() {
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        json!([
            { "tick": 5, "fire": { "unit": 1 } },
            { "tick": 9, "fire": { "unit": 1 } },
        ]),
        json!([]),
    );
    setup.rules.sensors.contact_lifetime_s = 0.1;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 8);
    let contact = b.observe(Side::Blue).contacts[0].id;
    let old = mount(&b, Side::Blue, 0, 0);
    assert_eq!(old.target, Some(TargetRef::Contact { id: contact }));
    assert!(
        old.aim > 0.0,
        "the previous episode really earned aim: {old:?}"
    );
    b.step();
    let fresh = &b.observe(Side::Blue).contacts[0];
    assert_eq!(fresh.id, contact);
    assert_eq!(fresh.evidence_tick, b.tick());
    let new = mount(&b, Side::Blue, 0, 0);
    assert_eq!(new.target, Some(TargetRef::Contact { id: contact }));
    assert!(
        new.aim < old.aim,
        "renewal earns aim anew: old {old:?}, fresh {new:?}"
    );
}

#[test]
fn a_remembered_tank_type_does_not_authorize_ap_against_its_firing_area() {
    // The tank drives beyond sight on flat ground: terrain cannot withhold
    // the ordered shot and conceal the ammunition choice this test owns.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [400, 300], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 600, "fire": { "unit": 1 } }]),
        json!([
            { "tick": 1, "side": "red", "order": { "kind": "move", "units": [1], "gesture": 1, "goal": [600, 300], "route": "shortest" } },
        ]),
    );
    run(&mut b, 600);
    let frame = b.observe(Side::Blue);
    assert!(frame.identified.is_empty(), "the tank moved beyond sight");
    assert_eq!(frame.contacts.len(), 1);
    let report = &frame.contacts[0];
    assert_eq!(report.source, contract::observation::ContactSource::Firing);
    assert_eq!(
        report.kind,
        Some(common::unit_kind("tank")),
        "the label remembers the identified type"
    );
    let id = report.id;
    Commander::new().ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id },
        },
    );
    for _ in 0..360 {
        for (_, unit, weapon) in run(&mut b, 1) {
            if unit == 0 && weapon.starts_with("tank_") {
                assert_eq!(
                    weapon, "tank_he",
                    "an uncertain area admits HE, never precision AP from remembered type"
                );
                return;
            }
        }
    }
    panic!("the cannon never fired into its ordered area");
}

#[test]
fn area_fire_at_a_contact_comes_down_within_its_area() {
    // A blue tank 500 m from a red squad it cannot see (its sight is 350 m).
    // The squad fires once, leaving blue a firing report; the tank is ordered
    // onto it. With launch spread off, each round passes a standing
    // soldier's middle exactly at its sampled aim point: every one inside the
    // squad-sized area, spread across it rather than piled at its centre.
    // This probe owns contact sampling, independently of the cinematic miss tail.
    let mut rules = rules();
    rules["physics"]["miss_fall_max_s"] = json!(0.0);
    rules["physics"]["min_spread_at_max_range_m"] = json!(0.0);
    for row in rules["weapons"].as_object_mut().unwrap().values_mut() {
        row["scatter_mrad"] = json!(0.0);
    }
    let map: Value = serde_json::from_str(&map(json!([]))).unwrap();
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [600, 300], "engagement": "return_fire_only" },
        ],
        "events": [{ "tick": 5, "fire": { "unit": 1 } }],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 5);
    for _ in 0..6 {
        b.step();
    }
    assert!(
        b.observe(Side::Blue).identified.is_empty(),
        "red stays unseen"
    );
    let area = b.observe(Side::Blue).contacts[0].clone();
    Commander::new().ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id: area.id },
        },
    );
    let aim_z = common::physics("infantry_aim_m");
    let centre = sim::math::v2(area.center[0], area.center[1]);
    let mut last: std::collections::BTreeMap<ProjectileId, sim::math::V3> = Default::default();
    let mut crossings = Vec::new();
    for _ in 0..ticks(40.0) {
        b.step();
        for (p, r) in b.rounds() {
            if r.unit != UnitId(0) {
                continue;
            }
            if let Some(&q) = last.get(&p.id) {
                // Coming down through the aim height: where it meant to pass.
                if q.z > aim_z && p.position.z <= aim_z {
                    // Intersect the ballistic arc, not its tick chord: at a
                    // shallow shell angle a millimetre of sag shifts x by metres.
                    let gravity = b.rules().physics.flight.gravity_mps2 * p.gravity_scale;
                    let dt = 1.0 / b.rules().tick_hz as f64;
                    let vz = p.velocity.z + gravity * dt;
                    let t = (vz + (vz * vz + 2.0 * gravity * (q.z - aim_z)).sqrt()) / gravity;
                    let at = q + p.velocity * t;
                    crossings.push((at.xy() - centre).length());
                }
            }
            last.insert(p.id, p.position);
        }
    }
    assert!(
        crossings.len() >= 5,
        "rounds at the area: {}",
        crossings.len()
    );
    let tolerance = 0.05; // solver tolerance
    for d in &crossings {
        assert!(
            *d <= area.radius + tolerance,
            "a round {d:.2} m from the centre of a {:.2} m area",
            area.radius
        );
    }
    let widest = crossings.iter().cloned().fold(0.0, f64::max);
    assert!(
        widest > area.radius / 2.0,
        "aimed across the area, not just its centre: widest {widest:.2} of {:.2}",
        area.radius
    );
}

#[test]
fn the_engagement_policy_switches_per_unit() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "rifle", "position": [100, 340] },
            { "side": "red", "kind": "rifle", "position": [300, 320], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::SetEngagement {
            units: vec![UnitId(1)],
            policy: Engagement::ReturnFireOnly,
        },
    );
    let shots = run(&mut b, 90);
    assert!(
        !shots_by(&shots, 0, "rifle").is_empty(),
        "the other unit keeps firing at will"
    );
    assert!(shots_by(&shots, 1, "rifle").is_empty());
    assert_eq!(
        mount(&b, Side::Blue, 1, 0).reason,
        ActionReason::HoldingFire
    );
}

#[test]
fn attack_move_halts_for_what_only_a_stationary_weapon_reaches() {
    // A tank 550 m away, inside the AT squad's own sight: its rifles cannot
    // hurt it, so only the stationary ATGM can engage.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [650, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 100.0],
        },
    );
    let shots = run(&mut b, 240);
    assert_eq!(own(&b, Side::Blue, 0).state, MoveState::Halted);
    assert!(
        !shots_by(&shots, 0, "atgm").is_empty(),
        "the halted launcher fires"
    );
}

#[test]
fn attack_move_never_halts_for_a_target_it_cannot_hurt() {
    // Only rifles, against a tank in rifle range: they may plink (W09), not stop.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 100.0],
        },
    );
    for _ in 0..150 {
        b.step();
        assert_ne!(own(&b, Side::Blue, 0).state, MoveState::Halted);
    }
    assert!(own(&b, Side::Blue, 0).position[1] < 290.0, "it kept moving");
}

#[test]
fn an_attack_pursues_the_last_report_never_the_hidden_unit() {
    // Red is seen past a wall's end, out of rifle range, then steps behind it.
    let wall =
        json!([{ "kind": "wall", "center": [700, 400], "yaw": 0, "half_extents": [1, 70, 5] }]);
    let scripts = json!([{ "tick": 20, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [730, 380], "route": "shortest" } }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [100, 340] },
            { "side": "red", "kind": "rifle", "position": [720, 322], "engagement": "return_fire_only" },
        ]),
        json!([]),
        scripts,
    );
    run(&mut b, 5);
    let id = b.observe(Side::Blue).identified[0].id;
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id },
        },
    );
    let mut last_seen = None;
    let mut unseen_for = 0;
    for _ in 0..900 {
        b.step();
        let o = b.observe(Side::Blue);
        if let Some(e) = o.identified.first() {
            last_seen = Some([e.position[0], e.position[1]]);
            unseen_for = 0;
            continue;
        }
        unseen_for += 1;
        // Two seconds on: the goal is still the last report, not where red went.
        if unseen_for == 60 {
            let goal = own(&b, Side::Blue, 0).goal.expect("still pursuing");
            let seen = last_seen.expect("seen first");
            assert!(
                (goal[0] - seen[0]).abs() < 1.0 && (goal[1] - seen[1]).abs() < 1.0,
                "{goal:?} vs {seen:?}"
            );
            let hidden = own(&b, Side::Red, 2).position;
            assert!(
                (hidden[1] - seen[1]).abs() > 3.0,
                "red moved on unseen: {hidden:?}"
            );
            return;
        }
    }
    panic!("red was never lost");
}

#[test]
fn a_cannon_loads_ap_against_armour_and_he_once_ap_is_spent() {
    // A tank duel: AP while any remains; once AP is spent the cannon fires HE,
    // which still hurts armour, but only by its armour fraction.
    let duel = |condition: Value| {
        let mut b = battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "tank", "position": [300, 300], "condition": condition },
                { "side": "red", "kind": "tank", "position": [600, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        let shots = run(&mut b, ticks(20.0));
        let cannon: Vec<String> = shots
            .iter()
            .filter(|s| s.1 == 0 && s.2.starts_with("tank_"))
            .map(|s| s.2.clone())
            .collect();
        (cannon, b.unit(UnitId(1)).unwrap().hp)
    };
    let (full, _) = duel(json!({}));
    assert_eq!(
        full.first().map(String::as_str),
        Some("tank_ap"),
        "armour draws AP first: {full:?}"
    );
    let (spent, hp) = duel(json!({ "spent": { "tank_ap": 20 } }));
    assert!(
        !spent.is_empty() && spent.iter().all(|w| w == "tank_he"),
        "{spent:?}"
    );
    let he = weapon("tank_he");
    let per_hit = he["damage"].as_f64().unwrap() * he["armor_fraction"].as_f64().unwrap();
    let lost = 100.0 - hp;
    assert!(lost > 0.0, "HE hurts armour partially");
    assert!(
        (lost / per_hit - (lost / per_hit).round()).abs() < 1e-9,
        "whole HE hits of {per_hit}: lost {lost}"
    );
    assert!(
        per_hit < weapon("tank_ap")["damage"].as_f64().unwrap(),
        "HE is far weaker than AP against armour"
    );
}

#[test]
fn a_tank_round_leaves_the_muzzle_past_the_hull_front_on_the_turret_bearing() {
    // The cannon is a realistic gun: its muzzle sits past the hull's front
    // face (about 5.9 m ahead of the hull centre at 2 m: its `mounts` row),
    // and the turret carries it round the hull origin.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [300, 300] },
            { "side": "red", "kind": "tank", "position": [500, 450], "yaw": std::f64::consts::PI },
        ]),
        json!([]),
        json!([]),
    );
    let hull_front = common::hull("tank").half_extents_m[0];
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
    let half = common::hull("tank").half_extents_m;
    let (hull_front, hull_top) = (half[0], 2.0 * half[2]);
    for deg in [90.0f64, 180.0, -90.0] {
        let a = deg.to_radians();
        // Inside the tank's rear sight (`sensors.sight_shape`).
        let squad = [300.0 + 80.0 * a.cos(), 300.0 + 80.0 * a.sin()];
        let mut b = battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "tank", "position": [300, 300] },
                { "side": "red", "kind": "tank", "position": [520, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
                { "side": "red", "kind": "rifle", "position": squad, "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        run(&mut b, 1);
        let tank = b
            .observe(Side::Blue)
            .identified
            .iter()
            .find(|e| e.kind == common::unit_kind("tank"))
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
            json!({ "side": "blue", "kind": "tank", "position": [300, 300] }),
            json!({ "side": "red", "kind": "rifle", "position": [300, 380], "engagement": "return_fire_only" }),
        ];
        if jeep {
            units.push(json!({ "side": "blue", "kind": "jeep", "position": [300, 303.5], "engagement": "return_fire_only" }));
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
            { "side": "blue", "kind": "tank", "position": [300, 300] },
            { "side": "red", "kind": "tank", "position": [520, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [220, 300], "engagement": "return_fire_only" },
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
        .find(|e| e.kind == common::unit_kind("tank"))
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
    serde_json::to_value(&common::rules().catalog.by_id("tank").mounts[n]).unwrap()
}

#[test]
fn engaged_rifles_and_hmgs_fire_bursts_then_reload_only_empty_magazines() {
    for (kind, weapon, seed) in [
        ("rifle", "rifle", 1),
        ("rifle", "rifle", 5),
        ("rifle", "rifle", 19),
        ("tank", "hmg", 1),
        ("tank", "hmg", 5),
        ("tank", "hmg", 19),
    ] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                { "side": "blue", "kind": kind, "position": [100, 300] },
                { "side": "red", "kind": "tank", "position": [1100, 550], "engagement": "return_fire_only" }
            ]),
            json!([]),
            json!([]),
        );
        let capacity = if kind == "rifle" { 30 } else { 8 };
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
        if kind == "rifle" {
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
    for (kind, weapon, index) in [("tank", "hmg", 1), ("rifle", "rifle", 0)] {
        let mut setup = scenario_with(
            &map(json!([])),
            json!([
                { "side": "blue", "kind": kind, "position": [100, 300] },
                { "side": "red", "kind": "tank", "position": [1100, 550], "engagement": "return_fire_only" }
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
        setup.rules.service.round_costs.insert(weapon.into(), 1);
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
            { "side": "blue", "kind": "rifle", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [1100, 550], "engagement": "return_fire_only" }
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
        json!([{ "side": "blue", "kind": "tank", "position": [100, 300] }]),
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
    setup.rules.service.round_costs.insert("hmg".into(), 1);
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
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [430, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
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
    gun.ballistics.range_m = 300.0;
    gun.ballistics.scatter_mrad = 8.0;
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
