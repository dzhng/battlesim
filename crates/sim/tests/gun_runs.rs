//! A gun run as a war film shows one: every fixed-wing row in the game that
//! carries a gun dives with its nose on the aim, fires along its nose, and
//! pulls out above the floor.
use crate::attack_runs::{run_field, run_rules, RUN_START};
use contract::catalog::AltitudeLayer;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::air::pass::{pass_weapon, Dive, PassPhase, PassWeapon};
use sim::fixtures;
use sim::math::{v3, V3};

/// The angle between two directions, radians.
fn angle(a: V3, b: V3) -> f64 {
    (a.dot(b) / (a.length() * b.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

/// Every fixed-wing row in the game's catalog with a gun it strafes with,
/// and the ids of its other weapons.
fn gun_rows() -> Vec<(String, Vec<String>)> {
    let rules: contract::scenario::Rules = serde_json::from_value(fixtures::rules()).unwrap();
    let catalog = &rules.catalog;
    let rows: Vec<String> = catalog
        .indices()
        .filter(|&t| {
            catalog.get(t).mobility.flight() == Some(contract::catalog::FlightModel::FixedWing)
        })
        .map(|t| catalog.id(t).to_string())
        .collect();
    let rows: Vec<_> = rows
        .into_iter()
        .filter_map(|row| {
            let battle = gun_run(&row, &[]);
            let jet = battle.unit(UnitId(0)).unwrap();
            let arsenal = battle.arsenal();
            let weapons: Vec<_> = (jet.mounts.iter())
                .flat_map(|m| arsenal.specs(jet.kind)[m.spec].kinds.iter())
                .map(|&k| &arsenal.weapons[k])
                .collect();
            weapons
                .iter()
                .any(|w| pass_weapon(&w.def) == Some(PassWeapon::Gun))
                .then(|| {
                    let others = weapons
                        .iter()
                        .filter(|w| pass_weapon(&w.def) != Some(PassWeapon::Gun))
                        .map(|w| w.id.clone())
                        .collect();
                    (row, others)
                })
        })
        .collect();
    assert!(!rows.is_empty(), "the game fields gun-armed jets");
    rows
}

/// `row` heading east over a flat field, ordered to strafe the open ground
/// 1.7 km ahead, its `empty` weapons out of rounds so the gun flies the run.
fn gun_run(row: &str, empty: &[String]) -> sim::battle::Battle {
    let mut rules = fixtures::rules();
    for id in empty {
        rules["weapons"][id.as_str()]["ammo"] = json!(0);
    }
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{ "side": "blue", "kind": row, "position": [1000, 2000], "yaw": 0.0 }],
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
            units: vec![UnitId(0)],
            target: TargetRef::Ground { point: AIM },
        },
    });
    assert_eq!(ack.error, None, "{row}");
    battle
}

const AIM: [f64; 3] = [2700.0, 2000.0, 0.0];

/// Every gun-armed jet's strafe: while it fires, the aim lies inside its
/// nose-on cone seen from the muzzle, nose and pitch both, and each round
/// leaves along the nose within its row's dispersion, not toward the aim
/// whatever the nose does. It flies along its nose, dives and pulls out at
/// a sane pitch, and never comes below its floor.
#[test]
fn every_gun_jet_dives_its_nose_onto_the_aim_and_fires_along_it() {
    // Every round's path off the nose across it (the solve lifts a round
    // above the nose against its drop, never across it) against its row's
    // dispersion, squared, and how many: their mean is that axis's variance.
    let (mut spread, mut fired) = (0.0, 0);
    for (row, others) in gun_rows() {
        let mut battle = gun_run(&row, &others);
        let rules = battle.rules().clone();
        let cone = rules.air.fixed_wing.nose_on_deg.to_radians();
        let floor = rules.air.floor_agl_m(AltitudeLayer::HighAir).unwrap();
        let aim = V3 {
            x: AIM[0],
            y: AIM[1],
            z: AIM[2],
        };
        let mut seen = std::collections::BTreeSet::new();
        let (mut rounds, mut deepest) = (0, 0.0_f64);
        for _ in 0..90 * rules.tick_hz {
            battle.step();
            let jet = battle.unit(UnitId(0)).unwrap();
            let velocity = jet.air.unwrap().velocity;
            let nose = jet.nose();
            assert!(
                angle(velocity, nose) < 1e-6,
                "{row} flew off its nose's line"
            );
            assert!(
                jet.pitch().abs() < 45_f64.to_radians(),
                "{row} pitched {:.1} deg",
                jet.pitch().to_degrees()
            );
            assert!(
                jet.position.z >= floor - 1e-6,
                "{row} came down to {:.1} m",
                jet.position.z
            );
            deepest = deepest.max(-jet.pitch());
            for (round, fired) in battle.rounds() {
                if fired.unit != jet.id || !seen.insert(round.id) {
                    continue;
                }
                let def = &battle.arsenal().weapons[fired.weapon].def;
                assert_eq!(pass_weapon(def), Some(PassWeapon::Gun), "{row}");
                let scatter = def.against(AltitudeLayer::Ground).unwrap().scatter_mrad / 1000.0;
                let on_aim = angle(nose, aim - round.position);
                assert!(
                    on_aim <= cone + 1e-9,
                    "{row} fired {:.2} deg off the aim",
                    on_aim.to_degrees()
                );
                let along = angle(round.velocity - velocity, nose);
                assert!(
                    along <= 5.0 * scatter,
                    "{row}'s round left {:.2} deg off its nose",
                    along.to_degrees()
                );
                let across = nose.cross(v3(0.0, 0.0, 1.0)).normalized();
                let off = round.velocity.normalized().dot(across).asin();
                spread += (off / scatter).powi(2);
                rounds += 1;
            }
        }
        assert!(rounds > 0, "{row} never fired on its gun runs");
        fired += rounds;
        let dive = rules.air.fixed_wing.strafe_dive_deg.to_radians();
        assert!(
            deepest > 0.5 * dive,
            "{row} strafed without diving ({:.1} deg)",
            deepest.to_degrees()
        );
    }
    // The flight lays the gun, so its rounds spread as its row is authored,
    // with no penalty for moving (which would make them √2 wider).
    let sigma = (spread / f64::from(fired)).sqrt();
    assert!(
        sigma < 1.2,
        "jet guns spread {sigma:.2} times their authored dispersion"
    );
}

/// The gun's fire gate is the nose, heading and pitch both: a level jet
/// bears on a point level ahead of it, not on one off to the side, nor on
/// one below it it would need to dive at.
#[test]
fn the_gun_fires_only_nose_on_a_legal_target_in_range() {
    let battle = crate::common::air_field::AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    let unit = battle.unit(UnitId(0)).unwrap();
    let (at, z) = (unit.position, unit.position.z);
    // A test-defined cone: 200 m off at 200 m out is 45 degrees.
    let cone = 10_f64.to_radians();
    assert!(sim::weapons::nose_on(
        unit,
        at,
        v3(500.0, 1500.0, z),
        700.0,
        cone
    ));
    assert!(!sim::weapons::nose_on(
        unit,
        at,
        v3(500.0, 1700.0, z),
        700.0,
        cone
    ));
    assert!(!sim::weapons::nose_on(
        unit,
        at,
        v3(500.0, 1500.0, z - 200.0),
        700.0,
        cone
    ));
    assert!(!sim::weapons::nose_on(
        unit,
        at,
        v3(1500.0, 1500.0, z),
        700.0,
        cone
    ));
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
    // The mounts are the test's own: the gun first, the missile second. The
    // run that first fires either is a missile run, and the gun fires on it
    // before it is over.
    let mut fired_in_run = false;
    for _ in 0..900 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let pass = unit.air.unwrap().pass;
        let fired = unit.mounts[1].shots > 0 || unit.mounts[0].shots > 0;
        if fired && !fired_in_run {
            assert_eq!(
                pass.map(|p| p.weapon),
                Some(PassWeapon::GroundMissile),
                "no missile run at the tank"
            );
            fired_in_run = true;
        }
        if fired_in_run && pass.is_none_or(|p| p.phase != PassPhase::Run) {
            assert!(
                unit.mounts[0].shots > 0,
                "missile pass crossed without a nose-on gun burst"
            );
            return;
        }
    }
    panic!("ground missile pass never ended");
}

/// Every gun-armed jet that carries a missile its run fires at the ground (a
/// stand-off missile fires on its own), sent at a lone tank a recon has
/// spotted (the tank itself, or the ground it stands on), dives on it and
/// pickles the missile on the way down its first dive: in the dive, with its
/// nose on its aim, before it pulls out; and the missile strikes the tank.
/// The tank outlasts the gun, so the missile's hit is its own.
#[test]
fn every_gun_jet_pickles_its_ground_missile_on_the_dive() {
    let mut fielded = 0;
    for (row, _) in gun_rows() {
        for at_the_tank in [true, false] {
            fielded += usize::from(pickles_on_its_first_dive(&row, at_the_tank));
        }
    }
    assert!(fielded > 0, "the game fields gun jets with ground missiles");
}

/// [`every_gun_jet_pickles_its_ground_missile_on_the_dive`] for one row,
/// ordered onto the tank or onto the ground under it; `false` for a row with
/// no missile its run fires.
fn pickles_on_its_first_dive(row: &str, at_the_tank: bool) -> bool {
    const TANK_AT: [f64; 2] = [2700.0, 2000.0];
    // The game's jets with the test units to spot and stand for them.
    let mut rules = fixtures::rules();
    rules["catalog"] = json!(fixtures::catalog_documents(fixtures::CatalogSet::Test));
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    );
    // Heading away, it lines up and turns in on the tank.
    let mut battle = run_field(
        rules,
        json!([
            { "side": "blue", "kind": row, "position": RUN_START, "yaw": std::f64::consts::PI },
            { "side": "blue", "kind": "test_recon", "position": [TANK_AT[0], TANK_AT[1] + 100.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": TANK_AT, "yaw": 0.0, "engagement": "return_fire_only" }
        ]),
    );
    let (jet, tank) = (UnitId(0), UnitId(2));
    let missile = |battle: &sim::battle::Battle, weapon: usize| {
        let def = &battle.arsenal().weapons[weapon].def;
        pass_weapon(def) == Some(PassWeapon::GroundMissile) && !def.fires_on_its_own
    };
    let unit = battle.unit(jet).unwrap();
    let carries = (unit.mounts.iter())
        .flat_map(|m| battle.arsenal().specs(unit.kind)[m.spec].kinds.iter())
        .any(|&k| missile(&battle, k));
    if !carries {
        return false;
    }
    battle.step();
    let id = (battle.observe(Side::Blue).identified.first())
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let target = if at_the_tank {
        TargetRef::Identified { id }
    } else {
        TargetRef::Ground {
            point: [TANK_AT[0], TANK_AT[1], 0.0],
        }
    };
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![jet],
            target,
        },
    });
    assert_eq!(ack.error, None, "{row}");
    let cone = battle.rules().air.fixed_wing.nose_on_deg.to_radians();
    let hull = sim::flight::BodyId(sim::weapons::VEHICLE_BODY_BASE + tank.0);
    let (mut missiles, mut struck) = (std::collections::BTreeSet::new(), false);
    for _ in 0..90 * battle.rules().tick_hz {
        battle.step();
        for event in battle.flight_events() {
            if let sim::flight::FlightEvent::Impact(i) = event {
                struck |=
                    missiles.contains(&i.projectile) && i.struck == sim::flight::Struck::Body(hull);
            }
        }
        let unit = battle.unit(jet).unwrap();
        let pass = unit.air.unwrap().pass;
        for (round, fired) in battle.rounds() {
            if fired.unit != jet || !missile(&battle, fired.weapon) || !missiles.insert(round.id) {
                continue;
            }
            let run = pass.expect("a missile leaves on a run");
            assert_eq!(
                run.dive,
                Some(Dive::Diving),
                "{row} pickled its missile out of the dive"
            );
            let off = angle(unit.nose(), run.aim - round.position);
            assert!(
                off <= cone,
                "{row} pickled its missile {:.1} deg off its aim",
                off.to_degrees()
            );
        }
        assert!(
            pass.is_none_or(|p| p.dive != Some(Dive::PulledOut)) || !missiles.is_empty(),
            "{row} pulled out of its first dive with its missile on the rail"
        );
        if struck {
            return true;
        }
    }
    panic!("{row}'s missile never struck the tank");
}
