//! The showground stands a squad as a fresh battle squad stands: the
//! simulation's own scatter on flat ground, and each special weapon with the
//! soldier who first carries it.
use contract::scenario::Rules;
use game_wasm::{Showground, SquadAtRest};
use sim::arrangement;
use sim::math::v2;
use sim::weapons::Arsenal;
use sim::world::WorldGeometry;

fn rules() -> Rules {
    serde_json::from_value(sim::fixtures::rules()).unwrap()
}

fn showground(rules: &Rules) -> Showground {
    Showground::admit(&serde_json::to_string(rules).unwrap()).unwrap()
}

/// Every kind in the game's catalog that fields soldiers on foot.
fn squads(rules: &Rules) -> Vec<String> {
    let catalog = &rules.catalog;
    catalog
        .ids()
        .iter()
        .filter(|id| catalog.by_id(id).hull().is_none() && catalog.by_id(id).slots().is_some())
        .cloned()
        .collect()
}

fn at_rest(ground: &Showground, kind: &str, seed: u32) -> SquadAtRest {
    serde_json::from_str(&ground.squad(kind, seed).unwrap()).unwrap()
}

#[test]
fn a_squad_at_rest_stands_as_the_battle_scatters_it() {
    let rules = rules();
    let ground = showground(&rules);
    // The battle's scatter, on flat ground of its own and about its own
    // centre: flat ground scatters the same anywhere.
    let world = WorldGeometry::flat([400.0, 400.0], &rules);
    let centre = v2(200.0, 200.0);
    let solid = |p: &sim::world::Prop| p.blocks(contract::map::MoverClass::Infantry);
    let kinds = squads(&rules);
    assert!(!kinds.is_empty(), "the game fields squads");
    for kind in &kinds {
        let count = rules.catalog.by_id(kind).slots().unwrap().len();
        for seed in 0..40 {
            let squad = at_rest(&ground, kind, seed);
            let battle = arrangement::squad_spots(
                &world,
                centre,
                count,
                &rules.infantry_movement,
                rules.physics.soldier_radius_m,
                &solid,
                &mut arrangement::rng(u64::from(seed), 0, 0),
            );
            assert_eq!(squad.soldiers.len(), count, "{kind}: one soldier a slot");
            for (soldier, spot) in squad.soldiers.iter().zip(&battle) {
                let offset = *spot - centre;
                assert!(
                    (soldier.at[0] - offset.x).abs() < 1e-9
                        && (soldier.at[1] - offset.y).abs() < 1e-9,
                    "{kind} seed {seed}: slot {} stands at {:?}, the battle puts it at {offset:?}",
                    soldier.slot,
                    soldier.at,
                );
            }
        }
    }
}

#[test]
fn every_special_weapon_starts_with_its_original_carrier() {
    let rules = rules();
    let ground = showground(&rules);
    let arsenal = Arsenal::new(&rules);
    let mut operated = 0;
    for kind in squads(&rules) {
        let specs = arsenal.specs(rules.catalog.index(&kind).unwrap());
        let squad = at_rest(&ground, &kind, 7);
        assert_eq!(
            squad.operators.len(),
            specs.len(),
            "{kind}: one entry a mount"
        );
        let slot_of = |id: u32| squad.soldiers.iter().find(|s| s.id == id).map(|s| s.slot);
        let mut holding = Vec::new();
        for (m, (spec, operator)) in specs.iter().zip(&squad.operators).enumerate() {
            if spec.squad {
                assert_eq!(
                    *operator, None,
                    "{kind}: the squad's gun {m} has no one operator"
                );
                continue;
            }
            match operator {
                Some(id) => {
                    let slot = slot_of(*id).expect("the operator is one of the squad");
                    assert!(
                        spec.carriers.contains(&slot),
                        "{kind}: mount {m} went to slot {slot}, which doesn't carry it"
                    );
                    assert!(
                        !holding.contains(id),
                        "{kind}: soldier {id} operates two weapons"
                    );
                    holding.push(*id);
                    operated += 1;
                }
                None => assert!(
                    squad
                        .soldiers
                        .iter()
                        .filter(|s| spec.carriers.contains(&s.slot))
                        .all(|s| squad.operators.contains(&Some(s.id))),
                    "{kind}: mount {m} has a free carrier but no operator"
                ),
            }
        }
    }
    assert!(operated > 0, "some squad carries a special weapon");
}

#[test]
fn a_kind_with_a_hull_is_refused() {
    let rules = rules();
    let ground = showground(&rules);
    let hull = rules
        .catalog
        .ids()
        .iter()
        .find(|id| rules.catalog.by_id(id).hull().is_some())
        .expect("the game fields vehicles");
    assert!(
        ground.at_rest(hull, 1).is_err(),
        "{hull} has a hull, not a squad"
    );
    assert!(ground.at_rest("no_such_kind", 1).is_err());
}
