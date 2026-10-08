//! The menu reel is a film: its battles may field any units with any stats,
//! but every approved shot stays exactly the same.
//! Each scene of `fixtures/menu-backdrop.json` is replayed natively, as the
//! menu's worker runs it (same rules, catalog set, seed and ticks), and its
//! event log (who fires which round, what each round strikes, who falls, by
//! tick) must equal the log recorded from the approved reel, line for line.
//! Digests can't be the gate: a unit's type index moves when the catalog
//! gains a type, and the reel may swap one unit type for another.
//!
//! The recorded logs are `tests/fixtures/menu-reel/<map>-<encounter>.log`.
//! Re-record one (`BLESS_MENU_REEL=1`) only for a reel the user has approved.
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

/// The units the approved reel was filmed with, and the menu units that
/// play their parts now (same behaviour, a roster look): `(side, filmed,
/// playing)`. A filmed kind not listed plays itself.
const CAST: [(&str, &str, &str); 8] = [
    ("Blue", "test_tank", "menu_us_tank"),
    ("Blue", "test_jeep", "menu_us_jeep"),
    ("Blue", "test_rifle", "menu_rifle"),
    ("Blue", "test_at", "menu_us_at"),
    ("Blue", "test_recon", "menu_us_recon"),
    ("Red", "test_rifle", "menu_rifle"),
    ("Red", "test_at", "menu_eastern_at"),
    ("Red", "test_tank", "menu_eastern_tank"),
];

/// One scene of the backdrop: its saved battle and the shots that film it.
struct Scene {
    map: String,
    encounter: String,
    seed: u64,
    warm_s: f64,
    /// Each shot's seconds and the unit it follows, if any.
    shots: Vec<(f64, Option<u32>)>,
}

impl Scene {
    fn name(&self) -> String {
        format!("{}-{}", self.map, self.encounter)
    }

    /// Each shot's first and last tick, and the unit it follows.
    fn windows(&self, hz: f64) -> Vec<(u64, u64, Option<u32>)> {
        let mut start = self.warm_s;
        self.shots
            .iter()
            .map(|&(seconds, follow)| {
                let window = (
                    (start * hz).round() as u64,
                    ((start + seconds) * hz).round() as u64,
                );
                start += seconds;
                (window.0, window.1, follow)
            })
            .collect()
    }
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
/// event, tick first, in the order the battle made them. The cast comes
/// first (`0 unit <id> <side> <kind>`); then `fire <unit> <weapon>` for each
/// round launched, `hit <shooter> <weapon> <what>` for each round's end on
/// ground, a prop, a hull or a soldier, `fell <unit> soldier <slot>` and
/// `died <unit>`.
fn event_log(scene: &Scene) -> String {
    let map = sim::maps::load(&scene.map).unwrap().definition;
    let encounter = sim::maps::encounter(&scene.map, &scene.encounter).unwrap();
    let rules = menu_rules();
    let end = scene.windows(rules.tick_hz as f64).last().unwrap().1;
    let mut battle = Battle::new(&encounter.on(map, rules), scene.seed);
    let count = (0..)
        .take_while(|&i| battle.unit(UnitId(i)).is_some())
        .count() as u32;
    let mut log = String::new();
    // Which unit each soldier belongs to (a soldier's body is his id).
    let mut soldier_of = BTreeMap::new();
    for i in 0..count {
        let u = battle.unit(UnitId(i)).unwrap();
        let kind = battle.rules().catalog.id(u.kind);
        writeln!(log, "0 unit {i} {:?} {kind}", u.side).unwrap();
        for s in &u.members {
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
    // Each round's shooter and weapon, from its launch.
    let mut rounds: BTreeMap<_, (u32, String)> = BTreeMap::new();
    let mut before = alive(&battle);
    while battle.tick() < end {
        battle.step();
        let tick = battle.tick();
        for e in battle.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let (shooter, weapon) = &rounds[&i.projectile];
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
        // A round is launched at the end of a tick and flies the next.
        for (p, r) in battle.rounds() {
            if let std::collections::btree_map::Entry::Vacant(new) = rounds.entry(p.id) {
                let weapon = battle.arsenal().weapons[r.weapon].id.clone();
                writeln!(log, "{tick} fire {} {weapon}", r.unit.0).unwrap();
                new.insert((r.unit.0, weapon));
            }
        }
    }
    log
}

/// The recorded log with each filmed unit recast as the unit playing it.
fn recast(recorded: &str) -> String {
    recorded
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            if let ["0", "unit", id, side, kind] = words[..] {
                let playing = CAST
                    .iter()
                    .find(|&&(s, filmed, _)| s == side && filmed == kind)
                    .map_or(kind, |&(_, _, playing)| playing);
                format!("0 unit {id} {side} {playing}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect()
}

fn recorded_path(scene: &Scene) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/menu-reel")
        .join(format!("{}.log", scene.name()))
}

/// The scene `map`'s reel plays exactly as recorded, and no shot opens on a
/// unit it follows that has already fallen (the reel would frame where it
/// was, not the unit).
fn plays_as_recorded(map: &str) {
    let scene = scenes()
        .into_iter()
        .find(|s| s.map == map)
        .expect("the scene is in the backdrop");
    let log = event_log(&scene);
    let path = recorded_path(&scene);
    if std::env::var_os("BLESS_MENU_REEL").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &log).unwrap();
    }
    let recorded = recast(&std::fs::read_to_string(&path).unwrap());
    let windows = scene.windows(menu_rules().tick_hz as f64);
    let shot_at = |tick: u64| windows.iter().position(|w| w.0 <= tick && tick < w.1);
    for (n, (want, got)) in recorded.lines().zip(log.lines()).enumerate() {
        if want != got {
            let tick: u64 = want.split(' ').next().unwrap().parse().unwrap();
            panic!(
                "{}: event {n} differs, in shot {:?}: recorded `{want}`, played `{got}`",
                scene.name(),
                shot_at(tick)
            );
        }
    }
    assert_eq!(
        recorded.lines().count(),
        log.lines().count(),
        "{}: the reel plays a different number of events",
        scene.name()
    );
    let fell: BTreeMap<u32, u64> = log
        .lines()
        .filter_map(|line| match line.split(' ').collect::<Vec<_>>()[..] {
            [tick, "died", unit] => Some((unit.parse().unwrap(), tick.parse().unwrap())),
            _ => None,
        })
        .collect();
    for (shot, &(start, _, follow)) in windows.iter().enumerate() {
        if let Some(died) = follow.and_then(|f| fell.get(&f)) {
            assert!(
                *died > start,
                "{}: shot {shot} follows unit {follow:?}, fallen at tick {died}",
                scene.name()
            );
        }
    }
}

#[test]
fn every_backdrop_scene_has_its_recorded_reel() {
    for scene in scenes() {
        assert!(
            recorded_path(&scene).exists(),
            "{} has no recorded reel",
            scene.name()
        );
    }
}

#[test]
fn market_towns_reel_plays_as_recorded() {
    plays_as_recorded("market-town");
}

#[test]
fn paris_corners_reel_plays_as_recorded() {
    plays_as_recorded("paris-corner");
}
