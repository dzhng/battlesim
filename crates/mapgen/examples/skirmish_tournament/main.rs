//! Two-controller skirmish self-play and corpus runner.
//!
//! This stays outside browser setup: each side observes its own frame, asks
//! the policy for ordinary orders, accepts them through Battle, and advances
//! one authoritative tick. The JSONL report is deliberately evidence rather
//! than a winner fixture.
//!
//! Balance probes shape a game without touching its rules: `--factions
//! us,eastern` picks Blue's and Red's armies; `--buy-blue` / `--buy-red
//! <card,…[|card,…]>` replace a side's own shopping with a list bought in
//! order (cards before `|` once, the rest repeating). The report's `air`
//! block reads the referee's spawn and kill log.

mod inputs;
mod model;
mod runner;

use contract::catalog::Faction;
use inputs::Inputs;
use model::{Corpus, GeneratedMap, MapSource, Row};
use runner::run_safe;

const DEFAULT_MAX_BATTLE_SECONDS: u64 = 1_800;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
    };
    let factions = value("--factions").map_or([Faction::Us, Faction::Europe], |pair| {
        let names: Vec<Faction> = pair
            .split(',')
            .map(|name| serde_json::from_value(name.into()).expect("--factions us,europe|eastern"))
            .collect();
        [names[0], names[1]]
    });
    let mut inputs = Inputs::load().expect("load generation inputs");
    inputs.factions = factions;
    for (side, flag) in ["--buy-blue", "--buy-red"].into_iter().enumerate() {
        inputs.buy[side] = value(flag).map(|list| {
            list.replace('|', ",|,")
                .split(',')
                .map(String::from)
                .collect()
        });
    }
    if args.iter().any(|arg| arg == "--corpus") {
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../../../fixtures/skirmish-corpus.json"))
                .expect("valid skirmish corpus");
        assert_eq!(corpus.revision, 1, "unsupported corpus revision");
        for row in corpus.rows {
            println!(
                "{}",
                serde_json::to_string(&run_safe(&inputs, row)).unwrap()
            );
        }
    } else {
        let seed = args
            .iter()
            .position(|arg| arg == "--seed")
            .and_then(|index| args.get(index + 1))
            .map(|value| value.parse::<u64>().expect("--seed must be a u64"))
            .unwrap_or_else(random_seed);
        let max_seconds = args
            .iter()
            .position(|arg| arg == "--max-seconds")
            .and_then(|index| args.get(index + 1))
            .map(|value| value.parse::<u64>().expect("--max-seconds must be a u64"))
            .unwrap_or(DEFAULT_MAX_BATTLE_SECONDS);
        println!(
            "{}",
            serde_json::to_string(&runner::run_safe(
                &inputs,
                self_play_row(seed, max_seconds, inputs.rules.tick_hz),
            ))
            .unwrap()
        );
    }
}

fn random_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos() as u64
}

pub(crate) fn self_play_row(seed: u64, max_seconds: u64, tick_hz: u32) -> Row {
    Row {
        id: format!("self_play_{seed}"),
        map: MapSource::Generated(GeneratedMap {
            kind: "generated".into(),
            map_type: "mixed".into(),
            size: "small".into(),
        }),
        seed,
        horizon_ticks: max_seconds.saturating_mul(u64::from(tick_hz)),
        no_progress_ticks: u64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::{GeneratedMap, MapSource, Row, Status};
    use runner::run;

    #[test]
    fn default_self_play_is_a_generated_small_map() {
        let row = self_play_row(17, DEFAULT_MAX_BATTLE_SECONDS, 30);
        assert_eq!(row.seed, 17);
        assert_eq!(row.horizon_ticks, 54_000);
        assert_eq!(row.no_progress_ticks, u64::MAX);
        assert!(matches!(
            row.map,
            MapSource::Generated(GeneratedMap {
                kind,
                map_type,
                size
            }) if kind == "generated" && map_type == "mixed" && size == "small"
        ));
    }

    #[test]
    fn corpus_is_versioned_and_repeatable() {
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../../../fixtures/skirmish-corpus.json"))
                .unwrap();
        assert_eq!(corpus.revision, 1);
        let inputs = Inputs::load().unwrap();
        let row = corpus.rows.into_iter().next().unwrap();
        let left = run(
            &inputs,
            Row {
                id: row.id.clone(),
                map: row.map.clone(),
                seed: row.seed,
                horizon_ticks: 90,
                no_progress_ticks: 30,
            },
        );
        let right = run(
            &inputs,
            Row {
                id: row.id,
                map: row.map,
                seed: row.seed,
                horizon_ticks: 90,
                no_progress_ticks: 30,
            },
        );
        assert_eq!(left.battle_digest, right.battle_digest);
        assert_eq!(left.replay_digest, right.replay_digest);
        assert_eq!(left.replay_match, Some(true));
        assert_eq!(left.site_count, 3);
        assert!(left.map_identity.is_some());
        assert!(matches!(left.status, Status::Timeout));
        assert_eq!(left.accepted_orders, right.accepted_orders);
        assert_eq!(left.refused_orders, right.refused_orders);
    }
}
