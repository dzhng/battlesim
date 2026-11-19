//! Every shipped scenario with a solution, played by it to its end: each
//! chapter opens in turn, every beat fires, no order the solution or the
//! director gives is refused, and the replay is the battle. Then its latest
//! chapter restarts, resumed from that replay, and its beats fire again. It
//! names no scenario and pins nothing about one; a change that breaks a
//! scenario's story fails here by the beat that never fired.

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
                let now = battle.director().unwrap().chapter();
                if now != before {
                    openings.push((battle.tick(), now));
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
                openings.windows(2).all(|w| w[0].0 < w[1].0),
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
            // The last chapter to open, not the end of the last one.
            if let Some(&(opened, index)) = openings.iter().rfind(|(_, i)| *i < chapters.len()) {
                restarts(&setup, &battle, opened, &chapters[index], &id, &name);
                starts_at(&setup, seed, limit, &chapters[index], index, &id, &name);
            }
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

/// The chapter that opened at `opened`, restarted from `played`'s replay:
/// resumed at its opening, it plays out again and every one of its beats
/// fires again. The resume's cost prints as a loose tripwire, never a target.
fn restarts(
    setup: &contract::scenario::ScenarioDefinition,
    played: &Battle,
    opened: u64,
    chapter: &contract::director::Chapter,
    id: &str,
    name: &str,
) {
    let started = std::time::Instant::now();
    let mut resumed = Battle::resume(setup, &played.replay(), opened)
        .unwrap_or_else(|e| panic!("{id}/{name}: chapter {} doesn't resume: {e}", chapter.id));
    eprintln!(
        "{id}/{name}: chapter {} resumed at tick {opened} in {:.1} s",
        chapter.id,
        started.elapsed().as_secs_f64()
    );
    let fired = |battle: &Battle| {
        chapter
            .beats
            .iter()
            .filter(|b| battle.director().unwrap().fired_at(&b.id).is_some())
            .count()
    };
    while resumed.tick() < played.tick() && fired(&resumed) < chapter.beats.len() {
        resumed.step();
    }
    assert_eq!(
        fired(&resumed),
        chapter.beats.len(),
        "{id}/{name}: chapter {}'s beats fire again after a restart",
        chapter.id
    );
}

/// The chapters' dev loop: a battle started at `chapter` by its solution
/// opens there, with the human seats handed back to a person.
fn starts_at(
    setup: &contract::scenario::ScenarioDefinition,
    seed: u64,
    limit: u64,
    chapter: &contract::director::Chapter,
    index: usize,
    id: &str,
    name: &str,
) {
    let (replay, tick) = Battle::chapter_start(setup, seed, &chapter.id, None, limit)
        .unwrap_or_else(|e| panic!("{id}/{name}: {e}"));
    let started = Battle::resume(setup, &replay, tick).unwrap();
    assert_eq!(started.director().unwrap().chapter(), index, "{id}/{name}");
    let humans: Vec<_> = setup
        .players
        .iter()
        .filter(|p| p.human_capable)
        .map(|p| &p.id)
        .collect();
    for (player, controller) in started.replay().controllers {
        if humans.contains(&&player) {
            assert_eq!(
                controller,
                contract::players::ControllerKind::Human,
                "{id}/{name}: {player}"
            );
        }
    }
}
