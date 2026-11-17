//! What a jet's bombs do to a ground aim: each run releases its round (every
//! bomblet of a cluster canister together), every bomb lands inside the
//! strike circle the attack-ground cursor draws round the aim, and a
//! canister's bomblets burst one after another across their disc.
use std::collections::{BTreeMap, BTreeSet};

use crate::attack_runs::{attack_ground, run_field, run_rules, RUN_START};
use contract::catalog::TypeIndex;
use contract::ids::UnitId;
use serde_json::json;
use sim::flight::FlightEvent;
use sim::math::{v2, V2};

/// A test jet's bombing of `aim` with the shipped `row` over `window_s`: each
/// release (the tick its bombs left the jet) with where and when, in
/// seconds, each of them landed, and the strike circle's radius round the
/// aim.
fn bombing(row: &str, aim: [f64; 2], window_s: f64) -> (Vec<Vec<(V2, f64)>>, f64) {
    let mut battle = run_field(
        run_rules(&[row]),
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    assert_eq!(battle.accept(attack_ground(1, aim)).error, None);
    let rules = battle.rules().clone();
    let kind = rules.catalog.ids().iter().position(|id| id == "test_jet");
    let circle = sim::strike::footprint_m(
        &rules,
        &sim::weapons::Arsenal::new(&rules),
        TypeIndex(kind.expect("test_jet") as u16),
        0.0,
    )
    .expect("a bomber strikes an area");
    let dt = 1.0 / f64::from(rules.tick_hz);
    let mut released: BTreeMap<u64, Vec<_>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut landed = BTreeMap::new();
    // Bombs released in the window are given its last bomb's fall to land.
    let release_ticks = (window_s / dt) as u64;
    for tick in 1..=release_ticks + (10.0 / dt) as u64 {
        battle.step();
        for (p, r) in battle.rounds() {
            if tick <= release_ticks && r.unit == UnitId(0) && seen.insert(p.id) {
                released.entry(tick).or_default().push(p.id);
            }
        }
        for e in battle.flight_events() {
            if let FlightEvent::Impact(i) = e {
                landed.insert(i.projectile, (i.point.xy(), tick as f64 * dt));
            }
        }
    }
    let releases = released
        .into_values()
        .map(|ids| {
            ids.iter()
                .map(|id| *landed.get(id).expect("every bomb lands"))
                .collect()
        })
        .collect();
    (releases, circle)
}

#[test]
fn every_jet_bomb_lands_inside_the_strike_circle() {
    let aim = [2000.0, 2000.0];
    for row in ["air_bomb", "air_bomb_cluster"] {
        let (releases, circle) = bombing(row, aim, 90.0);
        assert!(
            releases.len() >= 2,
            "{row}: {} runs released",
            releases.len()
        );
        for (run, bombs) in releases.iter().enumerate() {
            for (point, _) in bombs {
                let miss = (*point - v2(aim[0], aim[1])).length();
                assert!(
                    miss <= circle,
                    "{row} run {run}: a bomb landed {miss:.0} m from the aim, outside its {circle:.0} m circle"
                );
            }
        }
    }
}

#[test]
fn a_cluster_canister_bursts_into_all_its_bomblets_across_its_disc() {
    let aim = [2000.0, 2000.0];
    let (releases, circle) = bombing("air_bomb_cluster", aim, 90.0);
    let rules: contract::scenario::Rules = serde_json::from_value(run_rules(&[])).unwrap();
    let canister = rules.weapons["air_bomb_cluster"]
        .submunitions
        .expect("a cluster canister carries bomblets");
    assert!(!releases.is_empty(), "the jet never released");
    for bombs in &releases {
        // All of a canister's bomblets leave on its one release.
        assert_eq!(bombs.len(), canister.count as usize, "{bombs:?}");
        let middle =
            bombs.iter().fold(v2(0.0, 0.0), |sum, (p, _)| sum + *p) * (1.0 / bombs.len() as f64);
        assert!(
            (middle - v2(aim[0], aim[1])).length() < circle / 2.0,
            "the carpet's middle {middle:?} is far off the aim"
        );
        // They come down across the whole disc, not clumped in its middle:
        // about half inside the inner disc of half its area, and the
        // outermost near its rim.
        let out: Vec<f64> = bombs.iter().map(|(p, _)| (*p - middle).length()).collect();
        let inner = out
            .iter()
            .filter(|&&r| r < canister.spread_m / std::f64::consts::SQRT_2)
            .count();
        let n = bombs.len();
        assert!(
            (n / 4..=3 * n / 4).contains(&inner),
            "{inner} of {n} bomblets fell in the inner half: {out:?}"
        );
        let widest = out.iter().copied().fold(0.0, f64::max);
        assert!(
            (0.8 * canister.spread_m..1.2 * canister.spread_m).contains(&widest),
            "the outermost bomblet fell {widest:.0} m out of a {} m disc",
            canister.spread_m
        );
        // They burst one after another over the canister's ripple, not on
        // one tick.
        let first = bombs.iter().map(|(_, t)| *t).fold(f64::INFINITY, f64::min);
        let last = bombs.iter().map(|(_, t)| *t).fold(0.0, f64::max);
        let dt = 1.0 / f64::from(rules.tick_hz);
        assert!(
            last - first > canister.ripple_s / 2.0 && last - first <= canister.ripple_s + dt,
            "the bomblets landed over {:.2} s, for a ripple of {} s",
            last - first,
            canister.ripple_s
        );
    }
}

/// With no launch spread a bomb comes down on its aim, every run, whatever
/// heading the run comes in on: the release is timed by the jet's middle
/// on a whole tick, but the bomb leaves its rack, ahead of that, between
/// ticks, and still lands on the aim.
#[test]
fn a_bomb_without_spread_lands_on_its_aim() {
    let aim = [2000.0, 2000.0];
    let mut rules = run_rules(&["air_bomb"]);
    rules["weapons"]["air_bomb"]["targets"]["ground"]["scatter_mrad"] = json!(0.0);
    let mut battle = run_field(
        rules,
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    assert_eq!(battle.accept(attack_ground(1, aim)).error, None);
    let dt = 1.0 / f64::from(battle.rules().tick_hz);
    let mut landed = 0;
    for _ in 0..(90.0 / dt) as u64 {
        battle.step();
        for e in battle.flight_events() {
            if let FlightEvent::Impact(i) = e {
                let miss = (i.point.xy() - v2(aim[0], aim[1])).length();
                assert!(miss < 0.5, "a bomb landed {miss:.1} m from its aim");
                landed += 1;
            }
        }
    }
    assert!(landed >= 3, "only {landed} bombs landed");
}
