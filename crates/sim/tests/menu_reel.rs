//! The menu reel is a film: its battles may field any units, but every
//! approved shot stays exactly the same (unit-models slice 08). Each scene of
//! `fixtures/menu-backdrop.json` is replayed natively, as the menu's worker
//! runs it, and its event log (who fires what, what each round strikes, who
//! falls, by tick) is held to the log recorded from the approved reel.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use contract::ids::UnitId;
use contract::scenario::Rules;
use serde_json::Value;
use sim::battle::Battle;
use sim::fixtures::{self, CatalogSet};
use sim::flight::{FlightEvent, Struck};
use sim::weapons::VEHICLE_BODY_BASE;

/// One scene of the backdrop: its saved battle and the shots that film it.
struct Scene {
    map: String,
    encounter: String,
    seed: u64,
    warm_s: f64,
    /// Each shot's seconds and the unit it follows, if any.
    shots: Vec<(f64, Option<u32>)>,
}

fn scenes() -> Vec<Scene> {
    let path = fixtures::dir().join("menu-backdrop.json");
    let backdrop: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    backdrop["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| Scene {
            map: s["map"].as_str().unwrap().into(),
            encounter: s["encounter"].as_str().unwrap().into(),
            seed: s["seed"].as_u64().unwrap(),
            warm_s: s["warm_s"].as_f64().unwrap(),
            shots: s["reel"]["shots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|shot| {
                    let follow = shot["follow"].as_u64().map(|f| f as u32);
                    (shot["seconds"].as_f64().unwrap(), follow)
                })
                .collect(),
        })
        .collect()
}

/// The rules the menu's battles run: `game.json` with the menu's catalog.
fn menu_rules() -> Rules {
    let path = fixtures::dir().join("game.json");
    let mut game: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    game["catalog"] = Value::Array(fixtures::catalog_documents(CatalogSet::Menu));
    serde_json::from_value(game).unwrap()
}

/// A scene's battle played to its reel's end, as its event log: one line per
/// event, `tick` first, in the order the battle made them.
fn event_log(scene: &Scene) -> String {
    let map = sim::maps::load(&scene.map).unwrap().definition;
    let encounter = sim::maps::encounter(&scene.map, &scene.encounter).unwrap();
    let rules = menu_rules();
    let hz = rules.tick_hz as f64;
    let end = ((scene.warm_s + scene.shots.iter().map(|s| s.0).sum::<f64>()) * hz).round() as u64;
    let mut battle = Battle::new(&encounter.on(map, rules), scene.seed);
    let count = (0..)
        .take_while(|&i| battle.unit(UnitId(i)).is_some())
        .count() as u32;
    // Which unit each soldier belongs to (soldier bodies are their ids).
    let mut soldier_of = BTreeMap::new();
    for i in 0..count {
        for s in &battle.unit(UnitId(i)).unwrap().members {
            soldier_of.insert(s.id, i);
        }
    }
    let alive = |b: &Battle| -> Vec<(bool, Vec<bool>)> {
        (0..count)
            .map(|i| {
                let u = b.unit(UnitId(i)).unwrap();
                (u.alive(), u.members.iter().map(|s| s.alive()).collect())
            })
            .collect()
    };
    let mut log = String::new();
    let mut rounds: BTreeMap<_, (u32, String)> = BTreeMap::new();
    let mut before = alive(&battle);
    while battle.tick() < end {
        battle.step();
        let tick = battle.tick();
        for e in battle.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let Some((shooter, weapon)) = rounds.get(&i.projectile) else {
                continue;
            };
            let struck = match i.struck {
                Struck::Terrain => "ground".to_string(),
                Struck::Prop(_) => "prop".to_string(),
                Struck::Body(b) if b.0 >= VEHICLE_BODY_BASE => {
                    format!("hull {}", b.0 - VEHICLE_BODY_BASE)
                }
                Struck::Body(b) => format!("soldier of {}", soldier_of[&b.0]),
            };
            writeln!(log, "{tick} hit {shooter} {weapon} {struck}").unwrap();
        }
        let now = alive(&battle);
        for (i, ((was, was_members), (is, is_members))) in before.iter().zip(&now).enumerate() {
            for (slot, (a, b)) in was_members.iter().zip(is_members).enumerate() {
                if *a && !b {
                    writeln!(log, "{tick} fell {i} soldier {slot}").unwrap();
                }
            }
            if *was && !is {
                writeln!(log, "{tick} died {i}").unwrap();
            }
        }
        before = now;
        if std::env::var_os("MENU_REEL_DIGESTS").is_some() && tick % 300 == 0 {
            writeln!(log, "{tick} digest {:016x}", battle.digest()).unwrap();
        }
        for (p, r) in battle.rounds() {
            if !rounds.contains_key(&p.id) {
                let weapon = battle.arsenal().weapons[r.weapon].id.clone();
                writeln!(log, "{tick} fire {} {weapon}", r.unit.0).unwrap();
                rounds.insert(p.id, (r.unit.0, weapon));
            }
        }
    }
    log
}

#[test]
#[ignore = "records the reel's event logs into throwaway/menu-target"]
fn record_the_reel() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../throwaway/menu-target");
    std::fs::create_dir_all(&out).unwrap();
    for scene in scenes() {
        let log = event_log(&scene);
        std::fs::write(out.join(format!("{}-{}.log", scene.map, scene.encounter)), log).unwrap();
    }
}
