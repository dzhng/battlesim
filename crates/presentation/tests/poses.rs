//! The crate poses units as the browser did: the tank's articulation at
//! fixed inputs and a body's joint worlds sampled from its clip table agree
//! with the browser's values (written by its TypeScript before that was
//! deleted) to the fixture's rounding (1e-6); and on a recorded browser run,
//! every drawn muzzle sits on its drawn model. The run is a sample of
//! publications, not a pin on what it shows: nothing here compares its
//! frames, so the menu reel it was filmed from may change freely.
use std::path::PathBuf;
use std::sync::Arc;

use presentation::effects::MuzzleSource;
use presentation::fielded;
use presentation::seed::side_key;
use presentation::units::articulation::{articulate, rest_locals};
use presentation::units::drawn::{ClipTables, DrawnModels};
use presentation::units::driver::{FeedFallen, FeedFrame, FeedSoldier, FeedUnit, PoseDriver};
use presentation::units::feed::ObservationFeed;
use presentation::units::interpolate::TickInterpolator;
use presentation::units::pose::{Articulation, PoseFrame};
use presentation::units::skin::joint_worlds;
use presentation::world::{Appearance, UnitsSection, Vec3};
use serde_json::Value;
use sim::publication::reader::Reader;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/"
);

fn read(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}{path}")).unwrap()).unwrap()
}

/// The units section regenerated with the shot's kick (`tests/fixtures`).
fn kicked_units() -> Value {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/paris-corner-units.json"
    );
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

/// `actual` matches `wanted`, a value the browser rounded to 1e-6.
fn near(actual: f64, wanted: &Value, what: &str) {
    let wanted = wanted
        .as_f64()
        .unwrap_or_else(|| panic!("{what}: {wanted} is not a number"));
    assert!(
        (actual - wanted).abs() <= 0.5e-6 + 1e-12,
        "{what} is {actual}, not {wanted}"
    );
}

fn near_all(actual: &[f64], wanted: &Value, what: &str) {
    let wanted = wanted.as_array().unwrap();
    assert_eq!(actual.len(), wanted.len(), "{what} length");
    for (a, w) in actual.iter().zip(wanted) {
        near(*a, w, what);
    }
}

/// A clip table written beside the fixture run's manifest: raw little-endian f32.
fn clip_table(name: &str) -> Vec<f32> {
    let bytes = std::fs::read(format!("{FIXTURES}run/paris-corner/units-{name}.u32")).unwrap();
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&b| f32::from_le_bytes(b))
        .collect()
}

#[test]
fn the_drawn_muzzles_sit_on_the_drawn_models() {
    let units: UnitsSection = serde_json::from_value(kicked_units()).unwrap();
    let capture = read("run/paris-corner/capture.json");
    // The battle's effects, resolved from the repository as native resolves them.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workload = &capture["workload"];
    let map = workload["map"].as_str().unwrap();
    let kinds =
        fielded::encounter_kinds(&root, map, workload["encounter"].as_str().unwrap()).unwrap();
    let effects = fielded::fielded(
        &root,
        &presentation::static_world::files::rules(&root, map).unwrap(),
        &kinds,
    )
    .unwrap()
    .effects;
    let tick_hz = capture["tickHz"].as_f64().unwrap();
    let mut reader = Reader::new(capture["layout"].as_str().unwrap()).unwrap();
    let observations: Vec<_> = capture["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let words: Vec<u32> = serde_json::from_value(s["publication"].clone()).unwrap();
            Arc::new(reader.decode(&words).unwrap().expect("a fresh record"))
        })
        .collect();

    let mut driver = PoseDriver::new(&units).unwrap();
    let mut feed = ObservationFeed::new(capture["side"].as_str().unwrap());
    let mut interpolator = TickInterpolator::new(tick_hz);
    let mut pushed = 0;
    let mut last = PoseFrame::default();
    for record in capture["frames"].as_array().unwrap() {
        let tick = record["tick"].as_f64().unwrap();
        while pushed < observations.len() && observations[pushed].tick <= tick {
            interpolator.push(
                observations[pushed].clone(),
                observations[pushed].tick / tick_hz,
            );
            pushed += 1;
        }
        let sample = interpolator
            .frame(record["simTime"].as_f64().unwrap())
            .unwrap();
        last.clone_from(driver.update(feed.frame(sample, &units, &effects), &units, None));
    }
    assert!(
        !last.vehicles.is_empty() && last.soldiers.iter().any(|s| s.active_mount.is_some()),
        "the run's last frame poses a vehicle and an armed soldier"
    );

    // The drawn muzzles sit on the drawn models: the tank's gun and each
    // rifleman's weapon, as the effects ask after them (side keys, the
    // presented side, blue, even).
    let tables: ClipTables = units
        .appearances
        .values()
        .filter_map(|a| match a.as_ref() {
            Appearance::Skinned { table, .. } => Some((table.clone(), clip_table(table).into())),
            Appearance::Articulated { .. } => None,
        })
        .collect();
    let mut drawn = DrawnModels::new(Arc::new(units.clone()), Arc::new(tables), "blue");
    drawn.update(&last);
    let reach = |a: Vec3, b: Vec3| (a[0] - b[0]).hypot(a[1] - b[1]);
    for v in &last.vehicles {
        let muzzle = drawn
            .muzzle(side_key(v.unit, &*v.side == "blue"), 0, None)
            .expect("the gun's muzzle is drawn");
        assert!(
            reach(muzzle, v.position) < 8.0,
            "the gun's muzzle {muzzle:?} is on the tank at {:?}",
            v.position
        );
    }
    for s in last.soldiers.iter().filter(|s| s.active_mount.is_some()) {
        let muzzle = drawn
            .muzzle(
                side_key(s.unit, &*s.side == "blue"),
                s.active_mount.unwrap(),
                Some(s.soldier),
            )
            .expect("his muzzle is drawn");
        assert!(
            reach(muzzle, s.position) < 1.5,
            "soldier {}'s muzzle {muzzle:?} is in his hands at {:?}",
            s.soldier,
            s.position
        );
    }
    assert!(drawn.rotors().is_empty(), "nothing flies");
}

#[test]
fn the_tank_articulates_as_the_browser_s() {
    let expected = read("poses/paris-corner-poses.json");
    let units: UnitsSection = serde_json::from_value(kicked_units()).unwrap();
    let Some(Appearance::Articulated { nodes, rig, .. }) = units
        .appearances
        .get(expected["vehicle"].as_str().unwrap())
        .map(AsRef::as_ref)
    else {
        panic!("the fixture's vehicle is articulated");
    };
    let mut locals = rest_locals(nodes);
    for case in expected["articulations"].as_array().unwrap() {
        let input = &case["input"];
        let f = |k: &str| input[k].as_f64().unwrap();
        let input = Articulation {
            turret_yaw: f("turret_yaw"),
            gun_pitch: f("gun_pitch"),
            recoil: f("recoil"),
            hmg_yaw: f("hmg_yaw"),
            hmg_pitch: f("hmg_pitch"),
            travel_l: f("travel_l"),
            travel_r: f("travel_r"),
            deploy: f("deploy"),
            rotor: f("rotor"),
            rotor_blur: f("rotor_blur"),
            gear_up: 0.0,
        };
        articulate(&mut locals, nodes, rig, &input);
        for ((l, w), node) in locals
            .iter()
            .zip(case["locals"].as_array().unwrap())
            .zip(nodes)
        {
            let what = format!("{input:?} node {}", node.name);
            near_all(&l.t, &w["t"], &what);
            near_all(&l.r, &w["r"], &what);
            near_all(&l.s, &w["s"], &what);
        }
    }
}

/// Metres a joint may stand from the browser's vectors. Those come from its
/// CPU sampler (`poseWorlds`: the raw int16/32767 rotations, an unnormalised
/// slerp, each bind's own scale), while the crate samples as the browser's
/// GPU kernel draws (the float32 clip table, the kernel's normalised slerp,
/// a uniform scale). The two differ by up to 6e-5 m in this fixture; the
/// CPU sampler's math over the same table reproduces the vectors to their
/// 1e-6 rounding.
const SAMPLER_TOLERANCE: f64 = 1e-4;

#[test]
fn a_body_s_joints_sample_as_the_browser_s() {
    let expected = read("poses/paris-corner-poses.json");
    let units: UnitsSection = serde_json::from_value(kicked_units()).unwrap();
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        ..
    }) = units
        .appearances
        .get(expected["body"].as_str().unwrap())
        .map(AsRef::as_ref)
    else {
        panic!("the fixture's body is skinned");
    };
    let table = clip_table(table);
    let mut worlds = Vec::new();
    for case in expected["joints"].as_array().unwrap() {
        let clip = case["clip"].as_str().unwrap();
        let phase = case["phase"].as_f64().unwrap();
        joint_worlds(joints, clips, &table, clip, phase, None, &mut worlds);
        for ((m, w), joint) in worlds
            .iter()
            .zip(case["worlds"].as_array().unwrap())
            .zip(joints)
        {
            for (a, w) in m[12..15].iter().zip(w.as_array().unwrap()) {
                let w = w.as_f64().unwrap();
                assert!(
                    (a - w).abs() <= SAMPLER_TOLERANCE,
                    "{clip} at {phase} joint {} is {a}, not {w}",
                    joint.name
                );
            }
        }
    }
}

/// The fixture never sees anyone fall, so a death is driven by hand: a
/// soldier seen alive plays his fall, then lies; a body first seen fallen
/// lies at once; past the cap the oldest drawn corpse sinks away and goes.
#[test]
fn the_fallen_play_their_death_lie_and_give_way_past_the_cap() {
    let mut units = kicked_units();
    units["pose"]["corpses"]["max"] = 1.into();
    let units: UnitsSection = serde_json::from_value(units).unwrap();
    let mut driver = PoseDriver::new(&units).unwrap();
    let soldier = |id: f64| FeedSoldier {
        id,
        position: [id, 0.0, 0.0],
        ..FeedSoldier::default()
    };
    let squad = |ids: &[f64]| FeedUnit {
        kind: "capture_rifle".into(),
        side: "blue".into(),
        soldiers: ids.iter().map(|&id| soldier(id)).collect(),
        ..FeedUnit::default()
    };
    let fallen = |ids: &[f64]| {
        Arc::new(
            ids.iter()
                .map(|&id| FeedFallen {
                    soldier: id,
                    position: [id, 0.0, 0.0],
                    yaw: 0.0,
                    kind: "capture_rifle".into(),
                    slot: 0,
                    side: "blue".into(),
                })
                .collect::<Vec<_>>(),
        )
    };
    let mut step = |time: f64, ids: &[f64], down: &Arc<Vec<FeedFallen>>| {
        let frame = FeedFrame {
            time,
            units: vec![squad(ids)],
            fallen: down.clone(),
        };
        let poses = driver.update(&frame, &units, None);
        let soldiers: Vec<_> = poses
            .soldiers
            .iter()
            .map(|s| (s.soldier, s.clip.to_string()))
            .collect();
        let corpses: Vec<_> = poses.corpses.iter().map(|c| c.soldier).collect();
        let fading: Vec<_> = poses.fading.iter().map(|f| f.corpse.soldier).collect();
        (soldiers, corpses, fading)
    };
    let none = fallen(&[]);
    assert_eq!(step(0.0, &[1.0, 2.0], &none).0.len(), 2);
    let one = fallen(&[1.0]);
    let (soldiers, corpses, _) = step(0.1, &[2.0], &one);
    assert_eq!(
        soldiers[1],
        (1.0, "death".to_string()),
        "he falls as he stood"
    );
    assert!(corpses.is_empty());
    // The fall runs its clip (1.07 s) and its fade out of the idle.
    let (soldiers, corpses, _) = step(1.5, &[2.0], &one);
    assert_eq!(
        (soldiers.len(), corpses),
        (1, vec![1.0]),
        "his death played out, he lies"
    );
    let both = fallen(&[1.0, 3.0]);
    let (_, corpses, fading) = step(1.6, &[2.0], &both);
    assert_eq!(
        (corpses, fading),
        (vec![3.0], vec![1.0]),
        "the oldest gives way to the cap"
    );
    let (_, corpses, fading) = step(4.7, &[2.0], &both);
    assert_eq!(
        (corpses, fading),
        (vec![3.0], vec![]),
        "and is gone once his fade has run"
    );
}
