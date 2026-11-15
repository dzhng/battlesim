//! A ground strike's footprint (`sim::strike::footprint_m`): how far from an
//! attack-ground aim a unit's rounds can do damage. The page draws it round the
//! attack-ground cursor, so it must say what the simulation does.
use contract::ballistics::Delivery;
use contract::catalog::{FlightModel, TypeIndex};
use contract::scenario::Rules;
use sim::strike::footprint_m;
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
fn a_footprint_holds_the_blast_and_grows_with_range_until_reach() {
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
        .filter(|&t| footprint_m(&rules, &arsenal, t, 0.0).is_some())
        .collect();
    assert!(
        !ground.is_empty(),
        "the roster has ground units that strike an area"
    );
    for &t in &ground {
        let id = &rules.catalog.ids()[t.0 as usize];
        for d in [0.0, 300.0, 1500.0] {
            let (base, plus) = (
                footprint_m(&rules, &arsenal, t, d).unwrap(),
                footprint_m(&wider, &wider_arsenal, t, d).unwrap(),
            );
            assert!(
                (plus - base - 10.0).abs() < 1e-9,
                "{id} at {d} m: {base} -> {plus}"
            );
        }
        // Farther fire strays farther, until it closes to its reach to fire.
        let near = footprint_m(&rules, &arsenal, t, 50.0).unwrap();
        let far = footprint_m(&rules, &arsenal, t, 1e6).unwrap();
        assert!(far >= near, "{id}");
        assert_eq!(
            far,
            footprint_m(&rules, &arsenal, t, 2e6).unwrap(),
            "{id}: past its reach it fires from its reach"
        );
    }
    assert!(
        ground
            .iter()
            .any(|&t| footprint_m(&rules, &arsenal, t, 1e6).unwrap()
                > footprint_m(&rules, &arsenal, t, 50.0).unwrap()),
        "some ground fire strays with range"
    );
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
