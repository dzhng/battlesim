//! Objective victory rules through a small authored battle, without AI or travel.
use contract::command::Order;
use contract::ids::Side;
use contract::skirmish::{MatchResult, MatchRuleSet, MatchRules, Phase};
use sim::battle::Battle;

use crate::common::command;

fn holding(rules: MatchRules, held: usize) -> Battle {
    let mut deck = super::skirmish::deck();
    sim::fixtures::patch_catalog(
        &mut deck,
        "units",
        "test_tank",
        serde_json::json!({"cost": 1}),
    );
    let mut setup = super::skirmish::match_on(deck);
    let skirmish = setup.skirmish.as_mut().unwrap();
    skirmish.rules = rules;
    for site in skirmish.sites.objectives.iter_mut().take(held) {
        site.center = skirmish.sites.entries[0].center;
    }
    let destination = skirmish.sites.entries[0].center;
    let limit = u64::from(
        skirmish.rules.preparation_s
            + skirmish.rules.capture_s
            + skirmish.rules.dispatch_interval_s
            + 1,
    ) * u64::from(setup.rules.tick_hz);
    let mut battle = Battle::new(&setup, 1);
    for (side, seq, order) in [
        (
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination,
            },
        ),
        (Side::Blue, 2, Order::Ready),
        (Side::Red, 1, Order::Ready),
    ] {
        assert!(battle.accept(command(side, seq, order)).error.is_none());
    }
    loop {
        let view = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
        if view
            .objectives
            .iter()
            .filter(|o| o.owner == Some(Side::Blue))
            .count()
            == held
        {
            assert_eq!(view.scores, [0.0; 2]);
            return battle;
        }
        assert!(
            battle.tick() < limit,
            "authored unit never captured its objectives"
        );
        battle.step();
    }
}

#[test]
fn disabling_all_objectives_victory_keeps_scoring_until_the_holding_target() {
    let rules = sim::skirmish::resolve_rules(MatchRuleSet::Frontline);
    let count = super::skirmish::setup()
        .skirmish
        .unwrap()
        .sites
        .objectives
        .len();
    let mut battle = holding(rules.clone(), count);
    assert_eq!(
        battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase,
        Phase::Active
    );
    let threshold =
        u64::from(rules.majority_s) * (count as u64 / 2 + 1) * u64::from(battle.rules().tick_hz);
    let winning_ticks = threshold.div_ceil(count as u64);
    for _ in 1..winning_ticks {
        battle.step();
        assert_eq!(
            battle.observe(Side::Blue).skirmish.as_ref().unwrap().result,
            None
        );
    }
    assert!(battle.observe(Side::Blue).skirmish.as_ref().unwrap().scores[0] > 0.0);
    battle.step();
    let view = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(view.result, Some(MatchResult::Winner { side: Side::Blue }));
    assert_eq!(view.scores, [rules.victory_score, 0.0]);
    let tick = battle.tick();
    let digest = battle.digest();
    battle.step();
    assert_eq!(battle.tick(), tick);
    assert_eq!(battle.digest(), digest);
}

#[test]
fn all_objectives_score_faster_than_a_bare_majority_without_ending_the_match() {
    let rules = sim::skirmish::resolve_rules(MatchRuleSet::Frontline);
    let count = super::skirmish::setup()
        .skirmish
        .unwrap()
        .sites
        .objectives
        .len();
    let majority = count / 2 + 1;
    let mut all = holding(rules.clone(), count);
    let mut some = holding(rules.clone(), majority);
    for sample in 1..=2 {
        for _ in 0..all.rules().tick_hz {
            all.step();
            some.step();
        }
        let a = all.observe(Side::Blue).skirmish.as_ref().unwrap();
        let b = some.observe(Side::Blue).skirmish.as_ref().unwrap();
        assert_eq!(a.phase, Phase::Active);
        assert_eq!(b.phase, Phase::Active);
        assert!(a.scores[0] > b.scores[0]);
        assert!((a.scores[0] / b.scores[0] - count as f64 / majority as f64).abs() < 1e-12);
        println!(
            "{}",
            serde_json::json!({
                "sample": sample, "tick": all.tick(), "phase": a.phase,
                "all_objectives": {"held": count, "scores": a.scores, "result": a.result},
                "bare_majority": {"held": majority, "scores": b.scores, "result": b.result}
            })
        );
    }
}

#[test]
fn frontline_resolves_only_its_three_overrides_over_the_base() {
    use std::collections::BTreeSet;
    let overrides: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/frontline.json")).unwrap();
    assert_eq!(
        overrides
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["majority_s", "victory_score", "all_objectives_victory"])
    );
    let mut base = serde_json::to_value(sim::skirmish::starter_rules()).unwrap();
    let mut expected = base.clone();
    expected.as_object_mut().unwrap().extend(overrides.clone());
    let expected: MatchRules = serde_json::from_value(expected).unwrap();
    let mut resolved =
        serde_json::to_value(sim::skirmish::resolve_rules(MatchRuleSet::Frontline)).unwrap();
    assert_eq!(resolved, serde_json::to_value(expected).unwrap());
    for key in overrides.keys() {
        base.as_object_mut().unwrap().remove(key);
        resolved.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(
        resolved, base,
        "every other base field must survive resolution"
    );
}

#[test]
fn both_rule_sets_display_the_same_points_for_the_same_holding_time() {
    let skirmish = sim::skirmish::starter_rules();
    let frontline = sim::skirmish::resolve_rules(MatchRuleSet::Frontline);
    let count = super::skirmish::setup()
        .skirmish
        .unwrap()
        .sites
        .objectives
        .len();
    let majority = count / 2 + 1;
    let mut short = holding(skirmish.clone(), majority);
    let mut long = holding(frontline, majority);
    let ticks = u64::from(skirmish.majority_s) * u64::from(short.rules().tick_hz);
    for _ in 0..ticks {
        short.step();
        long.step();
        let a = short.observe(Side::Blue).skirmish.as_ref().unwrap();
        let b = long.observe(Side::Blue).skirmish.as_ref().unwrap();
        assert!(
            (a.scores[0] - b.scores[0]).abs() < 1e-9,
            "same holding time must display the same points: {:?} vs {:?}",
            a.scores,
            b.scores
        );
        assert_eq!(b.result, None);
    }
    let a = short.observe(Side::Blue).skirmish.as_ref().unwrap();
    assert_eq!(a.result, Some(MatchResult::Winner { side: Side::Blue }));
    assert_eq!(a.scores[0], skirmish.victory_score);
}

#[test]
fn the_default_keeps_replay_identities_but_disabling_it_changes_the_scenario() {
    let setup = super::skirmish::setup();
    let legacy = Battle::new(&setup, 1);
    let replay = legacy.replay();
    let mut document = serde_json::to_value(&setup).unwrap();
    document["skirmish"]["rules"]["all_objectives_victory"] = serde_json::json!(true);
    let explicit = serde_json::from_value(document.clone()).unwrap();
    let explicit = Battle::new(&explicit, 1);
    assert_eq!(explicit.digest(), legacy.digest());
    assert_eq!(explicit.replay().config_digest, replay.config_digest);
    assert_eq!(explicit.replay().scenario_digest, replay.scenario_digest);
    document["skirmish"]["rules"]["all_objectives_victory"] = serde_json::json!(false);
    let disabled = serde_json::from_value(document).unwrap();
    let disabled_replay = Battle::new(&disabled, 1).replay();
    assert_eq!(disabled_replay.config_digest, replay.config_digest);
    assert_ne!(disabled_replay.scenario_digest, replay.scenario_digest);
    assert_eq!(
        Battle::from_replay(&disabled, &replay).err(),
        Some(sim::battle::ReplayError::ScenarioMismatch)
    );
}
