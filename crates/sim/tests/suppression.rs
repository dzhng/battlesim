//! Suppression's tiers (P14): near misses build a hidden level that fades
//! after a lull, and only its tier (none, suppressed, pinned) has effects,
//! each tier one fixed penalty. The tier is what the side observes.
use contract::command::{MoveDirection, Order, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::SuppressionTier;
use contract::scenario::{Rules, ScenarioDefinition, SuppressionRules};
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

const HZ: f64 = 30.0;

fn rules() -> SuppressionRules {
    common::rules().suppression
}

/// `units` on an empty 1200 × 600 m field.
fn setup(units: Value) -> ScenarioDefinition {
    let map =
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    common::scenario_with(&map, units, json!([]), json!([]))
}

fn battle(units: Value, seed: u64) -> Battle {
    Battle::new(&setup(units), seed)
}

/// A squad's hidden level, as the authority holds it.
fn level(b: &Battle, id: u32) -> f64 {
    b.unit(UnitId(id)).unwrap().suppression
}

/// A squad's tier, as its side observes it.
fn tier(b: &Battle, side: Side, id: u32) -> SuppressionTier {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .suppression
}

/// A red rifle squad starting at hidden `level`, alone but for a blue tank
/// far out of sight: nothing fires on it.
fn alone(level: f64, extra: Value) -> Battle {
    let mut red = json!({ "side": "red", "kind": "rifle", "position": [100, 100],
        "engagement": "return_fire_only", "condition": { "suppression": level } });
    for (k, v) in extra.as_object().unwrap() {
        red[k] = v.clone();
    }
    battle(
        json!([
            { "side": "blue", "kind": "tank", "position": [1150, 550], "engagement": "return_fire_only" },
            red,
        ]),
        3,
    )
}

#[test]
fn one_near_miss_never_suppresses_but_sustained_fire_does() {
    // Blue's tank fires at the ground past red's squad, a few metres off
    // its nearest soldiers: sustained fire, every round a near miss.
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [200, 303], "engagement": "return_fire_only" },
        ]),
        4,
    );
    common::order(
        &mut b,
        Side::Blue,
        1,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [320.0, 300.0, 0.0],
            },
        },
    );
    let (mut near_misses, mut last) = (0, 0.0);
    let mut reached = Vec::new();
    for _ in 0..(15.0 * HZ) as u64 {
        b.step();
        let l = level(&b, 1);
        if l > last {
            near_misses += 1;
            if near_misses == 1 {
                assert_eq!(tier(&b, Side::Red, 1), SuppressionTier::None, "{l}");
            }
        }
        last = l;
        let t = tier(&b, Side::Red, 1);
        if reached.last().is_none_or(|&(r, _)| r != t) {
            reached.push((t, near_misses));
        }
    }
    let tiers: Vec<SuppressionTier> = reached.iter().map(|&(t, _)| t).collect();
    assert_eq!(
        tiers,
        [
            SuppressionTier::None,
            SuppressionTier::Suppressed,
            SuppressionTier::Pinned
        ],
        "sustained fire climbs the tiers: {reached:?}"
    );
    assert!(
        reached[1].1 > 1,
        "suppressed only after many near misses: {reached:?}"
    );
}

/// How far a squad starting at `level` walks north in 2.5 s: under the
/// recovery delay, so its tier holds.
fn walked(level: f64) -> f64 {
    let mut b = alone(level, json!({}));
    common::order(
        &mut b,
        Side::Red,
        1,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [100.0, 500.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
    let start = b.unit(UnitId(1)).unwrap().members.clone();
    for _ in 0..(2.5 * HZ) as u64 {
        b.step();
    }
    let end = &b.unit(UnitId(1)).unwrap().members;
    let n = start.len() as f64;
    start
        .iter()
        .zip(end)
        .map(|(a, b)| b.position.y - a.position.y)
        .sum::<f64>()
        / n
}

#[test]
fn each_tier_slows_movement_by_its_one_fixed_share() {
    let s = rules();
    let calm = walked(0.0);
    assert!(calm > 1.0, "the calm squad walks: {calm}");
    // Below the suppressed level there is no penalty at all.
    assert_eq!(walked(s.suppressed.level - 0.01), calm);
    // Anywhere inside a tier, the same penalty.
    let suppressed = walked(s.suppressed.level);
    assert_eq!(walked(s.pinned.level - 0.01), suppressed);
    let pinned = walked(s.pinned.level);
    assert_eq!(walked(1.0), pinned);
    // Each tier's share of speed is lost.
    for (walked, tier) in [(suppressed, &s.suppressed), (pinned, &s.pinned)] {
        let ratio = walked / calm;
        assert!(
            (ratio - (1.0 - tier.move_penalty)).abs() < 0.03,
            "{ratio} for a penalty of {}",
            tier.move_penalty
        );
    }
}

/// The ticks a squad starting at `level` fires rifle rounds at the ground
/// in its first 2.5 s: under the recovery delay, so its tier holds.
fn shots(level: f64) -> Vec<u64> {
    let mut b = alone(level, json!({ "engagement": "fire_at_will" }));
    common::order(
        &mut b,
        Side::Red,
        1,
        Order::Attack {
            units: vec![UnitId(1)],
            target: TargetRef::Ground {
                point: [100.0, 200.0, 0.0],
            },
        },
    );
    let mut seen = std::collections::BTreeSet::new();
    let mut ticks = Vec::new();
    for _ in 0..(2.5 * HZ) as u64 {
        b.step();
        for (p, r) in b.rounds() {
            if r.unit == UnitId(1) && seen.insert(p.id) {
                ticks.push(b.tick());
            }
        }
    }
    ticks
}

#[test]
fn each_tier_slows_the_reload_cycle_by_its_one_fixed_share() {
    let s = rules();
    let calm = shots(0.0);
    assert!(calm.len() > 2, "the calm squad fires: {calm:?}");
    assert_eq!(shots(s.suppressed.level - 0.01), calm, "no penalty below");
    let suppressed = shots(s.suppressed.level);
    assert_eq!(
        shots(s.pinned.level - 0.01),
        suppressed,
        "one penalty a tier"
    );
    let pinned = shots(s.pinned.level);
    assert_eq!(shots(1.0), pinned, "one penalty a tier");
    assert!(
        calm.len() > suppressed.len() && suppressed.len() > pinned.len(),
        "{} > {} > {}",
        calm.len(),
        suppressed.len(),
        pinned.len()
    );
}

#[test]
fn after_the_lull_a_pinned_squad_recovers_tier_by_tier() {
    let s = rules();
    let mut b = alone(1.0, json!({}));
    // The level holds for the recovery delay, then fades at the decay rate.
    let at = |level: f64| (s.recovery_delay_s + (1.0 - level) / s.decay_per_s) * HZ;
    let (unpinned, calm) = (at(s.pinned.level), at(s.suppressed.level));
    let mut changes = Vec::new();
    let mut last = tier(&b, Side::Red, 1);
    assert_eq!(last, SuppressionTier::Pinned);
    while b.tick() < (calm + 60.0) as u64 {
        b.step();
        let t = tier(&b, Side::Red, 1);
        if t != last {
            changes.push((t, b.tick()));
            last = t;
        }
    }
    assert_eq!(changes.len(), 2, "{changes:?}");
    assert_eq!(changes[0].0, SuppressionTier::Suppressed);
    assert_eq!(changes[1].0, SuppressionTier::None);
    assert!(
        (changes[0].1 as f64 - unpinned).abs() <= 2.0,
        "{changes:?} vs {unpinned}"
    );
    assert!(
        (changes[1].1 as f64 - calm).abs() <= 2.0,
        "{changes:?} vs {calm}"
    );
}

#[test]
fn tiers_that_do_not_climb_or_cost_less_deeper_fail_at_load() {
    let load = |patch: Value| {
        let mut fixture = common::game();
        for (k, v) in patch.as_object().unwrap() {
            fixture["suppression"][k] = v.clone();
        }
        serde_json::from_value::<Rules>(fixture)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    assert_eq!(load(json!({})), Ok(()));
    let tier = |level: f64, penalty: f64| json!({ "level": level, "move_penalty": penalty, "reload_cycle_penalty": penalty, "scatter_multiplier": 1.0 + penalty });
    for broken in [
        json!({ "suppressed": tier(0.9, 0.3), "pinned": tier(0.85, 0.7) }),
        json!({ "suppressed": tier(0.0, 0.3) }),
        json!({ "pinned": tier(1.2, 0.7) }),
        json!({ "pinned": tier(0.85, 0.2) }),
        json!({ "pinned": tier(0.85, 1.0) }),
        json!({ "decay_per_s": 0.0 }),
    ] {
        let e = load(broken.clone()).unwrap_err();
        assert!(e.contains("rules.suppression: "), "{broken}: {e}");
    }
    // The old continuous keys are gone, not ignored.
    let e = load(json!({ "max_move_penalty": 0.7 })).unwrap_err();
    assert!(e.contains("max_move_penalty"), "{e}");
}

#[test]
fn a_suppressed_battle_replays_identically() {
    // A squad starting suppressed, then fired on: its tier moves both ways.
    let setup = setup(json!([
        { "side": "blue", "kind": "tank", "position": [100, 300] },
        { "side": "red", "kind": "rifle", "position": [200, 303], "engagement": "return_fire_only", "condition": { "suppression": 0.5 } },
    ]));
    let mut live = Battle::new(&setup, 6);
    let mut digests = Vec::new();
    let mut tiers = Vec::new();
    for t in 0..(20.0 * HZ) as u64 {
        if t == (5.0 * HZ) as u64 {
            common::order(
                &mut live,
                Side::Blue,
                1,
                Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Ground {
                        point: [320.0, 300.0, 0.0],
                    },
                },
            );
        }
        live.step();
        digests.push(live.digest());
        let now = tier(&live, Side::Red, 1);
        if tiers.last() != Some(&now) {
            tiers.push(now);
        }
    }
    assert!(tiers.len() > 1, "the tier moved: {tiers:?}");
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    for (t, expected) in digests.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.digest(),
            *expected,
            "first mismatch at tick {}",
            t + 1
        );
    }
}

#[test]
fn suppression_widens_actual_launch_directions_at_each_tier() {
    let spread = |level: f64| {
        (1..=16).map(|seed| {
            let mut setup = setup(json!([
                { "side": "blue", "kind": "rifle", "position": [100, 300], "condition": { "suppression": level } },
                { "side": "red", "kind": "tank", "position": [1150, 550], "engagement": "return_fire_only" }
            ]));
            // Pin reach so only the suppression tier changes launch scatter.
            setup.rules.weapons.get_mut("rifle").unwrap().ballistics.range_m = 450.0;
            let mut b = Battle::new(&setup, seed);
            common::order(&mut b, Side::Blue, 1, Order::Attack { units: vec![UnitId(0)],
                target: TargetRef::Ground { point: [400.0, 300.0, 0.0] } });
            for _ in 0..90 {
                b.step();
                for (p,r) in b.rounds() {
                    if r.unit != UnitId(0) || b.arsenal().weapons[r.weapon].id != "rifle" { continue; }
                    let body = p.shooter.unwrap().body;
                    let soldier = b.unit(UnitId(0)).unwrap().members.iter().find(|s| s.id == body.0).unwrap();
                    let expected = (300.0-soldier.position.y).atan2(400.0-soldier.position.x);
                    let actual = p.velocity.y.atan2(p.velocity.x);
                    return (actual-expected).powi(2);
                }
            }
            panic!("shooter never fired");
        }).sum::<f64>()
    };
    let s = rules();
    let calm = spread(0.0);
    let suppressed = spread(s.suppressed.level);
    let pinned = spread(s.pinned.level);
    assert!(
        suppressed > calm * 1.8 && pinned > suppressed * 1.8,
        "launch-direction variance must widen with each tier: {calm}, {suppressed}, {pinned}"
    );
}
