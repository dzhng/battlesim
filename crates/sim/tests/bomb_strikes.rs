//! What a jet's bombs do to a ground aim: where each leaves the jet on its
//! pass, that each run releases its round (every bomblet of a cluster
//! canister together), every bomb lands inside the strike circle the
//! attack-ground cursor draws round the aim, and a canister's bomblets burst
//! one after another across their disc.
use std::collections::{BTreeMap, BTreeSet};

use crate::attack_passes::{far, ground_attack_battle, ground_attack_battle_with, state, wing};
use crate::attack_runs::{
    attack_ground, pattern_leg_m, rounds, run_field, run_rules, square_field,
    RUN_BOMB_GRAVITY_SCALE, RUN_CRUISE_KMH, RUN_EDGE_MARGIN_M, RUN_GRAVITY_MPS2, RUN_ROLL_IN_S,
    RUN_START, RUN_WINDOW_S,
};
use crate::common::air_field::wing_limits;
use contract::catalog::TypeIndex;
use contract::ids::UnitId;
use contract::random::Rng;
use serde_json::json;
use sim::air::pass::{advance, begin, PassAim, PassPhase, PassWeapon};
use sim::air::Bounds;
use sim::fixtures;
use sim::flight::{FlightEvent, Release};
use sim::math::{v2, v3, V2};

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

/// A bombing run commits nose-on, releases within half a step of its release
/// point, and is over with the release: the jet lines up for the next one.
#[test]
fn a_bombing_run_releases_at_its_release_point_and_is_over() {
    let aim = v3(1000.0, 0.0, 0.0);
    let mut plan = begin(
        &state(250.0),
        &wing_limits(),
        aim,
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(7),
    );
    assert_eq!(plan.phase, PassPhase::LineUp);
    let release = Release {
        fall_s: 3.0,
        ahead_m: 300.0,
        at: v2(700.0, 0.0),
    };
    let aim_at = PassAim {
        point: aim,
        release,
        reach_m: 50.0,
        dive: None,
    };
    // 100 m/s in 0.6 s steps: 60 m a step.
    let dt = 0.6;
    let mut released_at = None;
    for step in 0..20 {
        let agl = plan.agl;
        let mut current = state(agl);
        current.pos.x = step as f64 * 60.0;
        let (_, cue) = advance(
            &mut plan,
            &current,
            agl,
            &wing_limits(),
            &far(),
            &aim_at,
            dt,
        );
        if cue {
            assert!(released_at.is_none(), "one cue per pass");
            // Where this tick's step carries the jet.
            released_at = Some(current.pos.x + 60.0);
        }
    }
    let x = released_at.expect("the pass never released");
    assert!(
        (x - 700.0).abs() <= 30.0,
        "released {x} m along, more than half a step from the release point"
    );
    assert_eq!(plan.phase, PassPhase::LineUp);
}

/// A bomb leaves the jet whenever one dropped now would land where it still
/// does its damage at the aim, whatever the jet is doing: a pilot passing
/// near the target drops on it without first lining up a perfect run. One
/// that would land past its reach stays on: flying over the aim with no
/// time left for the fall to carry it there, or abeam it too far out.
#[test]
fn a_bomb_leaves_whenever_it_would_land_on_the_aim() {
    let aim = v3(1000.0, 0.0, 0.0);
    // A drop flies 300 m on as it falls, and does its damage 130 m round
    // where it lands.
    let reach_m = 130.0;
    // 100 m/s in 0.6 s steps: 60 m a step, flying east along `y`, from `x`.
    let fly = |y: f64, from_x: f64| {
        let mut plan = begin(
            &state(250.0),
            &wing_limits(),
            aim,
            PassWeapon::Bomb,
            &wing(),
            &mut Rng::new(7),
        );
        let mut released = Vec::new();
        for step in 0..20 {
            let mut current = state(250.0);
            current.pos = v3(from_x + step as f64 * 60.0, y, 250.0);
            let aim_at = PassAim {
                point: aim,
                release: Release {
                    fall_s: 3.0,
                    ahead_m: 300.0,
                    at: v2(700.0, 0.0),
                },
                reach_m,
                dive: None,
            };
            let (_, cue) = advance(
                &mut plan,
                &current,
                250.0,
                &wing_limits(),
                &far(),
                &aim_at,
                0.6,
            );
            if cue {
                // Where this tick's step carries the jet.
                released.push(current.pos.x + 60.0);
            }
        }
        released
    };
    // 50 m abeam, its nose never on the aim: it drops as its fall carries
    // the bomb level with the aim, once.
    let released = fly(50.0, 0.0);
    assert_eq!(released.len(), 1, "released at {released:?}");
    assert!(
        (released[0] - 700.0).abs() <= 30.0,
        "released {} m along, more than half a step from where it lands level with the aim",
        released[0]
    );
    // Over the aim: a drop now lands its throw long, past its reach.
    assert_eq!(fly(0.0, 1000.0), Vec::<f64>::new(), "dropped long");
    // Abeam past where a bomb's damage reaches.
    assert_eq!(fly(200.0, 0.0), Vec::<f64>::new(), "dropped wide");
}

/// A run the edge guard takes over is broken, and the jet lines up again;
/// a bomb that lands on the aim from where the guard takes it still leaves,
/// as the jet breaks away.
#[test]
fn a_run_the_edge_guard_takes_over_is_broken() {
    let aim = v3(1000.0, 0.0, 0.0);
    let mut plan = begin(
        &state(150.0),
        &wing_limits(),
        aim,
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(7),
    );
    let aim_at = PassAim {
        point: aim,
        release: Release {
            fall_s: 3.0,
            ahead_m: 300.0,
            at: v2(700.0, 0.0),
        },
        reach_m: 50.0,
        dive: None,
    };
    // The east edge lies just past the aim: by the release point the jet,
    // heading straight at it, is at its point of no return.
    let limits = wing_limits();
    let bounds = Bounds::new(
        v2(-5_000.0, -5_000.0),
        v2(700.0 + limits.edge_margin_m + 1.0, 5_000.0),
    );
    // Nose-on, 400 m short of its release point: it commits.
    let mut current = state(plan.agl);
    current.pos.x = 300.0;
    advance(&mut plan, &current, 150.0, &limits, &bounds, &aim_at, 0.05);
    assert_eq!(plan.phase, PassPhase::Dive);
    // 100 m/s in 0.05 s steps: this step would carry it to the release point.
    current.pos.x = 697.0;
    let (_, cue) = advance(&mut plan, &current, 150.0, &limits, &bounds, &aim_at, 0.05);
    assert!(
        cue,
        "held a bomb that lands on the aim as the edge guard turned it away"
    );
    assert_eq!(plan.phase, PassPhase::LineUp);
}

#[test]
fn a_bombing_pass_never_fires_the_gun() {
    let mut battle = ground_attack_battle(&["air_gun", "air_bomb"]);
    let mut bomb_tick = None;
    for _ in 0..900 {
        battle.step();
        let unit = battle.unit(UnitId(0)).unwrap();
        let specs = battle.rules().catalog.mounts(unit.kind);
        let gun_shots: u32 = unit
            .mounts
            .iter()
            .zip(specs)
            .filter(|(_, s)| s.def.weapons.iter().any(|w| w == "air_gun"))
            .map(|(m, _)| m.shots)
            .sum();
        if gun_shots > 0 && bomb_tick.is_none() {
            panic!("gun fired before bomb: {:?}", unit.air.unwrap().pass);
        }
        let bomb_shots: u32 = unit
            .mounts
            .iter()
            .zip(specs)
            .filter(|(_, s)| s.def.weapons.iter().any(|w| w == "air_bomb"))
            .map(|(m, _)| m.shots)
            .sum();
        if bomb_shots > 0 {
            bomb_tick = Some(gun_shots);
            break;
        }
    }
    assert_eq!(bomb_tick.expect("bomb pass never released"), 0);
}

/// A bomb leaves the jet: on the tick it is released it is where the jet
/// is, and it falls onto the aim because the jet released it at the right
/// moment, not because it was placed there.
#[test]
fn a_bomb_leaves_the_jet_and_falls_onto_the_aim() {
    let blast_m = 20.0;
    let mut battle = ground_attack_battle_with(
        &["air_bomb"],
        1,
        |rules| {
            rules["weapons"]["air_bomb"]["targets"]["ground"]["scatter_mrad"] = json!(0.0);
            rules["weapons"]["air_bomb"]["blast_radius_m"] = json!(blast_m);
        },
        &[],
    );
    let mut bomb = None;
    for _ in 0..1200 {
        battle.step();
        if let Some((p, _)) = battle.rounds().find(|(_, r)| r.unit == UnitId(0)) {
            let jet = battle.unit(UnitId(0)).unwrap().position;
            assert!(
                (p.position - jet).length() < 15.0,
                "the bomb appeared {:.0} m from the jet",
                (p.position - jet).length()
            );
            bomb = Some(p.id);
            break;
        }
    }
    let bomb = bomb.expect("the jet never released its bomb");
    for _ in 0..600 {
        battle.step();
        let landed = battle.flight_events().iter().find_map(|e| match e {
            sim::flight::FlightEvent::Impact(i) if i.projectile == bomb => Some(i.point),
            _ => None,
        });
        if let Some(point) = landed {
            let miss = (point.xy() - v2(2000.0, 2000.0)).length();
            assert!(miss < blast_m, "the bomb landed {miss:.0} m from the aim");
            return;
        }
    }
    panic!("the bomb never landed");
}

/// A jet holds its bombs while a friendly vehicle sits in their blast at
/// the aim, as any gun holds for a friendly in its line.
#[test]
fn a_bomb_holds_for_a_friendly_vehicle_in_its_blast() {
    let mut battle = ground_attack_battle_with(
        &["air_bomb"],
        1,
        |rules| rules["weapons"]["air_bomb"]["blast_radius_m"] = json!(20.0),
        &[
            json!({ "side": "blue", "kind": "test_tank", "position": [2000, 2008], "yaw": 0.0, "engagement": "return_fire_only" }),
        ],
    );
    let mut cued = false;
    for _ in 0..1200 {
        battle.step();
        assert!(
            battle.rounds().all(|(_, r)| r.unit != UnitId(0)),
            "the jet bombed its own tank"
        );
        cued |= battle.unit(UnitId(0)).unwrap().air.unwrap().pass_release;
    }
    // It held at the release point, not short of it.
    assert!(cued, "the jet never reached its release point");
}

/// When, in seconds from the order, each of the jet's rounds left it over
/// `window_s`.
fn release_times(battle: &mut sim::battle::Battle, window_s: f64) -> Vec<f64> {
    let hz = f64::from(battle.rules().tick_hz);
    let mut released = Vec::new();
    for tick in 1..=(window_s * hz) as u64 {
        let before = rounds(battle);
        battle.step();
        released.extend((before..rounds(battle)).map(|_| tick as f64 / hz));
    }
    released
}

/// A target in the middle of a field with little more room round it than a
/// pilot's own pattern needs is bombed on every lap: the jet never circles
/// over it for want of a long run-in. The room on every side is an edge
/// margin, a turn's diameter, and a run-in of its roll out of the turn and
/// a bomb's throw from the lowest it bombs from. The jet turns wide, so its
/// pattern fills the field.
#[test]
fn a_jet_bombs_the_middle_of_a_field_barely_bigger_than_its_pattern() {
    const WIDE_TURN_DEG_S: f64 = 30.0;
    const FLOOR_M: f64 = 100.0;
    let speed = RUN_CRUISE_KMH / 3.6;
    let diameter_m = 2.0 * speed / WIDE_TURN_DEG_S.to_radians();
    let throw_m = speed * (2.0 * FLOOR_M / (RUN_GRAVITY_MPS2 * RUN_BOMB_GRAVITY_SCALE)).sqrt();
    let half = RUN_EDGE_MARGIN_M + diameter_m + 0.5 * RUN_ROLL_IN_S * speed + throw_m;
    let aim = [half, half];
    // It comes in from just inside the west margin, heading east at it.
    let start = [RUN_EDGE_MARGIN_M + 50.0, half];
    let mut rules = run_rules(&["air_bomb"]);
    rules["air"]["high_air"]["floor_agl_m"] = json!(FLOOR_M);
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mobility": { "air": { "turn_deg_s": WIDE_TURN_DEG_S } } }),
    );
    let mut battle = square_field(
        2.0 * half,
        rules,
        json!([{ "side": "blue", "kind": "test_jet", "position": start, "yaw": 0.0 }]),
    );
    assert_eq!(battle.accept(attack_ground(1, aim)).error, None);
    let released = release_times(&mut battle, RUN_WINDOW_S);
    // One lap: a turn round, and the leg out past the aim and back.
    let lap_s = std::f64::consts::TAU / WIDE_TURN_DEG_S.to_radians()
        + 2.0 * pattern_leg_m(WIDE_TURN_DEG_S) / speed;
    let mut last = 0.0;
    let mut limit = (half - start[0]) / speed;
    for end in released.iter().copied().chain([RUN_WINDOW_S]) {
        assert!(
            end - last <= limit + 1.0,
            "no bomb fell between {last:.1}s and {end:.1}s on a field {:.0} m across; released at {released:?}",
            2.0 * half
        );
        last = end;
        limit = lap_s;
    }
}
