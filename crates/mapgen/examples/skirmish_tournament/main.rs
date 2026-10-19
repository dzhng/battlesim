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
//! order. Without `|`, the list repeats from its first card; with `|`, cards
//! before the bar run once and cards after it repeat. The report's `air`
//! block reads the referee's spawn and kill log.
//!
//! Each match ends by replaying its command stream from the start and
//! checking the digest matches, which costs nearly a second simulation.
//! `--no-replay-check` skips it for quick iteration (`replay_match` is then
//! null); a confirmation run keeps it.

mod inputs;
mod model;
mod runner;

use contract::catalog::Faction;
use inputs::Inputs;
use model::{Corpus, GeneratedMap, MapSource, Row};
use runner::run_safe;

/// The safety cap when no `--max-seconds` is given: the preparation plus
/// three times the majority hold, so a contested match can still end by the
/// rules (`fixtures/skirmish.json`) rather than the cap.
fn default_max_battle_seconds() -> u64 {
    let rules = sim::skirmish::starter_rules();
    u64::from(rules.preparation_s) + 3 * u64::from(rules.majority_s)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
    };
    let mut inputs = Inputs::load().expect("load generation inputs");
    if let Some(pair) = value("--factions") {
        let names: Vec<Faction> = pair
            .split(',')
            .map(|name| serde_json::from_value(name.into()).expect("--factions us,europe|eastern"))
            .collect();
        inputs.factions = [names[0], names[1]];
    }
    for (side, flag) in ["--buy-blue", "--buy-red"].into_iter().enumerate() {
        inputs.buy[side] = value(flag).map(|list| {
            list.replace('|', ",|,")
                .split(',')
                .map(String::from)
                .collect()
        });
        // A card the side can never buy would hold its list forever.
        let faction = inputs.factions[side];
        for card in inputs.buy[side]
            .iter()
            .flatten()
            .filter(|card| *card != "|")
        {
            let roster = inputs
                .rules
                .catalog
                .index(card)
                .and_then(|kind| inputs.rules.catalog.get(kind).roster.as_ref());
            assert!(
                roster.is_some_and(|roster| roster.factions.contains(&faction)),
                "{flag} card {card} is not in {faction:?}'s roster"
            );
        }
    }
    inputs.check_replay = !args.iter().any(|arg| arg == "--no-replay-check");
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
        let seed = value("--seed")
            .map(|value| value.parse::<u64>().expect("--seed must be a u64"))
            .unwrap_or_else(random_seed);
        let max_seconds = value("--max-seconds")
            .map(|value| value.parse::<u64>().expect("--max-seconds must be a u64"))
            .unwrap_or_else(default_max_battle_seconds);
        let players_per_side = value("--players")
            .map(|value| {
                value
                    .parse::<usize>()
                    .expect("--players must be a positive integer")
            })
            .unwrap_or(1);
        println!(
            "{}",
            serde_json::to_string(&runner::run_safe(
                &inputs,
                self_play_row_with_players(
                    seed,
                    max_seconds,
                    inputs.rules.tick_hz,
                    players_per_side
                ),
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

pub(crate) fn self_play_row_with_players(
    seed: u64,
    max_seconds: u64,
    tick_hz: u32,
    players_per_side: usize,
) -> Row {
    let size = match players_per_side {
        0..=1 => "small",
        2 => "medium",
        3..=4 => "large",
        _ => "xl",
    };
    Row {
        id: format!("self_play_{seed}"),
        map: MapSource::Generated(GeneratedMap {
            kind: "generated".into(),
            map_type: "mixed".into(),
            size: size.into(),
        }),
        seed,
        horizon_ticks: max_seconds.saturating_mul(u64::from(tick_hz)),
        no_progress_ticks: u64::MAX,
        players_per_side,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::{GeneratedMap, MapSource, Row, Status};
    use runner::run;

    #[test]
    fn the_default_cap_leaves_room_to_win_by_holding_a_majority() {
        // A side holding a bare majority from the first second wins after the
        // preparation and `majority_s`; the cap must let a contested match go
        // well past that, or the tournament reports a timeout the rules
        // would have decided.
        let rules = sim::skirmish::starter_rules();
        let ideal = u64::from(rules.preparation_s) + u64::from(rules.majority_s);
        assert!(default_max_battle_seconds() >= 2 * ideal);
    }

    #[test]
    fn default_self_play_is_a_generated_small_map() {
        let row = self_play_row_with_players(17, 600, 30, 1);
        assert_eq!(row.seed, 17);
        assert_eq!(row.horizon_ticks, 18_000);
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
                players_per_side: row.players_per_side,
            },
        );
        // The same match without the replay check plays identically and
        // reports the check as not run.
        let unchecked = Inputs {
            check_replay: false,
            ..Inputs::load().unwrap()
        };
        let right = run(
            &unchecked,
            Row {
                id: row.id,
                map: row.map,
                seed: row.seed,
                horizon_ticks: 90,
                no_progress_ticks: 30,
                players_per_side: row.players_per_side,
            },
        );
        assert_eq!(left.battle_digest, right.battle_digest);
        assert_eq!(left.replay_digest, right.replay_digest);
        assert_eq!(left.replay_match, Some(true));
        assert_eq!(right.replay_match, None);
        assert_eq!(left.site_count, 3);
        assert!(left.map_identity.is_some());
        assert!(matches!(left.status, Status::Timeout));
        assert_eq!(left.accepted_orders, right.accepted_orders);
        assert_eq!(left.refused_orders, right.refused_orders);
        assert!(left.accepted_orders > 0, "self-play accepted no AI orders");
        assert!(left.credit_spend.iter().any(|spent| *spent > 0));
        assert_eq!(left.doctrine_pivots, right.doctrine_pivots);
    }

    #[test]
    fn scripted_buy_owns_the_player_purchase_stream() {
        let mut inputs = Inputs::load().unwrap();
        let card = "us_m1129_stryker_mortar_120_mm_mortar_carrier".to_owned();
        inputs.buy[0] = Some(vec![card.clone()]);
        let report = run(
            &inputs,
            self_play_row_with_players(7, 90, inputs.rules.tick_hz, 1),
        );
        assert_eq!(report.refused_by_side[0], 0);
        assert!(report.opening[0].len() > 1, "a list without | must repeat");
        assert!(report.opening[0].iter().all(|bought| bought == &card));
    }
}
