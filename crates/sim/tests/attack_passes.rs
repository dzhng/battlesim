use crate::common;
use crate::common::air_field::wing_limits;
use crate::common::Seat;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use contract::scenario::FixedWingFlight;
use serde_json::json;
use sim::air::pass::{advance, begin, choose_weapon, PassPhase, PassWeapon};
use sim::air::{Bounds, FixedWingState};
use sim::fixtures::{self, test_rules};
use sim::flight::Release;
use sim::math::v3;

fn ground_attack_battle(weapons: &[&str]) -> sim::battle::Battle {
    ground_attack_battle_with_bomb_ammo(weapons, 1)
}

fn ground_attack_battle_with_bomb_ammo(weapons: &[&str], bomb_ammo: u32) -> sim::battle::Battle {
    ground_attack_battle_with(weapons, bomb_ammo, |_| {}, &[])
}

/// A test jet with `weapons` ordered to attack the ground under a red test
/// tank at (2000, 2000), 1 km east of it in the middle of a 4 km field (far
/// inside the edge guard); `patch` edits the rules and `units` adds more.
fn ground_attack_battle_with(
    weapons: &[&str],
    bomb_ammo: u32,
    patch: impl FnOnce(&mut serde_json::Value),
    units: &[serde_json::Value],
) -> sim::battle::Battle {
    let mut rules = test_rules();
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
    fixtures::patch_catalog(&mut rules, "units", "test_jet", json!({ "mounts": mounts }));
    rules["weapons"]["air_bomb"]["ammo"] = json!(bomb_ammo);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    patch(&mut rules);
    let mut placed = vec![
        json!({ "side": "blue", "kind": "test_jet", "position": [1000, 2000], "yaw": 0.0 }),
        json!({ "side": "red", "kind": "test_tank", "position": [2000, 2000], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }),
    ];
    placed.extend(units.iter().cloned());
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": placed,
        "events": [], "scripts": []
    })).unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
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

/// The edge margin for the narrow fields of the order and reaction tests,
/// where a jet's turn circles would not fit inside the shipped margin.
const NARROW_FIELD_MARGIN_M: f64 = 50.0;

fn air_battle() -> sim::battle::Battle {
    let mut rules = test_rules();
    // A narrow field: the edge guard, not this test's subject, keeps out of it.
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(NARROW_FIELD_MARGIN_M);
    fixtures::patch_catalog(
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
        afterburner_scatter_multiplier: 1.8,
        edge_margin_m: 300.0,
        nose_on_deg: 2.0,
    }
}

/// Bounds far beyond any pass in the pass tests: no edge guard.
fn far() -> Bounds {
    Bounds::new(
        sim::math::v2(-100_000.0, -100_000.0),
        sim::math::v2(100_000.0, 100_000.0),
    )
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

/// A bombing run commits nose-on, releases within half a step of its release
/// point, and is over with the release: the jet lines up for the next one.
#[test]
fn a_bombing_run_releases_at_its_release_point_and_is_over() {
    let aim = v3(1000.0, 0.0, 0.0);
    let mut plan = begin(
        &state(250.0),
        0.0,
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
    // 100 m/s in 0.6 s steps: 60 m a step.
    let dt = 0.6;
    let mut released_at = None;
    for step in 0..20 {
        let mut current = state(plan.agl);
        current.pos.x = step as f64 * 60.0;
        let (_, cue) = advance(
            &mut plan,
            &current,
            &wing_limits(),
            &far(),
            aim,
            &release,
            dt,
        );
        if cue {
            assert!(released_at.is_none(), "one cue per pass");
            // Where this tick's step carries the jet.
            released_at = Some(current.pos.x + 60.0);
        }
    }
    let x = released_at.expect("the pass never released");
    assert!(
        (x - 700.0).abs() <= 30.0,
        "released {x} m along, more than half a step from the release point"
    );
    assert_eq!(plan.phase, PassPhase::LineUp);
}

/// A run the edge guard takes over is broken: it releases nothing, though
/// this tick would have carried it to its release point, and the jet lines
/// up again.
#[test]
fn a_run_the_edge_guard_takes_over_is_broken() {
    let aim = v3(1000.0, 0.0, 0.0);
    let mut plan = begin(
        &state(150.0),
        0.0,
        &wing_limits(),
        aim,
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(7),
    );
    let release = Release {
        fall_s: 3.0,
        ahead_m: 300.0,
        at: sim::math::v2(700.0, 0.0),
    };
    // The east edge lies just past the aim: by the release point the jet,
    // heading straight at it, is at its point of no return.
    let limits = wing_limits();
    let bounds = Bounds::new(
        sim::math::v2(-5_000.0, -5_000.0),
        sim::math::v2(700.0 + limits.edge_margin_m + 1.0, 5_000.0),
    );
    // Nose-on, 400 m short of its release point: it commits.
    let mut current = state(plan.agl);
    current.pos.x = 300.0;
    advance(&mut plan, &current, &limits, &bounds, aim, &release, 0.05);
    assert_eq!(plan.phase, PassPhase::Dive);
    // 100 m/s in 0.05 s steps: this step would carry it to the release point.
    current.pos.x = 697.0;
    let (_, cue) = advance(&mut plan, &current, &limits, &bounds, aim, &release, 0.05);
    assert!(!cue, "released as the edge guard turned it away");
    assert_eq!(plan.phase, PassPhase::LineUp);
}

#[test]
fn pass_height_is_seeded_between_floor_and_ceiling() {
    let p = begin(
        &state(250.0),
        0.0,
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
        0.0,
        &wing_limits(),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(9),
    );
    assert!(p.agl <= 150.0);
}

/// The pass ceiling is a height above the ground under the jet: over a
/// rise, a jet already under it keeps its height above ground rather than
/// counting the rise as height of its own.
#[test]
fn a_pass_over_raised_ground_keeps_the_height_above_it() {
    let wing = wing();
    let limits = wing_limits();
    let under = (limits.floor_agl_m + wing.strafe_ceiling_m) * 0.5;
    for ground in [5.0, 60.0] {
        let p = begin(
            &state(ground + under),
            ground,
            &limits,
            v3(1000.0, 0.0, ground),
            PassWeapon::Gun,
            &wing,
            &mut Rng::new(9),
        );
        assert_eq!(p.agl, under, "over {ground} m ground");
    }
}

#[test]
fn a_limited_ground_weapon_is_chosen_before_the_gun() {
    assert_eq!(
        choose_weapon(&[PassWeapon::Gun, PassWeapon::Bomb]),
        Some(PassWeapon::Bomb)
    );
    assert_eq!(choose_weapon(&[PassWeapon::Gun]), Some(PassWeapon::Gun));
}

/// Every fixed-wing row in the game's catalog.
fn fixed_wing_rows() -> Vec<String> {
    let rules: contract::scenario::Rules = serde_json::from_value(fixtures::rules()).unwrap();
    let catalog = &rules.catalog;
    let rows: Vec<String> = catalog
        .indices()
        .filter(|&t| {
            catalog.get(t).mobility.flight() == Some(contract::catalog::FlightModel::FixedWing)
        })
        .map(|t| catalog.id(t).to_string())
        .collect();
    assert!(!rows.is_empty(), "the game fields fixed-wing aircraft");
    rows
}

/// Every loadout fires at a legal target: each fixed-wing row in the game's
/// catalog, ordered to attack an enemy its rows engage, fires on it. A row
/// with pass weapons attacks a ground vehicle a friendly crew spots for it,
/// and a pass weapon fires or releases on its passes; a fighter with none
/// attacks an enemy fighter and fires on it.
#[test]
fn every_loadout_fires_at_a_legal_target() {
    let rules: contract::scenario::Rules = serde_json::from_value(fixtures::rules()).unwrap();
    let catalog = &rules.catalog;
    let vehicle = catalog
        .indices()
        .find(|&t| catalog.get(t).mobility.flight().is_none() && catalog.get(t).hull().is_some())
        .map(|t| catalog.id(t).to_string())
        .expect("the game fields a ground vehicle");
    for row in &fixed_wing_rows() {
        let battle_with = |others: serde_json::Value| {
            let mut units =
                vec![json!({ "side": "blue", "kind": row, "position": [1000, 2000], "yaw": 0.0 })];
            units.extend(others.as_array().unwrap().iter().cloned());
            let setup = serde_json::from_value(json!({
                "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
                "rules": fixtures::rules(),
                "units": units,
                "events": [], "scripts": []
            }))
            .unwrap();
            sim::battle::Battle::new(&setup, 1)
        };
        let alone = battle_with(json!([]));
        let jet = alone.unit(UnitId(0)).unwrap();
        let specs = alone.arsenal().specs(jet.kind);
        let pass_mounts: Vec<usize> = (0..jet.mounts.len())
            .filter(|&i| {
                specs[jet.mounts[i].spec].kinds.iter().any(|&k| {
                    sim::air::pass::pass_weapon(&alone.arsenal().weapons[k].def).is_some()
                })
            })
            .collect();
        let fighter = pass_mounts.is_empty();
        let (mut battle, mounts) = if fighter {
            let battle = battle_with(json!([
                { "side": "red", "kind": row, "position": [2600, 2000], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
            ]));
            (battle, (0..jet.mounts.len()).collect::<Vec<_>>())
        } else {
            let battle = battle_with(json!([
                { "side": "red", "kind": vehicle, "position": [2000, 2000], "engagement": "return_fire_only" },
                { "side": "blue", "kind": vehicle, "position": [2000, 2150], "engagement": "return_fire_only" }
            ]));
            (battle, pass_mounts)
        };
        let mut seen = None;
        for _ in 0..600 {
            battle.step();
            seen = battle.observe(Side::Blue).identified.first().map(|e| e.id);
            if seen.is_some() {
                break;
            }
        }
        let id = seen.unwrap_or_else(|| panic!("{row}: blue never sees its target"));
        let ack = battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Identified { id },
            },
        });
        assert_eq!(ack.error, None, "{row}");
        let fired = |b: &sim::battle::Battle| {
            let jet = b.unit(UnitId(0)).unwrap();
            mounts.iter().any(|&i| jet.mounts[i].shots > 0)
        };
        let mut dived = false;
        for _ in 0..300 * battle.rules().tick_hz {
            if fired(&battle) {
                break;
            }
            battle.step();
            let pass = battle.unit(UnitId(0)).unwrap().air.unwrap().pass;
            dived |= pass.is_some_and(|p| p.phase == PassPhase::Dive);
        }
        assert!(
            fired(&battle),
            "{row} attacked an enemy it engages without firing from mounts {mounts:?}"
        );
        assert!(dived || fighter, "{row} fired without flying a pass");
    }
}

/// Every fixed-wing row in the game's catalog lives the whole life a player
/// buys it for (D44): it arrives through a purchase, already flying, within
/// its authored wait, then turns for home on low fuel and leaves by its back
/// edge, refunded.
#[test]
fn every_fixed_wing_row_arrives_and_withdraws_on_low_fuel() {
    for row in &fixed_wing_rows() {
        bought_jet_arrives_and_withdraws_on_low_fuel(row);
    }
}

/// Blue buys `row` on an open 3 km field and the battle runs its life out.
fn bought_jet_arrives_and_withdraws_on_low_fuel(row: &str) {
    let mut rules = fixtures::rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        row,
        json!({ "roster": { "factions": ["us"] } }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [3000, 3000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "players": crate::common::human_duel(), "skirmish": {"rules": { "credits_per_minute": 200, "starting_minutes": 20, "preparation_s": 60, "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": { "entries": [
                { "side": "blue", "center": [1500, 10], "yaw": std::f64::consts::FRAC_PI_2 },
                { "side": "red", "center": [1500, 2990], "yaw": -std::f64::consts::FRAC_PI_2 }
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
        player: Some("blue-1".into()),
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
        player: Some("red-1".into()),
        ..envelope(1, Order::Ready)
    });
    let credits = |b: &sim::battle::Battle| b.observe(Side::Blue).seat().credits;
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
    // A test-defined cone: 200 m off at 200 m out is 45 degrees.
    let cone = 10_f64.to_radians();
    assert!(sim::weapons::nose_on(
        unit,
        v3(500.0, 1500.0, unit.position.z),
        700.0,
        cone
    ));
    assert!(!sim::weapons::nose_on(
        unit,
        v3(500.0, 1700.0, unit.position.z),
        700.0,
        cone
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

/// A bomb leaves the jet: on the tick it is released it is where the jet
/// is, and it falls onto the aim because the jet released it at the right
/// moment, not because it was placed there.
#[test]
fn a_bomb_leaves_the_jet_and_falls_onto_the_aim() {
    let blast_m = 20.0;
    let mut battle = ground_attack_battle_with(
        &["air_bomb"],
        1,
        |rules| {
            rules["weapons"]["air_bomb"]["targets"]["ground"]["scatter_mrad"] = json!(0.0);
            rules["weapons"]["air_bomb"]["blast_radius_m"] = json!(blast_m);
        },
        &[],
    );
    let mut bomb = None;
    for _ in 0..1200 {
        battle.step();
        if let Some((p, _)) = battle.rounds().find(|(_, r)| r.unit == UnitId(0)) {
            let jet = battle.unit(UnitId(0)).unwrap().position;
            assert!(
                (p.position - jet).length() < 15.0,
                "the bomb appeared {:.0} m from the jet",
                (p.position - jet).length()
            );
            bomb = Some(p.id);
            break;
        }
    }
    let bomb = bomb.expect("the jet never released its bomb");
    for _ in 0..600 {
        battle.step();
        let landed = battle.flight_events().iter().find_map(|e| match e {
            sim::flight::FlightEvent::Impact(i) if i.projectile == bomb => Some(i.point),
            _ => None,
        });
        if let Some(point) = landed {
            let miss = (point.xy() - sim::math::v2(2000.0, 2000.0)).length();
            assert!(miss < blast_m, "the bomb landed {miss:.0} m from the aim");
            return;
        }
    }
    panic!("the bomb never landed");
}

/// A jet holds its bombs while a friendly vehicle sits in their blast at
/// the aim, as any gun holds for a friendly in its line.
#[test]
fn a_bomb_holds_for_a_friendly_vehicle_in_its_blast() {
    let mut battle = ground_attack_battle_with(
        &["air_bomb"],
        1,
        |rules| rules["weapons"]["air_bomb"]["blast_radius_m"] = json!(20.0),
        &[
            json!({ "side": "blue", "kind": "test_tank", "position": [2000, 2008], "yaw": 0.0, "engagement": "return_fire_only" }),
        ],
    );
    let mut cued = false;
    for _ in 0..1200 {
        battle.step();
        assert!(
            battle.rounds().all(|(_, r)| r.unit != UnitId(0)),
            "the jet bombed its own tank"
        );
        cued |= battle.unit(UnitId(0)).unwrap().air.unwrap().pass_release;
    }
    // It held at the release point, not short of it.
    assert!(cued, "the jet never reached its release point");
}

#[test]
fn a_missile_pass_fires_the_gun_when_nose_on() {
    // A missile run is flown at a vehicle the side has identified (a recon
    // spots it), not at the bare point under it.
    let mut battle = run_field(
        run_rules(&["air_gun", "air_ground_missile"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [2000, 2100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [2000, 2000], "yaw": 0.0, "engagement": "return_fire_only" }
        ]),
    );
    battle.step();
    let tank = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    // The mounts are the test's own: the gun first, the missile second.
    for _ in 0..900 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let missile = unit.mounts[1].shots > 0;
        let gun = unit.mounts[0].shots;
        if missile || gun > 0 {
            let pass = unit.air.unwrap().pass.map(|p| p.weapon);
            assert_eq!(
                pass,
                Some(PassWeapon::GroundMissile),
                "no missile run at the tank"
            );
            assert!(gun > 0, "missile pass crossed without a nose-on gun burst");
            return;
        }
    }
    panic!("ground missile pass never reached a nose-on gun burst");
}

/// A store that may not fire at a bare point flies no run at one: a jet
/// carrying its gun and a missile that engages only vehicles, ordered onto
/// open ground, strafes it with the gun.
#[test]
fn a_vehicle_only_missile_leaves_a_ground_point_to_the_gun() {
    let mut rules = run_rules(&["air_gun", "air_ground_missile"]);
    common::json_ground_reach(&mut rules["weapons"]["air_ground_missile"], 2000.0);
    let mut battle = run_field(
        rules,
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    assert_eq!(
        battle.accept(attack_ground(1, [2000.0, 2000.0])).error,
        None
    );
    let hz = f64::from(battle.rules().tick_hz);
    for _ in 0..(RUN_WINDOW_S * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        assert_eq!(jet.mounts[1].shots, 0, "the missile fired at a bare point");
        if jet.mounts[0].shots > 0 {
            return;
        }
    }
    panic!("the jet never strafed the point with its gun");
}

/// An attack order on the ground takes only the weapons that can hit a
/// point; a weapon that can't keeps choosing its own targets. A jet sent to
/// strafe open ground beside an identified enemy tank still fires its
/// vehicle-only missile at the tank.
#[test]
fn a_vehicle_missile_fires_on_its_own_at_a_tank_beside_an_ordered_ground_point() {
    let mut rules = run_rules(&["air_gun", "air_ground_missile"]);
    common::json_ground_reach(&mut rules["weapons"]["air_ground_missile"], 2000.0);
    let mut battle = run_field(
        rules,
        json!([
            { "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [2000, 2100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [2000, 2000], "yaw": 0.0, "engagement": "return_fire_only" }
        ]),
    );
    // Ordered before anything is seen, so every shot below is fired under
    // the order. The point is open ground 50 m short of the tank.
    assert_eq!(
        battle.accept(attack_ground(1, [2000.0, 1950.0])).error,
        None
    );
    let hz = f64::from(battle.rules().tick_hz);
    for _ in 0..(RUN_WINDOW_S * hz) as u64 {
        battle.step();
        // The mounts are the test's own: the gun first, the missile second.
        if battle.unit(UnitId(0)).unwrap().mounts[1].shots > 0 {
            assert!(
                !battle.observe(Side::Blue).identified.is_empty(),
                "the missile fired at a tank its side hasn't identified"
            );
            return;
        }
    }
    panic!("the missile never fired on its own at the tank beside the ordered point");
}

/// The limited store a jet takes to a bare point is one that may fire at
/// it: carrying a vehicle-only missile ahead of its bombs, it bombs the
/// point instead of flying missile runs it can never fire.
#[test]
fn a_vehicle_only_missile_leaves_a_ground_point_to_the_bombs() {
    let mut rules = run_rules(&["air_gun", "air_ground_missile", "air_bomb"]);
    common::json_ground_reach(&mut rules["weapons"]["air_ground_missile"], 2000.0);
    let mut battle = run_field(
        rules,
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    assert_eq!(
        battle.accept(attack_ground(1, [2000.0, 2000.0])).error,
        None
    );
    let hz = f64::from(battle.rules().tick_hz);
    for _ in 0..(RUN_WINDOW_S * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        assert_eq!(jet.mounts[1].shots, 0, "the missile fired at a bare point");
        if jet.mounts[2].shots > 0 {
            return;
        }
    }
    panic!("the jet never bombed the point");
}

#[test]
fn an_air_missile_fires_at_an_enemy_in_its_cone_during_a_move() {
    let mut battle = air_battle();
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
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
    let mut rules = test_rules();
    // A narrow field: the edge guard, not this test's subject, keeps out of it.
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(NARROW_FIELD_MARGIN_M);
    if fire {
        rules["sensors"]["contact_lifetime_s"] = json!(100.0);
    }
    fixtures::patch_catalog(
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
    let mut rules = test_rules();
    // A narrow field: the edge guard, not this test's subject, keeps out of it.
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(NARROW_FIELD_MARGIN_M);
    fixtures::patch_catalog(
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
        player: None,
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
        player: None,
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
    let mut in_run = false;
    for _ in 0..5000 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let run = jet
            .air
            .is_some_and(|a| a.pass.is_some_and(|p| p.phase == PassPhase::Dive));
        if run && !in_run {
            starts += 1;
        }
        in_run = run;
    }
    assert!(starts >= 2, "explicit attack only made {starts} run(s)");
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

/// Attack-move picks up what lies along the way: an enemy its side knows
/// of, but farther off the leg than the jet's weapons reach, is left alone,
/// and the jet flies on to its goal.
#[test]
fn attack_move_leaves_an_enemy_beyond_reach_of_its_leg() {
    const REACH_M: f64 = 500.0;
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "air gun", "name": "Air Gun", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    rules["weapons"]["air_gun"]["targets"]["ground"]["range_m"] = json!(REACH_M);
    let off_leg = 1_000.0 + REACH_M * 3.0;
    let setup = serde_json::from_value(json!({
        "map": { "size": [5000, 3000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [300, 1000], "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [1900, off_leg - 100.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [1900, off_leg], "yaw": 0.0, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    attack_move(&mut battle, 1, [3_500.0, 1_000.0]);
    let mut known = false;
    for _ in 0..1_500 {
        battle.step();
        known |= !battle.observe(Side::Blue).identified.is_empty();
        let jet = battle.unit(UnitId(0)).unwrap();
        assert!(
            jet.air.unwrap().pass.is_none(),
            "the jet turned off its leg"
        );
        if jet.orders.is_empty() {
            break;
        }
    }
    assert!(known, "the recon sees the tank");
    let jet = battle.unit(UnitId(0)).unwrap();
    assert!(jet.orders.is_empty(), "the jet never reached its goal");
    assert_eq!(jet.mounts[0].shots, 0);
}

/// Attack-move picks up only what the jet can hit: a squad its side knows
/// of, well inside its missile's reach of the leg but of a class no row it
/// carries engages, is left alone, and the jet holds its leg to its goal.
#[test]
fn attack_move_leaves_an_enemy_no_row_engages() {
    const OFF_LEG_M: f64 = 400.0;
    let mut rules = test_rules();
    // The mount replaces the jet's own cannon: its one row is a missile
    // that engages vehicles alone.
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "gun", "name": "AGM", "weapons": ["air_ground_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    common::json_ground_reach(&mut rules["weapons"]["air_ground_missile"], OFF_LEG_M * 5.0);
    let setup = serde_json::from_value(json!({
        "map": { "size": [5000, 3000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [300, 1000], "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [1900, 1000.0 + OFF_LEG_M - 100.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [1900, 1000.0 + OFF_LEG_M], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    attack_move(&mut battle, 1, [3_500.0, 1_000.0]);
    let mut known = false;
    let mut strayed: f64 = 0.0;
    for _ in 0..1_500 {
        battle.step();
        known |= !battle.observe(Side::Blue).identified.is_empty();
        let jet = battle.unit(UnitId(0)).unwrap();
        strayed = strayed.max((jet.position.y - 1000.0).abs());
        if jet.orders.is_empty() {
            break;
        }
    }
    assert!(known, "the recon sees the squad");
    assert!(
        strayed < OFF_LEG_M / 4.0,
        "the jet turned {strayed:.0} m off its leg at a squad it cannot engage"
    );
    assert!(
        battle.unit(UnitId(0)).unwrap().orders.is_empty(),
        "the jet never reached its goal"
    );
}

#[test]
fn a_holding_jet_starts_no_runs() {
    let mut battle = holding_ground_battle("fire_at_will", false);
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
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
    // Red is ordered onto blue, and fires on it.
    let blue = battle
        .observe(Side::Red)
        .identified
        .first()
        .map(|t| t.id)
        .expect("red sees blue");
    let ack = battle.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(1)],
            target: TargetRef::Identified { id: blue },
        },
    });
    assert_eq!(ack.error, None);
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
        player: None,
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
    let mut in_run = false;
    for _ in 0..1800 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let run = jet
            .air
            .is_some_and(|a| a.pass.is_some_and(|p| p.phase == PassPhase::Dive));
        if run && !in_run {
            starts += 1;
        }
        in_run = run;
    }
    assert!(starts >= 2, "incoming fire interrupted explicit runs");
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
        player: None,
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
        "rules": test_rules(),
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
        player: None,
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

/// A jet carrying bombs and its gun answers a helicopter that fires on it
/// from above (hovering over a mesa higher than the jet flies): a bomb
/// can't reach an aircraft, so it strafes with the gun, and flies on
/// rather than circling on a release point no bomb could fall to.
#[test]
fn a_bomb_carrier_strafes_a_helicopter_attacker_above_it() {
    let mut rules = test_rules();
    let mount = |id: &str, weapon: &str| {
        json!({ "id": id, "name": "Test Mount", "weapons": [weapon], "turret": false,
            "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0] })
    };
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        // Armoured past the helicopter's fire, so it lives to answer it.
        json!({ "mounts": [mount("bombs", "air_bomb"), mount("gun", "air_gun")],
            "body": { "hull": { "hp": 1_000_000 } } }),
    );
    // The helicopter's gun reaches up to the jet, so it is an attacker.
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({ "mounts": [{ "id": "gun", "name": "Gun", "weapons": ["air_gun"], "turret": true,
            "pivot_m": [0, 0, 1.2], "muzzle_m": [1, 0, 0] }] }),
    );
    let jet_agl = rules["air"]["high_air"]["cruise_agl_m"].as_f64().unwrap();
    let setup = serde_json::from_value(json!({
        "map": {
            "size": [3000, 1200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "relief": [{ "kind": "mesa", "rect": [900, 300, 1500, 900], "height_m": jet_agl + 60.0, "side_degrees": 40 }]
        },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 600], "yaw": 0.0 },
            { "side": "red", "kind": "test_heli", "position": [1000, 600], "yaw": std::f64::consts::PI }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let mut strafed = false;
    let mut dived = false;
    let mut answered_from_below = None;
    for _ in 0..1800 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let heli = battle.unit(UnitId(1)).unwrap();
        assert!(
            jet.position.x.is_finite() && jet.position.y.is_finite() && jet.position.z.is_finite(),
            "the jet's flight went NaN"
        );
        let Some(pass) = jet.air.unwrap().pass else {
            continue;
        };
        answered_from_below.get_or_insert(heli.position.z > jet.position.z);
        assert_ne!(
            pass.weapon,
            PassWeapon::Bomb,
            "the jet flew a bomb run at a helicopter"
        );
        strafed |= pass.weapon == PassWeapon::Gun;
        dived |= pass.phase == PassPhase::Dive;
    }
    assert!(strafed, "the jet never answered the helicopter");
    assert!(dived, "the jet's strafing run never dived in");
    assert_eq!(
        answered_from_below,
        Some(true),
        "the helicopter flies above the jet when it answers"
    );
}

/// The test jet's own numbers for the run tests: every bound they assert is
/// derived from these, not from the shipped catalog.
const RUN_CRUISE_KMH: f64 = 540.0;
const RUN_TURN_DEG_S: f64 = 60.0;
const RUN_EDGE_MARGIN_M: f64 = 300.0;
const RUN_BOMB_CEILING_M: f64 = 200.0;
const RUN_STRAFE_CEILING_M: f64 = 130.0;
const RUN_GRAVITY_MPS2: f64 = 9.81;
const RUN_BOMB_GRAVITY_SCALE: f64 = 4.5;
const RUN_GUN_RANGE_M: f64 = 900.0;
/// The side of the square field the run tests fly over.
const RUN_FIELD_M: f64 = 4000.0;
/// Where the jet starts, heading east.
const RUN_START: [f64; 2] = [1000.0, 2000.0];
/// How long the run tests watch a jet attack.
const RUN_WINDOW_S: f64 = 120.0;

/// What the run tests' bounds are made of, from the numbers above.
struct RunBounds {
    speed: f64,
    /// The longest one run may take, from where the last one ended: flying
    /// straight out until a turn in leaves a run-in (2 s of flight plus a
    /// bomb's forward throw from its ceiling) and a turn diameter, a lap
    /// turning in, one more lap the edge guard may add, then the run-in.
    run_s: f64,
}

fn run_bounds() -> RunBounds {
    let speed = RUN_CRUISE_KMH / 3.6;
    let turn = RUN_TURN_DEG_S.to_radians();
    let lap_s = std::f64::consts::TAU / turn;
    let fall_s = (2.0 * RUN_BOMB_CEILING_M / (RUN_GRAVITY_MPS2 * RUN_BOMB_GRAVITY_SCALE)).sqrt();
    let leg = 2.0 * speed + speed * fall_s + 2.0 * speed / turn;
    RunBounds {
        speed,
        run_s: 2.0 * lap_s + 2.0 * leg / speed,
    }
}

/// The test rules for the run tests, with the test jet carrying `weapons`,
/// each on its own mount.
fn run_rules(weapons: &[&str]) -> serde_json::Value {
    let mut rules = test_rules();
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(RUN_EDGE_MARGIN_M);
    rules["air"]["fixed_wing"]["bomb_ceiling_m"] = json!(RUN_BOMB_CEILING_M);
    rules["air"]["fixed_wing"]["strafe_ceiling_m"] = json!(RUN_STRAFE_CEILING_M);
    rules["physics"]["gravity_mps2"] = json!(RUN_GRAVITY_MPS2);
    rules["weapons"]["air_bomb"]["gravity_scale"] = json!(RUN_BOMB_GRAVITY_SCALE);
    rules["weapons"]["air_bomb"]["ammo"] = json!(1000);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    // Rearming between runs is not these tests' subject.
    rules["weapons"]["air_bomb"]["reload_s"] = json!(1.0);
    common::json_ground_reach(&mut rules["weapons"]["air_gun"], RUN_GUN_RANGE_M);
    // Mounts merge by id: the first takes the place of the jet's own gun.
    let mounts: Vec<_> = weapons
        .iter()
        .enumerate()
        .map(|(i, weapon)| {
            let id = if i == 0 {
                "gun".into()
            } else {
                format!("stores_{i}")
            };
            json!({
                "id": id, "name": "Stores", "weapons": [*weapon],
                "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0]
            })
        })
        .collect();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "mobility": { "air": { "cruise_kmh": RUN_CRUISE_KMH, "turn_deg_s": RUN_TURN_DEG_S } },
            "mounts": mounts
        }),
    );
    rules
}

/// A battle on the run field with `units` placed, the test jet first.
fn run_field(rules: serde_json::Value, units: serde_json::Value) -> sim::battle::Battle {
    let setup = serde_json::from_value(json!({
        "map": { "size": [RUN_FIELD_M, RUN_FIELD_M], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": units,
        "events": [], "scripts": []
    }))
    .unwrap();
    sim::battle::Battle::new(&setup, 1)
}

/// A test jet carrying `weapons` at `RUN_START` on the run field, heading
/// east, under an explicit attack order on the ground at `aim`.
fn run_battle(weapons: &[&str], aim: [f64; 2]) -> sim::battle::Battle {
    let mut battle = run_field(
        run_rules(weapons),
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    let ack = battle.accept(attack_ground(1, aim));
    assert_eq!(ack.error, None);
    battle
}

fn attack_ground(seq: u64, aim: [f64; 2]) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [aim[0], aim[1], 0.0],
            },
        },
    }
}

/// One run: from the tick the jet committed (nose on the aim) to the tick it
/// ended, with how far out it committed, how far off the nose the aim lay
/// then, and the rounds it fired or dropped.
#[derive(Debug)]
struct Run {
    start_s: f64,
    end_s: Option<f64>,
    commit_m: f64,
    off_deg: f64,
    fired: u32,
}

/// What a jet flew over a window: its runs, and each stretch of one phase
/// (`None` with no run planned) with when it began and how long it lasted.
struct Flown {
    runs: Vec<Run>,
    stretches: Vec<(Option<PassPhase>, f64, f64)>,
}

fn rounds(battle: &sim::battle::Battle) -> u32 {
    battle
        .unit(UnitId(0))
        .unwrap()
        .mounts
        .iter()
        .map(|m| m.shots)
        .sum()
}

fn fly_runs(battle: &mut sim::battle::Battle, aim: [f64; 2], window_s: f64) -> Flown {
    let hz = f64::from(battle.rules().tick_hz);
    let aim = sim::math::v2(aim[0], aim[1]);
    let mut flown = Flown {
        runs: Vec::new(),
        stretches: Vec::new(),
    };
    let mut current: Option<(Option<PassPhase>, f64)> = None;
    let mut rounds_at_commit = 0;
    for tick in 1..=(window_s * hz) as u64 {
        let before = rounds(battle);
        let at = battle.unit(UnitId(0)).unwrap().position.xy();
        battle.step();
        let now = tick as f64 / hz;
        let jet = battle.unit(UnitId(0)).unwrap();
        let phase = jet.air.unwrap().pass.map(|p| p.phase);
        if current.is_some_and(|(p, _)| p == phase) {
            continue;
        }
        if let Some((previous, began)) = current {
            flown.stretches.push((previous, began, now - began));
            if previous == Some(PassPhase::Dive) {
                let run = flown.runs.last_mut().unwrap();
                run.end_s = Some(now);
                run.fired = rounds(battle) - rounds_at_commit;
            }
        }
        if phase == Some(PassPhase::Dive) {
            // The jet committed from where it was when this tick began.
            let to_aim = aim - at;
            let off = sim::math::wrap_angle(libm::atan2(to_aim.y, to_aim.x) - jet.yaw);
            flown.runs.push(Run {
                start_s: now,
                end_s: None,
                commit_m: to_aim.length(),
                off_deg: off.to_degrees(),
                fired: 0,
            });
            rounds_at_commit = before;
        }
        current = Some((phase, now));
    }
    if let Some((phase, began)) = current {
        flown.stretches.push((phase, began, window_s - began));
    }
    flown
}

/// The run contract, for a jet that started at `RUN_START` and attacked
/// `aim` for `window_s`: each run begins nose-on and is over (fired, or
/// broken) within the time to fly its own run-in; no phase is held longer
/// than one run's length; and runs that fire keep coming.
fn assert_runs_keep_firing(weapon: &str, aim: [f64; 2], flown: &Flown, window_s: f64) {
    let bounds = run_bounds();
    let approach_s = (sim::math::v2(aim[0], aim[1]) - sim::math::v2(RUN_START[0], RUN_START[1]))
        .length()
        / bounds.speed
        + bounds.run_s;
    let at = format!("{weapon} at {aim:?}");
    assert!(
        !flown.runs.is_empty(),
        "{at}: the jet never committed to a run"
    );
    for run in &flown.runs {
        assert!(
            run.off_deg.abs() <= 10.0,
            "{at}: a run began {:.0} deg off the aim, {:.0} m out, at {:.1}s",
            run.off_deg,
            run.commit_m,
            run.start_s
        );
        if let Some(end) = run.end_s {
            assert!(
                end - run.start_s <= run.commit_m / bounds.speed + 1.0,
                "{at}: a run committed {:.0} m out at {:.1}s lasted {:.1}s",
                run.commit_m,
                run.start_s,
                end - run.start_s
            );
        }
    }
    let first_end = flown.runs[0].end_s.unwrap_or(window_s);
    for &(phase, began, held) in &flown.stretches {
        let limit = if began < first_end {
            approach_s
        } else {
            bounds.run_s
        };
        assert!(
            held <= limit,
            "{at}: held {phase:?} for {held:.1}s from {began:.1}s, longer than {limit:.1}s"
        );
    }
    let mut last = 0.0;
    let mut limit = approach_s + bounds.run_s;
    for end in flown
        .runs
        .iter()
        .filter(|r| r.fired > 0)
        .filter_map(|r| r.end_s)
        .chain([window_s])
    {
        assert!(
            end - last <= limit,
            "{at}: no run fired between {last:.1}s and {end:.1}s; runs {:?}",
            flown.runs
        );
        last = end;
        limit = 2.0 * bounds.run_s;
    }
}

fn runs_keep_firing(aim: [f64; 2]) {
    for weapon in ["air_bomb", "air_gun"] {
        let mut battle = run_battle(&[weapon], aim);
        let flown = fly_runs(&mut battle, aim, RUN_WINDOW_S);
        assert_runs_keep_firing(weapon, aim, &flown, RUN_WINDOW_S);
    }
}

/// Strafing or bombing a ground target is back and forth: line up, commit
/// nose-on, fire or release, bank away, come round and go again. Every run
/// after the first must line up as well as the first.
#[test]
fn repeated_passes_each_dive_in_nose_on_and_fire() {
    // 1 km east of the jet, in the middle of the field (far inside the
    // edge guard).
    runs_keep_firing([2000.0, 2000.0]);
}

/// A target by the map's edge gets run after run like any other: the jet
/// commits when its nose comes on, fires or drops if it gets there before
/// the edge guard turns it away, and comes round again. It never circles
/// over the target unable to attack.
#[test]
fn runs_at_an_aim_near_an_edge_keep_firing() {
    for from_edge in [50.0, 100.0, 150.0] {
        runs_keep_firing([RUN_FIELD_M - from_edge, 2000.0]);
    }
}

#[test]
fn runs_at_an_aim_in_a_corner_keep_firing() {
    runs_keep_firing([RUN_FIELD_M - 100.0, RUN_FIELD_M - 100.0]);
}

/// A run at a target that dies is over at once: the jet doesn't fly on at
/// the wreck.
#[test]
fn a_run_ends_when_its_target_dies() {
    let mut battle = run_field(
        run_rules(&["air_gun"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [2000, 2100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [2000, 2000], "yaw": 0.0, "engagement": "return_fire_only", "condition": { "hp": 1 } }
        ]),
    );
    for _ in 0..30 {
        battle.step();
    }
    let tank = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    let phase = |b: &sim::battle::Battle| {
        b.unit(UnitId(0))
            .unwrap()
            .air
            .unwrap()
            .pass
            .map(|p| p.phase)
    };
    let mut in_run_when_it_died = None;
    for _ in 0..(60.0 * f64::from(battle.rules().tick_hz)) as u64 {
        let before = phase(&battle);
        battle.step();
        if !battle.unit(UnitId(2)).unwrap().alive() {
            in_run_when_it_died = Some(before == Some(PassPhase::Dive));
            break;
        }
    }
    assert_eq!(
        in_run_when_it_died,
        Some(true),
        "the tank must die to a run's fire"
    );
    battle.step();
    battle.step();
    assert_ne!(
        phase(&battle),
        Some(PassPhase::Dive),
        "the jet flew on at the dead tank"
    );
}

/// Every order a jet takes ends the run it was flying, whatever the order
/// and even if it repeats the attack: on the next tick no release cue or
/// dogfight link survives, and the jet is either off the attack or lining
/// up afresh from where it is, as a re-issued attack does, which then fires.
#[test]
fn every_order_ends_the_run_at_once() {
    let aim = [2000.0, 2000.0];
    let elsewhere = [1000.0, 3000.0];
    let orders: Vec<(&str, Order)> = vec![
        (
            "move",
            Order::Move {
                units: vec![UnitId(0)],
                gesture: 2,
                goal: elsewhere,
                route: contract::command::RoutePolicy::Shortest,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
            },
        ),
        (
            "attack-move",
            Order::AttackMove {
                units: vec![UnitId(0)],
                gesture: 2,
                goal: elsewhere,
            },
        ),
        (
            "stop",
            Order::Stop {
                units: vec![UnitId(0)],
            },
        ),
        ("attack elsewhere", attack_ground(2, elsewhere).order),
        ("the same attack", attack_ground(2, aim).order),
        (
            "engagement",
            Order::SetEngagement {
                units: vec![UnitId(0)],
                policy: contract::command::Engagement::FireAtWill,
            },
        ),
        (
            "afterburner",
            Order::SetAfterburner {
                units: vec![UnitId(0)],
                enabled: false,
            },
        ),
    ];
    let aim_v = sim::math::v2(aim[0], aim[1]);
    let bearing = |b: &sim::battle::Battle| {
        let to_aim = aim_v - b.unit(UnitId(0)).unwrap().position.xy();
        libm::atan2(to_aim.y, to_aim.x)
    };
    let plan = |b: &sim::battle::Battle| b.unit(UnitId(0)).unwrap().air.unwrap().pass;
    for phase in [PassPhase::LineUp, PassPhase::Dive] {
        for (name, order) in &orders {
            let mut battle = run_battle(&["air_gun"], aim);
            let hz = f64::from(battle.rules().tick_hz);
            // Mid-run: in a dive, where a fresh line-up would commit along a
            // different line than the one the dive is flying.
            let mut reached = false;
            for _ in 0..(60.0 * hz) as u64 {
                battle.step();
                reached = plan(&battle).is_some_and(|p| {
                    p.phase == phase
                        && (phase == PassPhase::LineUp || p.heading != bearing(&battle))
                });
                if reached {
                    break;
                }
            }
            assert!(reached, "{name}: the jet never reached {phase:?}");
            let from = bearing(&battle);
            let ack = battle.accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 2,
                queued: false,
                order: order.clone(),
            });
            assert_eq!(ack.error, None, "{name}");
            battle.step();
            let air = battle.unit(UnitId(0)).unwrap().air.unwrap();
            assert!(
                !air.pass_release,
                "{name} in {phase:?}: a release cue survived"
            );
            assert_eq!(air.dogfight_target, None, "{name} in {phase:?}");
            // Still attacking the same aim, it lines up afresh: either not
            // yet committed, or committed along the line from where it was.
            if let Some(p) = air.pass.filter(|p| p.aim.xy() == aim_v) {
                assert!(
                    p.phase == PassPhase::LineUp || p.heading == from,
                    "{name} in {phase:?}: the jet kept flying its old run {p:?}"
                );
            }
        }
    }
    // A re-issued attack on the same target starts a fresh run that fires.
    let mut battle = run_battle(&["air_gun"], aim);
    for _ in 0..(60.0 * f64::from(battle.rules().tick_hz)) as u64 {
        battle.step();
        if plan(&battle).is_some_and(|p| p.phase == PassPhase::Dive) {
            break;
        }
    }
    battle.accept(attack_ground(2, aim));
    let flown = fly_runs(&mut battle, aim, run_bounds().run_s * 2.0);
    assert!(
        flown.runs.iter().any(|r| r.fired > 0),
        "the re-issued attack never fired: {:?}",
        flown.runs
    );
}

/// Once its ordered target dies, a jet circles where the attack ended (the
/// doc's "where its last order or run ended"), not where it was before the
/// order, kilometres back.
#[test]
fn a_jet_circles_where_its_attack_ended() {
    const CRUISE_KMH: f64 = 540.0;
    const TURN_DEG_S: f64 = 60.0;
    let radius = CRUISE_KMH / 3.6 / TURN_DEG_S.to_radians();
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mobility": { "air": { "cruise_kmh": CRUISE_KMH, "turn_deg_s": TURN_DEG_S } } }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [6000, 2000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [400, 1000], "yaw": 0.0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_recon", "position": [3500, 1100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [3600, 1000], "yaw": 0.0, "engagement": "return_fire_only", "condition": { "hp": 1 } }
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
        .first()
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let start = battle.unit(UnitId(0)).unwrap().position.xy();
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    let hz = f64::from(battle.rules().tick_hz);
    let mut ended = None;
    for _ in 0..(120.0 * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        if jet.orders.is_empty() {
            ended = Some(jet.position.xy());
            break;
        }
    }
    let ended = ended.expect("the attack never ended");
    assert!(
        !battle.unit(UnitId(2)).unwrap().alive(),
        "the attack ended with the tank alive"
    );
    assert!(
        (ended - start).length() > 10.0 * radius,
        "the attack ended near the start"
    );
    // A circle round where it ended passes within its diameter of it.
    for _ in 0..(60.0 * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap().position.xy();
        assert!(
            (jet - ended).length() <= 3.0 * radius,
            "{:.0} m from where its attack ended, {:.0} m from where it began",
            (jet - ended).length(),
            (jet - start).length()
        );
    }
}

/// Two jets strafe the same target in trail, the wingman a little above
/// his lead. The wingman's rounds reach the lead's line only after the lead
/// has flown on, so the lead's current pose is clear of them and the pose
/// it flies into is not: the wingman holds rather than shoot it down.
#[test]
fn a_wingman_in_trail_never_strafes_through_his_lead() {
    for behind_m in [50.0, 70.0, 100.0, 140.0] {
        let mut battle = trail_strafe(behind_m);
        let jets = [UnitId(0), UnitId(1)];
        let mut fired = 0;
        for _ in 0..1200 {
            let blue: std::collections::BTreeMap<_, _> = battle
                .rounds()
                .filter(|(_, r)| jets.contains(&r.unit))
                .map(|(p, r)| (p.id, r.unit))
                .collect();
            battle.step();
            fired += battle
                .rounds()
                .filter(|(p, r)| r.unit == UnitId(1) && !blue.contains_key(&p.id))
                .count();
            for event in battle.flight_events() {
                let sim::flight::FlightEvent::Impact(hit) = event else {
                    continue;
                };
                let Some(shooter) = blue.get(&hit.projectile) else {
                    continue;
                };
                let struck = |jet: &UnitId| {
                    hit.struck
                        == sim::flight::Struck::Body(sim::flight::BodyId(
                            sim::weapons::VEHICLE_BODY_BASE + jet.0,
                        ))
                };
                if let Some(jet) = jets.iter().find(|j| *j != shooter && struck(j)) {
                    panic!(
                        "{behind_m} m in trail: {shooter:?} shot {jet:?} at tick {}",
                        battle.tick()
                    );
                }
            }
        }
        // Holding is not a freeze: once his lead is clear he strafes too.
        assert!(fired > 0, "{behind_m} m in trail: the wingman never fired");
    }
}

/// A test-jet lead at (500, 2000) and its wingman `behind_m` behind it,
/// both strafing the ground under a red tank 2.5 km east with their guns.
/// The strafe band reaches well below cruise, so their seeded pass heights
/// stand the wingman some metres above his lead.
fn trail_strafe(behind_m: f64) -> sim::battle::Battle {
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "test_mount_0", "name": "Test Mount", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0]
        }] }),
    );
    rules["air"]["high_air"]["floor_agl_m"] = json!(40);
    rules["air"]["fixed_wing"]["strafe_ceiling_m"] = json!(240);
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 2000], "yaw": 0.0 },
            { "side": "blue", "kind": "test_jet", "position": [500.0 - behind_m, 2000], "yaw": 0.0 },
            { "side": "red", "kind": "test_tank", "position": [3000, 2000], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0), UnitId(1)],
            target: TargetRef::Ground {
                point: [3000.0, 2000.0, 0.0],
            },
        },
    });
    assert_eq!(ack.error, None);
    battle
}
