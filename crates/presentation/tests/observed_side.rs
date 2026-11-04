//! A presentation is loaded again for each side it is switched to (its
//! units and effects are that side's). What the side already knows is drawn
//! at once: smoke over a building it saw destroyed rises from the
//! publication already taken, without waiting for the battle to move on.
//! The battle is the city-ruins lab's: the camera lab's shelling, blue
//! watching both buildings fall, red behind the slabs seeing neither.
use contract::ids::Side;
use presentation::fielded;
use presentation::presentation::Presentation;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use serde_json::Value;
use sim::battle::Battle;
use sim::publication::{self, Publisher};
use std::path::{Path, PathBuf};

const MAP: &str = "camera-lab";
const ENCOUNTER: &str = "shelling";
/// The lab's seed.
const SEED: u64 = 7;
/// Past the shelling: by now blue knows both buildings destroyed.
const SHELLED_TICKS: u64 = 600;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The world a page observing as `side` loads.
fn world(root: &Path, game: &Value, side: Side) -> WorldInput {
    let kinds = fielded::encounter_kinds(root, MAP, ENCOUNTER).unwrap();
    let f = fielded::fielded(
        root,
        &presentation::static_world::files::rules(root, MAP).unwrap(),
        &kinds,
    )
    .unwrap();
    let physics = &game["physics"];
    WorldInput {
        battle: Some(BattleInput {
            units: f.units,
            effects: f.effects,
            side: if side == Side::Blue { "blue" } else { "red" }.into(),
            tick_hz: game["tick_hz"].as_f64().unwrap(),
            soldier: SoldierBody {
                radius_m: physics["soldier_radius_m"].as_f64().unwrap(),
                height_m: physics["soldier_height_m"].as_f64().unwrap(),
            },
            xray: Xray {
                own: [0.0; 4],
                selected: [0.0; 4],
                ally: [0.0; 4],
            },
            blocked: [0.0; 4],
        }),
        sun_azimuth: 0.0,
        sun_elevation: 0.5,
        buildings: None,
    }
}

fn words(record: &[f32]) -> Vec<u32> {
    record.iter().map(|f| f.to_bits()).collect()
}

/// The smoke sources the side's presentation burns, and the tick its
/// effects last took.
fn smoke(p: &Presentation) -> (u64, f64) {
    let probe: Value = serde_json::from_str(&p.probe_json()).unwrap();
    let e = &probe["effects"];
    (e["sources"].as_u64().unwrap(), e["tick"].as_f64().unwrap())
}

#[test]
fn a_side_switched_back_to_smokes_at_once_over_what_it_saw_destroyed() {
    let root = root();
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let rules = presentation::static_world::files::rules(&root, MAP).unwrap();
    let scenario = sim::maps::scenario(MAP, ENCOUNTER, rules).unwrap();
    let mut battle = Battle::new(&scenario, SEED);
    for _ in 0..SHELLED_TICKS {
        battle.step();
    }
    let mut publisher = Publisher::new();
    let mut p = Presentation::new(
        Some(&publication::layout_json(&battle)),
        battle.rules().tick_hz as f64,
    )
    .unwrap();
    p.load_world(world(&root, &game, Side::Blue), Default::default(), None)
        .unwrap();
    assert!(p
        .push_publication(&words(publisher.publish(&battle, Side::Blue).unwrap()), 0.0)
        .unwrap());
    let (blue, _) = smoke(&p);
    assert!(blue > 0, "blue, having watched the shelling, sees smoke");

    // Observe as red, then as blue again, each publication taken before
    // the side's world is loaded (as a page's arrive), the battle paused.
    for side in [Side::Red, Side::Blue] {
        let name = if side == Side::Blue { "blue" } else { "red" };
        p.invalidate(Some(name)).unwrap();
        battle.step();
        publisher.resync();
        assert!(p
            .push_publication(&words(publisher.publish(&battle, side).unwrap()), 0.0)
            .unwrap());
        p.load_world(world(&root, &game, side), Default::default(), None)
            .unwrap();
        let (sources, tick) = smoke(&p);
        assert_eq!(
            tick,
            battle.tick() as f64,
            "{name}'s effects have taken the publication already received"
        );
        let want = if side == Side::Blue { blue } else { 0 };
        assert_eq!(sources, want, "{name} smokes over what it saw destroyed");
    }
}

/// The wave field's epoch after a frame at `time`; none without a battle.
fn wave_epoch(p: &mut Presentation, time: f64) -> Option<u32> {
    let camera = presentation::view::Camera {
        eye: [200.0, 140.0, 80.0],
        target: [200.0, 200.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 0.5,
        size: [1280, 720],
    };
    p.frame(time, &camera, &presentation::presentation::Ui::default(), 0)
        .unwrap();
    p.waves().map(|w| w.step.epoch)
}

/// The wave field starts over the moment the side switches, before the new
/// side's world loads: no wave the old side saw made lingers for the new.
#[test]
fn a_side_switch_starts_the_wave_field_over_before_the_new_side_loads() {
    let root = root();
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let rules = presentation::static_world::files::rules(&root, MAP).unwrap();
    let scenario = sim::maps::scenario(MAP, ENCOUNTER, rules).unwrap();
    let mut battle = Battle::new(&scenario, SEED);
    battle.step();
    let mut publisher = Publisher::new();
    let mut p = Presentation::new(
        Some(&publication::layout_json(&battle)),
        battle.rules().tick_hz as f64,
    )
    .unwrap();
    p.load_world(world(&root, &game, Side::Blue), Default::default(), None)
        .unwrap();
    p.push_publication(&words(publisher.publish(&battle, Side::Blue).unwrap()), 0.0)
        .unwrap();
    let blue = wave_epoch(&mut p, 1.0).expect("a battle's field");
    assert_eq!(
        wave_epoch(&mut p, 1.1),
        Some(blue),
        "it holds while blue watches"
    );
    p.invalidate(Some("red")).unwrap();
    let switched = wave_epoch(&mut p, 1.2);
    assert!(
        switched.is_some_and(|e| e != blue),
        "{switched:?} after blue's {blue}"
    );
}
