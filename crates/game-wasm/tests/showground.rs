//! The showground stands a squad as a fresh battle squad stands: the
//! simulation's own scatter on flat ground, and each special weapon with the
//! soldier who first carries it.
use contract::ids::UnitId;
use contract::map::MapDefinition;
use contract::scenario::{Rules, ScenarioDefinition};
use game_wasm::{Showground, SquadAtRest};
use serde_json::json;
use sim::battle::Battle;

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

/// A fresh battle of `seed` fielding one squad of `kind` on flat ground.
fn fielded(rules: &Rules, kind: &str, seed: u32) -> (Battle, [f64; 2]) {
    let centre = [200.0, 200.0];
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": MapDefinition::<contract::map::BuildingDefinition>::flat([400.0, 400.0]),
        "rules": rules,
        "units": [{ "side": "blue", "kind": kind, "position": centre }],
    }))
    .unwrap();
    (Battle::new(&setup, u64::from(seed)), centre)
}

#[test]
fn a_squad_at_rest_stands_as_a_fresh_battle_squad_stands() {
    let rules = rules();
    let ground = showground(&rules);
    let kinds = squads(&rules);
    assert!(!kinds.is_empty(), "the game fields squads");
    // Operated special weapons seen, so the operators' check never passes
    // on squads that carry none.
    let mut operated = 0;
    for kind in &kinds {
        for seed in 0..20 {
            let squad = at_rest(&ground, kind, seed);
            let (battle, centre) = fielded(&rules, kind, seed);
            let unit = battle.unit(UnitId(0)).expect("the squad is fielded");
            assert_eq!(
                squad.soldiers.len(),
                unit.members.len(),
                "{kind}: one soldier a member"
            );
            for (soldier, member) in squad.soldiers.iter().zip(&unit.members) {
                assert_eq!(
                    (soldier.id, soldier.slot),
                    (member.id, member.slot),
                    "{kind}"
                );
                let offset = [member.position.x - centre[0], member.position.y - centre[1]];
                assert!(
                    (soldier.at[0] - offset[0]).abs() < 1e-9
                        && (soldier.at[1] - offset[1]).abs() < 1e-9,
                    "{kind} seed {seed}: slot {} stands at {:?}, the battle puts it at {offset:?}",
                    soldier.slot,
                    soldier.at,
                );
            }
            let operators: Vec<_> = unit.mounts.iter().map(|m| m.operator).collect();
            assert_eq!(
                squad.operators, operators,
                "{kind} seed {seed}: each weapon's operator"
            );
            operated += operators.iter().flatten().count();
        }
    }
    assert!(operated > 0, "some squad operates a special weapon");
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
