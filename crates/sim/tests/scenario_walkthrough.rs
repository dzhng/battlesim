//! Every shipped scenario with a solution, played by it to its end: each
//! chapter opens in turn, every beat fires, no order the solution or the
//! director gives is refused, and the replay is the battle. It names no
//! scenario and pins nothing about one; a change that breaks a scenario's
//! story fails here by the beat that never fired.

use contract::maps::MapCategory;
use contract::scenario::Rules;
use sim::battle::Battle;

#[test]
fn every_scenario_plays_through_by_its_solution() {
    let catalogue = sim::maps::Catalogue::shipped();
    let rules: Rules = serde_json::from_value(sim::fixtures::rules()).unwrap();
    let mut played = 0;
    for id in catalogue.ids() {
        if catalogue.category(&id).unwrap() != MapCategory::Scenario {
            continue;
        }
        for name in catalogue.scenarios(&id).unwrap() {
            let Some(solution) = catalogue.scenario_solution(&id, &name).unwrap() else {
                continue;
            };
            let budget = (solution.max_minutes * 60.0) as u64;
            let document = catalogue.scenario_document(&id, &name).unwrap();
            let seed = document.battle_seed.value();
            let document = document.with_solution(solution).unwrap();
            let map = catalogue.load(&id).unwrap().definition;
            let setup = document
                .on(map, rules.clone(), &sim::skirmish::starter_rules())
                .unwrap_or_else(|e| panic!("{id}/{name}: {e}"));
            let chapters = setup.director.as_ref().unwrap().chapters.clone();
            let mut battle = Battle::new(&setup, seed);
            let limit = budget * u64::from(battle.rules().tick_hz);
            let mut openings = Vec::new();
            while battle.director().unwrap().chapter() < chapters.len() && battle.tick() < limit {
                let before = battle.director().unwrap().chapter();
                battle.step();
                if battle.director().unwrap().chapter() != before {
                    openings.push(battle.tick());
                }
            }
            let run = battle.director().unwrap();
            let unfired: Vec<_> = chapters
                .iter()
                .flat_map(|c| c.beats.iter().map(move |b| format!("{}.{}", c.id, b.id)))
                .filter(|at| run.fired_at(at.split('.').nth(1).unwrap()).is_none())
                .collect();
            assert!(
                unfired.is_empty(),
                "{id}/{name}: by minute {}, beats never fired: {unfired:?}",
                battle.tick() / u64::from(battle.rules().tick_hz) / 60
            );
            assert!(
                run.refusals().is_empty(),
                "{id}/{name}: refused {:?}",
                run.refusals()
            );
            assert!(
                openings.windows(2).all(|w| w[0] < w[1]),
                "chapters open in turn"
            );
            let mut replayed = Battle::from_replay(&setup, &battle.replay()).unwrap();
            while replayed.tick() < battle.tick() {
                replayed.step();
            }
            assert_eq!(
                replayed.digest(),
                battle.digest(),
                "{id}/{name}: the replay is the battle"
            );
            if let Some(dir) = std::env::var_os("WALKTHROUGH_REPLAY") {
                let path = std::path::Path::new(&dir).join(format!("{id}-{name}.replay.json"));
                std::fs::write(path, serde_json::to_vec(&battle.replay()).unwrap()).unwrap();
            }
            eprintln!(
                "{id}/{name}: played through in {} game seconds",
                battle.tick() / u64::from(battle.rules().tick_hz)
            );
            played += 1;
        }
    }
    assert!(played > 0, "the catalogue ships a scenario with a solution");
}
