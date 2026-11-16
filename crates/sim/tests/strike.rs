//! A ground strike's footprint (`sim::strike::footprint_m`): how far from an
//! attack-ground aim a unit's rounds can do damage, and the reach
//! (`sim::strike::reach_m`) a ground unit strikes from where it stands. The
//! page draws them round the attack-ground cursor and the unit, so they must
//! say what the simulation does.
use contract::ballistics::Delivery;
use contract::catalog::{FlightModel, TypeIndex};
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::json;
use sim::battle::Battle;
use sim::strike::{footprint_m, reach_m};
use sim::weapons::Arsenal;

/// The game's rules with its whole roster.
fn roster() -> Rules {
    serde_json::from_value(sim::fixtures::rules()).unwrap()
}

/// Every type in `rules` with a row that `pick` accepts, by index.
fn types_with(
    rules: &Rules,
    pick: impl Fn(&contract::weapons::WeaponDefinition) -> bool,
) -> Vec<TypeIndex> {
    let arsenal = Arsenal::new(rules);
    (0..rules.catalog.ids().len() as u16)
        .map(TypeIndex)
        .filter(|&t| {
            arsenal
                .specs(t)
                .iter()
                .flat_map(|m| m.kinds.iter())
                .any(|&k| pick(&arsenal.weapons[k].def))
        })
        .collect()
}

fn jet(rules: &Rules, t: TypeIndex) -> bool {
    rules.catalog.get(t).mobility.flight() == Some(FlightModel::FixedWing)
}

fn bomb(def: &contract::weapons::WeaponDefinition) -> bool {
    def.ballistics.delivery == Delivery::Unpowered
}

#[test]
fn a_jets_bomb_footprint_is_its_release_spread_wherever_the_jet_is_now() {
    let rules = roster();
    let arsenal = Arsenal::new(&rules);
    let bombers: Vec<_> = types_with(&rules, bomb)
        .into_iter()
        .filter(|&t| jet(&rules, t))
        .collect();
    assert!(!bombers.is_empty(), "the roster flies bombers");
    for t in bombers {
        let near = footprint_m(&rules, &arsenal, t, 100.0).unwrap();
        let far = footprint_m(&rules, &arsenal, t, 6000.0).unwrap();
        assert_eq!(
            near,
            far,
            "{}: a jet bombs from its run",
            rules.catalog.ids()[t.0 as usize]
        );
    }
}

#[test]
fn a_footprint_holds_the_blast_and_grows_with_range_within_reach() {
    let rules = roster();
    let arsenal = Arsenal::new(&rules);
    // Every round's blast reaches 10 m farther: every footprint reaches 10 m
    // farther, near or far.
    let mut wider = rules.clone();
    for def in wider.weapons.values_mut() {
        def.blast_radius_m += 10.0;
    }
    let wider_arsenal = Arsenal::new(&wider);
    let ground: Vec<_> = (0..rules.catalog.ids().len() as u16)
        .map(TypeIndex)
        .filter(|&t| rules.catalog.get(t).mobility.flight().is_none())
        .filter(|&t| reach_m(&rules, &arsenal, t).is_some())
        .collect();
    assert!(
        !ground.is_empty(),
        "the roster has ground units that strike an area"
    );
    for &t in &ground {
        let id = &rules.catalog.ids()[t.0 as usize];
        let (near, far) = reach_m(&rules, &arsenal, t).expect("a ground unit has a reach");
        for d in [near, (near + far) / 2.0, far] {
            let (base, plus) = (
                footprint_m(&rules, &arsenal, t, d).unwrap(),
                footprint_m(&wider, &wider_arsenal, t, d).unwrap(),
            );
            assert!(
                (plus - base - 10.0).abs() < 1e-9,
                "{id} at {d} m: {base} -> {plus}"
            );
        }
        // An attack on the ground never closes: past its reach, or inside
        // its minimum range, it strikes nothing there.
        assert_eq!(footprint_m(&rules, &arsenal, t, far + 1.0), None, "{id}");
        if near > 0.0 {
            assert_eq!(footprint_m(&rules, &arsenal, t, near - 1.0), None, "{id}");
        }
        assert!(
            footprint_m(&rules, &arsenal, t, far).unwrap()
                >= footprint_m(&rules, &arsenal, t, near).unwrap(),
            "{id}: farther fire strays farther"
        );
    }
    assert!(
        ground.iter().any(|&t| {
            let (near, far) = reach_m(&rules, &arsenal, t).unwrap();
            footprint_m(&rules, &arsenal, t, far).unwrap()
                > footprint_m(&rules, &arsenal, t, near).unwrap()
        }),
        "some ground fire strays with range"
    );
    assert!(
        ground
            .iter()
            .any(|&t| reach_m(&rules, &arsenal, t).unwrap().0 > 0.0),
        "some ground fire has a minimum range"
    );
}

/// The reach the page draws is the one the simulation holds to: ordered to
/// strike just inside its reach a ground unit fires, and just past it, or
/// just inside its minimum range, it holds where it stands.
#[test]
fn a_ground_unit_strikes_only_inside_its_reach_and_never_closes() {
    let mut fixture = sim::fixtures::test_rules();
    fixture["weapons"]["test_launcher"] = json!({
        "extends": "artillery_mortar", "range_m": 650, "min_range_m": 200, "round_cost": 1,
        "requires_clear_launch_path": false,
        "targets": { "ground": { "range_m": 650, "scatter_mrad": 26 } },
    });
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_jeep",
        // Its one mount, in place of its machine gun (mounts merge by id).
        json!({ "mounts": [{ "id": "HMG", "name": "Launcher", "weapons": ["test_launcher"],
            "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }] }),
    );
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let arsenal = Arsenal::new(&rules);
    let jeep = rules.catalog.index("test_jeep").unwrap();
    let (near, far) = reach_m(&rules, &arsenal, jeep).unwrap();
    let at = [100.0, 300.0];
    for (d, fires) in [
        (near - 10.0, false),
        (near + 10.0, true),
        (far - 10.0, true),
        (far + 10.0, false),
    ] {
        assert_eq!(
            footprint_m(&rules, &arsenal, jeep, d).is_some(),
            fires,
            "the footprint at {d} m"
        );
        let setup = serde_json::from_value(json!({
            "map": { "size": [1000, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": fixture,
            "units": [{ "side": "blue", "kind": "test_jeep", "position": at, "yaw": 0.0 }],
            "events": [], "scripts": [],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [at[0] + d, at[1], 0.0],
                },
            },
        });
        assert_eq!(ack.error, None);
        let mut fired = false;
        for _ in 0..30 * rules.tick_hz {
            b.step();
            fired |= b.rounds().any(|(_, r)| r.unit == UnitId(0));
        }
        assert_eq!(fired, fires, "struck at {d} m");
        let moved = b.unit(UnitId(0)).unwrap().position.xy() - sim::math::v2(at[0], at[1]);
        assert!(
            moved.length() < 0.5,
            "it closed {:.1} m on an aim {d} m off",
            moved.length()
        );
    }
}

#[test]
fn a_salvo_from_a_moving_jet_widens_by_its_walk() {
    let rules = roster();
    let salvo: Vec<_> = types_with(&rules, |d| bomb(d) && d.magazine.is_some())
        .into_iter()
        .filter(|&t| jet(&rules, t))
        .collect();
    assert!(!salvo.is_empty(), "the roster flies salvo bombers");
    let t = salvo[0];
    let before = footprint_m(&rules, &Arsenal::new(&rules), t, 0.0).unwrap();
    // The same salvo with one more round walks one more interval along the
    // jet's path, half of it on each side of the aim.
    let mut longer = rules.clone();
    let mut step = 0.0;
    for def in longer.weapons.values_mut() {
        if let (true, Some(m)) = (bomb(def), def.magazine.as_mut()) {
            m.rounds += 1;
            step = m.shot_interval_s;
        }
    }
    let after = footprint_m(&longer, &Arsenal::new(&longer), t, 0.0).unwrap();
    let speed = rules.catalog.get(t).mobility.speeds_kmh().0 / 3.6;
    assert!(
        (after - before - step * speed / 2.0).abs() < 1e-6,
        "{before} -> {after}"
    );
}
