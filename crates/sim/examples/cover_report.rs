//! Paired native battles separate incoming scatter from physical interception.
//! Run with --quick for the first rifle/middle-distance calibration, then expand.
use std::collections::BTreeSet;
use std::time::Instant;

use contract::command::{Engagement, Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde::Serialize;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{BodyId, FlightEvent, Struck};
use sim::rng::Rng;

#[path = "../tests/common/mod.rs"]
mod common;

#[derive(Clone, Copy, Debug)]
enum Position {
    Light,
    Medium,
    Heavy,
    Building,
}

impl Position {
    fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
            Self::Building => "building",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Scatter,
    Interception,
    Splash,
}
impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Scatter => "scatter",
            Self::Interception => "interception",
            Self::Splash => "splash",
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Case {
    position: Position,
    weapon: &'static str,
    distance: f64,
    mode: Mode,
}
impl Case {
    fn calibration(position: Position, weapon: &'static str, distance: f64) -> Self {
        Self {
            position,
            weapon,
            distance,
            mode: Mode::Scatter,
        }
    }
    fn targets(self) -> usize {
        if self.mode != Mode::Interception || matches!(self.position, Position::Building) {
            1
        } else {
            8
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct Sample {
    seed: u64,
    shots: u32,
    hits: u32,
    prop_hits: u32,
    damage: f64,
    eligible_fraction: f64,
    target_motion_m: f64,
    detonations: u32,
    range_m: f64,
}

fn trial(
    case: Case,
    seed: u64,
    seconds: u32,
    protected: bool,
    factors: Option<[f64; 4]>,
    baseline: &Value,
) -> Sample {
    let Case {
        position,
        weapon,
        distance,
        mode,
    } = case;
    let building = matches!(position, Position::Building);
    let target = if building {
        [36.0, 100.0]
    } else {
        [30.0, 100.0]
    };
    let garrison = building && (mode != Mode::Interception || protected);
    let hz = baseline["tick_hz"].as_u64().unwrap() as u32;
    let warm_ticks = 10 * hz;
    let props = if mode == Mode::Interception && !protected {
        json!([])
    } else if building {
        json!([{ "kind": "building", "center": [30, 100], "yaw": 0,
            "half_extents": [4, 4, 4] }])
    } else if mode == Mode::Interception {
        json!([{ "kind": match position { Position::Light => "crate", Position::Medium => "sandbags", _ => "wall" }, "center": [31.5, 100], "yaw": 0,
            "half_extents": [0.25, 20, 0.3] }])
    } else {
        // A ground body guarantees every direction is eligible while the
        // same tiny body in both arms leaves the direct aim above it.
        json!([{ "kind": "rubble", "center": target, "yaw": 0,
            "half_extents": [12, 12, 0.025] }])
    };
    let map = json!({ "size": [distance + 110.0, 200], "fog_cell_m": 8,
        "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": if weapon == "rifle" || mode == Mode::Splash { "rifle" } else { "jeep" },
          "position": [target[0] + distance, target[1]], "yaw": std::f64::consts::PI,
          "engagement": "return_fire_only" },
        { "side": "red", "kind": "recon", "position": target,
          "engagement": "return_fire_only" },
        // A nearby unarmed observer holds identification even at the
        // long garrison ranges; concealment is not a scatter multiplier.
        { "side": "blue", "kind": "recon", "position": [target[0] + 50.0, target[1] + 40.0],
          "engagement": "return_fire_only" }
    ]);
    let mut rules = baseline.clone();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "scout",
        json!({ "hp": 1.0e6, "mounts": [] }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "recon",
        json!({ "body": { "squad": { "slots": vec!["scout"; case.targets()] } } }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "rifle",
        json!({ "body": { "squad": { "slots": vec![if mode == Mode::Splash { "grenadier" } else { "rifleman" }; 8] } } }),
    );
    if mode == Mode::Splash {
        sim::fixtures::patch_catalog(&mut rules, "soldiers", "rifleman", json!({ "mounts": [] }));
    }
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "building",
        json!({ "body": { "hp": 1.0e9 } }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "wall",
        json!({ "body": { "hp": 1.0e9 } }),
    );
    for kind in ["crate", "sandbags"] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "props",
            kind,
            json!({ "body": { "hp": 1.0e9 } }),
        );
    }
    if !building {
        sim::fixtures::patch_catalog(
            &mut rules,
            "props",
            "rubble",
            json!({ "body": { "cover_tier": position.name() } }),
        );
    }
    let mut setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(&map).unwrap(), "rules": rules,
        "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    setup.map = common::physical_map(setup.map, &setup.rules);
    if let Some([light, medium, heavy, building]) = factors {
        let t = &mut setup.rules.cover.tiers;
        (t.light, t.medium, t.heavy) = (light, medium, heavy);
        setup.rules.cover.building_spread_multiplier = building;
    }
    if !protected && mode == Mode::Splash {
        setup.rules.cover.building_fragment_probability_multiplier = 1.0;
    } else if !protected {
        let t = &mut setup.rules.cover.tiers;
        (t.light, t.medium, t.heavy) = (1.0, 1.0, 1.0);
        setup.rules.cover.building_spread_multiplier = 1.0;
    }
    let mut b = Battle::new(&setup, seed);
    if garrison {
        common::order(
            &mut b,
            Side::Red,
            1,
            Order::Garrison {
                units: vec![UnitId(1)],
                building: 0,
            },
        );
    }
    for _ in 0..warm_ticks {
        b.step();
    }
    let target = b.unit(UnitId(1)).unwrap();
    assert!(
        target.mounts.is_empty(),
        "the receiving soldier cannot return fire"
    );
    let positions: Vec<_> = target.members.iter().map(|s| s.position).collect();
    let hp = target.members.iter().map(|s| s.hp).sum::<f64>();
    let ids = target
        .members
        .iter()
        .map(|s| BodyId(s.id))
        .collect::<BTreeSet<_>>();
    let source = b.unit(UnitId(0)).unwrap();
    let range_m = (source.position.xy() - target.members[0].position.xy()).length();
    let origins = if source.members.is_empty() {
        vec![source.position.xy()]
    } else {
        source.members.iter().map(|s| s.position.xy()).collect()
    };
    let covered = target
        .members
        .iter()
        .filter(|s| {
            if garrison {
                sim::garrison::shelter(target, b.rules()) > 0.0
            } else {
                origins.iter().all(|&from| {
                    sim::cover::at(b.world(), b.ground(), &[], b.rules(), s.position.xy(), from)
                        .is_some_and(|t| format!("{t:?}").to_lowercase() == position.name())
                })
            }
        })
        .count();
    let eligible_fraction = covered as f64 / target.members.len() as f64;
    if mode == Mode::Scatter {
        assert_eq!(eligible_fraction, 1.0, "{case:?}: eligible target");
    }
    let burst_point = [
        target.members[0].position.x + 5.0,
        target.members[0].position.y,
        0.0,
    ];
    if mode == Mode::Splash {
        common::order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground { point: burst_point },
            },
        );
    } else {
        common::order(
            &mut b,
            Side::Blue,
            1,
            Order::SetEngagement {
                units: vec![UnitId(0)],
                policy: Engagement::FireAtWill,
            },
        );
    }
    let (mut hits, mut prop_hits, mut detonations) = (0, 0, 0);
    let mut target_motion_m: f64 = 0.0;
    for _ in 0..seconds * b.rules().tick_hz {
        b.step();
        for (member, before) in b.unit(UnitId(1)).unwrap().members.iter().zip(&positions) {
            target_motion_m = target_motion_m.max((member.position.xy() - before.xy()).length());
        }
        for e in b.flight_events() {
            if let FlightEvent::Impact(i) = e {
                detonations += u32::from(i.detonated);
                match i.struck {
                    Struck::Body(id) if ids.contains(&id) => hits += 1,
                    Struck::Prop(_) => prop_hits += 1,
                    _ => {}
                }
            }
        }
    }
    Sample {
        seed,
        hits,
        prop_hits,
        eligible_fraction,
        target_motion_m,
        detonations,
        range_m,
        shots: b
            .unit(UnitId(0))
            .unwrap()
            .mounts
            .iter()
            .map(|m| m.shots)
            .sum(),
        damage: hp
            - b.unit(UnitId(1))
                .unwrap()
                .members
                .iter()
                .map(|s| s.hp)
                .sum::<f64>(),
    }
}

#[derive(Serialize)]
struct Row {
    mode: String,
    targets: usize,
    position: String,
    weapon: String,
    distance_m: f64,
    reduction: f64,
    ci95: [f64; 2],
    neutral: Vec<Sample>,
    protected: Vec<Sample>,
}

fn reduction(a: &[Sample], b: &[Sample]) -> f64 {
    1.0 - b.iter().map(|s| s.damage).sum::<f64>() / a.iter().map(|s| s.damage).sum::<f64>()
}

fn paired_ci(a: &[Sample], b: &[Sample]) -> [f64; 2] {
    let mut rng = Rng::new(7919);
    let mut ratios = Vec::with_capacity(2000);
    for _ in 0..2000 {
        let (mut neutral, mut protected) = (0.0, 0.0);
        for _ in a {
            let k = (rng.unit() * a.len() as f64) as usize;
            neutral += a[k].damage;
            protected += b[k].damage;
        }
        if neutral > 0.0 {
            ratios.push(1.0 - protected / neutral);
        }
    }
    ratios.sort_by(f64::total_cmp);
    assert!(!ratios.is_empty(), "the control must hurt the target");
    [ratios[ratios.len() / 40], ratios[ratios.len() * 39 / 40]]
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let value = |name| {
        args.windows(2)
            .find(|p| p[0] == name)
            .map(|p| p[1].as_str())
    };
    let quick = args.iter().any(|a| a == "--quick");
    let seeds: u64 = value("--seeds").map_or(if quick { 8 } else { 16 }, |s| s.parse().unwrap());
    let seconds: u32 =
        value("--seconds").map_or(if quick { 15 } else { 30 }, |s| s.parse().unwrap());
    let factors = value("--factors").map(|s| {
        s.split(',')
            .map(|v| v.parse::<f64>().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .expect("four factors")
    });
    let baseline = common::scenario_rules();
    let cover = &baseline["cover"];
    let resolved_factors = factors.unwrap_or([
        cover["tiers"]["light"].as_f64().unwrap(),
        cover["tiers"]["medium"].as_f64().unwrap(),
        cover["tiers"]["heavy"].as_f64().unwrap(),
        cover["building_spread_multiplier"].as_f64().unwrap(),
    ]);
    let start = Instant::now();
    let mut rows = Vec::new();
    let mut cases = Vec::new();
    if args.iter().any(|a| a == "--effects") {
        for position in [
            Position::Light,
            Position::Medium,
            Position::Heavy,
            Position::Building,
        ] {
            for weapon in ["rifle", "hmg"] {
                cases.push(Case {
                    position,
                    weapon,
                    distance: 200.0,
                    mode: Mode::Interception,
                });
            }
        }
        for position in [
            Position::Light,
            Position::Medium,
            Position::Heavy,
            Position::Building,
        ] {
            cases.push(Case {
                position,
                weapon: "grenade",
                distance: 60.0,
                mode: Mode::Splash,
            });
        }
    } else {
        for position in [
            Position::Light,
            Position::Medium,
            Position::Heavy,
            Position::Building,
        ] {
            for weapon in if quick {
                vec!["rifle"]
            } else {
                vec!["rifle", "hmg"]
            } {
                if value("--weapon").is_some_and(|w| w != weapon) {
                    continue;
                }
                let distances = if let Some(list) = value("--distances") {
                    list.split(',').map(|s| s.parse().unwrap()).collect()
                } else if quick {
                    vec![200.0]
                } else if weapon == "rifle" {
                    vec![60.0, 200.0, 350.0]
                } else {
                    vec![80.0, 275.0, 500.0]
                };
                for distance in distances {
                    cases.push(Case::calibration(position, weapon, distance));
                }
            }
        }
    }
    cases.retain(|case| value("--weapon").is_none_or(|w| w == case.weapon));
    cases.retain(|case| value("--position").is_none_or(|p| p == case.position.name()));
    let seed_start: u64 = value("--seed-start").map_or(0, |s| s.parse().unwrap());
    for case in cases {
        let Case {
            position,
            weapon,
            distance,
            mode,
        } = case;
        let mut neutral = Vec::new();
        let mut protected = Vec::new();
        for seed in seed_start..seed_start + seeds {
            neutral.push(trial(case, seed, seconds, false, factors, &baseline));
            protected.push(trial(case, seed, seconds, true, factors, &baseline));
        }
        assert!(
            neutral
                .iter()
                .all(|s| s.damage > 0.0 && (s.shots > 0 || s.detonations > 0)),
            "zero-control sample: {case:?} {neutral:?}"
        );
        assert!(
            protected.iter().all(|s| s.shots > 0),
            "protected arm must receive fire: {case:?}"
        );
        if mode == Mode::Splash {
            assert!(
                neutral
                    .iter()
                    .chain(&protected)
                    .all(|s| s.hits == 0 && s.detonations > 0),
                "splash comparison requires real detonations without direct hits"
            );
        }
        let r = reduction(&neutral, &protected);
        let ci = paired_ci(&neutral, &protected);
        println!(
            "{} {} {weapon} {distance:.0}m: {:.1}% [{:.1}, {:.1}] shots {}/{} hits {}/{}",
            mode.name(),
            position.name(),
            r * 100.0,
            ci[0] * 100.0,
            ci[1] * 100.0,
            neutral.iter().map(|s| s.shots).sum::<u32>(),
            protected.iter().map(|s| s.shots).sum::<u32>(),
            neutral.iter().map(|s| s.hits).sum::<u32>(),
            protected.iter().map(|s| s.hits).sum::<u32>()
        );
        rows.push(Row {
            mode: mode.name().into(),
            targets: case.targets(),
            position: position.name().into(),
            weapon: weapon.into(),
            distance_m: distance,
            reduction: r,
            ci95: ci,
            neutral,
            protected,
        });
    }
    eprintln!(
        "{} cells in {:.2}s",
        rows.len(),
        start.elapsed().as_secs_f64()
    );
    if let Some(path) = value("--save") {
        std::fs::write(
            path,
            serde_json::to_string(
                &json!({ "protocol": 3, "seconds": seconds, "seeds": seeds, "seed_start": seed_start, "factors": resolved_factors, "rows": rows }),
            )
            .unwrap()
                + "\n",
        )
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paired_report_detects_cover_without_softening_a_hit() {
        let baseline = common::scenario_rules();
        let total = |protected, factors| {
            (0..8)
                .map(|seed| {
                    trial(
                        Case::calibration(Position::Heavy, "rifle", 200.0),
                        seed,
                        15,
                        protected,
                        factors,
                        &baseline,
                    )
                })
                .fold((0.0, 0u32, 0u32), |(damage, hits, shots), s| {
                    assert_eq!(s.eligible_fraction, 1.0);
                    assert_eq!(
                        s.target_motion_m, 0.0,
                        "calibration targets remain stationary"
                    );
                    (damage + s.damage, hits + s.hits, shots + s.shots)
                })
        };
        let a = total(false, Some([1.15, 1.4, 1.8, 3.0]));
        let b = total(true, Some([1.15, 1.4, 1.8, 3.0]));
        assert!(a.0 > b.0 && b.0 > 0.0, "{a:?} {b:?}");
        assert_eq!(a.0 / a.1 as f64, b.0 / b.1 as f64);
        assert_eq!(a.2, b.2, "cover changes hits, not incoming fire volume");
        assert_eq!(
            a,
            total(true, Some([1.0; 4])),
            "neutral cover keeps the same harm"
        );
    }
    #[test]
    fn stronger_tiers_reduce_more_incoming_hits() {
        let baseline = common::scenario_rules();
        let harm = |position| {
            (0..8)
                .map(|seed| {
                    trial(
                        Case::calibration(position, "rifle", 200.0),
                        seed,
                        10,
                        true,
                        Some([1.15, 1.4, 1.8, 3.0]),
                        &baseline,
                    )
                    .damage
                })
                .sum::<f64>()
        };
        let (light, medium, heavy) = (
            harm(Position::Light),
            harm(Position::Medium),
            harm(Position::Heavy),
        );
        assert!(
            light > medium && medium > heavy && heavy > 0.0,
            "{light} {medium} {heavy}"
        );
    }
}
