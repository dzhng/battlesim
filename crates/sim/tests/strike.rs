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

/// The test rules with the test jeep's one mount a launcher that strikes
/// only between a minimum range and its reach.
fn launcher_rules() -> serde_json::Value {
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
    fixture
}

/// Where the launcher stands, facing east along its aims.
const AT: [f64; 2] = [100.0, 300.0];

/// `units` of blue on a flat field under `fixture`, every one ordered to
/// strike the ground `d` metres east of [`AT`]: the battle and the order's
/// refusal, if it was refused whole.
fn shelling(
    fixture: &serde_json::Value,
    units: serde_json::Value,
    d: f64,
) -> (Battle, Option<contract::command::OrderError>) {
    let count = units.as_array().unwrap().len() as u32;
    let setup = serde_json::from_value(json!({
        "map": { "size": [1400, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: (0..count).map(UnitId).collect(),
            target: TargetRef::Ground {
                point: [AT[0] + d, AT[1], 0.0],
            },
        },
    });
    (b, ack.error)
}

fn jeep() -> serde_json::Value {
    json!({ "side": "blue", "kind": "test_jeep", "position": AT, "yaw": 0.0 })
}

/// Whether `unit` fired over `seconds`.
fn fires(b: &mut Battle, unit: UnitId, seconds: u32) -> bool {
    let mut fired = false;
    for _ in 0..seconds * b.rules().tick_hz {
        b.step();
        fired |= b.rounds().any(|(_, r)| r.unit == unit);
    }
    fired
}

fn moved(b: &Battle, unit: UnitId, from: [f64; 2]) -> f64 {
    (b.unit(unit).unwrap().position.xy() - sim::math::v2(from[0], from[1])).length()
}

/// The reach the page draws is the one the simulation holds to: a ground
/// unit strikes an aim within it from where it stands, and only there.
#[test]
fn a_ground_unit_strikes_an_aim_within_its_reach_from_where_it_stands() {
    let fixture = launcher_rules();
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let arsenal = Arsenal::new(&rules);
    let kind = rules.catalog.index("test_jeep").unwrap();
    let (near, far) = reach_m(&rules, &arsenal, kind).unwrap();
    for d in [near - 10.0, far + 10.0] {
        assert_eq!(
            footprint_m(&rules, &arsenal, kind, d),
            None,
            "the footprint at {d} m"
        );
    }
    for d in [near + 10.0, far - 10.0] {
        assert!(
            footprint_m(&rules, &arsenal, kind, d).is_some(),
            "the footprint at {d} m"
        );
        let (mut b, refused) = shelling(&fixture, json!([jeep()]), d);
        assert_eq!(refused, None);
        assert!(fires(&mut b, UnitId(0), 30), "it never struck at {d} m");
        assert!(
            moved(&b, UnitId(0), AT) < 0.5,
            "it moved to strike at {d} m"
        );
    }
}

/// Ordered to strike past its reach, a unit drives toward the aim until it
/// can strike it, stops there and fires, as an attack on a unit closes.
#[test]
fn an_attack_on_ground_past_reach_closes_until_it_can_strike() {
    let fixture = launcher_rules();
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let kind = rules.catalog.index("test_jeep").unwrap();
    let (_, far) = reach_m(&rules, &Arsenal::new(&rules), kind).unwrap();
    let d = far + 300.0;
    let (mut b, refused) = shelling(&fixture, json!([jeep()]), d);
    assert_eq!(refused, None);
    assert!(fires(&mut b, UnitId(0), 90), "it never struck");
    let closed = moved(&b, UnitId(0), AT);
    assert!(closed > 250.0, "it closed only {closed:.0} m of 300");
    // Its reach runs from its muzzle, a little ahead of where it stands.
    let left = d - (b.unit(UnitId(0)).unwrap().position.x - AT[0]);
    assert!(
        left < far + 5.0 && left > far - 100.0,
        "it stopped {left:.0} m from the aim"
    );
}

/// Ordered to strike inside its minimum range, the order is refused, and
/// says so: alone, the whole order is refused; beside a unit that can strike
/// there, that unit takes it and the one too close is refused on its own.
#[test]
fn an_attack_on_ground_inside_minimum_range_is_refused() {
    let fixture = launcher_rules();
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let kind = rules.catalog.index("test_jeep").unwrap();
    let (near, _) = reach_m(&rules, &Arsenal::new(&rules), kind).unwrap();
    let d = near - 50.0;
    let (mut b, refused) = shelling(&fixture, json!([jeep()]), d);
    assert_eq!(refused, Some(contract::command::OrderError::TooClose));
    assert!(!fires(&mut b, UnitId(0), 10), "it struck too close");

    let tank = json!({ "side": "blue", "kind": "test_tank", "position": [AT[0], AT[1] + 30.0], "yaw": 0.0 });
    let (mut b, refused) = shelling(&fixture, json!([jeep(), tank]), d);
    assert_eq!(refused, None, "the tank can strike there");
    b.step();
    let told = b.observe(Side::Blue).refusals.clone();
    assert_eq!(
        told,
        vec![contract::command::Refusal {
            unit: UnitId(0),
            reason: contract::command::OrderError::TooClose
        }]
    );
    let (mut tank_fired, mut jeep_fired) = (false, false);
    for _ in 0..20 * rules.tick_hz {
        b.step();
        tank_fired |= b.rounds().any(|(_, r)| r.unit == UnitId(1));
        jeep_fired |= b.rounds().any(|(_, r)| r.unit == UnitId(0));
    }
    assert!(tank_fired, "the tank never struck");
    assert!(!jeep_fired, "the jeep struck too close");
}

#[test]
fn a_salvo_from_a_moving_jet_widens_by_its_walk() {
    let rules = roster();
    let arsenal = Arsenal::new(&rules);
    // The same salvos with one more round each walk one more interval along
    // the jet's path, half of it on each side of the aim.
    let mut longer = rules.clone();
    for def in longer.weapons.values_mut() {
        if let Some(m) = def.magazine.as_mut() {
            m.rounds += 1;
        }
    }
    let longer_arsenal = Arsenal::new(&longer);
    let mut walked = 0;
    for t in (0..rules.catalog.ids().len() as u16).map(TypeIndex) {
        if !jet(&rules, t) {
            continue;
        }
        let (Some(before), Some(after)) = (
            footprint_m(&rules, &arsenal, t, 0.0),
            footprint_m(&longer, &longer_arsenal, t, 0.0),
        ) else {
            continue;
        };
        if after == before {
            continue;
        }
        walked += 1;
        let speed = rules.catalog.get(t).mobility.speeds_kmh().0 / 3.6;
        let id = &rules.catalog.ids()[t.0 as usize];
        assert!(
            arsenal
                .specs(t)
                .iter()
                .flat_map(|m| m.kinds.iter())
                .filter_map(|&k| arsenal.weapons[k].def.magazine)
                .any(|m| (after - before - m.shot_interval_s * speed / 2.0).abs() < 1e-6),
            "{id}: {before} -> {after}"
        );
    }
    assert!(walked > 0, "the roster flies jets that strike in salvos");
}
